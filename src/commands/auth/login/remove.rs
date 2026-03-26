//! Remove account command

use crate::i18n::is_zh;
use crate::telegram::SessionManager;
use anyhow::{bail, Result};
use colored::Colorize;

pub fn run(user_id: i64) -> Result<()> {
    if !SessionManager::exists(user_id) {
        bail!(
            "{}",
            if is_zh() {
                format!("未找到账号 {}", user_id)
            } else {
                format!("Account {} not found", user_id)
            }
        );
    }

    // Get display name before removing
    let display_name = SessionManager::get_account(user_id)?
        .map(|a| a.display_name)
        .unwrap_or_else(|| user_id.to_string());

    SessionManager::remove(user_id)?;

    // Update active if needed
    if SessionManager::get_active()? == Some(user_id) {
        SessionManager::clear_active();
        if let Some(first) = SessionManager::list_user_ids()?.first() {
            SessionManager::set_active(*first)?;
            println!(
                "{}",
                if is_zh() {
                    format!("已切换到 {}", first)
                } else {
                    format!("Switched to {}", first)
                }
            );
        }
    }

    if is_zh() {
        println!(
            "{} 已移除账号 {}（{}）。",
            "✓".green(),
            user_id,
            display_name
        );
    } else {
        println!(
            "{} Account {} ({}) removed.",
            "✓".green(),
            user_id,
            display_name
        );
    }
    Ok(())
}
