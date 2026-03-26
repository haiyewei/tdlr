//! Active account tracking

use crate::i18n::is_zh;
use anyhow::{bail, Result};
use std::fs;

/// Get active user_id
pub fn get_active() -> Result<Option<i64>> {
    let active_file = super::manager::active_file();
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
        bail!(
            "{}",
            if is_zh() {
                format!("未找到账号 {}", user_id)
            } else {
                format!("Account {} not found", user_id)
            }
        );
    }

    fs::write(super::manager::active_file(), user_id.to_string())?;
    Ok(())
}

/// Clear active account
pub fn clear_active() {
    let _ = fs::remove_file(super::manager::active_file());
}
