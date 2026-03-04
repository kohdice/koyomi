mod cli;
mod commands;

use anyhow::Result;
use clap::Parser;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

use crate::cli::{Cli, Commands};
use crate::commands::event::EventCommand;

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
    let client = koyomi_core::Client::new()?;

    match cli.command {
        None => {
            koyomi_core::get_valid_token(&client).await?;
            koyomi_ui::tui::run(client, cli.calendar, tz).await?;
            Ok(())
        }
        Some(Commands::Login) => commands::login::handle(&client, cli.quiet).await,
        Some(Commands::Logout) => commands::logout::handle(&client, cli.quiet).await,
        Some(Commands::Event(event_args)) => match event_args.command {
            EventCommand::List(args) => {
                commands::event::handle_list(&client, args, cli.calendar, tz).await
            }
            EventCommand::Add(args) => {
                commands::event::handle_add(&client, args, cli.calendar).await
            }
            EventCommand::Update(args) => {
                commands::event::handle_update(&client, args, cli.calendar).await
            }
            EventCommand::Delete(args) => {
                commands::event::handle_delete(&client, args, cli.calendar, cli.quiet).await
            }
        },
    }
}
