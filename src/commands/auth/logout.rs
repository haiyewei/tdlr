//! Logout command

use crate::i18n::{is_zh, pick};
use crate::telegram::SessionManager;
use anyhow::{bail, Result};
use colored::Colorize;

pub fn run(user_id: Option<i64>, all: bool) -> Result<()> {
    if all {
        let ids = SessionManager::list_user_ids()?;
        if ids.is_empty() {
            println!(
                "{}",
                pick("没有可退出的账号。", "No accounts to logout from.").yellow()
            );
            return Ok(());
        }

        for id in &ids {
            SessionManager::remove(*id)?;
        }
        SessionManager::clear_active();

        if is_zh() {
            println!("{} 已退出 {} 个账号。", "✓".green(), ids.len());
        } else {
            println!("{} Logged out from {} account(s).", "✓".green(), ids.len());
        }
    } else {
        let id = match user_id {
            Some(id) => id,
            None => SessionManager::get_active()?.ok_or_else(|| {
                anyhow::anyhow!(pick(
                    "当前没有活跃账号。请指定 user_id 或使用 --all。",
                    "No active account. Specify a user_id or use --all."
                ))
            })?,
        };

        if !SessionManager::exists(id) {
            bail!(
                "{}",
                if is_zh() {
                    format!("未找到账号 {}", id)
                } else {
                    format!("Account {} not found", id)
                }
            );
        }

        let display_name = SessionManager::get_account(id)?
            .map(|a| a.display_name)
            .unwrap_or_else(|| id.to_string());

        SessionManager::remove(id)?;

        if SessionManager::get_active()? == Some(id) {
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
            println!("{} 已退出 {}（{}）。", "✓".green(), id, display_name);
        } else {
            println!("{} Logged out from {} ({}).", "✓".green(), id, display_name);
        }
    }

    Ok(())
}
