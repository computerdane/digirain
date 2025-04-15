use std::{
    cmp::{max, min},
    fmt::Display,
    process,
    sync::{Arc, Mutex},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use rand::{rngs::ThreadRng, Rng};
use signal_hook::{
    consts::{SIGINT, SIGTERM, SIGWINCH},
    iterator::Signals,
};
use termion::terminal_size;

// const SYMBOLS: [char; 75] = [
//     ' ', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R',
//     'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z', 'ｦ', 'ｧ', 'ｨ', 'ｩ', 'ｪ', 'ｫ', 'ｬ', 'ｭ', 'ｮ', 'ｯ', 'ﾝ',
//     'ｱ', 'ｲ', 'ｳ', 'ｴ', 'ｵ', 'ｶ', 'ｷ', 'ｸ', 'ｹ', 'ｺ', 'ｻ', 'ｼ', 'ﾔ', 'ｽ', 'ｿ', '0', '1', '2', '3',
//     'Ɛ', '4', '5', '6', '7', '8', '9', 'ρ', 'ﾃ', 'ﾊ', 'ﾌ', 'ﾉ', 'ﾎ', 'ﾒ', 'ﾄ', 'ﾁ', 'ﾆ', 'ﾂ',
// ];

const SYMBOLS: [char; 73] = [
    '　', 'Ａ', 'Ｂ', 'Ｃ', 'Ｄ', 'Ｅ', 'Ｆ', 'Ｇ', 'Ｈ', 'Ｉ', 'Ｊ', 'Ｋ', 'Ｌ', 'Ｍ', 'Ｎ', 'Ｏ',
    'Ｐ', 'Ｑ', 'Ｒ', 'Ｓ', 'Ｔ', 'Ｕ', 'Ｖ', 'Ｗ', 'Ｘ', 'Ｙ', 'Ｚ', 'ヲ', 'ァ', 'ィ', 'ゥ', 'ェ',
    'ォ', 'ャ', 'ュ', 'ョ', 'ッ', 'ン', 'ア', 'イ', 'ウ', 'エ', 'オ', 'カ', 'キ', 'ク', 'ケ', 'コ',
    'サ', 'シ', 'ヤ', 'ス', 'ソ', '０', '１', '２', '３', '４', '５', '６', '７', '８', '９', 'テ',
    'ハ', 'フ', 'ノ', 'ホ', 'メ', 'ト', 'チ', 'ニ', 'ツ',
];

fn random_symbol(rng: &mut ThreadRng) -> char {
    let random_index = rng.random_range(0..SYMBOLS.len());
    SYMBOLS[random_index]
}

fn current_time_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
}

fn clamp<T: Ord>(value: T, min_value: T, max_value: T) -> T {
    max(min(value, max_value), min_value)
}

fn clamp_min_zero<T: Ord + Default>(value: T, len: T) -> T {
    clamp(value, T::default(), len)
}

struct Line {
    row: i32,
    col: i32,
    len: i32,
    update_interval: u128,
    last_updated_at: u128,
}

#[derive(Default, Clone)]
struct Color {
    r: u8,
    g: u8,
    b: u8,
}

impl Color {
    fn to_ansi256(&self) -> u8 {
        if self.r == self.g && self.g == self.b {
            if self.r < 8 {
                16
            } else if self.r > 248 {
                231
            } else {
                ((self.r as u16 - 8) / 10) as u8 + 232
            }
        } else {
            let r = (self.r as u16 * 5 / 255) as u8;
            let g = (self.g as u16 * 5 / 255) as u8;
            let b = (self.b as u16 * 5 / 255) as u8;
            16 + 36 * r + 6 * g + b
        }
    }
}

impl PartialEq for Color {
    fn eq(&self, other: &Self) -> bool {
        self.r == other.r && self.g == other.g && self.b == other.b
    }
}

#[derive(Clone)]
struct Drop {
    char: char,
    color: Color,
}

impl Drop {
    fn new() -> Self {
        Drop {
            char: ' ',
            color: Color::default(),
        }
    }
}

impl PartialEq for Drop {
    fn eq(&self, other: &Self) -> bool {
        self.char == other.char && self.color == other.color
    }
}

impl Display for Drop {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "\x1b[38;5;{}m{}", self.color.to_ansi256(), self.char)
    }
}

#[derive(Default)]
struct Rain {
    width: usize,
    height: usize,
    prev_frame: Box<Vec<Vec<Drop>>>,
    next_frame: Box<Vec<Vec<Drop>>>,
    lines: Vec<Line>,
}

impl Rain {
    fn update_frame_size(&mut self) {
        let (width, height) = terminal_size().expect("Failed to get terminal size");
        (self.width, self.height) = ((width / 2) as usize, (height) as usize);
        self.prev_frame = Box::new(vec![vec![Drop::new(); self.width]; self.height]);
        self.next_frame = Box::new(vec![vec![Drop::new(); self.width]; self.height]);
    }

