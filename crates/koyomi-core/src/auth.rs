mod device_flow;
pub mod refresh;
pub mod token;

use tracing::{debug, info, warn};

use crate::{Error, Result, config};

/// Execute the OAuth2 device authorization flow
///
/// # Errors
///
/// Returns an error if:
/// - The client secret configuration cannot be loaded
/// - The device code request fails
/// - Token polling times out or is denied
/// - The token cannot be saved
pub async fn login(http: &reqwest::Client) -> Result<()> {
    let secret = config::load()?;
    info!("Loaded OAuth2 client configuration");

    debug!("Requesting device code");
    let device_response =
        device_flow::start(http, &secret.installed.client_id, device_flow::DEVICE_CODE_URL).await?;

    eprintln!();
    eprintln!("To sign in, please visit: {}", device_response.verification_url);
    eprintln!("Enter this code: {}", device_response.user_code);
    eprintln!();

    if let Err(e) = open::that(&device_response.verification_url) {
        warn!("Could not open browser automatically: {}", e);
        eprintln!("Could not open browser automatically. Please open the URL above manually.");
    }

    eprintln!("Waiting for authorization...");

    let poll_config = device_flow::PollConfig {
        token_url: device_flow::TOKEN_URL.to_string(),
        initial_interval: device_response.interval,
        expires_in: device_response.expires_in,
    };

    debug!("Starting token polling");
    let token_response = device_flow::poll(
        http,
        &secret.installed.client_id,
        &secret.installed.client_secret,
        &device_response.device_code,
        &poll_config,
    )
    .await?;

    let stored_token = token::StoredToken::from_response(
        token_response.access_token,
        token_response.refresh_token,
        token_response.token_type,
        &token_response.scope,
        token_response.expires_in,
    )?;

    token::save(&stored_token)?;

    eprintln!();
    eprintln!("Successfully logged in!");

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
pub async fn get_valid_token(http: &reqwest::Client) -> Result<token::StoredToken> {
    let mut stored_token = token::load()?;

    let buffer = chrono::TimeDelta::minutes(TOKEN_REFRESH_BUFFER_MINUTES);
    if stored_token.is_expired_with_buffer(buffer) {
        debug!("Token expired or expiring soon, refreshing");

        let secret = config::load()?;

        let new_token = refresh::refresh_token(
            http,
            &secret.installed.client_id,
            &secret.installed.client_secret,
            &stored_token,
            refresh::TOKEN_URL,
        )
        .await?;

        token::save(&new_token)?;
        info!("Token refreshed and saved");

        stored_token = new_token;
    }

    Ok(stored_token)
}

/// Remove stored tokens
///
/// # Errors
///
/// Returns an error if the token file exists but cannot be deleted.
pub fn logout() -> Result<()> {
    match token::load() {
        Ok(_) => {
            token::delete()?;
            info!("Token file has been removed");
            eprintln!("Successfully logged out.");
        }
        Err(Error::TokenNotFound) => {
            eprintln!("Not currently logged in.");
        }
        Err(e) => {
            return Err(e);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Integration tests for login/logout would require mocking
    // the external Google OAuth endpoints, which is complex.
    // Instead, we test the component modules individually.

    #[test]
    fn token_module_is_accessible() {
        let _: fn() -> Result<token::StoredToken> = token::load;
    }

    #[test]
    fn refresh_module_is_accessible() {
        // Verify refresh module is public
        let _: &str = refresh::TOKEN_URL;
    }
}
