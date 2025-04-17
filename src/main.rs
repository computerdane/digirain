use std::{
    error::Error,
    fmt::Display,
    io::{stdout, Write},
    sync::{
        mpsc::{self, SyncSender},
        Arc, LazyLock, Mutex,
    },
    thread,
};

use chrono::{Duration, TimeDelta, Utc};
use clap::Parser;
use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    style::{Attribute, Color, SetAttribute, SetBackgroundColor},
    terminal::{self, disable_raw_mode, enable_raw_mode, Clear, ClearType},
};
use rand::{
    distr::{Bernoulli, Distribution, Uniform},
    rngs::SmallRng,
    SeedableRng,
};
use rayon::iter::{
    IndexedParallelIterator, IntoParallelIterator, IntoParallelRefIterator,
    IntoParallelRefMutIterator, ParallelIterator,
};

const SPECIAL_SYMBOLS: [char; 73] = [
    '　', 'Ａ', 'Ｂ', 'Ｃ', 'Ｄ', 'Ｅ', 'Ｆ', 'Ｇ', 'Ｈ', 'Ｉ', 'Ｊ', 'Ｋ', 'Ｌ', 'Ｍ', 'Ｎ', 'Ｏ',
    'Ｐ', 'Ｑ', 'Ｒ', 'Ｓ', 'Ｔ', 'Ｕ', 'Ｖ', 'Ｗ', 'Ｘ', 'Ｙ', 'Ｚ', 'ヲ', 'ァ', 'ィ', 'ゥ', 'ェ',
    'ォ', 'ャ', 'ュ', 'ョ', 'ッ', 'ン', 'ア', 'イ', 'ウ', 'エ', 'オ', 'カ', 'キ', 'ク', 'ケ', 'コ',
    'サ', 'シ', 'ヤ', 'ス', 'ソ', '０', '１', '２', '３', '４', '５', '６', '７', '８', '９', 'テ',
    'ハ', 'フ', 'ノ', 'ホ', 'メ', 'ト', 'チ', 'ニ', 'ツ',
];

const BASIC_SYMBOLS: [char; 37] = [
    ' ', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R',
    'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9',
];

#[derive(Parser, Clone)]
struct Args {
    #[arg(long, default_value_t = 0.04)]
    prob_randomize_symbol: f64,
    #[arg(long, default_value_t = 0.007)]
    prob_glow: f64,
    #[arg(long, default_value_t = 0.003)]
    prob_dim: f64,
    #[arg(long, default_value_t = 0.002)]
    prob_drop: f64,
    #[arg(long, default_value_t = 0.16)]
    prob_decay: f64,

    #[arg(long, default_value_t = 0x6e)]
    glow_value: u8,
    #[arg(long, default_value_t = 0x66)]
    dim_value: u8,

    #[arg(long, default_value_t = 0.9)]
    decay_scalar: f64,

    #[arg(long, default_value_t = 30)]
    min_drop_len: u16,
    #[arg(long, default_value_t = 40)]
    max_drop_len: u16,

    #[arg(long, default_value_t = 10)]
    drop_space_len: u16,

    #[arg(long, default_value_t = 6.0)]
    drop_segments: f64,

    #[arg(long, default_value_t = 1)]
    min_drop_fall_int: u16,
    #[arg(long, default_value_t = 3)]
    max_drop_fall_int: u16,

    #[arg(long, default_value_t = 60)]
    fps: u16,

    #[arg(long, default_value_t = 1)]
    channel_size: usize,

    #[arg(long, default_value_t = false)]
    debug_clear_frame: bool,

    #[arg(long, default_value_t = 10)]
    fps_step: u16,

    #[arg(long, default_value_t = false)]
    basic: bool,
}

static ARGS: LazyLock<Args> = LazyLock::new(|| Args::parse());

#[derive(Clone, PartialEq)]
struct Rune {
    symbol_index: usize,
    color: u32,
}

impl Rune {
    fn r(&self) -> u8 {
        ((self.color >> 16) & 0xff) as u8
    }

