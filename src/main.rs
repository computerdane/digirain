use std::fmt::Display;

use chrono::{Duration, TimeDelta, Utc};
use clap::Parser;
use crossterm::terminal;
use rand::{
    distr::{Bernoulli, Distribution},
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

#[derive(Parser)]
struct Args {
    #[arg(long, default_value_t = 0.04)]
    prob_randomize_symbol: f64,
    #[arg(long, default_value_t = 0.007)]
    prob_glow: f64,
    #[arg(long, default_value_t = 0.003)]
    prob_dim: f64,
    #[arg(long, default_value_t = 0.08)]
    prob_drop: f64,

    #[arg(long, default_value_t = 0x88)]
    glow_value: u8,
    #[arg(long, default_value_t = 0x66)]
    dim_value: u8,

    #[arg(long, default_value_t = 5)]
    decay_int: u16,

    #[arg(long, default_value_t = 0.9)]
    decay_scalar: f64,

    #[arg(long, default_value_t = 30)]
    min_drop_len: u16,
    #[arg(long, default_value_t = 40)]
    max_drop_len: u16,

    #[arg(long, default_value_t = 10)]
    drop_space_len: u16,

    #[arg(long, default_value_t = 1)]
    min_drop_fall_int: u16,
    #[arg(long, default_value_t = 3)]
    max_drop_fall_int: u16,

    #[arg(long, default_value_t = 60)]
    fps_cap: u16,
}

struct Rune {
    symbol_index: usize,
    r: u8,
    g: u8,
    b: u8,
    since_decay: u16,
    drop_index: u32,
    drop_len: u32,
    rng: SmallRng,
}

impl Display for Rune {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "\x1b[38;2;{};{};{}m{}",
            self.r, self.g, self.b, SYMBOLS[self.symbol_index]
        )
    }
}

impl Rune {
    fn new() -> Self {
        let mut rune = Rune {
            symbol_index: 0,
            r: 0,
            g: 0,
            b: 0,
            since_decay: 0,
            drop_index: 0,
            drop_len: 0,
            rng: SmallRng::from_os_rng(),
        };
        rune.randomize_symbol();
        rune
    }

    fn randomize_symbol(&mut self) {
        self.symbol_index = self.rng.random_range(0..SYMBOLS.len())
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
    drops: Vec<Drop>,
    rng: SmallRng,
    bern_randomize_symbol: Bernoulli,
    bern_glow: Bernoulli,
    bern_dim: Bernoulli,
    bern_drop: Bernoulli,
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
            drops: vec![],
            rng: SmallRng::from_os_rng(),
            bern_randomize_symbol: Bernoulli::new(args.prob_randomize_symbol).unwrap(),
            bern_glow: Bernoulli::new(args.prob_glow).unwrap(),
            bern_dim: Bernoulli::new(args.prob_dim).unwrap(),
            bern_drop: Bernoulli::new(args.prob_drop).unwrap(),
        }
    }

    fn set_size(&mut self, width: u16, height: u16) {
        self.width = width;
        self.height = height;
        self.runes.resize_with(self.height as usize, || vec![]);
        self.runes
            .par_iter_mut()
            .for_each(|row| row.resize_with(self.width as usize, Rune::new));
    }

    fn update(&mut self, args: &Args) {
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
                    rune.drop_index = drop.len - take as u32 + y as u32;
                    rune.drop_len = drop.len;
                })
        }

        self.runes.par_iter_mut().for_each(|row| {
            row.par_iter_mut().for_each(|rune| {
                if self.bern_randomize_symbol.sample(&mut rune.rng) {
                    rune.randomize_symbol();
                }
                if rune.drop_index > 0 {
                    let visible_len = rune.drop_len - args.drop_space_len as u32;
                    if rune.drop_index < visible_len - 1 {
                        rune.g = args.dim_value
                            + ((0xff - args.dim_value) as f64 * rune.drop_index as f64
                                / visible_len as f64) as u8;
                    } else if rune.drop_index == visible_len - 1 {
                        rune.r = 0;
                        rune.g = 0xff;
                        rune.b = 0;
                    } else if rune.drop_index == visible_len {
                        rune.r = 0xff;
                        rune.g = 0xff;
                        rune.b = 0xff;
                    } else if rune.drop_index == rune.drop_len - 1 {
                        rune.r = 0;
                        rune.g = 0;
                        rune.b = 0;
                    }
                } else {
                    if self.bern_glow.sample(&mut rune.rng) {
                        rune.g = args.glow_value;
                    }
                    if self.bern_dim.sample(&mut rune.rng) {
                        rune.g = args.dim_value;
                    }
                    if rune.g > 0 {
                        if rune.since_decay >= args.decay_int {
                            rune.g = (rune.g as f64 * args.decay_scalar) as u8;
                            rune.since_decay = 0;
                        } else {
                            rune.since_decay += 1;
                        }
                    }
                }
            })
        })
    }
}

fn main() {
    let args = Args::parse();
    let mut rain = Rain::new(&args);

    let (width, height) = terminal::size().unwrap();
    rain.set_size(width / 2, height);

    print!("\x1b[?25l");
    print!("\x1b[48;2;0;0;0m");

    let target_td = if args.fps_cap == 0 {
        TimeDelta::zero()
    } else {
        Duration::seconds(1) / (args.fps_cap as i32)
    };
    let mut last_t = Utc::now();

    loop {
        if Utc::now() - last_t < target_td {
            continue;
        }
        last_t = Utc::now();

        rain.update(&args);
        print!("{rain}");
    }
}
