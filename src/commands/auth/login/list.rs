//! List accounts command

use crate::telegram::SessionManager;
use anyhow::Result;
use colored::Colorize;

pub fn run() -> Result<()> {
    let accounts = SessionManager::list_accounts()?;
    let active = SessionManager::get_active()?;

    if accounts.is_empty() {
        println!(
            "{}",
            "No accounts. Use 'tdlr auth login add' to add one.".yellow()
        );
        return Ok(());
    }

    println!("{}:", "Accounts".cyan().bold());
    for account in accounts {
        let marker = if active == Some(account.user_id) {
            " (active)".green().to_string()
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
