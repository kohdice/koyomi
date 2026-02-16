use chrono::{DateTime, Utc};
use serde::Deserialize;
use tracing::{debug, info};

use super::types::{CalendarEvents, Event, ReminderOverride};
use crate::{CalendarError, Result};

pub(crate) const API_BASE_URL: &str = "https://www.googleapis.com/calendar/v3";

/// Fields to request from CalendarList info endpoint (Partial Response)
///
/// <https://developers.google.com/calendar/api/guides/performance#partial-response>
const CALENDAR_LIST_INFO_FIELDS: &str = "summary,defaultReminders";

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
    pub(crate) time_min: DateTime<Utc>,
    pub(crate) time_max: DateTime<Utc>,
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
    /// - `time_min` is not before `time_max`
    pub fn new(
        calendar_id: String,
        time_min: DateTime<Utc>,
        time_max: DateTime<Utc>,
        max_results: u32,
    ) -> crate::Result<Self> {
        if calendar_id.is_empty() {
            return Err(crate::Error::ConfigInvalid("calendar_id must not be empty".into()));
        }
        if max_results == 0 || max_results > MAX_RESULTS_LIMIT {
            return Err(crate::Error::ConfigInvalid(format!(
                "max_results must be between 1 and {MAX_RESULTS_LIMIT}"
            )));
        }
        if time_min >= time_max {
            return Err(crate::Error::ConfigInvalid("time_min must be before time_max".into()));
        }
        Ok(Self { calendar_id, time_min, time_max, max_results })
    }
}

