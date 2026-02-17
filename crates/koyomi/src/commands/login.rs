use anyhow::Result;

pub async fn handle(client: &koyomi_core::Client, quiet: bool) -> Result<()> {
    let session = koyomi_core::start_login(client).await?;

    // verification URL と user code は --quiet でも表示する（認証に必須）
    eprintln!();
    eprintln!("To sign in, please visit: {}", session.verification_uri());
    eprintln!("Enter this code: {}", session.user_code());
    eprintln!();

    if let Err(e) = open::that(session.verification_uri()) {
        tracing::warn!("Could not open browser automatically: {}", e);
        if !quiet {
            eprintln!("Could not open browser automatically. Please open the URL above manually.");
        }
    }

    if !quiet {
        eprintln!("Waiting for authorization...");
    }

    koyomi_core::complete_login(client, &session).await?;

    if !quiet {
        eprintln!();
        eprintln!("Successfully logged in!");
    }

    Ok(())
}
