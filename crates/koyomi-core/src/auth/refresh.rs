use serde::{Deserialize, Serialize};
use tracing::debug;

use super::token::StoredToken;
use crate::{Error, Result};

pub const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";

/// Request body for token refresh
#[derive(Serialize)]
struct RefreshRequest<'a> {
    client_id: &'a str,
    client_secret: &'a str,
    refresh_token: &'a str,
    grant_type: &'a str,
}

/// Response from token refresh endpoint
#[derive(Debug, Deserialize)]
struct RefreshResponse {
    access_token: String,
    refresh_token: Option<String>,
    token_type: String,
    expires_in: u64,
    scope: String,
}

/// Error response from token refresh endpoint
#[derive(Debug, Deserialize)]
struct RefreshErrorResponse {
    error: String,
    error_description: Option<String>,
}

/// Refresh an access token using a refresh token
///
/// # Errors
///
/// Returns an error if:
/// - The token does not have a refresh token
/// - The HTTP request fails
/// - The server returns an error response
pub async fn refresh_token(
    client: &reqwest::Client,
    client_id: &str,
    client_secret: &str,
    token: &StoredToken,
    token_url: &str,
) -> Result<StoredToken> {
    let refresh_token = token.refresh_token().ok_or(Error::NoRefreshToken)?;

    debug!("Refreshing access token");

    let response = client
        .post(token_url)
        .form(&RefreshRequest {
            client_id,
            client_secret,
            refresh_token,
            grant_type: "refresh_token",
        })
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_else(|e| {
            tracing::debug!("Failed to read error response body: {}", e);
            format!("(failed to read response body: {e})")
        });
        let message = match serde_json::from_str::<RefreshErrorResponse>(&body) {
            Ok(error) if error.error == "invalid_grant" => {
                let desc = error.error_description.as_deref().unwrap_or("token expired or revoked");
                format!(
                    "Failed to refresh token: {} - {}. Please run 'koyomi logout' then 'koyomi login'.",
                    error.error, desc
                )
            }
            Ok(error) => match error.error_description {
                Some(desc) => format!("Failed to refresh token: {} - {}", error.error, desc),
                None => format!("Failed to refresh token: {}", error.error),
            },
            Err(_) => format!("Failed to refresh token (HTTP {status}): {body}"),
        };
        return Err(Error::Auth(message));
    }

    let body = response.text().await?;
    let refresh_response: RefreshResponse = serde_json::from_str(&body)
        .map_err(|e| Error::Auth(format!("Failed to parse token refresh response: {e}")))?;

    StoredToken::from_response(
        refresh_response.access_token,
        refresh_response.refresh_token.or_else(|| token.refresh_token().map(String::from)),
        refresh_response.token_type,
        &refresh_response.scope,
        refresh_response.expires_in,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeDelta, Utc};
    use wiremock::matchers::{body_string_contains, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn create_test_token() -> StoredToken {
        let now = Utc::now();
        StoredToken {
            access_token: "old_access_token".to_string(),
            refresh_token: Some("test_refresh_token".to_string()),
            token_type: "Bearer".to_string(),
            scope: vec!["openid".to_string(), "email".to_string()],
            expires_at: now - TimeDelta::hours(1),
            obtained_at: now - TimeDelta::hours(2),
        }
    }

    #[tokio::test]
    async fn refresh_token_returns_new_token_on_success() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/token"))
            .and(body_string_contains("grant_type=refresh_token"))
            .and(body_string_contains("refresh_token=test_refresh_token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "new_access_token",
                "token_type": "Bearer",
                "expires_in": 3600,
                "scope": "openid email"
            })))
            .mount(&mock_server)
            .await;

        let client = reqwest::Client::new();
        let token = create_test_token();
        let url = format!("{}/token", mock_server.uri());

        let result = refresh_token(&client, "client_id", "client_secret", &token, &url).await;

        assert!(result.is_ok());
        let new_token = result.unwrap();
        assert_eq!(new_token.access_token, "new_access_token");
        assert_eq!(new_token.refresh_token, Some("test_refresh_token".to_string()));
        assert_eq!(new_token.token_type, "Bearer");
        assert!(!new_token.is_expired(TimeDelta::seconds(0)));
    }

    #[tokio::test]
    async fn refresh_token_returns_error_without_refresh_token() {
        let client = reqwest::Client::new();
        let now = Utc::now();
        let token = StoredToken {
            access_token: "old_access_token".to_string(),
            refresh_token: None,
            token_type: "Bearer".to_string(),
            scope: vec!["openid".to_string()],
            expires_at: now - TimeDelta::hours(1),
            obtained_at: now - TimeDelta::hours(2),
        };

        let result = refresh_token(&client, "client_id", "client_secret", &token, TOKEN_URL).await;

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(matches!(error, Error::NoRefreshToken));
        assert!(error.to_string().contains("no refresh token available"));
    }

    #[tokio::test]
    async fn refresh_token_returns_error_on_invalid_grant() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
                "error": "invalid_grant",
                "error_description": "Token has been expired or revoked."
            })))
            .mount(&mock_server)
            .await;

        let client = reqwest::Client::new();
        let token = create_test_token();
        let url = format!("{}/token", mock_server.uri());

        let result = refresh_token(&client, "client_id", "client_secret", &token, &url).await;

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.to_string().contains("invalid_grant"));
        assert!(error.to_string().contains("Token has been expired or revoked"));
    }

    #[tokio::test]
    async fn refresh_token_preserves_original_refresh_token() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "new_access_token",
                "token_type": "Bearer",
                "expires_in": 3600,
                "scope": "openid"
            })))
            .mount(&mock_server)
            .await;

        let client = reqwest::Client::new();
        let token = create_test_token();
        let url = format!("{}/token", mock_server.uri());

        let result = refresh_token(&client, "client_id", "client_secret", &token, &url).await;

        assert!(result.is_ok());
        let new_token = result.unwrap();
        assert_eq!(new_token.refresh_token, token.refresh_token);
    }
}
