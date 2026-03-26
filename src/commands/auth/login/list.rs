//! List accounts command

use crate::i18n::{is_zh, pick};
use crate::telegram::SessionManager;
use anyhow::Result;
use colored::Colorize;

pub fn run() -> Result<()> {
    let accounts = SessionManager::list_accounts()?;
    let active = SessionManager::get_active()?;

    if accounts.is_empty() {
        println!(
            "{}",
            pick(
                "暂无账号。请先使用 'tdlr auth login add' 添加账号。",
                "No accounts. Use 'tdlr auth login add' to add one."
            )
            .yellow()
        );
        return Ok(());
    }

    println!("{}:", pick("账号", "Accounts").cyan().bold());
    for account in accounts {
        let marker = if active == Some(account.user_id) {
            if is_zh() {
                "（当前）".green().to_string()
            } else {
                " (active)".green().to_string()
            }
        } else {
            "".to_string()
        };
        let username_str = account
            .username
            .map(|u| format!(" @{}", u).dimmed().to_string())
            .unwrap_or_default();
        println!(
            "  {} - {}{}{}",
            account.user_id.to_string().yellow(),
            account.display_name,
            username_str,
            marker
        );
    }

    Ok(())
}
