use anyhow::Result;

pub async fn handle(client: &koyomi_core::Client, quiet: bool) -> Result<()> {
    match koyomi_core::logout(client).await? {
        koyomi_core::LogoutResult::LoggedOut => {
            if !quiet {
                eprintln!("Successfully logged out.");
            }
        }
        koyomi_core::LogoutResult::LoggedOutRevocationFailed => {
            if !quiet {
                eprintln!(
                    "Logged out locally, but failed to revoke the token with Google. \
                     The token may still be valid on Google's side."
                );
            }
        }
        koyomi_core::LogoutResult::NotLoggedIn => {
            if !quiet {
                eprintln!("Not currently logged in.");
            }
        }
        koyomi_core::LogoutResult::CorruptTokenRemoved => {
            if !quiet {
                eprintln!(
                    "Token file was corrupt and has been removed. Please run 'koyomi login' again."
                );
            }
        }
    }
    Ok(())
}