    fn g(&self) -> u8 {
        ((self.color >> 8) & 0xff) as u8
    }

    fn b(&self) -> u8 {
        (self.color & 0xff) as u8
    }
}

impl Default for Rune {
    fn default() -> Self {
        Rune {
            symbol_index: 0,
            color: 0,
        }
    }
}

impl Display for Rune {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "\x1b[38;2;{};{};{}m{}{}",
            self.r(),
            self.g(),
            self.b(),
            if ARGS.basic {
                BASIC_SYMBOLS[self.symbol_index]
            } else {
                SPECIAL_SYMBOLS[self.symbol_index]
            },
            if ARGS.basic { " " } else { "" }
        )
    }
}

#[derive(Clone)]
struct Drop {
    x: u16,
    y: u16,
    len: u32,
    fall_int: u16,
    since_update: u16,
}

struct Rain {
    width: u16,
    height: u16,
    runes: Vec<Vec<Rune>>,
    rune_rngs: Vec<Vec<SmallRng>>,
    drops: Vec<Drop>,
    drop_rngs: Vec<SmallRng>,
    uniform_symbol_index: Uniform<usize>,
    uniform_x: Uniform<u16>,
    uniform_drop_len: Uniform<u16>,
    uniform_drop_fall_int: Uniform<u16>,
    bern_randomize_symbol: Bernoulli,
    bern_glow: Bernoulli,
    bern_dim: Bernoulli,
    bern_drop: Bernoulli,
    bern_decay: Bernoulli,
}

impl Display for Rain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("\x1b[1;1H")?;
        write!(
            f,
            "{}",
            self.runes
                .par_iter()
                .flat_map(|row| row
                    .par_iter()
                    .map(|rune| rune.to_string())
                    .collect::<Vec<String>>())
                .collect::<Vec<String>>()
                .join("")
        )
    }
}

impl Rain {
    fn new() -> Self {
        Rain {
            width: 0,
            height: 0,
            runes: vec![],
            rune_rngs: vec![],
            drops: vec![],
            drop_rngs: vec![],
            uniform_symbol_index: Uniform::new(
                0,
                if ARGS.basic {
                    BASIC_SYMBOLS.len()
                } else {
                    SPECIAL_SYMBOLS.len()
                },
            )
            .unwrap(),
            uniform_x: Uniform::new(0, 1).unwrap(),
            uniform_drop_len: Uniform::new(ARGS.min_drop_len, ARGS.max_drop_len + 1).unwrap(),
            uniform_drop_fall_int: Uniform::new(ARGS.min_drop_fall_int, ARGS.max_drop_fall_int + 1)
                .unwrap(),
            bern_randomize_symbol: Bernoulli::new(ARGS.prob_randomize_symbol).unwrap(),
            bern_glow: Bernoulli::new(ARGS.prob_glow).unwrap(),
            bern_dim: Bernoulli::new(ARGS.prob_dim).unwrap(),
            bern_drop: Bernoulli::new(ARGS.prob_drop).unwrap(),
            bern_decay: Bernoulli::new(ARGS.prob_decay).unwrap(),
        }
    }

    fn set_size(&mut self, width: u16, height: u16) {
        self.width = width;
        self.height = height;
        self.runes.resize_with(self.height as usize, || vec![]);
        self.runes
            .par_iter_mut()
            .for_each(|row| row.resize_with(self.width as usize, Rune::default));
        self.rune_rngs.resize_with(self.height as usize, || vec![]);
        self.rune_rngs
            .par_iter_mut()
            .for_each(|row| row.resize_with(self.width as usize, SmallRng::from_os_rng));
        self.drops = self
            .drops
            .clone()
            .into_par_iter()
            .filter(|drop| drop.x < self.width)
            .collect();
        self.drop_rngs
            .resize_with(self.width as usize, SmallRng::from_os_rng);
        self.uniform_x = Uniform::new(0, self.width).unwrap();
    }

