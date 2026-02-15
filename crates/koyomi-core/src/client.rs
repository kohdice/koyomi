use std::time::Duration;

use crate::Result;
use crate::auth::token::StoredToken;
use crate::calendar::{self, CalendarEvents, ListEventsConfig};

const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// High-level API client for Google Calendar operations.
///
/// Wraps an HTTP client and provides simplified methods for
/// interacting with the Google Calendar API.
pub struct Client {
    http: reqwest::Client,
}

impl Client {
    #[must_use]
    pub fn new() -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .build()
            .expect("Failed to build HTTP client");
        Self { http }
    }

    /// Returns a reference to the internal HTTP client.
    #[must_use]
    pub fn http(&self) -> &reqwest::Client {
        &self.http
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
        calendar::list_events(
            &self.http,
            &token.access_token,
            config,
            calendar::CALENDAR_API_BASE_URL,
            calendar::CALENDAR_API_BASE_URL,
        )
        .await
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}
