use crate::Result;
use crate::auth::token::StoredToken;
use crate::calendar::{self, CalendarEvents, ListEventsConfig};

/// High-level API client for Google Calendar operations.
///
/// Wraps an HTTP client and provides simplified methods for
/// interacting with the Google Calendar API.
pub struct Client {
    http: reqwest::Client,
}

impl Client {
    /// Creates a new `Client` with default HTTP settings.
    #[must_use]
    pub fn new() -> Self {
        Self { http: reqwest::Client::new() }
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
