//! Active account tracking

use anyhow::{bail, Result};
use std::fs;
use std::path::PathBuf;

const ACTIVE_FILE: &str = "sessions/.active";

/// Get active user_id
pub fn get_active() -> Result<Option<i64>> {
    let active_file = PathBuf::from(ACTIVE_FILE);
    if active_file.exists() {
        let content = fs::read_to_string(&active_file)?.trim().to_string();
        if let Ok(id) = content.parse::<i64>() {
            if super::manager::session_path(id).exists() {
                return Ok(Some(id));
            }
        }
    }
    Ok(None)
}

/// Set active account by user_id
pub fn set_active(user_id: i64) -> Result<()> {
    super::manager::ensure_dir()?;

    if !super::manager::session_path(user_id).exists() {
        bail!("Account {} not found", user_id);
    }

    fs::write(ACTIVE_FILE, user_id.to_string())?;
    Ok(())
}

/// Clear active account
pub fn clear_active() {
    let _ = fs::remove_file(ACTIVE_FILE);
}
