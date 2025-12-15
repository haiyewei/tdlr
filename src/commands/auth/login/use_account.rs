//! Switch active account command

use crate::telegram::SessionManager;
use anyhow::Result;
use colored::Colorize;

pub fn run(user_id: i64) -> Result<()> {
    SessionManager::set_active(user_id)?;

    let display_name = SessionManager::get_account(user_id)?
        .map(|a| a.display_name)
        .unwrap_or_else(|| user_id.to_string());

    println!("{} Now using {} ({})", "✓".green(), user_id, display_name);
    Ok(())
}
