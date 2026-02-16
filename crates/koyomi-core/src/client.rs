use std::time::Duration;

use tracing::{debug, warn};

use crate::Result;
use crate::auth::token::StoredToken;
use crate::calendar::{self, CalendarEvents, ListEventsConfig};

const DEFAULT_TIMEOUT_SECS: u64 = 30;
const MAX_RETRIES: u32 = 3;

/// High-level API client for Google Calendar operations.
///
/// Wraps an HTTP client and provides retry with exponential backoff
/// for transient errors (HTTP 429, 403 rate-limit, 5xx).
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
            .gzip(true)
            .build()?;
        Ok(Self { http })
    }

    /// Returns a reference to the internal HTTP client.
    #[must_use]
    pub(crate) fn http(&self) -> &reqwest::Client {
        &self.http
    }

    /// Send an authenticated GET request with retry on transient errors.
    ///
    /// Retries on HTTP 429, 5xx, and 403 rate-limit errors with
    /// exponential backoff and jitter.
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

            let is_retryable = status == reqwest::StatusCode::TOO_MANY_REQUESTS
                || status.is_server_error()
                || (status == reqwest::StatusCode::FORBIDDEN && is_rate_limit_forbidden(&response));

            if is_retryable {
                retries += 1;
                if retries > MAX_RETRIES {
                    warn!("HTTP {status} — giving up after {MAX_RETRIES} retries");
                    return Ok(response);
                }

                let retry_after =
                    response.headers().get(reqwest::header::RETRY_AFTER).and_then(|v| {
                        match v.to_str() {
                            Ok(s) => match s.parse::<u64>() {
                                Ok(n) => Some(n),
                                Err(e) => {
                                    debug!("Non-numeric Retry-After header '{s}': {e}");
                                    None
                                }
                            },
                            Err(e) => {
                                debug!("Non-ASCII Retry-After header: {e}");
                                None
                            }
                        }
                    });

                // Consume the response body to allow HTTP/2 connection reuse
                let _ = response.bytes().await;

                let base_secs = 2u64.pow(retries - 1); // 1, 2, 4
                let wait_secs = retry_after.map_or(base_secs, |ra| ra.max(base_secs));
                let jitter_ms = simple_jitter(wait_secs);
                let wait = Duration::from_millis(wait_secs * 1000 + jitter_ms);

                warn!(
                    "HTTP {status} — retrying in {}ms (attempt {retries}/{MAX_RETRIES})",
                    wait.as_millis()
                );
                tokio::time::sleep(wait).await;
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
        calendar::list_events(self, token.access_token(), config, calendar::API_BASE_URL).await
    }
}

/// Read error body helper for consistent error response handling.
pub(crate) async fn read_error_body(response: reqwest::Response) -> String {
    response.text().await.unwrap_or_else(|e| {
        debug!("Failed to read error response body: {}", e);
        format!("(failed to read response body: {e})")
    })
}

/// Check if a 403 response indicates a rate limit error.
///
/// Google Calendar API returns 403 for both rate limits (`rateLimitExceeded`,
/// `userRateLimitExceeded`) and permission errors. We peek at the
/// `x-ratelimit-remaining` header or `Retry-After` presence as hints.
///
/// Since we cannot read the body without consuming the response,
/// we use header-based heuristics when available.
fn is_rate_limit_forbidden(response: &reqwest::Response) -> bool {
    if let Some(remaining) = response.headers().get("x-ratelimit-remaining")
        && let Ok(s) = remaining.to_str()
        && let Ok(n) = s.parse::<u64>()
    {
        return n == 0;
    }
    // Retry-After header presence on 403 is a strong signal of rate limiting
    response.headers().contains_key(reqwest::header::RETRY_AFTER)
}

/// Generate a simple jitter value (0..base_secs*500 ms) without requiring a CSPRNG.
///
/// Uses the low bits of the current time as a cheap entropy source.
fn simple_jitter(base_secs: u64) -> u64 {
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as u64)
        .unwrap_or(0);
    let max_jitter_ms = base_secs * 500; // up to 50% of base
    if max_jitter_ms == 0 { 0 } else { seed % max_jitter_ms }
}
