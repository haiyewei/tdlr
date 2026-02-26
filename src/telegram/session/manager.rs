//! Session file management

use super::account::{self, AccountInfo};
use super::active;
use anyhow::Result;
use std::fs;
use std::path::PathBuf;

/// Session manager for handling multiple accounts
pub struct SessionManager;

impl SessionManager {
    /// Get sessions directory path
    pub fn sessions_dir() -> PathBuf {
        sessions_dir()
    }

    /// Get session file path by user_id
    pub fn session_path(user_id: i64) -> PathBuf {
        session_path(user_id)
    }

    /// Get session file path by string (for backwards compat during login)
    pub fn session_path_str(name: &str) -> PathBuf {
        session_path_str(name)
    }

    /// Ensure sessions directory exists
    pub fn ensure_dir() -> Result<()> {
        ensure_dir()
    }

    /// Load accounts metadata
    pub fn load_accounts() -> Result<std::collections::HashMap<i64, AccountInfo>> {
        account::load_accounts()
    }

    /// Save accounts metadata
    pub fn save_accounts(accounts: &std::collections::HashMap<i64, AccountInfo>) -> Result<()> {
        account::save_accounts(accounts)
    }

    /// Add or update account info
    pub fn save_account(info: AccountInfo) -> Result<()> {
        account::save_account(info)
    }

    /// Get account info by user_id
    pub fn get_account(user_id: i64) -> Result<Option<AccountInfo>> {
        account::get_account(user_id)
    }

    /// List all user_ids
    pub fn list_user_ids() -> Result<Vec<i64>> {
        list_user_ids()
    }

    /// List all accounts with info
    pub fn list_accounts() -> Result<Vec<AccountInfo>> {
        list_accounts()
    }

    /// Get active user_id
    pub fn get_active() -> Result<Option<i64>> {
        active::get_active()
    }

    /// Set active account by user_id
    pub fn set_active(user_id: i64) -> Result<()> {
        active::set_active(user_id)
    }

    /// Clear active account
    pub fn clear_active() {
        active::clear_active()
    }

    /// Remove account by user_id
    pub fn remove(user_id: i64) -> Result<()> {
        // Remove session directory or file
        let account_dir = sessions_dir().join(user_id.to_string());
        if account_dir.exists() {
            let _ = fs::remove_dir_all(&account_dir);
        } else {
            // Backward compatibility
            let session_file = sessions_dir().join(format!("{}.session", user_id));
            if session_file.exists() {
                let _ = fs::remove_file(&session_file);
            }
        }

        // Remove from accounts.json
        account::remove_account(user_id)?;

        Ok(())
    }

    /// Check if account exists by user_id
    pub fn exists(user_id: i64) -> bool {
        session_path(user_id).exists()
    }
}

// Internal functions used by other submodules

/// Get tdlr config directory path
pub fn config_dir() -> PathBuf {
    if let Some(mut path) = dirs::config_dir() {
        path.push("tdlr");
        path
    } else {
        PathBuf::from(".")
    }
}

/// Get sessions directory path
pub fn sessions_dir() -> PathBuf {
    config_dir().join("sessions")
}

/// Get accounts metadata file path
pub fn accounts_file() -> PathBuf {
    config_dir().join("accounts.json")
}

/// Get active session id file path
pub fn active_file() -> PathBuf {
    config_dir().join(".active")
}

/// Get session file path by user_id
pub fn session_path(user_id: i64) -> PathBuf {
    let dir = sessions_dir().join(user_id.to_string());
    let _ = fs::create_dir_all(&dir);
    dir.join(format!("{}.session", user_id))
}

/// Get session file path by string
pub fn session_path_str(name: &str) -> PathBuf {
    let dir = sessions_dir().join(name);
    let _ = fs::create_dir_all(&dir);
    dir.join(format!("{}.session", name))
}

/// Ensure sessions directory exists
pub fn ensure_dir() -> Result<()> {
    let config = config_dir();
    if !config.exists() {
        fs::create_dir_all(&config)?;
    }
    let dir = sessions_dir();
    if !dir.exists() {
        fs::create_dir_all(&dir)?;
    }
    Ok(())
}

/// List all user_ids from session files
pub fn list_user_ids() -> Result<Vec<i64>> {
    ensure_dir()?;

    let mut ids = Vec::new();
    let dir = sessions_dir();

    if dir.exists() {
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                if let Some(name) = path.file_name() {
                    let name_str = name.to_string_lossy();
                    if !name_str.starts_with("temp_") {
                        if let Ok(id) = name_str.parse::<i64>() {
                            let session_file = path.join(format!("{}.session", id));
                            if session_file.exists() {
                                ids.push(id);
                            }
                        }
                    }
                }
            } else if path.extension().map(|e| e == "session").unwrap_or(false) {
                // Backward compatibility for root session files
                if let Some(name) = path.file_stem() {
                    let name_str = name.to_string_lossy();
                    if !name_str.starts_with("temp_") {
                        if let Ok(id) = name_str.parse::<i64>() {
                            ids.push(id);
                        }
                    }
                }
            }
        }
    }

    ids.sort();
    ids.dedup();

    Ok(ids)
}

/// List all accounts with info
pub fn list_accounts() -> Result<Vec<AccountInfo>> {
    let ids = list_user_ids()?;
    let accounts = account::load_accounts()?;

    Ok(ids
        .iter()
        .filter_map(|id| {
            accounts.get(id).cloned().or_else(|| {
                // Fallback for sessions without metadata
                Some(AccountInfo {
                    user_id: *id,
                    display_name: id.to_string(),
                    username: None,
                })
            })
        })
        .collect())
}
