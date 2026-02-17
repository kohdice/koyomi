use anyhow::Result;
use clap::{Args, Subcommand, ValueEnum};

use koyomi_core::calendar::EventDateTime;

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

#[derive(Args, Debug)]
pub struct EventArgs {
    #[command(subcommand)]
    pub command: EventCommand,
}

#[derive(Subcommand, Debug)]
pub enum EventCommand {
    /// List calendar events
    List(ListArgs),
    /// Add a new calendar event
    Add(AddArgs),
    /// Update an existing calendar event
    Update(UpdateArgs),
    /// Delete a calendar event
    Delete(DeleteArgs),
}

#[derive(Args, Debug)]
pub struct ListArgs {
    /// Period: d(ay), w(eek), m(onth)
    #[arg(short, long, value_enum, default_value = "day")]
    pub period: Period,
    /// Show detailed event information
    #[arg(short, long)]
    pub details: bool,
    /// Maximum number of events to return (1-2500)
    #[arg(short = 'n', long, default_value_t = 250, value_parser = clap::value_parser!(u32).range(1..=koyomi_core::calendar::MAX_RESULTS_LIMIT as i64))]
    pub limit: u32,
}

#[derive(Args, Debug)]
pub struct AddArgs {
    /// Event title
    #[arg(short, long)]
    pub summary: String,
    /// Start time (RFC 3339 e.g. 2026-02-17T10:00:00+09:00, or date e.g. 2026-02-17)
    #[arg(long)]
    pub start: String,
    /// End time (same format as start)
    #[arg(long)]
    pub end: String,
    /// Event description
    #[arg(long)]
    pub description: Option<String>,
    /// Event location
    #[arg(long)]
    pub location: Option<String>,
}

#[derive(Args, Debug)]
pub struct UpdateArgs {
    /// Event ID to update
    pub id: String,
    /// New event title
    #[arg(short, long)]
    pub summary: Option<String>,
    /// New start time
    #[arg(long)]
    pub start: Option<String>,
    /// New end time
    #[arg(long)]
    pub end: Option<String>,
    /// New description
    #[arg(long)]
    pub description: Option<String>,
    /// New location
    #[arg(long)]
    pub location: Option<String>,
}

#[derive(Args, Debug)]
pub struct DeleteArgs {
    /// Event ID to delete
    pub id: String,
}

/// Parse a user-supplied date/time string into `EventDateTime`.
///
/// Tries RFC 3339 first (timed event), then `YYYY-MM-DD` (all-day event).
pub fn parse_event_datetime(s: &str) -> Result<EventDateTime, String> {
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
        return Ok(EventDateTime::DateTime { date_time: dt, time_zone: None });
    }

    if let Ok(date) = chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return Ok(EventDateTime::Date { date });
    }

    Err(format!(
        "invalid date/time '{s}': expected RFC 3339 (e.g. 2026-02-17T10:00:00+09:00) or YYYY-MM-DD (e.g. 2026-02-17)"
    ))
}

pub async fn handle_list(
    client: &koyomi_core::Client,
    args: ListArgs,
    calendar: String,
    tz: koyomi_core::calendar::TimeZone,
) -> Result<()> {
    let token = koyomi_core::get_valid_token(client).await?;

    let today = tz.today();
    let (time_min, time_max) = match args.period {
        Period::Day => koyomi_core::calendar::time_range::for_day(today, tz)?,
        Period::Week => koyomi_core::calendar::time_range::for_week(today, tz)?,
        Period::Month => koyomi_core::calendar::time_range::for_month(today, tz)?,
    };
    let config =
        koyomi_core::calendar::ListEventsConfig::new(calendar, time_min, time_max, args.limit)?;

    let events = client.list_events(&token, &config).await?;

    let mut stdout = std::io::stdout().lock();
    koyomi_ui::json::render(&mut stdout, &events, args.details)?;

    Ok(())
}

pub async fn handle_add(
    client: &koyomi_core::Client,
    args: AddArgs,
    calendar: String,
) -> Result<()> {
    let token = koyomi_core::get_valid_token(client).await?;

    let start = parse_event_datetime(&args.start).map_err(|e| anyhow::anyhow!(e))?;
    let end = parse_event_datetime(&args.end).map_err(|e| anyhow::anyhow!(e))?;

    let body = koyomi_core::calendar::InsertEventBody {
        summary: args.summary,
        start,
        end,
        description: args.description,
        location: args.location,
    };
    let config = koyomi_core::calendar::InsertEventConfig::new(calendar, body)?;

    let event = client.insert_event(&token, &config).await?;

    let stdout = std::io::stdout().lock();
    serde_json::to_writer_pretty(stdout, &event)?;
    println!();

    Ok(())
}

