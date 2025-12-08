mod cli;

use anyhow::Result;
use clap::Parser;

use crate::cli::{Cli, Commands};

pub async fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Login => {
            koyomi_core::login(cli.verbose).await?;
            Ok(())
        }
        Commands::Logout => {
            koyomi_core::logout(cli.verbose).await?;
            Ok(())
        }
    }
}
