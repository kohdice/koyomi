mod cli;

use anyhow::Result;
use clap::Parser;

use crate::cli::{Cli, Commands};

pub fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Point => {
            let result = koyomi_core::add(50, 50);
            println!("{} points!", result);
            Ok(())
        }
    }
}
