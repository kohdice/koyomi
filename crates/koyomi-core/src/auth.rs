mod device_flow;
pub mod token;

use chrono::Utc;

use crate::{Error, Result, config};

/// Execute the OAuth2 device authorization flow
pub async fn login() -> Result<()> {
    // 1. Load client configuration
    let secret = config::load()?;
    println!("Loaded OAuth2 client configuration.");

    // 2. Get device code
    let device_response =
        device_flow::start(&secret.installed.client_id, device_flow::DEVICE_CODE_URL).await?;

    // 3. Show instructions to user
    println!();
    println!("To sign in, please visit: {}", device_response.verification_url);
    println!("Enter this code: {}", device_response.user_code);
    println!();

    // 4. Try to open browser (best effort)
    if let Err(e) = open::that(&device_response.verification_url) {
        eprintln!("Could not open browser automatically: {}", e);
        eprintln!("Please open the URL manually.");
    }

    println!("Waiting for authorization...");

    // 5. Poll for token
    let poll_config = device_flow::PollConfig {
        token_url: device_flow::TOKEN_URL.to_string(),
        initial_interval: device_response.interval,
        expires_in: device_response.expires_in,
    };

    let token_response = device_flow::poll(
        &secret.installed.client_id,
        &secret.installed.client_secret,
        &device_response.device_code,
        &poll_config,
    )
    .await?;

    // 6. Calculate expiration time
    let now = Utc::now();
    let expires_at = now + chrono::Duration::seconds(token_response.expires_in as i64);

    // 7. Save token
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
    println!("Token saved to ~/.config/koyomi/google_tokens.json");

    Ok(())
}

/// Remove stored tokens
pub async fn logout() -> Result<()> {
    // Check if token exists first
    match token::load() {
        Ok(_) => {
            token::delete()?;
            println!("Successfully logged out.");
            println!("Token file has been removed.");
        }
        Err(Error::Auth(_)) => {
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
        // Verify the token module is publicly accessible
        let _: fn() -> Result<token::StoredToken> = token::load;
    }
}
