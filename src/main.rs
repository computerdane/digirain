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
    style::{Attribute, Color, SetAttribute, SetBackgroundColor},
    terminal::{Clear, ClearType},
};
use digirain::current_time_millis;
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
}

fn main() {
    let args = Args::parse();

    let rain = Arc::new(Mutex::new(Rain::new(args)));
    rain.lock().unwrap().update_frame_size();

    thread::spawn(|| {
        let mut signals =
            Signals::new(&[SIGTERM, SIGINT]).expect("Failed to create signal handler");
        for signal in signals.forever() {
            match signal {
                SIGTERM | SIGINT => {
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

    execute!(
        stdout(),
        cursor::Hide,
        SetBackgroundColor(Color::Rgb { r: 0, g: 0, b: 0 }),
        Clear(ClearType::All)
    )
    .unwrap();

    loop {
        {
            let mut rain = rain.lock().unwrap();
            let now = current_time_millis();

            rain.update_background_noise();
            rain.update_lines(now);
            rain.render();
        }

        thread::sleep(Duration::from_millis(15));
    }
}
