//! Add account command

use crate::cli::LoginMethod;
use crate::telegram::{
    auth::{login_with_phone, login_with_qrcode},
    session::AccountInfo,
    SessionManager, TelegramClient,
};
use anyhow::Result;
use colored::Colorize;
use std::fs;

/// API credentials (compiled in)
fn api_id() -> i32 {
    env!("TG_API_ID").parse().expect("Invalid TG_API_ID")
}

const API_HASH: &str = env!("TG_API_HASH");

pub async fn run(_name: Option<String>, method: LoginMethod) -> Result<()> {
    SessionManager::ensure_dir()?;

    // Use temp session for login
    let temp_name = format!(
        "temp_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    );

    let temp_session = SessionManager::session_path_str(&temp_name);

    println!("{}", "Connecting to Telegram...".dimmed());

    // Login and get user info
    let (user_id, display_name, username) = {
        let tg = TelegramClient::new_temp(&temp_name, api_id())?;

        let user = if tg.is_authorized().await? {
            println!("{}", "Already logged in!".yellow());
            tg.get_me().await?
        } else {
            match method {
                LoginMethod::Phone => login_with_phone(tg.inner(), API_HASH).await?,
                LoginMethod::Qr => login_with_qrcode(&tg, api_id(), API_HASH).await?,
            }
        };

        (
            user.raw.id(),
            user.first_name().unwrap_or("User").to_string(),
            user.username().map(|s| s.to_string()),
        )
        // tg is dropped here, releasing the file lock
    };

    // Small delay to ensure file is released
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Rename temp session to user_id.session
    let final_path = SessionManager::session_path(user_id);
    if final_path.exists() {
        println!(
            "{}",
            format!("Updating existing account {}...", user_id).yellow()
        );
        let _ = fs::remove_file(&final_path);
    }
    fs::rename(&temp_session, &final_path)?;

    // Save account metadata
    SessionManager::save_account(AccountInfo {
        user_id,
        display_name: display_name.clone(),
        username,
    })?;

    // Set as active
    SessionManager::set_active(user_id)?;

    println!("\n{} {}", "✓".green(), "Account added!".green().bold());
    println!("  {}: {}", "ID".cyan(), user_id);
    println!("  {}: {}", "Name".cyan(), display_name);
    println!("  {}", "(Set as active)".dimmed());
    Ok(())
}
