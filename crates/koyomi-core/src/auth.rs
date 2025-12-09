mod device_flow;
pub mod token;

use chrono::Utc;
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
pub async fn login() -> Result<()> {
    let secret = config::load()?;
    info!("Loaded OAuth2 client configuration");

    let client = reqwest::Client::new();

    debug!("Requesting device code");
    let device_response =
        device_flow::start(&client, &secret.installed.client_id, device_flow::DEVICE_CODE_URL)
            .await?;

    println!();
    println!("To sign in, please visit: {}", device_response.verification_url);
    println!("Enter this code: {}", device_response.user_code);
    println!();

    if let Err(e) = open::that(&device_response.verification_url) {
        warn!("Could not open browser automatically: {}", e);
        warn!("Please open the URL manually");
    }

    println!("Waiting for authorization...");

    let poll_config = device_flow::PollConfig {
        token_url: device_flow::TOKEN_URL.to_string(),
        initial_interval: device_response.interval,
        expires_in: device_response.expires_in,
    };

    debug!("Starting token polling");
    let token_response = device_flow::poll(
        &client,
        &secret.installed.client_id,
        &secret.installed.client_secret,
        &device_response.device_code,
        &poll_config,
    )
    .await?;

    let now = Utc::now();
    let expires_in_secs = i64::try_from(token_response.expires_in)
        .map_err(|_| Error::Auth("Token expiration time overflow".into()))?;
    let expires_at = now + chrono::Duration::seconds(expires_in_secs);

    let stored_token = token::StoredToken {
        access_token: token_response.access_token,
        refresh_token: token_response.refresh_token,
        token_type: token_response.token_type,
        scope: token_response.scope.split_whitespace().map(String::from).collect(),
        expires_at,
        obtained_at: now,
    };

    token::save(&stored_token)?;

    println!();
    println!("Successfully logged in!");

    Ok(())
}

/// Remove stored tokens
///
/// # Errors
///
/// Returns an error if the token file exists but cannot be deleted.
pub async fn logout() -> Result<()> {
    match token::load() {
        Ok(_) => {
            token::delete()?;
            info!("Token file has been removed");
            println!("Successfully logged out.");
        }
        Err(Error::TokenNotFound) => {
            println!("Not currently logged in.");
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
}
