mod app;
mod calendar_grid;
mod event_handler;
mod message;
mod model;
mod theme;
mod update;
mod view;
mod widget;

use std::io::{self, stdout};

use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use self::app::App;

/// Run the TUI application.
///
/// # Errors
///
/// Returns an error if:
/// - Terminal initialization fails
/// - The main event loop encounters an unrecoverable error
/// - Terminal restoration fails
pub async fn run(
    client: koyomi_core::Client,
    token: koyomi_core::StoredToken,
    calendar_id: String,
    tz: koyomi_core::calendar::TimeZone,
) -> anyhow::Result<()> {
    install_panic_hook();

    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout());
    let terminal = Terminal::new(backend)?;

    let result = App::new(terminal, client, token, calendar_id, tz).run().await;

    restore_terminal()?;

    result
}

fn install_panic_hook() {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = restore_terminal();
        original_hook(panic_info);
    }));
}

fn restore_terminal() -> io::Result<()> {
    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen)?;
    Ok(())
}
