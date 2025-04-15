use std::fmt::Display;

use digirain::{clamp_min_zero, random_symbol, SYMBOLS};
use rand::Rng;
use termion::terminal_size;

pub struct Line {
    row: i32,
    col: i32,
    len: i32,
    update_interval: u128,
    last_updated_at: u128,
}

#[derive(Default, Clone)]
pub struct Color {
    r: u8,
    g: u8,
    b: u8,
}

impl Color {
    pub fn to_ansi256(&self) -> u8 {
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

impl Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "\x1b[38;5;{}m", self.to_ansi256())
    }
}

#[derive(Clone)]
pub struct Drop {
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
        write!(f, "{}{}", self.color, self.char)
    }
}

#[derive(Default)]
pub struct Rain {
    width: usize,
    height: usize,
    prev_frame: Box<Vec<Vec<Drop>>>,
    next_frame: Box<Vec<Vec<Drop>>>,
    lines: Vec<Line>,
}

impl Rain {
    pub fn update_frame_size(&mut self) {
        let (width, height) = terminal_size().expect("Failed to get terminal size");
        (self.width, self.height) = ((width / 2) as usize, (height) as usize);
        self.prev_frame = Box::new(vec![vec![Drop::new(); self.width]; self.height]);
        self.next_frame = Box::new(vec![vec![Drop::new(); self.width]; self.height]);
    }

    pub fn clear(&self) {
        print!(
            "{}",
            vec![SYMBOLS[0].to_string().repeat(self.width); self.height].join("\n")
        );
    }

    pub fn render(&mut self) {
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
                        if self.next_frame[row][col].color == self.next_frame[row][col - 1].color {
                            delta.push_str(&self.next_frame[row][col].char.to_string());
                        } else {
                            delta.push_str(&self.next_frame[row][col].to_string());
                        }
                    }
                } else {
                    print!("{delta}");
                    delta.clear();
                }
            }
            print!("{delta}");
            delta.clear()
        }
        self.prev_frame = self.next_frame.clone();
    }

    pub fn update_background_noise(&mut self) {
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

    pub fn update_lines(&mut self, now: u128) {
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
            let col = clamp_min_zero(line.col, w - 1) as usize;
            for row in
                (clamp_min_zero(line.row - line.len, h - 1)..clamp_min_zero(line.row, h)).rev()
            {
                self.next_frame[row as usize][col].color = Color {
                    r: 0,
                    g: 0xff - ((line.row - row) * 5) as u8,
                    b: 0,
                };
            }
            for row in clamp_min_zero(line.row + 1, h - 1)..clamp_min_zero(line.row + 10, h) {
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

    pub fn add_line(&mut self) {
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
