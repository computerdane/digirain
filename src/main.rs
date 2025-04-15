mod rain;

use std::{
    io::stdout,
    process,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use clap::{Parser, ValueEnum};
use crossterm::{
    cursor, execute,
    style::{Attribute, SetAttribute, SetBackgroundColor},
    terminal::{Clear, ClearType},
};
use digirain::{current_time_millis, COLOR_BLACK};
use rain::Rain;
use signal_hook::{
    consts::{SIGINT, SIGTERM, SIGWINCH},
    iterator::Signals,
};

#[derive(ValueEnum, Clone)]
enum RainColor {
    Red,
    Green,
    Blue,
}

#[derive(Parser)]
pub struct Args {
    #[arg(long)]
    half_width: bool,

    #[arg(long, value_enum, default_value_t = RainColor::Green)]
    color: RainColor,

    #[arg(long, default_value_t = 0.04)]
    prob_symbol_change: f64,
    #[arg(long, default_value_t = 0.007)]
    prob_color: f64,
    #[arg(long, default_value_t = 0.003)]
    prob_color_dim: f64,
    #[arg(long, default_value_t = 0.16)]
    prob_color_fade: f64,

    #[arg(long, default_value_t = 0.92)]
    color_fade_scale: f64,

    #[arg(long, default_value_t = -100)]
    line_row_start: i32,

    #[arg(long, default_value_t = 30)]
    min_line_len: i32,
    #[arg(long, default_value_t = 40)]
    max_line_len: i32,

    #[arg(long, default_value_t = 30)]
    min_line_update_interval: u128,
    #[arg(long, default_value_t = 60)]
    max_line_update_interval: u128,

    #[arg(long, default_value_t = 80)]
    line_add_interval: u128,

    #[arg(long, default_value_t = 15)]
    target_delta: u128,
}

fn main() {
    let args = Args::parse();
    let target_delta = args.target_delta;

    let rain = Arc::new(Mutex::new(Rain::new(args)));
    rain.lock().unwrap().update_frame_size();

    {
        let rain = Arc::clone(&rain);
        thread::spawn(move || {
            let mut signals =
                Signals::new(&[SIGTERM, SIGINT]).expect("Failed to create signal handler");
            for signal in signals.forever() {
                match signal {
                    SIGTERM | SIGINT => {
                        rain.lock().unwrap().pause();

                        execute!(
                            stdout(),
                            SetAttribute(Attribute::Reset),
                            Clear(ClearType::All),
                            cursor::MoveTo(0, 0),
                            cursor::Show
                        )
                        .unwrap();

                        process::exit(0);
                    }
                    _ => unreachable!(),
                };
            }
        });
    }

    {
        let rain = Arc::clone(&rain);
        thread::spawn(move || {
            let mut signals = Signals::new(&[SIGWINCH]).expect("Failed to create signal handler");
            for signal in signals.forever() {
                match signal {
                    SIGWINCH => {
                        let mut rain = rain.lock().unwrap();
                        rain.update_frame_size();
                    }
                    _ => unreachable!(),
                };
            }
        });
    }

    execute!(
        stdout(),
        cursor::Hide,
        SetBackgroundColor(COLOR_BLACK),
        Clear(ClearType::All)
    )
    .unwrap();

    let mut start;
    let mut end = current_time_millis();
    loop {
        start = end;
        {
            let mut rain = rain.lock().unwrap();
            let now = current_time_millis();

            rain.update_background_noise();
            rain.update_lines(now);
            rain.render();
        }
        end = current_time_millis();
        let delta = end - start;

        if delta < target_delta {
            thread::sleep(Duration::from_millis((target_delta - delta) as u64));
        } else {
            thread::sleep(Duration::from_millis(1));
        }
    }
}
