use clap::{ArgAction, Parser, Subcommand, ValueEnum};

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
    /// This month's events (next 1 calendar month)
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
        /// Show detailed event information
        #[arg(short, long)]
        details: bool,
        /// Maximum number of events to return (1-2500)
        #[arg(short = 'n', long, default_value_t = 250, value_parser = clap::value_parser!(u32).range(1..=koyomi_core::calendar::MAX_RESULTS_LIMIT as i64))]
        limit: u32,
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
    fn cli_parses_events_command_with_defaults() {
        let cli = Cli::parse_from(["koyomi", "events"]);
        match cli.command {
            Some(Commands::Events { period, details, limit }) => {
                assert_eq!(period, Period::Day);
                assert_eq!(cli.calendar, "primary");
                assert!(!details);
                assert_eq!(limit, 250);
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
            "--limit",
            "100",
        ]);
        match cli.command {
            Some(Commands::Events { period, details, limit }) => {
                assert_eq!(period, Period::Month);
                assert_eq!(cli.calendar, "work@example.com");
                assert!(details);
                assert_eq!(limit, 100);
            }
            _ => panic!("Expected Events command"),
        }
    }

    #[test]
    fn cli_parses_events_command_with_short_options() {
        let cli = Cli::parse_from([
            "koyomi",
            "events",
            "-p",
            "m",
            "-c",
            "test@example.com",
            "-d",
            "-n",
            "50",
        ]);
        match cli.command {
            Some(Commands::Events { period, details, limit }) => {
                assert_eq!(period, Period::Month);
                assert_eq!(cli.calendar, "test@example.com");
                assert!(details);
                assert_eq!(limit, 50);
            }
            _ => panic!("Expected Events command"),
        }
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
        let result = Cli::try_parse_from(["koyomi", "-q", "-v", "events"]);
        assert!(result.is_err());
    }

    #[test]
    fn cli_defaults_quiet_to_false() {
        let cli = Cli::parse_from(["koyomi", "events"]);
        assert!(!cli.quiet);
    }

    #[test]
    fn cli_parses_timezone_jst() {
        let cli = Cli::parse_from(["koyomi", "--timezone", "JST", "events"]);
        assert_eq!(cli.timezone, Some(koyomi_core::calendar::TimeZone::Jst));
    }

    #[test]
    fn cli_parses_timezone_utc() {
        let cli = Cli::parse_from(["koyomi", "--timezone", "UTC", "events"]);
        assert_eq!(cli.timezone, Some(koyomi_core::calendar::TimeZone::Utc));
    }

    #[test]
    fn cli_timezone_default_is_none() {
        let cli = Cli::parse_from(["koyomi", "events"]);
        assert!(cli.timezone.is_none());
    }

    #[test]
    fn cli_timezone_rejects_lowercase() {
        let result = Cli::try_parse_from(["koyomi", "--timezone", "utc", "events"]);
        assert!(result.is_err());
    }

    #[test]
    fn cli_timezone_rejects_mixed_case() {
        let result = Cli::try_parse_from(["koyomi", "--timezone", "Utc", "events"]);
        assert!(result.is_err());
    }
}