pub async fn handle_update(
    client: &koyomi_core::Client,
    args: UpdateArgs,
    calendar: String,
) -> Result<()> {
    let token = koyomi_core::get_valid_token(client).await?;

    let start =
        args.start.map(|s| parse_event_datetime(&s)).transpose().map_err(|e| anyhow::anyhow!(e))?;
    let end =
        args.end.map(|s| parse_event_datetime(&s)).transpose().map_err(|e| anyhow::anyhow!(e))?;

    let body = koyomi_core::calendar::PatchEventBody {
        summary: args.summary,
        start,
        end,
        description: args.description,
        location: args.location,
    };
    let config = koyomi_core::calendar::PatchEventConfig::new(calendar, args.id, body)?;

    let event = client.patch_event(&token, &config).await?;

    let stdout = std::io::stdout().lock();
    serde_json::to_writer_pretty(stdout, &event)?;
    println!();

    Ok(())
}

pub async fn handle_delete(
    client: &koyomi_core::Client,
    args: DeleteArgs,
    calendar: String,
    quiet: bool,
) -> Result<()> {
    let token = koyomi_core::get_valid_token(client).await?;

    let config = koyomi_core::calendar::DeleteEventConfig::new(calendar, args.id.clone())?;

    client.delete_event(&token, &config).await?;

    if !quiet {
        eprintln!("Event '{}' deleted successfully.", args.id);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{Cli, Commands};
    use clap::Parser;

    #[test]
    fn period_default_is_day() {
        let period = Period::default();
        assert_eq!(period, Period::Day);
    }

    #[test]
    fn cli_parses_event_list_command_with_defaults() {
        let cli = Cli::parse_from(["koyomi", "event", "list"]);
        match cli.command {
            Some(Commands::Event(event_args)) => match event_args.command {
                EventCommand::List(args) => {
                    assert_eq!(args.period, Period::Day);
                    assert_eq!(cli.calendar, "primary");
                    assert!(!args.details);
                    assert_eq!(args.limit, 250);
                }
                _ => panic!("Expected List subcommand"),
            },
            _ => panic!("Expected Event command"),
        }
    }

    #[test]
    fn cli_parses_event_list_command_with_period_alias() {
        let cli = Cli::parse_from(["koyomi", "event", "list", "-p", "w"]);
        match cli.command {
            Some(Commands::Event(event_args)) => match event_args.command {
                EventCommand::List(args) => {
                    assert_eq!(args.period, Period::Week);
                }
                _ => panic!("Expected List subcommand"),
            },
            _ => panic!("Expected Event command"),
        }
    }

    #[test]
    fn cli_parses_event_list_command_with_all_options() {
        let cli = Cli::parse_from([
            "koyomi",
            "event",
            "list",
            "--period",
            "month",
            "--calendar",
            "work@example.com",
            "--details",
            "--limit",
            "100",
        ]);
        match cli.command {
            Some(Commands::Event(event_args)) => match event_args.command {
                EventCommand::List(args) => {
                    assert_eq!(args.period, Period::Month);
                    assert_eq!(cli.calendar, "work@example.com");
                    assert!(args.details);
                    assert_eq!(args.limit, 100);
                }
                _ => panic!("Expected List subcommand"),
            },
            _ => panic!("Expected Event command"),
        }
    }

    #[test]
    fn cli_parses_event_list_command_with_short_options() {
        let cli = Cli::parse_from([
            "koyomi",
            "event",
            "list",
            "-p",
            "m",
            "-c",
            "test@example.com",
            "-d",
            "-n",
            "50",
        ]);
        match cli.command {
            Some(Commands::Event(event_args)) => match event_args.command {
                EventCommand::List(args) => {
                    assert_eq!(args.period, Period::Month);
                    assert_eq!(cli.calendar, "test@example.com");
                    assert!(args.details);
                    assert_eq!(args.limit, 50);
                }
                _ => panic!("Expected List subcommand"),
            },
            _ => panic!("Expected Event command"),
        }
    }

    #[test]
    fn cli_parses_event_add_command() {
        let cli = Cli::parse_from([
            "koyomi",
            "event",
            "add",
            "--summary",
            "Meeting",
            "--start",
            "2026-02-17T10:00:00+09:00",
            "--end",
            "2026-02-17T11:00:00+09:00",
        ]);
        match cli.command {
            Some(Commands::Event(event_args)) => match event_args.command {
                EventCommand::Add(args) => {
                    assert_eq!(args.summary, "Meeting");
                    assert_eq!(args.start, "2026-02-17T10:00:00+09:00");
                    assert_eq!(args.end, "2026-02-17T11:00:00+09:00");
                    assert!(args.description.is_none());
                    assert!(args.location.is_none());
                }
                _ => panic!("Expected Add subcommand"),
            },
            _ => panic!("Expected Event command"),
        }
    }

    #[test]
    fn cli_parses_event_add_command_with_optional_fields() {
        let cli = Cli::parse_from([
            "koyomi",
            "event",
            "add",
            "--summary",
            "Meeting",
            "--start",
            "2026-02-17",
            "--end",
            "2026-02-18",
            "--description",
            "Team sync",
            "--location",
            "Room A",
        ]);
        match cli.command {
            Some(Commands::Event(event_args)) => match event_args.command {
                EventCommand::Add(args) => {
                    assert_eq!(args.description, Some("Team sync".to_string()));
                    assert_eq!(args.location, Some("Room A".to_string()));
                }
                _ => panic!("Expected Add subcommand"),
            },
            _ => panic!("Expected Event command"),
        }
    }

    #[test]
    fn cli_parses_event_update_command() {
        let cli = Cli::parse_from([
            "koyomi",
            "event",
            "update",
            "event123",
            "--summary",
            "Updated Title",
        ]);
        match cli.command {
            Some(Commands::Event(event_args)) => match event_args.command {
                EventCommand::Update(args) => {
                    assert_eq!(args.id, "event123");
                    assert_eq!(args.summary, Some("Updated Title".to_string()));
                    assert!(args.start.is_none());
                    assert!(args.end.is_none());
                    assert!(args.description.is_none());
                    assert!(args.location.is_none());
                }
                _ => panic!("Expected Update subcommand"),
            },
            _ => panic!("Expected Event command"),
        }
    }

    #[test]
    fn cli_parses_event_delete_command() {
        let cli = Cli::parse_from(["koyomi", "event", "delete", "event123"]);
        match cli.command {
            Some(Commands::Event(event_args)) => match event_args.command {
                EventCommand::Delete(args) => {
                    assert_eq!(args.id, "event123");
                }
                _ => panic!("Expected Delete subcommand"),
            },
            _ => panic!("Expected Event command"),
        }
    }

    #[test]
    fn cli_event_add_requires_summary() {
        let result = Cli::try_parse_from([
            "koyomi",
            "event",
            "add",
            "--start",
            "2026-02-17T10:00:00+09:00",
            "--end",
            "2026-02-17T11:00:00+09:00",
        ]);
        assert!(result.is_err());
    }

    #[test]
    fn cli_event_add_requires_start() {
        let result = Cli::try_parse_from([
            "koyomi",
            "event",
            "add",
            "--summary",
            "Meeting",
            "--end",
            "2026-02-17T11:00:00+09:00",
        ]);
        assert!(result.is_err());
    }

    #[test]
    fn cli_event_add_requires_end() {
        let result = Cli::try_parse_from([
            "koyomi",
            "event",
            "add",
            "--summary",
            "Meeting",
            "--start",
            "2026-02-17T10:00:00+09:00",
        ]);
        assert!(result.is_err());
    }

    #[test]
    fn cli_event_delete_requires_id() {
        let result = Cli::try_parse_from(["koyomi", "event", "delete"]);
        assert!(result.is_err());
    }

    #[test]
    fn cli_event_update_requires_id() {
        let result = Cli::try_parse_from(["koyomi", "event", "update", "--summary", "New"]);
        assert!(result.is_err());
    }

    // --- parse_event_datetime tests ---

    #[test]
    fn parse_event_datetime_rfc3339() {
        let result = parse_event_datetime("2026-02-17T10:00:00+09:00");
        assert!(result.is_ok());
        match result.unwrap() {
            EventDateTime::DateTime { date_time, time_zone } => {
                assert_eq!(
                    date_time,
                    chrono::DateTime::parse_from_rfc3339("2026-02-17T10:00:00+09:00").unwrap()
                );
                assert!(time_zone.is_none());
            }
            EventDateTime::Date { .. } => panic!("Expected DateTime variant"),
        }
    }

    #[test]
    fn parse_event_datetime_date_only() {
        let result = parse_event_datetime("2026-02-17");
        assert!(result.is_ok());
        match result.unwrap() {
            EventDateTime::Date { date } => {
                assert_eq!(date, chrono::NaiveDate::from_ymd_opt(2026, 2, 17).unwrap());
            }
            EventDateTime::DateTime { .. } => panic!("Expected Date variant"),
        }
    }

    #[test]
    fn parse_event_datetime_rfc3339_utc() {
        let result = parse_event_datetime("2026-02-17T01:00:00Z");
        assert!(result.is_ok());
        match result.unwrap() {
            EventDateTime::DateTime { .. } => {}
            EventDateTime::Date { .. } => panic!("Expected DateTime variant"),
        }
    }

    #[test]
    fn parse_event_datetime_rejects_invalid() {
        let result = parse_event_datetime("not-a-date");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("invalid date/time"));
    }

    #[test]
    fn parse_event_datetime_rejects_partial_time() {
        let result = parse_event_datetime("2026-02-17T10:00");
        assert!(result.is_err());
    }
}
