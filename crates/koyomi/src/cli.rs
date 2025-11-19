use clap::{Parser, Subcommand};

const ABOUT: &str = "Command line interface for Koyomi, a calendar tool.";
const LONG_ABOUT: &str = r#"Command line interface for Koyomi, a calendar tool.

Its name derives from the Japanese word "暦" (koyomi), meaning calendar."#;

#[derive(Parser, Debug)]
#[command(version, about = ABOUT, long_about = LONG_ABOUT)]
pub struct Cli {
    #[clap(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    Point,
}
