use clap::{ArgAction, Parser, Subcommand};

use crate::commands::event::EventArgs;

const ABOUT: &str = "Command line interface for Koyomi, a calendar tool.";
const LONG_ABOUT: &str = r#"Command line interface for Koyomi, a calendar tool.

Its name derives from the Japanese word "暦" (koyomi), meaning calendar."#;

fn parse_timezone(s: &str) -> Result<koyomi_core::calendar::TimeZone, String> {
    match s {
        "JST" => Ok(koyomi_core::calendar::TimeZone::Jst),
        "UTC" => Ok(koyomi_core::calendar::TimeZone::Utc),
        _ => Err(format!("invalid timezone '{s}': expected JST or UTC")),
    }
}

#[derive(Parser, Debug)]
#[command(version, about = ABOUT, long_about = LONG_ABOUT)]
pub struct Cli {
    /// Increase verbosity (-v, -vv)
    #[arg(short, long, action = ArgAction::Count, global = true)]
    pub verbose: u8,

    /// Suppress informational messages
    #[arg(short, long, global = true, conflicts_with = "verbose")]
    pub quiet: bool,

    /// Calendar ID (default: primary)
    #[arg(short, long, default_value = "primary", global = true,
          value_parser = clap::builder::NonEmptyStringValueParser::new())]
    pub calendar: String,

    /// Timezone for time range calculation (default: system timezone)
    #[arg(short = 't', long, value_parser = parse_timezone, global = true)]
    pub timezone: Option<koyomi_core::calendar::TimeZone>,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Log in to a Google account
    Login,
    /// Log out of a Google account
    Logout,
    /// Manage calendar events
    Event(Box<EventArgs>),
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn cli_accepts_no_subcommand_for_tui_mode() {
        let cli = Cli::parse_from(["koyomi"]);
        assert!(cli.command.is_none());
        assert_eq!(cli.calendar, "primary");
    }

    #[test]
    fn cli_accepts_no_subcommand_with_calendar() {
        let cli = Cli::parse_from(["koyomi", "--calendar", "work@example.com"]);
        assert!(cli.command.is_none());
        assert_eq!(cli.calendar, "work@example.com");
    }

    #[test]
    fn cli_parses_quiet_flag() {
        let cli = Cli::parse_from(["koyomi", "--quiet", "login"]);
        assert!(cli.quiet);
        assert_eq!(cli.verbose, 0);
    }

    #[test]
    fn cli_parses_quiet_short_flag() {
        let cli = Cli::parse_from(["koyomi", "-q", "logout"]);
        assert!(cli.quiet);
    }

    #[test]
    fn cli_quiet_conflicts_with_verbose() {
        let result = Cli::try_parse_from(["koyomi", "-q", "-v", "event", "list"]);
        assert!(result.is_err());
    }

    #[test]
    fn cli_defaults_quiet_to_false() {
        let cli = Cli::parse_from(["koyomi", "event", "list"]);
        assert!(!cli.quiet);
    }

    #[test]
    fn cli_parses_timezone_jst() {
        let cli = Cli::parse_from(["koyomi", "--timezone", "JST", "event", "list"]);
        assert_eq!(cli.timezone, Some(koyomi_core::calendar::TimeZone::Jst));
    }

    #[test]
    fn cli_parses_timezone_utc() {
        let cli = Cli::parse_from(["koyomi", "--timezone", "UTC", "event", "list"]);
        assert_eq!(cli.timezone, Some(koyomi_core::calendar::TimeZone::Utc));
    }

    #[test]
    fn cli_timezone_default_is_none() {
        let cli = Cli::parse_from(["koyomi", "event", "list"]);
        assert!(cli.timezone.is_none());
    }

    #[test]
    fn cli_timezone_rejects_lowercase() {
        let result = Cli::try_parse_from(["koyomi", "--timezone", "utc", "event", "list"]);
        assert!(result.is_err());
    }

    #[test]
    fn cli_timezone_rejects_mixed_case() {
        let result = Cli::try_parse_from(["koyomi", "--timezone", "Utc", "event", "list"]);
        assert!(result.is_err());
    }
}
