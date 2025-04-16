use std::{
    error::Error,
    fmt::Display,
    io::{stdout, Write},
    sync::{
        mpsc::{self, SyncSender},
        Arc, Mutex,
    },
    thread,
};

use chrono::{Duration, Utc};
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
    Rng, SeedableRng,
};
use rayon::iter::{
    IndexedParallelIterator, IntoParallelIterator, IntoParallelRefIterator,
    IntoParallelRefMutIterator, ParallelIterator,
};

const SYMBOLS: [char; 73] = [
    '　', 'Ａ', 'Ｂ', 'Ｃ', 'Ｄ', 'Ｅ', 'Ｆ', 'Ｇ', 'Ｈ', 'Ｉ', 'Ｊ', 'Ｋ', 'Ｌ', 'Ｍ', 'Ｎ', 'Ｏ',
    'Ｐ', 'Ｑ', 'Ｒ', 'Ｓ', 'Ｔ', 'Ｕ', 'Ｖ', 'Ｗ', 'Ｘ', 'Ｙ', 'Ｚ', 'ヲ', 'ァ', 'ィ', 'ゥ', 'ェ',
    'ォ', 'ャ', 'ュ', 'ョ', 'ッ', 'ン', 'ア', 'イ', 'ウ', 'エ', 'オ', 'カ', 'キ', 'ク', 'ケ', 'コ',
    'サ', 'シ', 'ヤ', 'ス', 'ソ', '０', '１', '２', '３', '４', '５', '６', '７', '８', '９', 'テ',
    'ハ', 'フ', 'ノ', 'ホ', 'メ', 'ト', 'チ', 'ニ', 'ツ',
];

#[derive(Parser, Clone)]
struct Args {
    #[arg(long, default_value_t = 0.04)]
    prob_randomize_symbol: f64,
    #[arg(long, default_value_t = 0.007)]
    prob_glow: f64,
    #[arg(long, default_value_t = 0.003)]
    prob_dim: f64,
    #[arg(long, default_value_t = 0.08)]
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
}

#[derive(Clone, Default, PartialEq)]
struct Rune {
    symbol: char,
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

impl Display for Rune {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "\x1b[38;2;{};{};{}m{}",
            self.r(),
            self.g(),
            self.b(),
            self.symbol
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
    rng: SmallRng,
    uniform_symbol_index: Uniform<usize>,
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
    fn new(args: &Args) -> Self {
        Rain {
            width: 0,
            height: 0,
            runes: vec![],
            rune_rngs: vec![],
            drops: vec![],
            rng: SmallRng::from_os_rng(),
            uniform_symbol_index: Uniform::new(0, SYMBOLS.len()).unwrap(),
            bern_randomize_symbol: Bernoulli::new(args.prob_randomize_symbol).unwrap(),
            bern_glow: Bernoulli::new(args.prob_glow).unwrap(),
            bern_dim: Bernoulli::new(args.prob_dim).unwrap(),
            bern_drop: Bernoulli::new(args.prob_drop).unwrap(),
            bern_decay: Bernoulli::new(args.prob_decay).unwrap(),
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
    }

    fn update(&mut self, args: &Args, tx: &SyncSender<Vec<Vec<Rune>>>) {
        self.drops = self
            .drops
            .clone()
            .into_par_iter()
            .filter(|drop| (drop.y as u32).saturating_sub(drop.len) < self.height as u32)
            .collect();

        if self.drops.len() < self.runes.len() && self.bern_drop.sample(&mut self.rng) {
            self.drops.push(Drop {
                x: self.rng.random_range(0..self.width),
                y: 0,
                len: self.rng.random_range(args.min_drop_len..=args.max_drop_len) as u32
                    + args.drop_space_len as u32,
                fall_int: self
                    .rng
                    .random_range(args.min_drop_fall_int..=args.max_drop_fall_int),
                since_update: 0,
            })
        }

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
                            rune.symbol = SYMBOLS[self.uniform_symbol_index.sample(rng)];
                        }
                        if self.bern_glow.sample(rng) {
                            rune.color = (args.glow_value as u32) << 8;
                        }
                        if self.bern_dim.sample(rng) {
                            rune.color = (args.dim_value as u32) << 8;
                        }
                        if rune.color > 0 && self.bern_decay.sample(rng) {
                            rune.color =
                                (((rune.color >> 8) as f64 * args.decay_scalar) as u32) << 8;
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
                    let visible_len = drop_len - args.drop_space_len as u32;
                    if drop_index < visible_len - 1 {
                        rune.color = (args.dim_value as u32
                            + ((0xff - args.dim_value) as f64
                                * (drop_index as f64 * args.drop_segments / visible_len as f64)
                                    .floor()
                                / args.drop_segments) as u32)
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

fn main() -> Result<(), Box<dyn Error>> {
    enable_raw_mode()?;

    let args = Args::parse();
    let mut rain = Rain::new(&args);

    let (width, height) = terminal::size()?;
    rain.set_size(width / 2, height);

    let rain = Arc::new(Mutex::new(rain));

    execute!(
        stdout(),
        Hide,
        SetBackgroundColor(Color::Rgb { r: 0, g: 0, b: 0 }),
        Clear(ClearType::All)
    )?;

    let stop = Arc::new(Mutex::new(false));

    let (tx, rx) = mpsc::sync_channel(args.channel_size);

    let update_handle = {
        let args = args.clone();
        let stop = Arc::clone(&stop);
        let rain = Arc::clone(&rain);
        thread::spawn(move || {
            let target_td = if args.fps == 0 {
                Duration::zero()
            } else {
                Duration::seconds(1) / (args.fps as i32)
            };
            let mut last_t = Utc::now();

            loop {
                if *stop.lock().unwrap() {
                    break;
                }

                if !target_td.is_zero() {
                    let td = Utc::now() - last_t;
                    if td < target_td {
                        thread::sleep((target_td - td).to_std().unwrap());
                        continue;
                    }
                    last_t = Utc::now() - (td - target_td);
                }

                rain.lock().unwrap().update(&args, &tx);
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

            if args.debug_clear_frame {
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
