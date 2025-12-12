use std::fs;
use std::path::Path;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::{Error, Result, config};

const TOKEN_FILE: &str = "google_tokens.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoredToken {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub token_type: String,
    pub scope: Vec<String>,
    pub expires_at: DateTime<Utc>,
    pub obtained_at: DateTime<Utc>,
}

impl StoredToken {
    /// Check if the token has expired
    pub fn is_expired(&self) -> bool {
        Utc::now() >= self.expires_at
    }

    /// Check if the token has expired or will expire within the given buffer duration
    ///
    /// This is useful for proactively refreshing tokens before they actually expire.
    /// A recommended buffer is 5 minutes.
    pub fn is_expired_with_buffer(&self, buffer: chrono::Duration) -> bool {
        Utc::now() + buffer >= self.expires_at
    }
}

/// Save token to the specified path
///
/// # Errors
///
/// Returns an error if:
/// - The parent directory cannot be created
/// - The file cannot be written
/// - File permissions cannot be set (Unix only)
pub fn save_to_path(token: &StoredToken, path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let content = serde_json::to_string_pretty(token)?;

    fs::write(path, &content)?;

    // Set permissions to 0600 (owner read/write only) on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(path)?.permissions();
        perms.set_mode(0o600);
        fs::set_permissions(path, perms)?;
    }

    Ok(())
}

/// Load token from the specified path
///
/// # Errors
///
/// Returns an error if:
/// - The file does not exist ([`Error::TokenNotFound`])
/// - The file cannot be read
/// - The JSON format is invalid
pub fn load_from_path(path: &Path) -> Result<StoredToken> {
    if !path.exists() {
        return Err(Error::TokenNotFound);
    }

    let content = fs::read_to_string(path)?;
    let token: StoredToken = serde_json::from_str(&content)?;

    Ok(token)
}

/// Delete token at the specified path
///
/// # Errors
///
/// Returns an error if the file exists but cannot be removed.
pub fn delete_path(path: &Path) -> Result<()> {
    if path.exists() {
        fs::remove_file(path)?;
    }

    Ok(())
}

/// Get the path to the token file
fn token_path() -> Result<std::path::PathBuf> {
    Ok(config::config_dir()?.join(TOKEN_FILE))
}

/// Save token to `~/.config/koyomi/google_tokens.json`
///
/// # Errors
///
/// Returns an error if:
/// - The config directory cannot be determined
/// - The parent directory cannot be created
/// - The file cannot be written
/// - File permissions cannot be set (Unix only)
pub fn save(token: &StoredToken) -> Result<()> {
    let path = token_path()?;
    save_to_path(token, &path)?;
    info!("Token saved to {}", path.display());
    Ok(())
}

/// Load token from `~/.config/koyomi/google_tokens.json`
///
/// # Errors
///
/// Returns an error if:
/// - The config directory cannot be determined
/// - The file does not exist ([`Error::TokenNotFound`])
/// - The file cannot be read
/// - The JSON format is invalid
pub fn load() -> Result<StoredToken> {
    load_from_path(&token_path()?)
}

