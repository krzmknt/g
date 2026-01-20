// Suppress warnings for unused code (will be used as features are implemented)
#![allow(dead_code)]
#![allow(unused_imports)]

mod error;
mod tui;
mod input;
mod git;
mod config;
mod widgets;
mod views;
mod app;

use app::App;
use error::Result;

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let mut app = App::new()?;
    app.run()
}
