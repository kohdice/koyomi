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
    /// Construct a `StoredToken` from an OAuth2 token response.
    ///
    /// # Errors
    ///
    /// Returns an error if `expires_in` exceeds the range of `i64`.
    pub fn from_response(
        access_token: String,
        refresh_token: Option<String>,
        token_type: String,
        scope_str: &str,
        expires_in: u64,
    ) -> crate::Result<Self> {
        let now = Utc::now();
        let expires_in_secs = i64::try_from(expires_in)
            .map_err(|_| crate::Error::Auth("Token expiration time overflow".into()))?;
        Ok(Self {
            access_token,
            refresh_token,
            token_type,
            scope: scope_str.split_whitespace().map(String::from).collect(),
            expires_at: now + chrono::TimeDelta::seconds(expires_in_secs),
            obtained_at: now,
        })
    }

    pub fn is_expired(&self) -> bool {
        Utc::now() >= self.expires_at
    }

    /// Check if the token has expired or will expire within the given buffer duration
    ///
    /// This is useful for proactively refreshing tokens before they actually expire.
    /// A recommended buffer is 5 minutes.
    pub fn is_expired_with_buffer(&self, buffer: chrono::TimeDelta) -> bool {
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
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            std::fs::DirBuilder::new().recursive(true).mode(0o700).create(parent)?;
        }
        #[cfg(not(unix))]
        {
            fs::create_dir_all(parent)?;
        }
    }

    let content = serde_json::to_string_pretty(token)?;

    #[cfg(unix)]
    {
        use std::fs::OpenOptions;
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;

        let mut file =
            OpenOptions::new().write(true).create(true).truncate(true).mode(0o600).open(path)?;
        file.write_all(content.as_bytes())?;
    }

    #[cfg(not(unix))]
    {
        fs::write(path, &content)?;
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
    let content = fs::read_to_string(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound { Error::TokenNotFound } else { Error::Io(e) }
    })?;
    let token: StoredToken = serde_json::from_str(&content).map_err(|e| {
        Error::Auth(format!(
            "Corrupt token file at {}: {}. Try running 'koyomi logout' then 'koyomi login' to fix.",
            path.display(),
            e
        ))
    })?;

    Ok(token)
}

/// Delete token at the specified path
///
/// # Errors
///
/// Returns an error if the file exists but cannot be removed.
pub fn delete_path(path: &Path) -> Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
}

fn token_path() -> Result<std::path::PathBuf> {
    Ok(config::config_dir()?.join(TOKEN_FILE))
}

/// Save token to the default token file path
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

/// Load token from the default token file path
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

/// Delete the default token file
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
    use chrono::TimeDelta;
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
            expires_at: now + TimeDelta::hours(1),
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
        assert!(error.to_string().contains("Token not found"));
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
            expires_at: now + TimeDelta::hours(1),
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
            expires_at: now + TimeDelta::hours(1),
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
            expires_at: now - TimeDelta::hours(1),
            obtained_at: now - TimeDelta::hours(2),
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
            expires_at: now + TimeDelta::minutes(3),
            obtained_at: now,
        };

        // Token expires in 3 minutes, so 5-minute buffer should consider it expired
        assert!(token.is_expired_with_buffer(TimeDelta::minutes(5)));
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
            expires_at: now + TimeDelta::hours(1),
            obtained_at: now,
        };

        // Token expires in 1 hour, so 5-minute buffer should not consider it expired
        assert!(!token.is_expired_with_buffer(TimeDelta::minutes(5)));
    }
}
