use std::io::stdout;

use crossterm::{
    cursor, execute,
    style::{Color, PrintStyledContent, StyledContent, Stylize},
    terminal::{self},
};
use digirain::{
    clamp_min_zero, interp, random_item, COLOR_BLACK, COLOR_WHITE, SYMBOLS, SYMBOLS_HALF,
};
use rand::Rng;

use crate::Args;

pub struct Line {
    row: i32,
    col: i32,
    len: i32,
    update_interval: u128,
    last_updated_at: u128,
    colors: Vec<Color>,
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
    color: Color,
    color_dim: Color,
    color_bright: Color,
    needs_refresh: bool,
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
            color: COLOR_BLACK,
            color_dim: COLOR_BLACK,
            color_bright: COLOR_BLACK,
            needs_refresh: false,
        };
        rain.symbols = if rain.args.half_width {
            &SYMBOLS_HALF
        } else {
            &SYMBOLS
        };
        match rain.args.color {
            crate::RainColor::Red => {
                rain.color = Color::Rgb {
                    r: 0x88,
                    g: 0,
                    b: 0,
                };
                rain.color_dim = Color::Rgb {
                    r: 0x66,
                    g: 0,
                    b: 0,
                };
                rain.color_bright = Color::Rgb {
                    r: 0xff,
                    g: 0,
                    b: 0,
                };
            }
            crate::RainColor::Green => {
                rain.color = Color::Rgb {
                    r: 0,
                    g: 0x88,
                    b: 0,
                };
                rain.color_dim = Color::Rgb {
                    r: 0,
                    g: 0x66,
                    b: 0,
                };
                rain.color_bright = Color::Rgb {
                    r: 0,
                    g: 0xff,
                    b: 0,
                };
            }
            crate::RainColor::Blue => {
                rain.color = Color::Rgb {
                    r: 0,
                    g: 0,
                    b: 0x88,
                };
                rain.color_dim = Color::Rgb {
                    r: 0,
                    g: 0,
                    b: 0x66,
                };
                rain.color_bright = Color::Rgb {
                    r: 0,
                    g: 0,
                    b: 0xff,
                };
            }
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
        let blank_symbol = self.symbols[0].with(COLOR_BLACK);
        self.prev_frame = Box::new(vec![vec![blank_symbol; self.width]; self.height]);
        self.next_frame = Box::new(vec![vec![blank_symbol; self.width]; self.height]);
        let mut rng = rand::rng();
        for row in 0..self.height {
            for col in 0..self.width {
                self.next_frame[row][col] = random_item(self.symbols, &mut rng).with(COLOR_BLACK);
            }
        }
    }

    pub fn render(&mut self) {
        for row in 0..self.height {
            for col in 0..self.width {
                let drop = self.next_frame[row][col];
                if self.needs_refresh || self.next_frame[row][col] != self.prev_frame[row][col] {
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
        self.needs_refresh = false;
    }

    pub fn update_background_noise(&mut self) {
        let mut rng = rand::rng();
        for row in 0..self.height {
            for col in 0..self.width {
                let drop = &mut self.next_frame[row][col];
                let color = drop.style().foreground_color.unwrap();

                if color != COLOR_BLACK && rng.random_range(0..100) < 4 {
                    *drop = StyledContent::new(*drop.style(), random_item(self.symbols, &mut rng));
                }

                let r = rng.random_range(0..1000);
                if r < 7 {
                    *drop = drop.with(self.color)
                } else if r < 10 {
                    *drop = drop.with(self.color_dim)
                }

                let r = rng.random_range(0..100);
                if r < 20 {
                    let color = drop.style().foreground_color.unwrap();
                    if color != COLOR_BLACK {
                        *drop = drop.content().with(interp(color, COLOR_BLACK, 0.92));
                    }
                }
            }
        }
    }

    pub fn update_lines(&mut self, now: u128) {
        if now - self.line_added_at > 80 {
            self.line_added_at = now;
            let mut rng = rand::rng();
            let mut line = Line {
                row: rng.random_range(-100..0),
                col: rng.random_range(0..(self.width as i32)),
                len: rng.random_range(30..40),
                update_interval: rng.random_range(30..60),
                last_updated_at: 0,
                colors: vec![],
            };
            for i in 0..line.len {
                line.colors.push(interp(
                    self.color_bright,
                    self.color,
                    i as f64 / (line.len - 1) as f64,
                ));
            }
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
            let mut color_index = 0;
            for row in clamp_min_zero(line.row - line.len, h)..clamp_min_zero(line.row, h) {
                let drop = &mut self.next_frame[row as usize][col];
                *drop = drop.content().with(line.colors[color_index]);
                color_index += 1;
            }
            for row in clamp_min_zero(line.row + 1, h)..clamp_min_zero(line.row + 10, h) {
                let drop = &mut self.next_frame[row as usize][col];
                *drop = drop.content().with(COLOR_BLACK);
            }
            if 0 <= line.row && line.row < h {
                let drop = &mut self.next_frame[line.row as usize][col];
                *drop = drop.content().with(COLOR_WHITE);
            }
        }
    }

    pub fn refresh(&mut self) {
        self.needs_refresh = true;
    }
}
