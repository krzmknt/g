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
use error::{Error, Result};

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

        // Show user-friendly error messages for common git errors
        let message = match &e {
            Error::Git(git_err) => match git_err.code() {
                git2::ErrorCode::NotFound => {
                    "Not a git repository.\n\nRun 'git init' to initialize a new repository."
                        .to_string()
                }
                git2::ErrorCode::UnbornBranch => {
                    "Repository has no commits yet.\n\nCreate your first commit to use g."
                        .to_string()
                }
                _ => format!("{}", e),
            },
            _ => format!("{}", e),
        };

        eprintln!("{}", message);
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
