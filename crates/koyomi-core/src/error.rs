use thiserror::Error;

/// Calendar API specific error types
#[derive(Debug, Error)]
pub enum CalendarError {
    /// Calendar or event not found (HTTP 404)
    #[error("Calendar not found: {calendar_id}")]
    NotFound { calendar_id: String },

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

    /// Invalid time or timezone conversion
    #[error("Invalid time: {0}")]
    InvalidTime(String),
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Authentication error: {0}")]
    Auth(String),

    #[error("Token not found")]
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
    fn config_error_displays_message() {
        let error = Error::Config("test config error".to_string());
        assert_eq!(error.to_string(), "Configuration error: test config error");
    }

    #[test]
    fn auth_error_displays_message() {
        let error = Error::Auth("test auth error".to_string());
        assert_eq!(error.to_string(), "Authentication error: test auth error");
    }

    #[test]
    fn io_error_converts_from_std_io() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let error: Error = io_error.into();
        assert!(error.to_string().contains("file not found"));
    }

    #[test]
    fn json_error_converts_from_serde_json() {
        let json_error = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
        let error: Error = json_error.into();
        assert!(error.to_string().starts_with("JSON error:"));
    }

    #[test]
    fn token_not_found_error_displays_message() {
        let error = Error::TokenNotFound;
        assert_eq!(error.to_string(), "Token not found");
    }

    #[test]
    fn token_not_found_is_distinct_from_auth() {
        let auth_error = Error::Auth("some auth error".to_string());
        let token_not_found = Error::TokenNotFound;

        // These should be different error types
        assert!(matches!(auth_error, Error::Auth(_)));
        assert!(matches!(token_not_found, Error::TokenNotFound));
        assert!(!matches!(token_not_found, Error::Auth(_)));
    }

    #[test]
    fn calendar_error_not_found_displays_message() {
        let error = CalendarError::NotFound { calendar_id: "test@example.com".to_string() };
        assert_eq!(error.to_string(), "Calendar not found: test@example.com");
    }

    #[test]
    fn calendar_error_forbidden_displays_message() {
        let error = CalendarError::Forbidden { calendar_id: "test@example.com".to_string() };
        assert_eq!(error.to_string(), "Access denied to calendar: test@example.com");
    }

    #[test]
    fn calendar_error_rate_limit_displays_message() {
        let error = CalendarError::RateLimitExceeded;
        assert_eq!(error.to_string(), "Rate limit exceeded. Please wait and try again.");
    }

    #[test]
    fn calendar_error_unauthenticated_displays_message() {
        let error = CalendarError::Unauthenticated;
        assert_eq!(error.to_string(), "Authentication required. Please re-authenticate.");
    }

    #[test]
    fn calendar_error_bad_request_displays_message() {
        let error = CalendarError::BadRequest { message: "invalid parameter".to_string() };
        assert_eq!(error.to_string(), "Invalid request: invalid parameter");
    }

    #[test]
    fn calendar_error_server_error_displays_message() {
        let error =
            CalendarError::ServerError { status: 503, message: "service unavailable".to_string() };
        assert_eq!(error.to_string(), "Server error (status 503): service unavailable");
    }

    #[test]
    fn calendar_error_invalid_time_displays_message() {
        let error = CalendarError::InvalidTime("failed to create midnight time".to_string());
        assert_eq!(error.to_string(), "Invalid time: failed to create midnight time");
    }

    #[test]
    fn calendar_error_converts_to_error() {
        let calendar_error = CalendarError::NotFound { calendar_id: "primary".to_string() };
        let error: Error = calendar_error.into();
        assert!(matches!(error, Error::Calendar(_)));
        assert!(error.to_string().contains("Calendar not found"));
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
