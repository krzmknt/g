// Suppress warnings for unused code (will be used as features are implemented)
#![allow(dead_code)]
#![allow(unused_imports)]

#[macro_use]
mod logger;
mod app;
mod config;
mod error;
mod git;
mod input;
mod tui;
mod views;
mod widgets;

use app::App;
use error::Result;

fn main() {
    // Set up panic handler to show errors before exiting
    std::panic::set_hook(Box::new(|panic_info| {
        // Try to restore terminal state
        let _ = std::io::Write::write_all(&mut std::io::stdout(), b"\x1b[?1049l\x1b[?25h");
        eprintln!("\nPanic: {}", panic_info);
        eprintln!("\nPress Enter to exit...");
        let _ = std::io::stdin().read_line(&mut String::new());
    }));

    if let Err(e) = run() {
        // Try to restore terminal state
        let _ = std::io::Write::write_all(&mut std::io::stdout(), b"\x1b[?1049l\x1b[?25h");
        eprintln!("Error: {}", e);
        eprintln!("\nPress Enter to exit...");
        let _ = std::io::stdin().read_line(&mut String::new());
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    logger::init();
    info!("Application starting");
    let mut app = App::new()?;
    app.run()
}
