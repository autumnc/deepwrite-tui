mod app;

use std::io;
use std::panic;
use std::path::PathBuf;

use clap::Parser;
use crossterm::{
    event::{
        DisableMouseCapture, EnableMouseCapture, KeyboardEnhancementFlags,
        PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen,
        LeaveAlternateScreen},
};
use ratatui::prelude::*;

use deepwrite::config::Config;

#[derive(Parser)]
#[command(
    name = "deepwrite",
    version,
    about = "A terminal Markdown writing tool with Focus Mode"
)]
struct Cli {
    /// Directory or file to open
    path: Option<PathBuf>,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let supports_keyboard_enhancement = matches!(
        crossterm::terminal::supports_keyboard_enhancement(),
        Ok(true)
    );

    // Determine start directory and optional start file.
    let (start_dir, start_file) = match cli.path {
        Some(path) => {
            let path = if path.is_absolute() {
                path
            } else {
                std::env::current_dir()
                    .unwrap_or_else(|_| ".".into())
                    .join(path)
            };

            if path.is_file() {
                // Open the file's parent directory, and queue the file to open.
                let dir = path
                    .parent()
                    .map(|p| p.to_path_buf())
                    .unwrap_or_else(|| ".".into());
                (dir, Some(path))
            } else if path.is_dir() {
                (path, None)
            } else {
                // Path doesn't exist yet — treat as a file to create/open.
                // Use the parent as the start directory.
                let dir = path
                    .parent()
                    .map(|p| p.to_path_buf())
                    .unwrap_or_else(|| ".".into());
                (dir, Some(path))
            }
        }
        None => {
            let cwd = std::env::current_dir().unwrap_or_else(|_| ".".into());
            (cwd, None)
        }
    };

    // Set up panic hook to restore terminal on panic
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        let _ = restore_terminal(supports_keyboard_enhancement);
        original_hook(panic_info);
    }));

    // Set up terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    if supports_keyboard_enhancement {
        execute!(
            stdout,
            EnterAlternateScreen,
            EnableMouseCapture,
            PushKeyboardEnhancementFlags(
                KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                    | KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES
                    | KeyboardEnhancementFlags::REPORT_ALTERNATE_KEYS
                    | KeyboardEnhancementFlags::REPORT_EVENT_TYPES
            ),
        )?;
    } else {
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    }
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Load config and run the app
    let config = Config::load();
    let mut app = app::App::new(config, start_dir);

    // If a file was specified on the command line, open it directly.
    if let Some(ref file_path) = start_file {
        app.open_file(file_path);
    }

    let result = app.run(&mut terminal);

    // Restore terminal
    restore_terminal(supports_keyboard_enhancement)?;

    result
}

fn restore_terminal(supports_keyboard_enhancement: bool) -> anyhow::Result<()> {
    disable_raw_mode()?;
    if supports_keyboard_enhancement {
        execute!(
            io::stdout(),
            PopKeyboardEnhancementFlags,
            LeaveAlternateScreen,
            Clear(ClearType::All),
            DisableMouseCapture
        )?;
    } else {
        execute!(io::stdout(), LeaveAlternateScreen, Clear(ClearType::All), DisableMouseCapture)?;
    }
    Ok(())
}
