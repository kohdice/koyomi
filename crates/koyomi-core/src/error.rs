use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CalendarError {
    /// Calendar not found (HTTP 404)
    #[error("Calendar not found: {calendar_id}")]
    NotFound { calendar_id: String },

    /// Event not found (HTTP 404)
    #[error("Event not found: {event_id} in calendar {calendar_id}")]
    EventNotFound { calendar_id: String, event_id: String },

    /// Access denied to calendar (HTTP 403)
    #[error("Access denied to calendar: {calendar_id}")]
    Forbidden { calendar_id: String },

    /// Rate limit exceeded (HTTP 429)
    #[error("Rate limit exceeded. Please wait and try again.")]
    RateLimitExceeded,

    /// Authentication required or token expired (HTTP 401)
    #[error("Authentication required. Please re-authenticate.")]
    Unauthenticated,

    /// Invalid request (HTTP 400)
    #[error("Invalid request: {message}")]
    BadRequest { message: String },

    /// Server error (HTTP 5xx)
    #[error("Server error (status {status}): {message}")]
    ServerError { status: u16, message: String },

    /// Unexpected HTTP status code
    #[error("Unexpected HTTP response (status {status}): {message}")]
    UnexpectedStatus { status: u16, message: String },

    /// Invalid time or timezone conversion
    #[error("Invalid time: {0}")]
    InvalidTime(String),
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Could not determine config directory")]
    ConfigDirNotFound,

    #[error("Config file not found: {path}. Please create this file with your OAuth2 credentials.")]
    ConfigFileNotFound { path: PathBuf },

    #[error("Invalid config format: {0}")]
    ConfigInvalid(String),

    #[error("Cannot refresh token: no refresh token available")]
    NoRefreshToken,

    #[error("Authorization timed out. Please try again.")]
    AuthTimeout,

    #[error("Access denied by user.")]
    AuthAccessDenied,

    #[error(
        "Authorization server did not return a refresh token. \
         Please revoke app access at https://myaccount.google.com/permissions and try again."
    )]
    AuthNoRefreshToken,

    #[error("Authentication error: {0}")]
    Auth(String),

    #[error("Token not found. Please run 'koyomi login' to authenticate first.")]
    TokenNotFound,

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Calendar API error: {0}")]
    Calendar(#[from] CalendarError),
}

pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn io_error_converts_from_std_io() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let error: Error = io_error.into();
        assert!(matches!(error, Error::Io(_)));
    }

    #[test]
    fn json_error_converts_from_serde_json() {
        let json_error = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
        let error: Error = json_error.into();
        assert!(matches!(error, Error::Json(_)));
    }

    #[test]
    fn calendar_error_converts_to_error() {
        let calendar_error = CalendarError::NotFound { calendar_id: "primary".to_string() };
        let error: Error = calendar_error.into();
        assert!(matches!(error, Error::Calendar(_)));
    }

    #[test]
    fn token_not_found_is_distinct_from_auth() {
        let auth_error = Error::Auth("some auth error".to_string());
        let token_not_found = Error::TokenNotFound;

        assert!(matches!(auth_error, Error::Auth(_)));
        assert!(matches!(token_not_found, Error::TokenNotFound));
        assert!(!matches!(token_not_found, Error::Auth(_)));
    }

    #[test]
    fn event_not_found_converts_to_error() {
        let calendar_error = CalendarError::EventNotFound {
            calendar_id: "primary".to_string(),
            event_id: "event123".to_string(),
        };
        let error: Error = calendar_error.into();
        assert!(matches!(error, Error::Calendar(_)));
        assert!(error.to_string().contains("Event not found: event123 in calendar primary"));
    }

    #[test]
    fn calendar_error_is_distinct_from_auth() {
        let auth_error = Error::Auth("some auth error".to_string());
        let calendar_error: Error =
            CalendarError::NotFound { calendar_id: "test".to_string() }.into();

        assert!(matches!(auth_error, Error::Auth(_)));
        assert!(matches!(calendar_error, Error::Calendar(_)));
        assert!(!matches!(calendar_error, Error::Auth(_)));
    }
}
