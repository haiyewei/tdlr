//! Remove account command

use crate::telegram::SessionManager;
use anyhow::{bail, Result};
use colored::Colorize;

pub fn run(user_id: i64) -> Result<()> {
    if !SessionManager::exists(user_id) {
        bail!("Account {} not found", user_id);
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
            println!("Switched to {}", first);
        }
    }

    println!(
        "{} Account {} ({}) removed.",
        "✓".green(),
        user_id,
        display_name
    );
    Ok(())
}
