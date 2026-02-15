use std::path::PathBuf;

use serde::Deserialize;

use crate::{Error, Result};

const CONFIG_DIR: &str = "koyomi";
const CLIENT_SECRET_FILE: &str = "client_secret.json";

/// Get the koyomi config directory
///
/// Respects `XDG_CONFIG_HOME` if set, otherwise falls back to `~/.config/koyomi`.
///
/// # Errors
///
/// Returns an error if the home directory cannot be determined
/// (when `XDG_CONFIG_HOME` is not set).
pub fn config_dir() -> Result<PathBuf> {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        let path = PathBuf::from(&xdg);
        if !xdg.is_empty() && path.is_absolute() {
            return Ok(path.join(CONFIG_DIR));
        }
        if !xdg.is_empty() && !path.is_absolute() {
            tracing::warn!(
                "XDG_CONFIG_HOME is set to a relative path '{}'; falling back to ~/.config",
                xdg
            );
        }
    }

    let home = dirs::home_dir()
        .ok_or_else(|| Error::Config("Could not determine home directory".into()))?;

    Ok(home.join(".config").join(CONFIG_DIR))
}

#[derive(Debug, Deserialize)]
pub struct ClientSecretFile {
    pub installed: ClientSecretInstalled,
}

#[derive(Deserialize)]
pub struct ClientSecretInstalled {
    pub client_id: String,
    pub client_secret: String,
}

impl std::fmt::Debug for ClientSecretInstalled {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClientSecretInstalled")
            .field("client_id", &self.client_id)
            .field("client_secret", &"[REDACTED]")
            .finish()
    }
}

/// Load client secret from the specified path
///
/// # Errors
///
/// Returns an error if:
/// - The file cannot be read
/// - The JSON format is invalid
/// - `client_id` or `client_secret` is missing or empty
pub fn load_from_path(path: &std::path::Path) -> Result<ClientSecretFile> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(metadata) = std::fs::metadata(path) {
            let mode = metadata.permissions().mode() & 0o777;
            if mode & 0o077 != 0 {
                tracing::warn!(
                    "{} has permissions {:o}; recommended 0600. Fix with: chmod 600 {}",
                    path.display(),
                    mode,
                    path.display()
                );
                eprintln!(
                    "Warning: {} has overly permissive file permissions ({:o}). \
                     Recommended: chmod 600 {}",
                    path.display(),
                    mode,
                    path.display()
                );
            }
        }
    }

    let content = std::fs::read_to_string(path).map_err(|e| {
        Error::Config(format!(
            "Failed to read {}: {}. Please create this file with your OAuth2 credentials.",
            path.display(),
            e
        ))
    })?;

    let secret: ClientSecretFile = serde_json::from_str(&content)
        .map_err(|e| Error::Config(format!("Invalid {CLIENT_SECRET_FILE} format: {e}")))?;

    if secret.installed.client_id.is_empty() {
        return Err(Error::Config("client_id is missing or empty".into()));
    }
    if secret.installed.client_secret.is_empty() {
        return Err(Error::Config("client_secret is missing or empty".into()));
    }

    Ok(secret)
}

/// Load client secret from the default config path
///
/// # Errors
///
/// Returns an error if:
/// - The config directory cannot be determined
/// - The file cannot be read
/// - The JSON format is invalid
/// - `client_id` or `client_secret` is missing or empty
pub fn load() -> Result<ClientSecretFile> {
    let path = config_dir()?.join(CLIENT_SECRET_FILE);
    load_from_path(&path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_config(dir: &TempDir, content: &str) -> std::path::PathBuf {
        let path = dir.path().join(CLIENT_SECRET_FILE);
        std::fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn load_valid_client_secret() {
        let dir = TempDir::new().unwrap();
        let content = r#"{
            "installed": {
                "client_id": "test-client-id.apps.googleusercontent.com",
                "client_secret": "test-client-secret"
            }
        }"#;
        let path = create_test_config(&dir, content);

        let result = load_from_path(&path);
        assert!(result.is_ok());

        let config = result.unwrap();
        assert_eq!(config.installed.client_id, "test-client-id.apps.googleusercontent.com");
        assert_eq!(config.installed.client_secret, "test-client-secret");
    }

    #[test]
    fn load_invalid_json_returns_error() {
        let dir = TempDir::new().unwrap();
        let content = "not valid json";
        let path = create_test_config(&dir, content);

        let result = load_from_path(&path);
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert!(error.to_string().contains("Invalid client_secret.json format"));
    }

    #[test]
    fn load_missing_client_id_returns_error() {
        let dir = TempDir::new().unwrap();
        let content = r#"{
            "installed": {
                "client_id": "",
                "client_secret": "test-client-secret"
            }
        }"#;
        let path = create_test_config(&dir, content);

        let result = load_from_path(&path);
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert!(error.to_string().contains("client_id is missing or empty"));
    }

    #[test]
    fn load_missing_client_secret_returns_error() {
        let dir = TempDir::new().unwrap();
        let content = r#"{
            "installed": {
                "client_id": "test-client-id.apps.googleusercontent.com",
                "client_secret": ""
            }
        }"#;
        let path = create_test_config(&dir, content);

        let result = load_from_path(&path);
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert!(error.to_string().contains("client_secret is missing or empty"));
    }

    #[test]
    fn load_nonexistent_file_returns_error() {
        let path = std::path::Path::new("/nonexistent/path/client_secret.json");

        let result = load_from_path(path);
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert!(error.to_string().contains("Failed to read"));
    }

    #[test]
    fn load_ignores_extra_fields() {
        let dir = TempDir::new().unwrap();
        let content = r#"{
            "installed": {
                "client_id": "test-client-id.apps.googleusercontent.com",
                "client_secret": "test-client-secret",
                "auth_uri": "https://accounts.google.com/o/oauth2/auth",
                "token_uri": "https://oauth2.googleapis.com/token",
                "extra_field": "should be ignored"
            }
        }"#;
        let path = create_test_config(&dir, content);

        let result = load_from_path(&path);
        assert!(result.is_ok());
    }
}
