use chrono::{DateTime, Local, TimeDelta, Utc};
use serde::Deserialize;
use tracing::{debug, info};

use super::types::{CalendarEvents, Event, EventPeriod};
use crate::{CalendarError, Result};

pub(crate) const CALENDAR_API_BASE_URL: &str = "https://www.googleapis.com/calendar/v3/calendars";

/// Fields to request from Calendar info endpoint (Partial Response)
///
/// <https://developers.google.com/calendar/api/guides/performance#partial-response>
const CALENDAR_INFO_FIELDS: &str = "summary";

/// Fields to request from Events list endpoint (Partial Response)
///
/// <https://developers.google.com/calendar/api/guides/performance#partial-response>
const EVENTS_LIST_FIELDS: &str = "\
    nextPageToken,\
    items(\
        id,\
        summary,\
        status,\
        organizer(email,displayName,self),\
        location,\
        start(date,dateTime,timeZone),\
        end(date,dateTime,timeZone),\
        description,\
        attendees(email,displayName,responseStatus,resource),\
        reminders(useDefault,overrides(method,minutes)),\
        conferenceData(entryPoints(entryPointType,uri),conferenceSolution(name)),\
        htmlLink\
    )";

/// Maximum value allowed for `max_results` (Google Calendar API limit)
pub const MAX_RESULTS_LIMIT: u32 = 2500;

/// Default value for `max_results` (matches Google Calendar API default)
const DEFAULT_MAX_RESULTS: u32 = 250;

/// Maximum number of pagination requests as a safeguard against infinite loops
const MAX_PAGES: u32 = 50;

#[derive(Debug, Clone)]
pub struct ListEventsConfig {
    pub(crate) calendar_id: String,
    pub(crate) period: EventPeriod,
    pub(crate) max_results: u32,
}

impl ListEventsConfig {
    /// Create a new `ListEventsConfig` with validation.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - `calendar_id` is empty
    /// - `max_results` is outside the range `1..=2500`
    pub fn new(calendar_id: String, period: EventPeriod, max_results: u32) -> crate::Result<Self> {
        if calendar_id.is_empty() {
            return Err(crate::Error::ConfigInvalid("calendar_id must not be empty".into()));
        }
        if max_results == 0 || max_results > MAX_RESULTS_LIMIT {
            return Err(crate::Error::ConfigInvalid(format!(
                "max_results must be between 1 and {MAX_RESULTS_LIMIT}"
            )));
        }
        Ok(Self { calendar_id, period, max_results })
    }
}

