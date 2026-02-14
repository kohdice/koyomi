use clap::{ArgAction, Parser, Subcommand, ValueEnum};

const ABOUT: &str = "Command line interface for Koyomi, a calendar tool.";
const LONG_ABOUT: &str = r#"Command line interface for Koyomi, a calendar tool.

Its name derives from the Japanese word "暦" (koyomi), meaning calendar."#;

#[derive(Parser, Debug)]
#[command(version, about = ABOUT, long_about = LONG_ABOUT)]
pub struct Cli {
    /// Increase verbosity (-v, -vv)
    #[arg(short, long, action = ArgAction::Count, global = true)]
    pub verbose: u8,

    #[clap(subcommand)]
    pub command: Option<Commands>,
}

/// Time period for event listing
#[derive(ValueEnum, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Period {
    /// Today's events
    #[default]
    #[value(alias = "d")]
    Day,
    /// This week's events (next 7 days)
    #[value(alias = "w")]
    Week,
    /// This month's events (next 30 days)
    #[value(alias = "m")]
    Month,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Log in to a Google account
    Login,
    /// Log out of a Google account
    Logout,
    /// List calendar events
    Events {
        /// Period: d(ay), w(eek), m(onth)
        #[arg(short, long, value_enum, default_value = "day")]
        period: Period,
        /// Calendar ID (default: primary)
        #[arg(short, long, default_value = "primary")]
        calendar: String,
        /// Show detailed event information
        #[arg(short, long)]
        details: bool,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn period_default_is_day() {
        let period = Period::default();
        assert_eq!(period, Period::Day);
    }

    #[test]
    fn cli_parses_no_subcommand() {
        let cli = Cli::parse_from(["koyomi"]);
        assert!(cli.command.is_none());
    }

    #[test]
    fn cli_parses_events_command_with_defaults() {
        let cli = Cli::parse_from(["koyomi", "events"]);
        match cli.command {
            Some(Commands::Events { period, calendar, details }) => {
                assert_eq!(period, Period::Day);
                assert_eq!(calendar, "primary");
                assert!(!details);
            }
            _ => panic!("Expected Events command"),
        }
    }

    #[test]
    fn cli_parses_events_command_with_period_alias() {
        let cli = Cli::parse_from(["koyomi", "events", "-p", "w"]);
        match cli.command {
            Some(Commands::Events { period, .. }) => {
                assert_eq!(period, Period::Week);
            }
            _ => panic!("Expected Events command"),
        }
    }

    #[test]
    fn cli_parses_events_command_with_all_options() {
        let cli = Cli::parse_from([
            "koyomi",
            "events",
            "--period",
            "month",
            "--calendar",
            "work@example.com",
            "--details",
        ]);
        match cli.command {
            Some(Commands::Events { period, calendar, details }) => {
                assert_eq!(period, Period::Month);
                assert_eq!(calendar, "work@example.com");
                assert!(details);
            }
            _ => panic!("Expected Events command"),
        }
    }

    #[test]
    fn cli_parses_events_command_with_short_options() {
        let cli = Cli::parse_from(["koyomi", "events", "-p", "m", "-c", "test@example.com", "-d"]);
        match cli.command {
            Some(Commands::Events { period, calendar, details }) => {
                assert_eq!(period, Period::Month);
                assert_eq!(calendar, "test@example.com");
                assert!(details);
            }
            _ => panic!("Expected Events command"),
        }
    }
}
