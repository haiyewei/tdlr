//! QR code login method with DC migration support
//!
//! Flow:
//! 1. Call auth.exportLoginToken on default DC
//! 2. Show QR code with token
//! 3. Poll auth.exportLoginToken until:
//!    - Success: login complete
//!    - MigrateTo: call auth.importLoginToken on target DC with the token
//!
//! After DC migration, the session's home DC is automatically updated.

use crate::telegram::TelegramClient;
use anyhow::{bail, Context, Result};
use grammers_tl_types as tl;
use qrcode::render::unicode;
use qrcode::QrCode;
use std::io;
use std::time::Duration;

/// Max retries for import token before generating new QR
const MAX_IMPORT_RETRIES: u32 = 5;

fn render_qr(url: &str) {
    match QrCode::new(url.as_bytes()) {
        Ok(code) => {
            let image = code
                .render::<unicode::Dense1x2>()
                .dark_color(unicode::Dense1x2::Light)
                .light_color(unicode::Dense1x2::Dark)
                .build();
            println!("{}", image);
        }
        Err(e) => {
            eprintln!("Failed to generate QR code: {}", e);
            println!("Login URL: {}", url);
        }
    }
}

/// Try to complete login on target DC with retries
async fn try_import_login(
    tg: &TelegramClient,
    dc_id: i32,
    token: Vec<u8>,
) -> Result<Option<grammers_client::types::User>> {
    let client = tg.inner();
    let import_request = tl::functions::auth::ImportLoginToken { token };

    for attempt in 0..MAX_IMPORT_RETRIES {
        match client.invoke_in_dc(dc_id, &import_request).await {
            Ok(tl::enums::auth::LoginToken::Success(s)) => {
                return Ok(Some(handle_success(tg, s, Some(dc_id)).await?));
            }
            Ok(tl::enums::auth::LoginToken::Token(_)) => {
                // Token returned, need to wait
                tokio::time::sleep(Duration::from_secs(2)).await;
                continue;
            }
            Ok(tl::enums::auth::LoginToken::MigrateTo(m2)) => {
                // Redirect to another DC
                let import2 = tl::functions::auth::ImportLoginToken { token: m2.token };
                if let Ok(tl::enums::auth::LoginToken::Success(s)) =
                    client.invoke_in_dc(m2.dc_id, &import2).await
                {
                    return Ok(Some(handle_success(tg, s, Some(m2.dc_id)).await?));
                }
                tokio::time::sleep(Duration::from_secs(2)).await;
                continue;
            }
            Err(e) => {
                let err_str = format!("{:?}", e);
                if err_str.contains("SESSION_PASSWORD_NEEDED") {
                    bail!("2FA required. Use: tdlr auth login add --method phone");
                }
                if err_str.contains("AUTH_TOKEN_ALREADY_ACCEPTED") {
                    tg.set_home_dc_id(dc_id);
                    tokio::time::sleep(Duration::from_millis(300)).await;
                    return Ok(Some(tg.get_me().await?));
                }
                if err_str.contains("AUTH_TOKEN_EXPIRED") || err_str.contains("AUTH_TOKEN_INVALID")
                {
                    // Token expired/invalid, need new QR
                    return Ok(None);
                }
                // Other error, retry after delay
                if attempt < MAX_IMPORT_RETRIES - 1 {
                    tokio::time::sleep(Duration::from_secs(2)).await;
                    continue;
                }
                return Ok(None);
            }
        }
    }
    Ok(None)
}