/// Delete token from `~/.config/koyomi/google_tokens.json`
///
/// # Errors
///
/// Returns an error if:
/// - The config directory cannot be determined
/// - The file exists but cannot be removed
pub fn delete() -> Result<()> {
    delete_path(&token_path()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;
    use tempfile::TempDir;

    fn create_test_token() -> StoredToken {
        let now = Utc::now();
        StoredToken {
            access_token: "test_access_token".to_string(),
            refresh_token: Some("test_refresh_token".to_string()),
            token_type: "Bearer".to_string(),
            scope: vec![
                "https://www.googleapis.com/auth/calendar.events".to_string(),
                "openid".to_string(),
            ],
            expires_at: now + Duration::hours(1),
            obtained_at: now,
        }
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(TOKEN_FILE);
        let token = create_test_token();

        let result = save_to_path(&token, &path);
        assert!(result.is_ok());

        let loaded = load_from_path(&path);
        assert!(loaded.is_ok());

        let loaded_token = loaded.unwrap();
        assert_eq!(loaded_token.access_token, token.access_token);
        assert_eq!(loaded_token.refresh_token, token.refresh_token);
        assert_eq!(loaded_token.token_type, token.token_type);
        assert_eq!(loaded_token.scope, token.scope);
    }

    #[test]
    fn delete_removes_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(TOKEN_FILE);
        let token = create_test_token();

        save_to_path(&token, &path).unwrap();
        assert!(path.exists());

        let result = delete_path(&path);
        assert!(result.is_ok());
        assert!(!path.exists());
    }

    #[test]
    fn delete_nonexistent_file_succeeds() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(TOKEN_FILE);

        let result = delete_path(&path);
        assert!(result.is_ok());
    }

    #[test]
    fn load_nonexistent_file_returns_token_not_found() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(TOKEN_FILE);

        let result = load_from_path(&path);
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert!(matches!(error, Error::TokenNotFound));
        assert_eq!(error.to_string(), "Token not found");
    }

    #[test]
    #[cfg(unix)]
    fn save_sets_file_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let dir = TempDir::new().unwrap();
        let path = dir.path().join(TOKEN_FILE);
        let token = create_test_token();

        save_to_path(&token, &path).unwrap();

        let metadata = fs::metadata(&path).unwrap();
        let mode = metadata.permissions().mode();

        // Check that the file is only readable/writable by owner (0600)
        assert_eq!(mode & 0o777, 0o600);
    }

    #[test]
    fn token_without_refresh_token() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(TOKEN_FILE);
        let now = Utc::now();
        let token = StoredToken {
            access_token: "test_access_token".to_string(),
            refresh_token: None,
            token_type: "Bearer".to_string(),
            scope: vec!["openid".to_string()],
            expires_at: now + Duration::hours(1),
            obtained_at: now,
        };

        save_to_path(&token, &path).unwrap();
        let loaded = load_from_path(&path).unwrap();

        assert!(loaded.refresh_token.is_none());
    }

    #[test]
    fn is_expired_returns_false_for_valid_token() {
        let now = Utc::now();
        let token = StoredToken {
            access_token: "test_access_token".to_string(),
            refresh_token: Some("test_refresh_token".to_string()),
            token_type: "Bearer".to_string(),
            scope: vec!["openid".to_string()],
            expires_at: now + Duration::hours(1),
            obtained_at: now,
        };

        assert!(!token.is_expired());
    }

    #[test]
    fn is_expired_returns_true_for_expired_token() {
        let now = Utc::now();
        let token = StoredToken {
            access_token: "test_access_token".to_string(),
            refresh_token: Some("test_refresh_token".to_string()),
            token_type: "Bearer".to_string(),
            scope: vec!["openid".to_string()],
            expires_at: now - Duration::hours(1),
            obtained_at: now - Duration::hours(2),
        };

        assert!(token.is_expired());
    }

    #[test]
    fn is_expired_with_buffer_returns_true_when_expiring_soon() {
        let now = Utc::now();
        let token = StoredToken {
            access_token: "test_access_token".to_string(),
            refresh_token: Some("test_refresh_token".to_string()),
            token_type: "Bearer".to_string(),
            scope: vec!["openid".to_string()],
            expires_at: now + Duration::minutes(3),
            obtained_at: now,
        };

        // Token expires in 3 minutes, so 5-minute buffer should consider it expired
        assert!(token.is_expired_with_buffer(Duration::minutes(5)));
        // But without buffer, it's still valid
        assert!(!token.is_expired());
    }

    #[test]
    fn is_expired_with_buffer_returns_false_when_not_expiring_soon() {
        let now = Utc::now();
        let token = StoredToken {
            access_token: "test_access_token".to_string(),
            refresh_token: Some("test_refresh_token".to_string()),
            token_type: "Bearer".to_string(),
            scope: vec!["openid".to_string()],
            expires_at: now + Duration::hours(1),
            obtained_at: now,
        };

        // Token expires in 1 hour, so 5-minute buffer should not consider it expired
        assert!(!token.is_expired_with_buffer(Duration::minutes(5)));
    }
}
