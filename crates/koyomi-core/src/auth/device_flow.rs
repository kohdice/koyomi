use serde::{Deserialize, Serialize};

use crate::{Error, Result};

pub const DEVICE_CODE_URL: &str = "https://oauth2.googleapis.com/device/code";
pub const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
pub const SCOPES: &str = "https://www.googleapis.com/auth/calendar openid email profile";
const GRANT_TYPE: &str = "urn:ietf:params:oauth:grant-type:device_code";

/// POST /device/code request body
#[derive(Serialize)]
pub struct DeviceCodeRequest<'a> {
    pub client_id: &'a str,
    pub scope: &'a str,
}

/// POST /device/code response
#[derive(Debug, Deserialize)]
pub struct DeviceCodeResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_url: String,
    pub expires_in: u64,
    pub interval: u64,
}

/// POST /token request body
#[derive(Serialize)]
pub struct TokenRequest<'a> {
    pub client_id: &'a str,
    pub client_secret: &'a str,
    pub device_code: &'a str,
    pub grant_type: &'a str,
}

/// POST /token success response
#[derive(Debug, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub token_type: String,
    pub expires_in: u64,
    pub scope: String,
}

/// POST /token error response
#[derive(Debug, Deserialize)]
pub struct TokenErrorResponse {
    pub error: String,
    pub error_description: Option<String>,
}

/// Start the device authorization flow
///
/// Returns a [`DeviceCodeResponse`] containing `user_code` and `verification_url`.
///
/// # Errors
///
/// Returns an error if:
/// - The HTTP request fails
/// - The server returns an error response
pub async fn start(
    client: &reqwest::Client,
    client_id: &str,
    device_code_url: &str,
) -> Result<DeviceCodeResponse> {
    let response = client
        .post(device_code_url)
        .form(&DeviceCodeRequest { client_id, scope: SCOPES })
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        let message = match serde_json::from_str::<TokenErrorResponse>(&body) {
            Ok(error) => format!(
                "Failed to get device code: {} - {}",
                error.error,
                error.error_description.unwrap_or_default()
            ),
            Err(_) => format!("Failed to get device code (HTTP {status}): {body}"),
        };
        return Err(Error::Auth(message));
    }

    Ok(response.json().await?)
}

#[derive(Debug, Clone)]
pub struct PollConfig {
    pub token_url: String,
    pub initial_interval: u64,
    pub expires_in: u64,
}

impl Default for PollConfig {
    fn default() -> Self {
        Self { token_url: TOKEN_URL.to_string(), initial_interval: 5, expires_in: 1800 }
    }
}

