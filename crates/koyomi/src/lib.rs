mod cli;

use anyhow::Result;
use clap::Parser;

use crate::cli::{Cli, Commands};

pub fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Login => {
            koyomi_core::login();
            Ok(())
        }
        Commands::Logout => {
            koyomi_core::logout();
            Ok(())
        }
    }
}
