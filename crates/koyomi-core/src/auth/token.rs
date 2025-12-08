use std::fs;
use std::path::Path;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{Error, Result, config};

const TOKEN_FILE: &str = "google_tokens.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StoredToken {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub token_type: String,
    pub scope: Vec<String>,
    pub expires_at: DateTime<Utc>,
    pub obtained_at: DateTime<Utc>,
}

/// Save token to the specified path
pub fn save_to_path(token: &StoredToken, path: &Path) -> Result<()> {
    // Create parent directory if it doesn't exist
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Convert to JSON (pretty printed)
    let content = serde_json::to_string_pretty(token)?;

    // Write file
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
pub fn load_from_path(path: &Path) -> Result<StoredToken> {
    if !path.exists() {
        return Err(Error::TokenNotFound);
    }

    let content = fs::read_to_string(path)?;
    let token: StoredToken = serde_json::from_str(&content)?;

    Ok(token)
}

/// Delete token at the specified path
pub fn delete_path(path: &Path) -> Result<()> {
    if path.exists() {
        fs::remove_file(path)?;
    }

    Ok(())
}

/// Save token to ~/.config/koyomi/google_tokens.json
pub fn save(token: &StoredToken) -> Result<()> {
    let path = config::config_dir()?.join(TOKEN_FILE);
    save_to_path(token, &path)
}

/// Load token from ~/.config/koyomi/google_tokens.json
pub fn load() -> Result<StoredToken> {
    let path = config::config_dir()?.join(TOKEN_FILE);
    load_from_path(&path)
}

/// Delete token from ~/.config/koyomi/google_tokens.json
pub fn delete() -> Result<()> {
    let path = config::config_dir()?.join(TOKEN_FILE);
    delete_path(&path)
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

        // Save
        let result = save_to_path(&token, &path);
        assert!(result.is_ok());

        // Load
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

        // Save first
        save_to_path(&token, &path).unwrap();

        // Verify file exists
        assert!(path.exists());

        // Delete
        let result = delete_path(&path);
        assert!(result.is_ok());

        // Verify file is gone
        assert!(!path.exists());
    }

    #[test]
    fn delete_nonexistent_file_succeeds() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(TOKEN_FILE);

        // Delete when file doesn't exist should succeed
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
}
