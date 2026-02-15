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

/// Exit codes following sysexits.h conventions where applicable.
///
/// - 1: General error
/// - 69 (EX_UNAVAILABLE): Service unavailable (HTTP errors, rate limiting)
/// - 77 (EX_NOPERM): Authentication required / access denied
/// - 78 (EX_CONFIG): Configuration error
const EX_NOPERM: u8 = 77;
const EX_CONFIG: u8 = 78;

/// Map an error to a process exit code.
///
/// Uses sysexits.h conventions:
/// - `EX_NOPERM` (77) for authentication-related errors
/// - `EX_CONFIG` (78) for configuration errors
/// - 1 for all other errors
#[must_use]
pub fn exit_code_for(error: &anyhow::Error) -> u8 {
    if let Some(e) = error.downcast_ref::<koyomi_core::Error>() {
        match e {
            koyomi_core::Error::TokenNotFound
            | koyomi_core::Error::AuthAccessDenied
            | koyomi_core::Error::AuthTimeout
            | koyomi_core::Error::NoRefreshToken => return EX_NOPERM,
            koyomi_core::Error::Calendar(koyomi_core::CalendarError::Unauthenticated) => {
                return EX_NOPERM;
            }
            koyomi_core::Error::ConfigDirNotFound
            | koyomi_core::Error::ConfigFileNotFound { .. }
            | koyomi_core::Error::ConfigInvalid(_) => return EX_CONFIG,
            koyomi_core::Error::Auth(_)
            | koyomi_core::Error::Http(_)
            | koyomi_core::Error::Json(_)
            | koyomi_core::Error::Io(_)
            | koyomi_core::Error::Calendar(_) => return 1,
        }
    }
    1
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

    match cli.command {
        Commands::Login => {
            let client = koyomi_core::Client::new()?;
            handle_login(&client, cli.quiet).await
        }
        Commands::Logout => {
            let client = koyomi_core::Client::new()?;
            handle_logout(&client, cli.quiet).await
        }
        Commands::Events { period, calendar, details, limit } => {
            let client = koyomi_core::Client::new()?;
            handle_events(&client, period, calendar, details, limit).await
        }
    }
}

async fn handle_login(client: &koyomi_core::Client, quiet: bool) -> Result<()> {
    let session = koyomi_core::start_login(client).await?;

    // verification URL と user code は --quiet でも表示する（認証に必須）
    eprintln!();
    eprintln!("To sign in, please visit: {}", session.verification_url());
    eprintln!("Enter this code: {}", session.user_code());
    eprintln!();

    if let Err(e) = open::that(session.verification_url()) {
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
) -> Result<()> {
    let token = koyomi_core::get_valid_token(client).await?;

    let config = koyomi_core::calendar::ListEventsConfig::new(calendar, period.into(), limit)?;

    let events = client.list_events(&token, &config).await?;

    let mut stdout = std::io::stdout().lock();
    koyomi_ui::json::render(&mut stdout, &events, details)?;

    Ok(())
}
