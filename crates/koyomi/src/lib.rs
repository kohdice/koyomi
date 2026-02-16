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

/// Run the CLI application
///
/// # Errors
///
/// Returns an error if any subcommand fails.
pub async fn run() -> Result<()> {
    let cli = Cli::parse();
    init_tracing(cli.verbose)?;

    let tz = cli.timezone.unwrap_or_default();

    match cli.command {
        None => {
            let client = koyomi_core::Client::new()?;
            let token = koyomi_core::get_valid_token(&client).await?;
            koyomi_ui::tui::run(client, token, cli.calendar, tz).await?;
            Ok(())
        }
        Some(Commands::Login) => {
            let client = koyomi_core::Client::new()?;
            handle_login(&client, cli.quiet).await
        }
        Some(Commands::Logout) => {
            let client = koyomi_core::Client::new()?;
            handle_logout(&client, cli.quiet).await
        }
        Some(Commands::Events { period, calendar, details, limit }) => {
            let client = koyomi_core::Client::new()?;
            handle_events(&client, period, calendar, details, limit, tz).await
        }
    }
}

async fn handle_login(client: &koyomi_core::Client, quiet: bool) -> Result<()> {
    let session = koyomi_core::start_login(client).await?;

    // verification URL と user code は --quiet でも表示する（認証に必須）
    eprintln!();
    eprintln!("To sign in, please visit: {}", session.verification_uri());
    eprintln!("Enter this code: {}", session.user_code());
    eprintln!();

    if let Err(e) = open::that(session.verification_uri()) {
        tracing::warn!("Could not open browser automatically: {}", e);
        if !quiet {
            eprintln!("Could not open browser automatically. Please open the URL above manually.");
        }
    }

    if !quiet {
        eprintln!("Waiting for authorization...");
    }

    koyomi_core::complete_login(client, &session).await?;

    if !quiet {
        eprintln!();
        eprintln!("Successfully logged in!");
    }

    Ok(())
}

async fn handle_logout(client: &koyomi_core::Client, quiet: bool) -> Result<()> {
    match koyomi_core::logout(client).await? {
        koyomi_core::LogoutResult::LoggedOut => {
            if !quiet {
                eprintln!("Successfully logged out.");
            }
        }
        koyomi_core::LogoutResult::NotLoggedIn => {
            if !quiet {
                eprintln!("Not currently logged in.");
            }
        }
        koyomi_core::LogoutResult::CorruptTokenRemoved => {
            if !quiet {
                eprintln!(
                    "Token file was corrupt and has been removed. Please run 'koyomi login' again."
                );
            }
        }
    }
    Ok(())
}

async fn handle_events(
    client: &koyomi_core::Client,
    period: Period,
    calendar: String,
    details: bool,
    limit: u32,
    tz: koyomi_core::calendar::TimeZone,
) -> Result<()> {
    let token = koyomi_core::get_valid_token(client).await?;

    let today = tz.today();
    let (time_min, time_max) = match period {
        Period::Day => koyomi_core::calendar::time_range::for_day(today, tz)?,
        Period::Week => koyomi_core::calendar::time_range::for_week(today, tz)?,
        Period::Month => koyomi_core::calendar::time_range::for_month(today, tz)?,
    };
    let config = koyomi_core::calendar::ListEventsConfig::new(calendar, time_min, time_max, limit)?;

    let events = client.list_events(&token, &config).await?;

    let mut stdout = std::io::stdout().lock();
    koyomi_ui::json::render(&mut stdout, &events, details)?;

    Ok(())
}
