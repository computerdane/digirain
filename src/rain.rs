use std::io::stdout;

use crossterm::{
    cursor, execute,
    style::{Color, PrintStyledContent, StyledContent, Stylize},
    terminal::{self},
};
use digirain::{clamp_min_zero, random_item, SYMBOLS, SYMBOLS_HALF};
use rand::Rng;

use crate::Args;

pub struct Line {
    row: i32,
    col: i32,
    len: i32,
    update_interval: u128,
    last_updated_at: u128,
}

pub struct Rain<'a> {
    width: usize,
    height: usize,
    line_added_at: u128,
    prev_frame: Box<Vec<Vec<StyledContent<char>>>>,
    next_frame: Box<Vec<Vec<StyledContent<char>>>>,
    lines: Vec<Line>,
    args: Args,
    symbols: &'a [char],
}

impl<'a> Rain<'a> {
    pub fn new(args: Args) -> Self {
        let mut rain = Rain {
            width: 0,
            height: 0,
            line_added_at: 0,
            prev_frame: Box::default(),
            next_frame: Box::default(),
            lines: Vec::default(),
            args,
            symbols: &[],
        };
        rain.symbols = if rain.args.half_width {
            &SYMBOLS_HALF
        } else {
            &SYMBOLS
        };
        rain
    }

    pub fn update_frame_size(&mut self) {
        let (width, height) = terminal::size().unwrap();
        (self.width, self.height) = (
            if self.args.half_width {
                width
            } else {
                width / 2
            } as usize,
            height as usize,
        );
        let blank_symbol = self.symbols[0].with(Color::Rgb { r: 0, g: 0, b: 0 });
        self.prev_frame = Box::new(vec![vec![blank_symbol; self.width]; self.height]);
        self.next_frame = Box::new(vec![vec![blank_symbol; self.width]; self.height]);
    }

    pub fn render(&mut self) {
        for row in 0..self.height {
            for col in 0..self.width {
                let drop = self.next_frame[row][col];
                if self.next_frame[row][col] != self.prev_frame[row][col] {
                    execute!(
                        stdout(),
                        cursor::MoveTo(
                            if self.args.half_width { col } else { col * 2 } as u16,
                            row as u16,
                        ),
                        PrintStyledContent(drop)
                    )
                    .unwrap();
                    self.prev_frame[row][col] = self.next_frame[row][col];
                }
            }
        }
    }

    pub fn update_background_noise(&mut self) {
        let mut rng = rand::rng();
        for row in 0..self.height {
            for col in 0..self.width {
                let drop = &mut self.next_frame[row][col];

                if rng.random_range(0..100) < 4 {
                    *drop = StyledContent::new(*drop.style(), random_item(self.symbols, &mut rng));
                }

                let r = rng.random_range(0..1000);
                if r < 10 {
                    *drop = drop.with(Color::Rgb {
                        r: 0,
                        g: 0x66,
                        b: 0,
                    })
                } else if r < 7 {
                    *drop = drop.with(Color::Rgb {
                        r: 0,
                        g: 0x88,
                        b: 0,
                    })
                }
            }
        }
    }

    pub fn update_lines(&mut self, now: u128) {
        if now - self.line_added_at > 80 {
            self.line_added_at = now;
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
            for row in (clamp_min_zero(line.row - line.len, h)..clamp_min_zero(line.row, h)).rev() {
                let drop = &mut self.next_frame[row as usize][col];
                *drop = drop.content().with(Color::Rgb {
                    r: 0,
                    g: 0xff - ((line.row - row) * 5) as u8,
                    b: 0,
                })
            }
            for row in clamp_min_zero(line.row + 1, h)..clamp_min_zero(line.row + 10, h) {
                let drop = &mut self.next_frame[row as usize][col];
                *drop = drop.content().with(Color::Rgb { r: 0, g: 0, b: 0 })
            }
            if 0 <= line.row && line.row < h {
                let drop = &mut self.next_frame[line.row as usize][col];
                *drop = drop.content().with(Color::Rgb {
                    r: 0xff,
                    g: 0xff,
                    b: 0xff,
                })
            }
        }
    }
}