/// Login using QR code scan
/// After successful login, returns the user. If DC migration occurred,
/// the session's home DC is automatically updated.
pub async fn login_with_qrcode(
    tg: &TelegramClient,
    api_id: i32,
    api_hash: &str,
) -> Result<grammers_client::types::User> {
    let client = tg.inner();
    println!("\n=== QR Code Login ===");
    println!("Scan the QR code with your Telegram app:");
    println!("(Open Telegram > Settings > Devices > Link Desktop Device)\n");

    // Input listener for manual refresh
    let (tx, mut rx) = tokio::sync::mpsc::channel::<()>(1);
    tokio::spawn(async move {
        loop {
            let mut input = String::new();
            if tokio::task::spawn_blocking(move || io::stdin().read_line(&mut input))
                .await
                .is_ok()
            {
                let _ = tx.send(()).await;
            }
        }
    });

    // Track if we're in the middle of completing a scanned QR
    let mut pending_migration: Option<(i32, Vec<u8>)> = None;

    loop {
        // If we have a pending migration from previous iteration, try to complete it first
        if let Some((dc_id, token)) = pending_migration.take() {
            println!("Completing login...");
            if let Some(user) = try_import_login(tg, dc_id, token).await? {
                return Ok(user);
            }
            println!("Session expired, generating new QR...\n");
        }

        // Step 1: Export login token
        let export_request = tl::functions::auth::ExportLoginToken {
            api_id,
            api_hash: api_hash.to_string(),
            except_ids: vec![],
        };

        let result = client
            .invoke(&export_request)
            .await
            .context("Failed to export login token")?;

        let token = match result {
            tl::enums::auth::LoginToken::Token(t) => t,
            tl::enums::auth::LoginToken::Success(s) => {
                return handle_success(tg, s, None).await;
            }
            tl::enums::auth::LoginToken::MigrateTo(m) => {
                // QR was already scanned, try to complete
                println!("Completing login...");
                if let Some(user) = try_import_login(tg, m.dc_id, m.token).await? {
                    return Ok(user);
                }
                println!("Session expired, generating new QR...\n");
                // Wait a bit before generating new QR to avoid rapid loop
                tokio::time::sleep(Duration::from_secs(2)).await;
                continue;
            }
        };

        // Step 2: Show QR code
        let url = format!("tg://login?token={}", base64_url::encode(&token.token));
        render_qr(&url);

        let expires_in = (token.expires - now()).max(1);
        println!("Waiting for scan... (expires in {}s)", expires_in);
        println!("Press Enter to refresh.\n");

        let deadline = tokio::time::Instant::now() + Duration::from_secs(expires_in as u64);

        // Step 3: Poll until success or MigrateTo
        loop {
            if rx.try_recv().is_ok() {
                println!("Refreshing...\n");
                break;
            }

            if tokio::time::Instant::now() >= deadline {
                println!("QR expired, refreshing...\n");
                break;
            }

            tokio::time::sleep(Duration::from_secs(2)).await;

            match client.invoke(&export_request).await {
                Ok(tl::enums::auth::LoginToken::Success(s)) => {
                    return handle_success(tg, s, None).await;
                }
                Ok(tl::enums::auth::LoginToken::Token(_)) => {
                    // Still waiting for scan
                    continue;
                }
                Ok(tl::enums::auth::LoginToken::MigrateTo(m)) => {
                    // QR was scanned! Try to complete login
                    println!("QR scanned! Completing login...");
                    if let Some(user) = try_import_login(tg, m.dc_id, m.token.clone()).await? {
                        return Ok(user);
                    }
                    // Failed, store for retry and break to generate new QR
                    pending_migration = Some((m.dc_id, m.token));
                    break;
                }
                Err(e) => {
                    let err_str = format!("{:?}", e);
                    if err_str.contains("SESSION_PASSWORD_NEEDED") {
                        bail!("2FA required. Use: tdlr auth login add --method phone");
                    }
                    // Other errors, keep polling
                    continue;
                }
            }
        }
    }
}

async fn handle_success(
    tg: &TelegramClient,
    success: tl::types::auth::LoginTokenSuccess,
    migrated_dc: Option<i32>,
) -> Result<grammers_client::types::User> {
    match success.authorization {
        tl::enums::auth::Authorization::Authorization(auth) => {
            if let tl::enums::User::User(raw_user) = auth.user {
                let name = raw_user.first_name.as_deref().unwrap_or("User");

                println!("\n✓ Login successful!");
                println!("Welcome, {}!", name);

                // If we migrated to a different DC, update session's home DC
                if let Some(dc_id) = migrated_dc {
                    tg.set_home_dc_id(dc_id);
                }

                // Small delay for session sync
                tokio::time::sleep(Duration::from_millis(300)).await;

                // Now get_me should work
                Ok(tg.get_me().await?)
            } else {
                bail!("Unexpected user type");
            }
        }
        tl::enums::auth::Authorization::SignUpRequired(_) => {
            bail!("Sign up required. Please register with official Telegram app first.");
        }
    }
}

fn now() -> i32 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i32
}
