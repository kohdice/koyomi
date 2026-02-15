mod device_flow;
pub(crate) mod refresh;
pub(crate) mod token;

pub use token::StoredToken;

use tracing::{debug, info, warn};

use crate::{Error, Result, config};

/// Information returned from the device authorization flow start.
///
/// Contains the URL and code that the user must use to authorize access.
pub struct DeviceFlowSession {
    verification_url: String,
    user_code: String,
    device_code: String,
    interval: u64,
    expires_in: u64,
    pub(crate) secret: config::ClientSecretFile,
}

impl DeviceFlowSession {
    /// Returns the URL where the user should visit to authorize.
    #[must_use]
    pub fn verification_url(&self) -> &str {
        &self.verification_url
    }

    /// Returns the code the user must enter at the verification URL.
    #[must_use]
    pub fn user_code(&self) -> &str {
        &self.user_code
    }
}

/// Result of a logout operation.
pub enum LogoutResult {
    /// Successfully logged out and token was removed.
    LoggedOut,
    /// No token was found; user was not logged in.
    NotLoggedIn,
    /// Token file was corrupt and has been removed.
    CorruptTokenRemoved,
}

/// Start the OAuth2 device authorization flow.
///
/// Returns a [`DeviceFlowSession`] with information that should be displayed
/// to the user (verification URL and user code).
///
/// # Errors
///
/// Returns an error if:
/// - The client secret configuration cannot be loaded
/// - The device code request fails
pub async fn start_login(client: &crate::client::Client) -> Result<DeviceFlowSession> {
    let secret = config::load()?;
    info!("Loaded OAuth2 client configuration");

    debug!("Requesting device code");
    let device_response = device_flow::start(
        client.http(),
        &secret.installed.client_id,
        device_flow::DEVICE_CODE_URL,
    )
    .await?;

    Ok(DeviceFlowSession {
        verification_url: device_response.verification_url,
        user_code: device_response.user_code,
        device_code: device_response.device_code,
        interval: device_response.interval,
        expires_in: device_response.expires_in,
        secret,
    })
}

/// Complete the OAuth2 device authorization flow by polling for user authorization.
///
/// # Errors
///
/// Returns an error if:
/// - Token polling times out or is denied
/// - The token cannot be saved
/// - The server did not return a refresh token
pub async fn complete_login(
    client: &crate::client::Client,
    session: &DeviceFlowSession,
) -> Result<()> {
    let poll_config = device_flow::PollConfig::new(
        device_flow::TOKEN_URL.to_string(),
        session.interval,
        session.expires_in,
    );

    debug!("Starting token polling");
    let token_response = device_flow::poll(
        client.http(),
        &session.secret.installed.client_id,
        &session.secret.installed.client_secret,
        &session.device_code,
        &poll_config,
    )
    .await?;

    if token_response.refresh_token.is_none() {
        return Err(Error::Auth(
            "Authorization server did not return a refresh token. \
             Please revoke app access at https://myaccount.google.com/permissions and try again."
                .into(),
        ));
    }

    let stored_token = token::StoredToken::from_response(
        token_response.access_token,
        token_response.refresh_token,
        token_response.token_type,
        &token_response.scope,
        token_response.expires_in,
    )?;

    let token_path = token::path()?;
    token::save(&stored_token, &token_path)?;

    Ok(())
}

const TOKEN_REFRESH_BUFFER_MINUTES: i64 = 5;

/// Get a valid access token, refreshing if necessary
///
/// This function loads the stored token and checks if it's expired or about to expire
/// (within 5 minutes). If so, it refreshes the token and saves the new one.
///
/// # Errors
///
/// Returns an error if:
/// - No token is stored ([`Error::TokenNotFound`])
/// - The config directory or client secret cannot be loaded
/// - Token refresh fails
/// - The new token cannot be saved
pub async fn get_valid_token(client: &crate::client::Client) -> Result<token::StoredToken> {
    let token_path = token::path()?;

    #[cfg(unix)]
    let _lock = {
        let lock_path = token_path.with_extension("lock");
        token::FileLock::acquire(&lock_path)?
    };

    let mut stored_token = token::load(&token_path)?;

    let buffer = chrono::TimeDelta::minutes(TOKEN_REFRESH_BUFFER_MINUTES);
    if stored_token.is_expired(buffer) {
        debug!("Token expired or expiring soon, refreshing");

        let secret = config::load()?;

        let new_token = refresh::refresh_token(
            client.http(),
            &secret.installed.client_id,
            &secret.installed.client_secret,
            &stored_token,
            refresh::TOKEN_URL,
        )
        .await?;

        token::save(&new_token, &token_path)?;
        info!("Token refreshed and saved");

        stored_token = new_token;
    }

    Ok(stored_token)
}

const REVOKE_URL: &str = "https://oauth2.googleapis.com/revoke";

/// Remove stored tokens and revoke them with Google.
///
/// Attempts to revoke the token with Google's revocation endpoint before
/// deleting the local token file. If revocation fails (e.g. network error),
/// the local token is still deleted with a warning.
///
/// Returns a [`LogoutResult`] indicating what happened.
///
/// # Errors
///
/// Returns an error if the token file exists but cannot be deleted.
pub async fn logout(client: &crate::client::Client) -> Result<LogoutResult> {
    let token_path = token::path()?;
    match token::load(&token_path) {
        Ok(stored_token) => {
            if let Some(refresh_token) = stored_token.refresh_token() {
                match client.http().post(REVOKE_URL).form(&[("token", refresh_token)]).send().await
                {
                    Ok(response) if response.status().is_success() => {
                        debug!("Token revoked successfully with Google");
                    }
                    Ok(response) => {
                        warn!(
                            "Token revocation returned HTTP {}: token may still be valid on Google's side",
                            response.status()
                        );
                    }
                    Err(e) => {
                        warn!(
                            "Failed to revoke token with Google (network error: {}): \
                             token may still be valid on Google's side",
                            e
                        );
                    }
                }
            }

            token::delete(&token_path)?;
            info!("Token file has been removed");
            Ok(LogoutResult::LoggedOut)
        }
        Err(Error::TokenNotFound) => Ok(LogoutResult::NotLoggedIn),
        Err(Error::Io(io_err)) => Err(Error::Io(io_err)),
        Err(e) => {
            warn!("Token file is corrupt or unreadable: {}", e);
            token::delete(&token_path)?;
            info!("Corrupt token file has been removed");
            Ok(LogoutResult::CorruptTokenRemoved)
        }
    }
}
