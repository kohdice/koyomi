use std::time::Duration;

use rand::Rng;
use tracing::warn;

use crate::Result;
use crate::auth::token::StoredToken;
use crate::calendar::{self, CalendarEvents, ListEventsConfig};

const DEFAULT_TIMEOUT_SECS: u64 = 30;
const MAX_RETRIES: u32 = 3;

/// Type-safe wrapper for OAuth2 access tokens.
pub(crate) struct AccessToken<'a>(&'a str);

impl<'a> AccessToken<'a> {
    pub(crate) fn new(token: &'a str) -> Self {
        debug_assert!(!token.is_empty(), "access token must not be empty");
        Self(token)
    }

    pub(crate) fn as_str(&self) -> &str {
        self.0
    }
}

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
        access_token: &AccessToken<'_>,
    ) -> std::result::Result<reqwest::Response, reqwest::Error> {
        let mut retries = 0u32;
        loop {
            let response = self.http.get(url).bearer_auth(access_token.as_str()).send().await?;

            let status = response.status();
            if status == reqwest::StatusCode::TOO_MANY_REQUESTS || status.is_server_error() {
                retries += 1;
                if retries > MAX_RETRIES {
                    return Ok(response);
                }

                // Retry-After ヘッダーがあれば尊重する（RFC 7231 Section 7.1.3）
                // 整数秒 or HTTP-date（RFC 2822）の両形式に対応
                let retry_after_ms = response
                    .headers()
                    .get(reqwest::header::RETRY_AFTER)
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| {
                        v.parse::<u64>().map(|secs| secs * 1000).ok().or_else(|| {
                            chrono::DateTime::parse_from_rfc2822(v).ok().and_then(|date| {
                                let diff = date.signed_duration_since(chrono::Utc::now());
                                u64::try_from(diff.num_milliseconds().max(0)).ok()
                            })
                        })
                    });

                // Exponential backoff（初期1秒）+ full jitter（CSPRNG ベース）
                let base_ms = 1000 * 2u64.pow(retries - 1);
                let backoff_ms = rand::rng().random_range(0..=base_ms);
                let wait_ms = retry_after_ms.map_or(backoff_ms, |ra| ra.max(backoff_ms));

                warn!("HTTP {status} — retrying in {wait_ms}ms (attempt {retries}/{MAX_RETRIES})");
                tokio::time::sleep(tokio::time::Duration::from_millis(wait_ms)).await;
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
        let access_token = AccessToken::new(token.access_token());
        calendar::list_events(self, &access_token, config, calendar::CALENDAR_API_BASE_URL).await
    }
}