impl Default for ListEventsConfig {
    fn default() -> Self {
        let tz = super::time_range::TimeZone::default();
        let (time_min, time_max) = super::time_range::for_day(tz.today(), tz)
            .expect("default time range should always be valid");
        Self {
            calendar_id: "primary".to_string(),
            time_min,
            time_max,
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
#[serde(rename_all = "camelCase")]
struct CalendarListInfo {
    summary: Option<String>,
    #[serde(default)]
    default_reminders: Vec<ReminderOverride>,
}

#[derive(Deserialize)]
struct GoogleApiError {
    error: GoogleApiErrorBody,
}

#[derive(Deserialize)]
struct GoogleApiErrorBody {
    message: String,
}

fn extract_error_message(body: &str) -> String {
    serde_json::from_str::<GoogleApiError>(body)
        .map(|e| e.error.message)
        .unwrap_or_else(|_| body.to_string())
}

fn status_to_calendar_error(
    status: reqwest::StatusCode,
    body: String,
    calendar_id: &str,
) -> CalendarError {
    let message = extract_error_message(&body);
    match status.as_u16() {
        401 => CalendarError::Unauthenticated,
        403 => CalendarError::Forbidden { calendar_id: calendar_id.to_string() },
        404 => CalendarError::NotFound { calendar_id: calendar_id.to_string() },
        429 => CalendarError::RateLimitExceeded,
        400 => CalendarError::BadRequest { message },
        status if status >= 500 => CalendarError::ServerError { status, message },
        status => CalendarError::UnexpectedStatus { status, message },
    }
}

/// Get calendar information from the CalendarList API
///
/// Uses the CalendarList endpoint instead of the Calendars endpoint
/// to obtain both the calendar name and default reminders.
///
/// # Errors
///
/// Returns an error if:
/// - The HTTP request fails
/// - The server returns an error response
async fn get_calendar_info(
    client: &crate::client::Client,
    access_token: &str,
    calendar_id: &str,
    base_url: &str,
) -> Result<CalendarListInfo> {
    let url = reqwest::Url::parse_with_params(
        &format!("{}/users/me/calendarList/{}", base_url, urlencoding::encode(calendar_id)),
        &[("fields", CALENDAR_LIST_INFO_FIELDS)],
    )
    .map_err(|e| CalendarError::BadRequest {
        message: format!("Failed to construct calendar info URL: {e}"),
    })?;

    debug!("Fetching calendar info: {}", url);

    let response = client.get(url.as_str(), access_token).await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_else(|e| {
            tracing::debug!("Failed to read error response body: {}", e);
            format!("(failed to read response body: {e})")
        });
        return Err(status_to_calendar_error(status, body, calendar_id).into());
    }

    let body = response.text().await?;
    serde_json::from_str(&body).map_err(|e| {
        CalendarError::BadRequest {
            message: format!("Failed to parse calendar info response: {e}"),
        }
        .into()
    })
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
    let time_min = config.time_min;
    let time_max = config.time_max;

    info!("Listing events for calendar '{}' from {} to {}", config.calendar_id, time_min, time_max);

    let calendar_info =
        get_calendar_info(client, access_token, &config.calendar_id, base_url).await?;
    let calendar_name = calendar_info.summary.unwrap_or_else(|| {
        tracing::warn!(
            "Calendar '{}' has no summary; using calendar ID as name",
            config.calendar_id
        );
        config.calendar_id.clone()
    });
    let default_reminders = calendar_info.default_reminders;

    let mut all_events: Vec<Event> = Vec::new();
    let mut page_token: Option<String> = None;
    let mut page_count: u32 = 0;
    let mut truncated = false;

    loop {
        page_count += 1;
        if page_count > MAX_PAGES {
            tracing::warn!(
                "Pagination exceeded {} pages; stopping to prevent infinite loop",
                MAX_PAGES
            );
            truncated = true;
            break;
        }

        let mut params: Vec<(&str, String)> = vec![
            ("maxResults", config.max_results.to_string()),
            ("timeMin", time_min.to_rfc3339()),
            ("timeMax", time_max.to_rfc3339()),
            ("singleEvents", "true".to_string()),
            ("orderBy", "startTime".to_string()),
            ("fields", EVENTS_LIST_FIELDS.to_string()),
        ];
        if let Some(token) = &page_token {
            params.push(("pageToken", token.clone()));
        }
        let url = reqwest::Url::parse_with_params(
            &format!("{}/calendars/{}/events", base_url, urlencoding::encode(&config.calendar_id)),
            &params,
        )
        .map_err(|e| CalendarError::BadRequest {
            message: format!("Failed to construct events list URL: {e}"),
        })?;

        debug!("Fetching events: {}", url);

        let response = client.get(url.as_str(), access_token).await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_else(|e| {
                tracing::debug!("Failed to read error response body: {}", e);
                format!("(failed to read response body: {e})")
            });
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
            truncated = true;
            break;
        }

        match page.next_page_token {
            Some(token) => page_token = Some(token),
            None => break,
        }
    }

    if !default_reminders.is_empty() {
        for event in &mut all_events {
            if let Some(reminders) = &mut event.reminders
                && reminders.use_default
                && reminders.overrides.is_empty()
            {
                reminders.overrides = default_reminders.clone();
            }
        }
    }

    info!("Found {} events", all_events.len());

    Ok(CalendarEvents { calendar: calendar_name, events: all_events, truncated })
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{header, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn sample_time_range() -> (DateTime<Utc>, DateTime<Utc>) {
        use super::super::time_range::TimeZone;
        use chrono::NaiveDate;
        super::super::time_range::for_day(
            NaiveDate::from_ymd_opt(2026, 2, 16).unwrap(),
            TimeZone::Jst,
        )
        .unwrap()
    }

    #[test]
    fn list_events_config_default() {
        let config = ListEventsConfig::default();
        assert_eq!(config.calendar_id, "primary");
        assert!(config.time_min < config.time_max);
        assert_eq!(config.max_results, 250);
    }

    #[tokio::test]
    async fn get_calendar_info_returns_summary() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/users/me/calendarList/primary"))
            .and(header("Authorization", "Bearer test_token"))
            .and(query_param("fields", CALENDAR_LIST_INFO_FIELDS))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "summary": "My Calendar",
                "defaultReminders": [{"method": "popup", "minutes": 10}]
            })))
            .mount(&mock_server)
            .await;

        let client = crate::client::Client::new().unwrap();
        let token = "test_token";
        let result = get_calendar_info(&client, token, "primary", &mock_server.uri()).await;

        assert!(result.is_ok());
        let info = result.unwrap();
        assert_eq!(info.summary, Some("My Calendar".to_string()));
        assert_eq!(info.default_reminders.len(), 1);
        assert_eq!(info.default_reminders[0].method, super::super::types::ReminderMethod::Popup);
        assert_eq!(info.default_reminders[0].minutes, 10);
    }

    #[tokio::test]
    async fn get_calendar_info_returns_none_when_no_summary() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/users/me/calendarList/primary"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
            .mount(&mock_server)
            .await;

        let client = crate::client::Client::new().unwrap();
        let token = "test_token";
        let result = get_calendar_info(&client, token, "primary", &mock_server.uri()).await;

        assert!(result.is_ok());
        let info = result.unwrap();
        assert!(info.summary.is_none());
        assert!(info.default_reminders.is_empty());
    }

    #[tokio::test]
    async fn get_calendar_info_returns_error_on_failure() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/users/me/calendarList/primary"))
            .respond_with(ResponseTemplate::new(404).set_body_string("Not Found"))
            .mount(&mock_server)
            .await;

        let client = crate::client::Client::new().unwrap();
        let token = "test_token";
        let result = get_calendar_info(&client, token, "primary", &mock_server.uri()).await;

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.to_string().contains("Calendar not found: primary"));
    }

    #[tokio::test]
    async fn list_events_returns_events() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/users/me/calendarList/primary"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "summary": "My Calendar",
                "defaultReminders": [{"method": "popup", "minutes": 10}]
            })))
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/calendars/primary/events"))
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
            .and(path("/users/me/calendarList/primary"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "summary": "My Calendar",
                "defaultReminders": [{"method": "popup", "minutes": 10}]
            })))
            .mount(&mock_server)
            .await;

        // First page
        Mock::given(method("GET"))
            .and(path("/calendars/primary/events"))
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
            .and(path("/calendars/primary/events"))
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
    async fn get_calendar_info_returns_unauthenticated_on_401() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/users/me/calendarList/primary"))
            .respond_with(ResponseTemplate::new(401).set_body_string("Unauthorized"))
            .mount(&mock_server)
            .await;

        let client = crate::client::Client::new().unwrap();
        let token = "invalid_token";
        let result = get_calendar_info(&client, token, "primary", &mock_server.uri()).await;

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.to_string().contains("Authentication required"));
    }

    #[tokio::test]
    async fn get_calendar_info_returns_forbidden_on_403() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/users/me/calendarList/private%40example.com"))
            .respond_with(ResponseTemplate::new(403).set_body_string("Forbidden"))
            .mount(&mock_server)
            .await;

        let client = crate::client::Client::new().unwrap();
        let token = "test_token";
        let result =
            get_calendar_info(&client, token, "private@example.com", &mock_server.uri()).await;

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
        let (time_min, time_max) = sample_time_range();
        let result = ListEventsConfig::new(String::new(), time_min, time_max, 250);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("calendar_id must not be empty"));
    }

    #[test]
    fn list_events_config_new_validates_max_results_zero() {
        let (time_min, time_max) = sample_time_range();
        let result = ListEventsConfig::new("primary".to_string(), time_min, time_max, 0);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("max_results must be between"));
    }

    #[test]
    fn list_events_config_new_validates_max_results_over_limit() {
        let (time_min, time_max) = sample_time_range();
        let result = ListEventsConfig::new("primary".to_string(), time_min, time_max, 2501);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("max_results must be between"));
    }

    #[test]
    fn list_events_config_new_validates_time_order() {
        let (time_min, time_max) = sample_time_range();
        let result = ListEventsConfig::new("primary".to_string(), time_max, time_min, 250);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("time_min must be before time_max"));
    }

    #[test]
    fn list_events_config_new_validates_equal_times() {
        let (time_min, _) = sample_time_range();
        let result = ListEventsConfig::new("primary".to_string(), time_min, time_min, 250);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("time_min must be before time_max"));
    }

    #[test]
    fn list_events_config_new_accepts_valid_params() {
        let (time_min, time_max) = sample_time_range();
        let result = ListEventsConfig::new("primary".to_string(), time_min, time_max, 100);
        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.calendar_id, "primary");
        assert_eq!(config.time_min, time_min);
        assert_eq!(config.time_max, time_max);
        assert_eq!(config.max_results, 100);
    }

    #[test]
    fn list_events_config_new_accepts_boundary_values() {
        let (time_min, time_max) = sample_time_range();
        assert!(ListEventsConfig::new("primary".to_string(), time_min, time_max, 1).is_ok());
        assert!(ListEventsConfig::new("primary".to_string(), time_min, time_max, 2500).is_ok());
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
        let token = "token";
        let url = format!("{}/test", mock_server.uri());
        let response = client.get(&url, token).await.unwrap();
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
        let token = "token";
        let url = format!("{}/test", mock_server.uri());
        let response = client.get(&url, token).await.unwrap();
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
        let token = "token";
        let url = format!("{}/test", mock_server.uri());
        let response = client.get(&url, token).await.unwrap();
        assert_eq!(response.status(), 503);
    }

    #[tokio::test]
    async fn list_events_resolves_use_default_reminders() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/users/me/calendarList/primary"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "summary": "My Calendar",
                "defaultReminders": [{"method": "popup", "minutes": 10}]
            })))
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/calendars/primary/events"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "items": [
                    {
                        "summary": "Default Reminder Event",
                        "start": {
                            "dateTime": "2025-12-09T10:00:00+09:00",
                            "timeZone": "Asia/Tokyo"
                        },
                        "end": {
                            "dateTime": "2025-12-09T11:00:00+09:00",
                            "timeZone": "Asia/Tokyo"
                        },
                        "reminders": {
                            "useDefault": true
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

        let events = result.unwrap();
        let reminders = events.events[0].reminders.as_ref().unwrap();
        assert!(reminders.use_default);
        assert_eq!(reminders.overrides.len(), 1);
        assert_eq!(reminders.overrides[0].method, super::super::types::ReminderMethod::Popup);
        assert_eq!(reminders.overrides[0].minutes, 10);
    }

    #[tokio::test]
    async fn list_events_preserves_custom_overrides() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/users/me/calendarList/primary"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "summary": "My Calendar",
                "defaultReminders": [{"method": "popup", "minutes": 10}]
            })))
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/calendars/primary/events"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "items": [
                    {
                        "summary": "Custom Reminder Event",
                        "start": {
                            "dateTime": "2025-12-09T10:00:00+09:00",
                            "timeZone": "Asia/Tokyo"
                        },
                        "end": {
                            "dateTime": "2025-12-09T11:00:00+09:00",
                            "timeZone": "Asia/Tokyo"
                        },
                        "reminders": {
                            "useDefault": false,
                            "overrides": [
                                {"method": "email", "minutes": 1440}
                            ]
                        },
                        "htmlLink": "https://calendar.google.com/event?eid=abc456"
                    }
                ]
            })))
            .mount(&mock_server)
            .await;

        let client = crate::client::Client::new().unwrap();
        let config = ListEventsConfig::default();
        let result = list_events(&client, "test_token", &config, &mock_server.uri()).await;

        let events = result.unwrap();
        let reminders = events.events[0].reminders.as_ref().unwrap();
        assert!(!reminders.use_default);
        assert_eq!(reminders.overrides.len(), 1);
        assert_eq!(reminders.overrides[0].method, super::super::types::ReminderMethod::Email);
        assert_eq!(reminders.overrides[0].minutes, 1440);
    }

    #[tokio::test]
    async fn list_events_no_resolve_when_defaults_empty() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/users/me/calendarList/primary"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "summary": "My Calendar",
                "defaultReminders": []
            })))
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/calendars/primary/events"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "items": [
                    {
                        "summary": "No Default Event",
                        "start": {
                            "dateTime": "2025-12-09T10:00:00+09:00",
                            "timeZone": "Asia/Tokyo"
                        },
                        "end": {
                            "dateTime": "2025-12-09T11:00:00+09:00",
                            "timeZone": "Asia/Tokyo"
                        },
                        "reminders": {
                            "useDefault": true
                        },
                        "htmlLink": "https://calendar.google.com/event?eid=abc789"
                    }
                ]
            })))
            .mount(&mock_server)
            .await;

        let client = crate::client::Client::new().unwrap();
        let config = ListEventsConfig::default();
        let result = list_events(&client, "test_token", &config, &mock_server.uri()).await;

        let events = result.unwrap();
        let reminders = events.events[0].reminders.as_ref().unwrap();
        assert!(reminders.use_default);
        assert!(reminders.overrides.is_empty());
    }
}
