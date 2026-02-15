use std::time::Duration;

use tracing::warn;

use crate::Result;
use crate::auth::token::StoredToken;
use crate::calendar::{self, CalendarEvents, ListEventsConfig};

const DEFAULT_TIMEOUT_SECS: u64 = 30;
const MAX_RETRIES: u32 = 3;

/// High-level API client for Google Calendar operations.
///
/// Wraps an HTTP client and provides retry with exponential backoff
/// for transient errors (HTTP 429, 5xx).
#[derive(Clone)]
pub struct Client {
    http: reqwest::Client,
}

impl Client {
    /// Creates a new client with default timeout settings.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP client cannot be initialized
    /// (e.g., TLS backend failure).
    pub fn new() -> Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .connect_timeout(Duration::from_secs(10))
            .user_agent(format!("koyomi/{}", env!("CARGO_PKG_VERSION")))
            .build()?;
        Ok(Self { http })
    }

    /// Returns a reference to the internal HTTP client.
    #[must_use]
    pub(crate) fn http(&self) -> &reqwest::Client {
        &self.http
    }

    /// Send an authenticated GET request.
    pub(crate) async fn get(
        &self,
        url: &str,
        access_token: &str,
    ) -> std::result::Result<reqwest::Response, reqwest::Error> {
        let mut retries = 0u32;
        loop {
            let response = self
                .http
                .get(url)
                .header(reqwest::header::ACCEPT, "application/json")
                .bearer_auth(access_token)
                .send()
                .await?;

            let status = response.status();
            if status == reqwest::StatusCode::TOO_MANY_REQUESTS || status.is_server_error() {
                retries += 1;
                if retries > MAX_RETRIES {
                    return Ok(response);
                }

                let retry_after = response
                    .headers()
                    .get(reqwest::header::RETRY_AFTER)
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.parse::<u64>().ok());

                let base_secs = 2u64.pow(retries - 1); // 1, 2, 4
                let wait_secs = retry_after.map_or(base_secs, |ra| ra.max(base_secs));

                warn!("HTTP {status} — retrying in {wait_secs}s (attempt {retries}/{MAX_RETRIES})");
                tokio::time::sleep(tokio::time::Duration::from_secs(wait_secs)).await;
                continue;
            }

            return Ok(response);
        }
    }

    /// Lists calendar events for the given token and configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The HTTP request fails
    /// - The server returns an error response
    /// - Time range calculation fails
    pub async fn list_events(
        &self,
        token: &StoredToken,
        config: &ListEventsConfig,
    ) -> Result<CalendarEvents> {
        calendar::list_events(self, token.access_token(), config, calendar::CALENDAR_API_BASE_URL)
            .await
    }
}
