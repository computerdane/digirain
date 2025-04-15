mod rain;

use std::{
    process,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use clap::Parser;
use digirain::current_time_millis;
use rain::Rain;
use signal_hook::{
    consts::{SIGINT, SIGTERM, SIGWINCH},
    iterator::Signals,
};

#[derive(Parser)]
pub struct Args {
    #[arg(long)]
    half_width: bool,
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
