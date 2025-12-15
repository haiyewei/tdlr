//! Logout command

use crate::telegram::SessionManager;
use anyhow::{bail, Result};
use colored::Colorize;

pub fn run(user_id: Option<i64>, all: bool) -> Result<()> {
    if all {
        let ids = SessionManager::list_user_ids()?;
        if ids.is_empty() {
            println!("{}", "No accounts to logout from.".yellow());
            return Ok(());
        }

        for id in &ids {
            SessionManager::remove(*id)?;
        }
        SessionManager::clear_active();

        println!("{} Logged out from {} account(s).", "✓".green(), ids.len());
    } else {
        let id = match user_id {
            Some(id) => id,
            None => SessionManager::get_active()?.ok_or_else(|| {
                anyhow::anyhow!("No active account. Specify a user_id or use --all.")
            })?,
        };

        if !SessionManager::exists(id) {
            bail!("Account {} not found", id);
        }

        let display_name = SessionManager::get_account(id)?
            .map(|a| a.display_name)
            .unwrap_or_else(|| id.to_string());

        SessionManager::remove(id)?;

        if SessionManager::get_active()? == Some(id) {
            SessionManager::clear_active();
            if let Some(first) = SessionManager::list_user_ids()?.first() {
                SessionManager::set_active(*first)?;
                println!("Switched to {}", first);
            }
        }

        println!("{} Logged out from {} ({}).", "✓".green(), id, display_name);
    }

    Ok(())
}
