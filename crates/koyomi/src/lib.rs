mod cli;

use anyhow::Result;
use clap::Parser;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

use crate::cli::{Cli, Commands, Period};

fn init_tracing(verbose: u8) {
    let level = match verbose {
        0 => return, // No logging
        1 => Level::INFO,
        _ => Level::DEBUG,
    };

    FmtSubscriber::builder()
        .with_max_level(level)
        .with_target(false)
        .with_writer(std::io::stderr)
        .without_time()
        .init();
}

/// Convert CLI Period to koyomi_core EventPeriod
fn convert_period(period: Period) -> koyomi_core::calendar::EventPeriod {
    match period {
        Period::Day => koyomi_core::calendar::EventPeriod::Day,
        Period::Week => koyomi_core::calendar::EventPeriod::Week,
        Period::Month => koyomi_core::calendar::EventPeriod::Month,
    }
}

/// Run the CLI application
///
/// # Errors
///
/// Returns an error if any subcommand fails.
pub async fn run() -> Result<()> {
    let cli = Cli::parse();
    init_tracing(cli.verbose);

    match cli.command {
        Commands::Login => {
            koyomi_core::login().await?;
            Ok(())
        }
        Commands::Logout => {
            koyomi_core::logout().await?;
            Ok(())
        }
        Commands::Events { period, calendar, details } => {
            handle_events(period, calendar, details).await
        }
    }
}

async fn handle_events(period: Period, calendar: String, details: bool) -> Result<()> {
    let token = koyomi_core::auth::get_valid_token().await?;

    let config = koyomi_core::calendar::ListEventsConfig {
        calendar_id: calendar,
        period: convert_period(period),
        details,
    };

    let client = reqwest::Client::new();
    let response = koyomi_core::calendar::list_events(
        &client,
        &token.access_token,
        &config,
        koyomi_core::calendar::CALENDAR_API_BASE_URL,
        koyomi_core::calendar::CALENDAR_API_BASE_URL,
    )
    .await?;

    let output = format_output(&response, details)?;
    println!("{}", output);

    Ok(())
}

/// Format the response based on the details flag
fn format_output(
    response: &koyomi_core::calendar::CalendarEventsResponse,
    details: bool,
) -> Result<String> {
    if details {
        Ok(serde_json::to_string_pretty(response)?)
    } else {
        let simplified = SimplifiedResponse::from(response);
        Ok(serde_json::to_string_pretty(&simplified)?)
    }
}

/// Simplified response format for default output
#[derive(serde::Serialize)]
struct SimplifiedResponse {
    calendar: String,
    events: Vec<SimplifiedEvent>,
}

#[derive(serde::Serialize)]
struct SimplifiedEvent {
    summary: Option<String>,
    status: Option<String>,
    organizer: Option<String>,
    location: Option<String>,
    start: Option<String>,
    end: Option<String>,
    description: Option<String>,
    attendees: Vec<String>,
    #[serde(rename = "conferenceData")]
    conference_data: Option<String>,
    #[serde(rename = "htmlLink")]
    html_link: Option<String>,
}

impl From<&koyomi_core::calendar::CalendarEventsResponse> for SimplifiedResponse {
    fn from(response: &koyomi_core::calendar::CalendarEventsResponse) -> Self {
        Self {
            calendar: response.calendar.clone(),
            events: response.events.iter().map(SimplifiedEvent::from).collect(),
        }
    }
}

impl From<&koyomi_core::calendar::Event> for SimplifiedEvent {
    fn from(event: &koyomi_core::calendar::Event) -> Self {
        Self {
            summary: event.summary.clone(),
            status: event.status.clone(),
            organizer: event.organizer.as_ref().and_then(|o| o.display_name.clone()),
            location: event.location.clone(),
            start: event
                .start
                .as_ref()
                .and_then(|dt| dt.date_time.clone().or_else(|| dt.date.clone())),
            end: event.end.as_ref().and_then(|dt| dt.date_time.clone().or_else(|| dt.date.clone())),
            description: event.description.clone(),
            attendees: event
                .attendees
                .iter()
                .filter_map(|a| a.display_name.clone().or_else(|| a.email.clone()))
                .collect(),
            conference_data: event
                .conference_data
                .as_ref()
                .and_then(|cd| cd.conference_solution.as_ref().map(|cs| cs.name.clone())),
            html_link: event.html_link.clone(),
        }
    }
}