    fn clear(&self) {
        print!(
            "{}",
            vec![SYMBOLS[0].to_string().repeat(self.width); self.height].join("\n")
        );
    }

    fn render(&mut self) {
        let mut delta = String::new();
        for row in 0..self.height {
            for col in 0..self.width {
                if self.next_frame[row][col] != self.prev_frame[row][col] {
                    if delta.is_empty() {
                        // Move the cursor to (row, col) and print the updated character
                        delta.push_str(&format!(
                            "\x1b[{};{}H{}",
                            row + 1,
                            (col * 2) + 1,
                            self.next_frame[row][col],
                        ));
                    } else {
                        delta.push_str(&self.next_frame[row][col].to_string());
                    }
                } else {
                    print!("{delta}");
                    delta.clear();
                }
            }
        }
        print!("{delta}");
        self.prev_frame = self.next_frame.clone();
    }

    fn update_background_noise(&mut self) {
        let mut rng = rand::rng();
        for row in 0..self.height {
            for col in 0..self.width {
                let drop = &mut self.next_frame[row][col];

                if rng.random_range(0..100) < 4 {
                    drop.char = random_symbol(&mut rng);
                }

                let r = rng.random_range(0..1000);
                if r < 10 {
                    drop.color.g = 0x66;
                }
                if r < 7 {
                    drop.color.g = 0x88;
                }

                drop.color.r = 0;
                drop.color.b = 0;
            }
        }
    }

    fn update_lines(&mut self, now: u128) {
        let (w, h) = (self.width as i32, self.height as i32);

        for line in &mut self.lines {
            if now - line.last_updated_at > line.update_interval {
                line.last_updated_at = now;
                line.row += 1;
            }
        }

        let mut i = 0;
        while i < self.lines.len() {
            if self.lines[i].row - self.lines[i].len > h {
                self.lines.remove(i);
            } else {
                i += 1;
            }
        }

        for line in &self.lines {
            let col = clamp_min_zero(line.col, w) as usize;
            for row in (clamp_min_zero(line.row - line.len, h)..clamp_min_zero(line.row, h)).rev() {
                self.next_frame[row as usize][col].color = Color {
                    r: 0,
                    g: 0xff - ((line.row - row) * 5) as u8,
                    b: 0,
                };
            }
            for row in clamp_min_zero(line.row + 1, h)..clamp_min_zero(line.row + 10, h) {
                self.next_frame[row as usize][col].color = Color::default();
            }
            if 0 <= line.row && line.row < h {
                self.next_frame[line.row as usize][col].color = Color {
                    r: 0xff,
                    g: 0xff,
                    b: 0xff,
                };
            }
        }
    }

    fn add_line(&mut self) {
        let mut rng = rand::rng();
        let line = Line {
            row: rng.random_range(-100..0),
            col: rng.random_range(0..(self.width as i32)),
            len: rng.random_range(30..40),
            update_interval: rng.random_range(30..60),
            last_updated_at: 0,
        };
        self.lines.push(line);
    }
}

fn main() {
    let rain = Arc::new(Mutex::new(Rain::default()));
    rain.lock().unwrap().update_frame_size();

    thread::spawn(|| {
        let mut signals =
            Signals::new(&[SIGTERM, SIGINT]).expect("Failed to create signal handler");
        for signal in signals.forever() {
            match signal {
                SIGTERM | SIGINT => {
                    print!("\x1b[0m"); // Set text back to normal
                    print!("\x1b[H\x1b[2J"); // Clear the screen
                    print!("\x1b[?25h"); // Show the cursor

                    process::exit(0);
                }
                _ => unreachable!(),
            };
        }
    });

    {
        let rain = Arc::clone(&rain);
        thread::spawn(move || {
            let mut signals = Signals::new(&[SIGWINCH]).expect("Failed to create signal handler");
            for signal in signals.forever() {
                match signal {
                    SIGWINCH => {
                        rain.lock().unwrap().update_frame_size();
                    }
                    _ => unreachable!(),
                };
            }
        });
    }

    print!("\x1b[?25l"); // Hide the cursor
    print!("\x1b[48;2;0;0;0m"); // Set background color to black
    print!("\x1b[H\x1b[2J"); // Clear the screen

    let mut cleared = false;
    let mut line_added_at = 0;

    loop {
        {
            let mut rain = rain.lock().unwrap();
            let now = current_time_millis();

            if !cleared {
                rain.clear();
                cleared = true;
            }

            if now - line_added_at > 80 {
                line_added_at = now;
                rain.add_line();
            }

            rain.update_background_noise();
            rain.update_lines(now);
            rain.render();
        }

        thread::sleep(Duration::from_millis(15));
    }
}
