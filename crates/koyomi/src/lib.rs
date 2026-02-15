mod cli;

use anyhow::Result;
use clap::Parser;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

use crate::cli::{Cli, Commands, Period};

fn init_tracing(verbose: u8) {
    let level = match verbose {
        0 => return, // No logging
        1 => Level::INFO,
        _ => Level::DEBUG,
    };

    FmtSubscriber::builder()
        .with_max_level(level)
        .with_target(false)
        .with_writer(std::io::stderr)
        .without_time()
        .init();
}

impl From<Period> for koyomi_core::calendar::EventPeriod {
    fn from(period: Period) -> Self {
        match period {
            Period::Day => Self::Day,
            Period::Week => Self::Week,
            Period::Month => Self::Month,
        }
    }
}

/// Run the CLI application
///
/// # Errors
///
/// Returns an error if any subcommand fails.
pub async fn run() -> Result<()> {
    let cli = Cli::parse();
    init_tracing(cli.verbose);

    match cli.command {
        Some(Commands::Login) => {
            koyomi_core::login().await?;
            Ok(())
        }
        Some(Commands::Logout) => {
            koyomi_core::logout()?;
            Ok(())
        }
        Some(Commands::Events { period, calendar, details, limit }) => {
            handle_events(period, calendar, details, limit).await
        }
        None => {
            eprintln!("TUI mode is not yet implemented. Use a subcommand.");
            eprintln!("Run `koyomi --help` for usage information.");
            Err(anyhow::anyhow!("No subcommand specified"))
        }
    }
}

async fn handle_events(period: Period, calendar: String, details: bool, limit: u32) -> Result<()> {
    let token = koyomi_core::auth::get_valid_token().await?;

    let config = koyomi_core::calendar::ListEventsConfig {
        calendar_id: calendar,
        period: period.into(),
        max_results: limit,
    };

    let client = koyomi_core::Client::new();
    let events = client.list_events(&token, &config).await?;

    let mut stdout = std::io::stdout().lock();
    koyomi_ui::json::render(&mut stdout, &events, details)?;

    Ok(())
}