/// Poll for token after user authorization
///
/// # Errors
///
/// Returns an error if:
/// - The authorization times out
/// - The user denies access
/// - The device code expires
/// - The HTTP request fails
pub async fn poll(
    client: &reqwest::Client,
    client_id: &str,
    client_secret: &str,
    device_code: &str,
    config: &PollConfig,
) -> Result<TokenResponse> {
    // Use std::time::Instant for real wall-clock timeout measurement
    // This ensures timeout works correctly even in tests with tokio::time::pause()
    let start_time = std::time::Instant::now();
    let timeout = std::time::Duration::from_secs(config.expires_in);
    let mut interval = config.initial_interval;

    loop {
        if start_time.elapsed() >= timeout {
            return Err(Error::Auth("Authorization timed out. Please try again.".into()));
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(interval)).await;

        let response = client
            .post(&config.token_url)
            .form(&TokenRequest { client_id, client_secret, device_code, grant_type: GRANT_TYPE })
            .send()
            .await?;

        let status = response.status();
        let body = response.text().await?;

        if !status.is_success() {
            if let Ok(error) = serde_json::from_str::<TokenErrorResponse>(&body) {
                match error.error.as_str() {
                    "authorization_pending" => continue,
                    "slow_down" => {
                        interval += 5;
                        continue;
                    }
                    "access_denied" => {
                        return Err(Error::Auth("Access denied by user.".into()));
                    }
                    "expired_token" => {
                        return Err(Error::Auth(
                            "Device code expired. Please run 'koyomi login' again.".into(),
                        ));
                    }
                    _ => {
                        return Err(Error::Auth(format!(
                            "Token request failed: {} - {}",
                            error.error,
                            error.error_description.unwrap_or_default()
                        )));
                    }
                }
            }
            return Err(Error::Auth(format!("Token request failed (HTTP {status}): {body}",)));
        }

        let token: TokenResponse = serde_json::from_str(&body)?;
        return Ok(token);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{body_string_contains, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn start_returns_device_code_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/device/code"))
            .and(body_string_contains("client_id=test-client-id"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "device_code": "test-device-code",
                "user_code": "ABCD-EFGH",
                "verification_url": "https://www.google.com/device",
                "expires_in": 1800,
                "interval": 5
            })))
            .mount(&mock_server)
            .await;

        let client = reqwest::Client::new();
        let url = format!("{}/device/code", mock_server.uri());
        let result = start(&client, "test-client-id", &url).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.device_code, "test-device-code");
        assert_eq!(response.user_code, "ABCD-EFGH");
        assert_eq!(response.verification_url, "https://www.google.com/device");
        assert_eq!(response.expires_in, 1800);
        assert_eq!(response.interval, 5);
    }

    #[tokio::test]
    async fn start_returns_error_on_failure() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/device/code"))
            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
                "error": "invalid_client",
                "error_description": "The OAuth client was not found."
            })))
            .mount(&mock_server)
            .await;

        let client = reqwest::Client::new();
        let url = format!("{}/device/code", mock_server.uri());
        let result = start(&client, "invalid-client-id", &url).await;

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.to_string().contains("Failed to get device code"));
        assert!(error.to_string().contains("invalid_client"));
    }

    #[tokio::test]
    async fn poll_returns_token_on_success() {
        tokio::time::pause();

        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "test-access-token",
                "refresh_token": "test-refresh-token",
                "token_type": "Bearer",
                "expires_in": 3600,
                "scope": "openid email profile"
            })))
            .mount(&mock_server)
            .await;

        let client = reqwest::Client::new();
        let config = PollConfig {
            token_url: format!("{}/token", mock_server.uri()),
            initial_interval: 1,
            expires_in: 60,
        };

        let result = poll(&client, "client-id", "client-secret", "device-code", &config).await;

        assert!(result.is_ok());
        let token = result.unwrap();
        assert_eq!(token.access_token, "test-access-token");
        assert_eq!(token.refresh_token, Some("test-refresh-token".to_string()));
        assert_eq!(token.token_type, "Bearer");
    }

    #[tokio::test]
    async fn poll_handles_authorization_pending() {
        tokio::time::pause();

        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
                "error": "authorization_pending",
                "error_description": "The authorization request is still pending."
            })))
            .up_to_n_times(2)
            .mount(&mock_server)
            .await;

        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "test-access-token",
                "refresh_token": "test-refresh-token",
                "token_type": "Bearer",
                "expires_in": 3600,
                "scope": "openid"
            })))
            .mount(&mock_server)
            .await;

        let client = reqwest::Client::new();
        let config = PollConfig {
            token_url: format!("{}/token", mock_server.uri()),
            initial_interval: 1,
            expires_in: 60,
        };

        let result = poll(&client, "client-id", "client-secret", "device-code", &config).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn poll_handles_access_denied() {
        tokio::time::pause();

        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
                "error": "access_denied",
                "error_description": "User denied access."
            })))
            .mount(&mock_server)
            .await;

        let client = reqwest::Client::new();
        let config = PollConfig {
            token_url: format!("{}/token", mock_server.uri()),
            initial_interval: 1,
            expires_in: 60,
        };

        let result = poll(&client, "client-id", "client-secret", "device-code", &config).await;

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.to_string().contains("Access denied by user"));
    }

    #[tokio::test]
    async fn poll_handles_expired_token() {
        tokio::time::pause();

        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
                "error": "expired_token",
                "error_description": "The device code has expired."
            })))
            .mount(&mock_server)
            .await;

        let client = reqwest::Client::new();
        let config = PollConfig {
            token_url: format!("{}/token", mock_server.uri()),
            initial_interval: 1,
            expires_in: 60,
        };

        let result = poll(&client, "client-id", "client-secret", "device-code", &config).await;

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.to_string().contains("Device code expired"));
    }

    #[tokio::test]
    async fn poll_handles_slow_down() {
        tokio::time::pause();

        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
                "error": "slow_down",
                "error_description": "Slow down polling."
            })))
            .up_to_n_times(1)
            .mount(&mock_server)
            .await;

        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "test-access-token",
                "refresh_token": "test-refresh-token",
                "token_type": "Bearer",
                "expires_in": 3600,
                "scope": "openid"
            })))
            .mount(&mock_server)
            .await;

        let client = reqwest::Client::new();
        let config = PollConfig {
            token_url: format!("{}/token", mock_server.uri()),
            initial_interval: 1,
            expires_in: 60,
        };

        let result = poll(&client, "client-id", "client-secret", "device-code", &config).await;

        assert!(result.is_ok());
    }
}
