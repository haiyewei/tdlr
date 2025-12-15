//! Phone number login method

use anyhow::{bail, Context, Result};
use grammers_client::Client;
use std::io::{self, Write};
use std::time::Duration;

/// Login using phone number and verification code
pub async fn login_with_phone(
    client: &Client,
    api_hash: &str,
) -> Result<grammers_client::types::User> {
    println!("\n=== Phone Login ===");

    print!("Enter your phone number (with country code, e.g. +8613800138000): ");
    io::stdout().flush()?;

    let mut phone = String::new();
    io::stdin().read_line(&mut phone)?;
    let phone = phone.trim();

    if phone.is_empty() {
        bail!("Phone number cannot be empty");
    }

    println!("Requesting login code for {}...", phone);

    let token = tokio::time::timeout(
        Duration::from_secs(30),
        client.request_login_code(phone, api_hash),
    )
    .await
    .context("Request timed out")?
    .context("Failed to request login code")?;

    println!("✓ Login code sent!");

    print!("Enter the verification code: ");
    io::stdout().flush()?;

    let mut code = String::new();
    io::stdin().read_line(&mut code)?;
    let code = code.trim();

    if code.is_empty() {
        bail!("Verification code cannot be empty");
    }

    println!("Signing in...");

    let result = tokio::time::timeout(Duration::from_secs(30), client.sign_in(&token, code))
        .await
        .context("Sign in timed out")?;

    let user = match result {
        Ok(user) => user,
        Err(grammers_client::SignInError::PasswordRequired(password_token)) => {
            println!("\n2FA is enabled.");
            print!("Enter your 2FA password: ");
            io::stdout().flush()?;

            let mut password = String::new();
            io::stdin().read_line(&mut password)?;

            tokio::time::timeout(
                Duration::from_secs(30),
                client.check_password(password_token, password.trim()),
            )
            .await
            .context("Password check timed out")?
            .context("Password verification failed")?
        }
        Err(e) => bail!("Login failed: {}", e),
    };

    println!("\n✓ Login successful!");
    println!("Welcome, {}!", user.first_name().unwrap_or("User"));
    Ok(user)
}