    fn update(&mut self, tx: &SyncSender<Vec<Vec<Rune>>>) {
        self.drops = self
            .drops
            .clone()
            .into_par_iter()
            .filter(|drop| (drop.y as u32).saturating_sub(drop.len) < self.height as u32)
            .collect();

        self.drops.extend(
            self.drop_rngs
                .iter_mut()
                .filter_map(|rng| {
                    if self.bern_drop.sample(rng) {
                        Some(Drop {
                            x: self.uniform_x.sample(rng),
                            y: 0,
                            len: self.uniform_drop_len.sample(rng) as u32
                                + ARGS.drop_space_len as u32,
                            fall_int: self.uniform_drop_fall_int.sample(rng),
                            since_update: 0,
                        })
                    } else {
                        None
                    }
                })
                .collect::<Vec<Drop>>(),
        );

        self.drops.par_iter_mut().for_each(|drop| {
            if drop.since_update >= drop.fall_int {
                drop.y += 1;
                drop.since_update = 0;
            } else {
                drop.since_update += 1;
            }
        });

        self.runes
            .par_iter_mut()
            .zip(self.rune_rngs.par_iter_mut())
            .for_each(|(row, rngs)| {
                row.par_iter_mut()
                    .zip(rngs.par_iter_mut())
                    .for_each(|(rune, rng)| {
                        if rune.color != 0 && self.bern_randomize_symbol.sample(rng) {
                            rune.symbol_index = self.uniform_symbol_index.sample(rng);
                        }
                        if self.bern_glow.sample(rng) {
                            rune.color = (ARGS.glow_value as u32) << 8;
                        }
                        if self.bern_dim.sample(rng) {
                            rune.color = (ARGS.dim_value as u32) << 8;
                        }
                        if rune.color > 0 && self.bern_decay.sample(rng) {
                            rune.color =
                                (((rune.color >> 8) as f64 * ARGS.decay_scalar) as u32) << 8;
                        }
                    })
            });

        for drop in &self.drops {
            let (skip, take) = if drop.y as u32 > drop.len {
                (drop.y as usize - drop.len as usize, drop.len as usize)
            } else {
                (0, drop.y as usize)
            };
            self.runes
                .par_iter_mut()
                .skip(skip)
                .take(take)
                .enumerate()
                .for_each(|(y, row)| {
                    let rune = &mut row[drop.x as usize];
                    let drop_index = drop.len - take as u32 + y as u32;
                    let drop_len = drop.len;
                    let visible_len = drop_len - ARGS.drop_space_len as u32;
                    if drop_index < visible_len - 1 {
                        rune.color = (ARGS.dim_value as u32
                            + ((0xff - ARGS.dim_value) as f64
                                * (drop_index as f64 * ARGS.drop_segments / visible_len as f64)
                                    .floor()
                                / ARGS.drop_segments) as u32)
                            << 8;
                    } else if drop_index == visible_len - 1 {
                        rune.color = 0x00ff00;
                    } else if drop_index == visible_len {
                        rune.color = 0xffffff;
                    } else {
                        rune.color = 0;
                    }
                })
        }

        tx.try_send(self.runes.clone()).unwrap_or_default();
    }
}

