mod cli;

use anyhow::Result;
use clap::Parser;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

use crate::cli::{Cli, Commands, Period};

fn init_tracing(verbose: u8) -> Result<()> {
    let level = match verbose {
        0 => return Ok(()),
        1 => Level::INFO,
        _ => Level::DEBUG,
    };

    FmtSubscriber::builder()
        .with_max_level(level)
        .with_target(false)
        .with_writer(std::io::stderr)
        .without_time()
        .try_init()
        .map_err(|e| anyhow::anyhow!("Failed to initialize logging: {e}"))
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
    init_tracing(cli.verbose)?;

    let client = koyomi_core::Client::new()?;

    match cli.command {
        Commands::Login => {
            koyomi_core::login(&client).await?;
            Ok(())
        }
        Commands::Logout => {
            koyomi_core::logout()?;
            Ok(())
        }
        Commands::Events { period, calendar, details, limit } => {
            handle_events(&client, period, calendar, details, limit).await
        }
    }
}

async fn handle_events(
    client: &koyomi_core::Client,
    period: Period,
    calendar: String,
    details: bool,
    limit: u32,
) -> Result<()> {
    let token = koyomi_core::get_valid_token(client).await?;

    let config = koyomi_core::calendar::ListEventsConfig::new(calendar, period.into(), limit)?;

    let events = client.list_events(&token, &config).await?;

    let mut stdout = std::io::stdout().lock();
    koyomi_ui::json::render(&mut stdout, &events, details)?;

    Ok(())
}