impl Default for ListEventsConfig {
    fn default() -> Self {
        Self {
            calendar_id: "primary".to_string(),
            period: EventPeriod::default(),
            max_results: DEFAULT_MAX_RESULTS,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PageResponse {
    items: Option<Vec<Event>>,
    next_page_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CalendarInfo {
    summary: Option<String>,
}

fn status_to_calendar_error(
    status: reqwest::StatusCode,
    body: String,
    calendar_id: &str,
) -> CalendarError {
    match status.as_u16() {
        401 => CalendarError::Unauthenticated,
        403 => CalendarError::Forbidden { calendar_id: calendar_id.to_string() },
        404 => CalendarError::NotFound { calendar_id: calendar_id.to_string() },
        429 => CalendarError::RateLimitExceeded,
        400 => CalendarError::BadRequest { message: body },
        status if status >= 500 => CalendarError::ServerError { status, message: body },
        status => CalendarError::UnexpectedStatus { status, message: body },
    }
}

/// Get the name of a calendar
///
/// # Errors
///
/// Returns an error if:
/// - The HTTP request fails
/// - The server returns an error response
pub(crate) async fn get_calendar_name(
    client: &crate::client::Client,
    access_token: &str,
    calendar_id: &str,
    base_url: &str,
) -> Result<String> {
    let url = format!(
        "{}/{}?fields={}",
        base_url,
        urlencoding::encode(calendar_id),
        CALENDAR_INFO_FIELDS
    );

    debug!("Fetching calendar info: {}", url);

    let response = client.get(&url, access_token).await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response
            .text()
            .await
            .unwrap_or_else(|e| format!("(failed to read response body: {e})"));
        return Err(status_to_calendar_error(status, body, calendar_id).into());
    }

    let body = response.text().await?;
    let info: CalendarInfo =
        serde_json::from_str(&body).map_err(|e| CalendarError::BadRequest {
            message: format!("Failed to parse calendar info response: {e}"),
        })?;

    Ok(info.summary.unwrap_or_else(|| calendar_id.to_string()))
}

/// Calculate time range based on period
///
/// # Errors
///
/// Returns an error if the time or timezone conversion fails
fn calculate_time_range(period: EventPeriod) -> Result<(DateTime<Utc>, DateTime<Utc>)> {
    let now = Local::now();
    let today_start = now
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| CalendarError::InvalidTime("failed to create midnight time".into()))?
        .and_local_timezone(now.timezone())
        .single()
        .ok_or_else(|| CalendarError::InvalidTime("timezone conversion failed".into()))?;

    let (start, end) = match period {
        EventPeriod::Day => {
            let end = today_start + TimeDelta::days(1);
            (today_start, end)
        }
        EventPeriod::Week => {
            let end = today_start + TimeDelta::days(7);
            (today_start, end)
        }
        EventPeriod::Month => {
            let end = today_start + TimeDelta::days(30);
            (today_start, end)
        }
    };

    Ok((start.with_timezone(&Utc), end.with_timezone(&Utc)))
}

/// List calendar events
///
/// # Errors
///
/// Returns an error if:
/// - The HTTP request fails
/// - The server returns an error response
pub(crate) async fn list_events(
    client: &crate::client::Client,
    access_token: &str,
    config: &ListEventsConfig,
    base_url: &str,
) -> Result<CalendarEvents> {
    let (time_min, time_max) = calculate_time_range(config.period)?;

    info!("Listing events for calendar '{}' from {} to {}", config.calendar_id, time_min, time_max);

    let calendar_name =
        get_calendar_name(client, access_token, &config.calendar_id, base_url).await?;

    let mut all_events: Vec<Event> = Vec::new();
    let mut page_token: Option<String> = None;
    let mut page_count: u32 = 0;

    loop {
        page_count += 1;
        if page_count > MAX_PAGES {
            tracing::warn!(
                "Pagination exceeded {} pages; stopping to prevent infinite loop",
                MAX_PAGES
            );
            eprintln!(
                "Warning: pagination limit ({MAX_PAGES} pages) reached; results may be incomplete."
            );
            break;
        }

        let mut url = format!(
            "{}/{}/events?maxResults={}&timeMin={}&timeMax={}&singleEvents=true&orderBy=startTime&fields={}",
            base_url,
            urlencoding::encode(&config.calendar_id),
            config.max_results,
            urlencoding::encode(&time_min.to_rfc3339()),
            urlencoding::encode(&time_max.to_rfc3339()),
            urlencoding::encode(EVENTS_LIST_FIELDS),
        );

        if let Some(token) = &page_token {
            url.push_str(&format!("&pageToken={}", urlencoding::encode(token)));
        }

        debug!("Fetching events: {}", url);

        let response = client.get(&url, access_token).await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response
                .text()
                .await
                .unwrap_or_else(|e| format!("(failed to read response body: {e})"));
            return Err(status_to_calendar_error(status, body, &config.calendar_id).into());
        }

        let body = response.text().await?;
        let page: PageResponse =
            serde_json::from_str(&body).map_err(|e| CalendarError::BadRequest {
                message: format!("Failed to parse events list response: {e}"),
            })?;

        if let Some(items) = page.items {
            all_events.extend(items);
        }

        if all_events.len() >= config.max_results as usize {
            all_events.truncate(config.max_results as usize);
            break;
        }

        match page.next_page_token {
            Some(token) => page_token = Some(token),
            None => break,
        }
    }

    info!("Found {} events", all_events.len());

    Ok(CalendarEvents { calendar: calendar_name, events: all_events })
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{header, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn list_events_config_default() {
        let config = ListEventsConfig::default();
        assert_eq!(config.calendar_id, "primary");
        assert_eq!(config.period, EventPeriod::Day);
        assert_eq!(config.max_results, 250);
    }

    #[test]
    fn calculate_time_range_day() {
        let (start, end) = calculate_time_range(EventPeriod::Day).unwrap();
        let diff = end - start;
        assert_eq!(diff.num_days(), 1);
    }

    #[test]
    fn calculate_time_range_week() {
        let (start, end) = calculate_time_range(EventPeriod::Week).unwrap();
        let diff = end - start;
        assert_eq!(diff.num_days(), 7);
    }

    #[test]
    fn calculate_time_range_month() {
        let (start, end) = calculate_time_range(EventPeriod::Month).unwrap();
        let diff = end - start;
        assert_eq!(diff.num_days(), 30);
    }

    #[tokio::test]
    async fn get_calendar_name_returns_summary() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/primary"))
            .and(header("Authorization", "Bearer test_token"))
            .and(query_param("fields", CALENDAR_INFO_FIELDS))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "summary": "My Calendar"
            })))
            .mount(&mock_server)
            .await;

        let client = crate::client::Client::new().unwrap();
        let result = get_calendar_name(&client, "test_token", "primary", &mock_server.uri()).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "My Calendar");
    }

    #[tokio::test]
    async fn get_calendar_name_returns_id_when_no_summary() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/primary"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
            .mount(&mock_server)
            .await;

        let client = crate::client::Client::new().unwrap();
        let result = get_calendar_name(&client, "test_token", "primary", &mock_server.uri()).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "primary");
    }

    #[tokio::test]
    async fn get_calendar_name_returns_error_on_failure() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/primary"))
            .respond_with(ResponseTemplate::new(404).set_body_string("Not Found"))
            .mount(&mock_server)
            .await;

        let client = crate::client::Client::new().unwrap();
        let result = get_calendar_name(&client, "test_token", "primary", &mock_server.uri()).await;

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.to_string().contains("Calendar not found: primary"));
    }

    #[tokio::test]
    async fn list_events_returns_events() {
        let mock_server = MockServer::start().await;

        // Mock calendar info
        Mock::given(method("GET"))
            .and(path("/primary"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "summary": "My Calendar"
            })))
            .mount(&mock_server)
            .await;

        // Mock events list
        Mock::given(method("GET"))
            .and(path("/primary/events"))
            .and(query_param("singleEvents", "true"))
            .and(query_param("orderBy", "startTime"))
            .and(query_param("fields", EVENTS_LIST_FIELDS))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "items": [
                    {
                        "summary": "Test Event",
                        "status": "confirmed",
                        "start": {
                            "dateTime": "2025-12-09T10:00:00+09:00",
                            "timeZone": "Asia/Tokyo"
                        },
                        "end": {
                            "dateTime": "2025-12-09T11:00:00+09:00",
                            "timeZone": "Asia/Tokyo"
                        },
                        "htmlLink": "https://calendar.google.com/event?eid=abc123"
                    }
                ]
            })))
            .mount(&mock_server)
            .await;

        let client = crate::client::Client::new().unwrap();
        let config = ListEventsConfig::default();
        let result = list_events(&client, "test_token", &config, &mock_server.uri()).await;

        assert!(result.is_ok());
        let events = result.unwrap();
        assert_eq!(events.calendar, "My Calendar");
        assert_eq!(events.events.len(), 1);
        assert_eq!(events.events[0].summary, Some("Test Event".to_string()));
    }

    #[tokio::test]
    async fn list_events_handles_pagination() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/primary"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "summary": "My Calendar"
            })))
            .mount(&mock_server)
            .await;

        // First page
        Mock::given(method("GET"))
            .and(path("/primary/events"))
            .and(wiremock::matchers::query_param_is_missing("pageToken"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "items": [
                    {
                        "summary": "Event 1",
                        "htmlLink": "https://calendar.google.com/event?eid=1"
                    }
                ],
                "nextPageToken": "token123"
            })))
            .mount(&mock_server)
            .await;

        // Second page
        Mock::given(method("GET"))
            .and(path("/primary/events"))
            .and(query_param("pageToken", "token123"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "items": [
                    {
                        "summary": "Event 2",
                        "htmlLink": "https://calendar.google.com/event?eid=2"
                    }
                ]
            })))
            .mount(&mock_server)
            .await;

        let client = crate::client::Client::new().unwrap();
        let config = ListEventsConfig::default();
        let result = list_events(&client, "test_token", &config, &mock_server.uri()).await;

        assert!(result.is_ok());
        let events = result.unwrap();
        assert_eq!(events.events.len(), 2);
        assert_eq!(events.events[0].summary, Some("Event 1".to_string()));
        assert_eq!(events.events[1].summary, Some("Event 2".to_string()));
    }

    #[test]
    fn status_to_calendar_error_returns_unauthenticated_for_401() {
        let status = reqwest::StatusCode::UNAUTHORIZED;
        let error = status_to_calendar_error(status, "token expired".to_string(), "primary");
        assert!(matches!(error, CalendarError::Unauthenticated));
    }

    #[test]
    fn status_to_calendar_error_returns_forbidden_for_403() {
        let status = reqwest::StatusCode::FORBIDDEN;
        let error =
            status_to_calendar_error(status, "access denied".to_string(), "test@example.com");
        assert!(
            matches!(error, CalendarError::Forbidden { calendar_id } if calendar_id == "test@example.com")
        );
    }

    #[test]
    fn status_to_calendar_error_returns_not_found_for_404() {
        let status = reqwest::StatusCode::NOT_FOUND;
        let error = status_to_calendar_error(status, "not found".to_string(), "primary");
        assert!(
            matches!(error, CalendarError::NotFound { calendar_id } if calendar_id == "primary")
        );
    }

    #[test]
    fn status_to_calendar_error_returns_rate_limit_for_429() {
        let status = reqwest::StatusCode::TOO_MANY_REQUESTS;
        let error = status_to_calendar_error(status, "rate limited".to_string(), "primary");
        assert!(matches!(error, CalendarError::RateLimitExceeded));
    }

    #[test]
    fn status_to_calendar_error_returns_bad_request_for_400() {
        let status = reqwest::StatusCode::BAD_REQUEST;
        let error = status_to_calendar_error(status, "invalid params".to_string(), "primary");
        assert!(
            matches!(error, CalendarError::BadRequest { message } if message == "invalid params")
        );
    }

    #[test]
    fn status_to_calendar_error_returns_server_error_for_5xx() {
        let status = reqwest::StatusCode::INTERNAL_SERVER_ERROR;
        let error = status_to_calendar_error(status, "internal error".to_string(), "primary");
        assert!(matches!(error, CalendarError::ServerError { status: 500, .. }));

        let status = reqwest::StatusCode::SERVICE_UNAVAILABLE;
        let error = status_to_calendar_error(status, "unavailable".to_string(), "primary");
        assert!(matches!(error, CalendarError::ServerError { status: 503, .. }));
    }

    #[tokio::test]
    async fn get_calendar_name_returns_unauthenticated_on_401() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/primary"))
            .respond_with(ResponseTemplate::new(401).set_body_string("Unauthorized"))
            .mount(&mock_server)
            .await;

        let client = crate::client::Client::new().unwrap();
        let result =
            get_calendar_name(&client, "invalid_token", "primary", &mock_server.uri()).await;

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.to_string().contains("Authentication required"));
    }

    #[tokio::test]
    async fn get_calendar_name_returns_forbidden_on_403() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/private%40example.com"))
            .respond_with(ResponseTemplate::new(403).set_body_string("Forbidden"))
            .mount(&mock_server)
            .await;

        let client = crate::client::Client::new().unwrap();
        let result =
            get_calendar_name(&client, "test_token", "private@example.com", &mock_server.uri())
                .await;

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.to_string().contains("Access denied to calendar"));
    }

    #[test]
    fn status_to_calendar_error_returns_unexpected_status_for_unknown_code() {
        let status = reqwest::StatusCode::from_u16(302).unwrap();
        let error = status_to_calendar_error(status, "redirect".to_string(), "primary");
        assert!(matches!(error, CalendarError::UnexpectedStatus { status: 302, .. }));
    }

    #[test]
    fn list_events_config_new_validates_empty_calendar_id() {
        let result = ListEventsConfig::new(String::new(), EventPeriod::Day, 250);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("calendar_id must not be empty"));
    }

    #[test]
    fn list_events_config_new_validates_max_results_zero() {
        let result = ListEventsConfig::new("primary".to_string(), EventPeriod::Day, 0);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("max_results must be between"));
    }

    #[test]
    fn list_events_config_new_validates_max_results_over_limit() {
        let result = ListEventsConfig::new("primary".to_string(), EventPeriod::Day, 2501);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("max_results must be between"));
    }

    #[test]
    fn list_events_config_new_accepts_valid_params() {
        let result = ListEventsConfig::new("primary".to_string(), EventPeriod::Week, 100);
        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.calendar_id, "primary");
        assert_eq!(config.period, EventPeriod::Week);
        assert_eq!(config.max_results, 100);
    }

    #[test]
    fn list_events_config_new_accepts_boundary_values() {
        assert!(ListEventsConfig::new("primary".to_string(), EventPeriod::Day, 1).is_ok());
        assert!(ListEventsConfig::new("primary".to_string(), EventPeriod::Day, 2500).is_ok());
    }

    #[tokio::test]
    async fn client_get_succeeds_on_first_attempt() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/test"))
            .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
            .expect(1)
            .mount(&mock_server)
            .await;

        let client = crate::client::Client::new().unwrap();
        let url = format!("{}/test", mock_server.uri());
        let response = client.get(&url, "token").await.unwrap();
        assert_eq!(response.status(), 200);
    }

    #[tokio::test]
    async fn client_get_retries_on_server_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/test"))
            .respond_with(ResponseTemplate::new(500).set_body_string("error"))
            .up_to_n_times(1)
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/test"))
            .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
            .mount(&mock_server)
            .await;

        let client = crate::client::Client::new().unwrap();
        let url = format!("{}/test", mock_server.uri());
        let response = client.get(&url, "token").await.unwrap();
        assert_eq!(response.status(), 200);
    }

    #[tokio::test]
    async fn client_get_gives_up_after_max_retries() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/test"))
            .respond_with(ResponseTemplate::new(503).set_body_string("unavailable"))
            .mount(&mock_server)
            .await;

        let client = crate::client::Client::new().unwrap();
        let url = format!("{}/test", mock_server.uri());
        let response = client.get(&url, "token").await.unwrap();
        assert_eq!(response.status(), 503);
    }
}