fn get_target_td(fps: u16) -> TimeDelta {
    if fps == 0 {
        Duration::zero()
    } else {
        Duration::seconds(1) / (fps as i32)
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    enable_raw_mode()?;

    let mut rain = Rain::new();

    let (width, height) = terminal::size()?;
    rain.set_size(width / 2, height);

    let rain = Arc::new(Mutex::new(rain));

    execute!(
        stdout(),
        Hide,
        SetBackgroundColor(Color::Rgb { r: 0, g: 0, b: 0 }),
        Clear(ClearType::All)
    )?;

    let mut fps = ARGS.fps;

    let stop = Arc::new(Mutex::new(false));
    let target_td = Arc::new(Mutex::new(get_target_td(fps)));

    let (tx, rx) = mpsc::sync_channel(ARGS.channel_size);

    let update_handle = {
        let stop = Arc::clone(&stop);
        let target_td = Arc::clone(&target_td);
        let rain = Arc::clone(&rain);
        thread::spawn(move || {
            let mut last_t = Utc::now();

            loop {
                if *stop.lock().unwrap() {
                    break;
                }

                let target_td = *target_td.lock().unwrap();

                if !target_td.is_zero() {
                    let td = Utc::now() - last_t;
                    if td < target_td {
                        thread::sleep((target_td - td).to_std().unwrap());
                        continue;
                    }
                    last_t = Utc::now() - (td - target_td);
                }

                rain.lock().unwrap().update(&tx);
            }
        })
    };

    let event_handle = {
        let stop = Arc::clone(&stop);
        let rain = Arc::clone(&rain);
        thread::spawn(move || loop {
            if *stop.lock().unwrap() {
                break;
            }

            if event::poll(std::time::Duration::from_secs(100)).unwrap() {
                match event::read().unwrap() {
                    Event::Key(key_event) => match (key_event.modifiers, key_event.code) {
                        (_, KeyCode::Char('q'))
                        | (_, KeyCode::Esc)
                        | (KeyModifiers::CONTROL, KeyCode::Char('c')) => {
                            *stop.lock().unwrap() = true
                        }
                        (_, KeyCode::Right) => {
                            fps = fps.saturating_add(ARGS.fps_step);
                            *target_td.lock().unwrap() = get_target_td(fps);
                        }
                        (_, KeyCode::Left) => {
                            fps = fps.saturating_sub(ARGS.fps_step);
                            if fps == 0 {
                                fps = 1;
                            }
                            *target_td.lock().unwrap() = get_target_td(fps);
                        }
                        _ => (),
                    },
                    Event::Resize(width, height) => {
                        rain.lock().unwrap().set_size(width / 2, height);
                    }
                    _ => (),
                }
            }
        })
    };

    let mut w = stdout().lock();
    let mut runes_prev = vec![];
    loop {
        if *stop.lock().unwrap() {
            break;
        }

        if let Ok(runes) = rx.recv() {
            let redraw = runes_prev.is_empty()
                || runes_prev.len() != runes.len()
                || runes_prev.first().unwrap_or(&vec![]).len()
                    != runes.first().unwrap_or(&vec![]).len();

            if redraw {
                runes_prev = runes.clone();
            }

            if ARGS.debug_clear_frame {
                execute!(
                    w,
                    SetBackgroundColor(Color::Rgb {
                        r: 0x66,
                        g: 0x66,
                        b: 0x66
                    }),
                    Clear(ClearType::All),
                    SetBackgroundColor(Color::Rgb { r: 0, g: 0, b: 0 }),
                )?;
            }

            write!(
                w,
                "{}",
                runes
                    .par_iter()
                    .zip(&runes_prev)
                    .enumerate()
                    .flat_map(|(y, (row, row_prev))| row
                        .par_iter()
                        .zip(row_prev)
                        .enumerate()
                        .filter_map(|(x, (rune, rune_prev))| {
                            if redraw || rune != rune_prev {
                                Some((x, rune))
                            } else {
                                None
                            }
                        })
                        .fold_with((String::new(), 0), |(s, last_x), (x, rune)| {
                            if last_x != 0 && last_x == x - 1 {
                                return (format!("{s}{rune}"), x);
                            }
                            (format!("{s}\x1b[{};{}H{}", y + 1, (x * 2) + 1, rune), x)
                        })
                        .map(|(s, _)| s)
                        .collect::<Vec<String>>())
                    .collect::<Vec<String>>()
                    .join("")
            )?;
            w.flush()?;

            runes_prev = runes;
        } else {
            break;
        }
    }

    update_handle.join().unwrap();
    event_handle.join().unwrap();

    execute!(
        stdout(),
        SetAttribute(Attribute::Reset),
        Clear(ClearType::All),
        MoveTo(0, 0),
        Show
    )?;
    disable_raw_mode()?;

    Ok(())
}
