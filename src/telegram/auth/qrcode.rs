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

use crate::i18n::{is_zh, pick};
use crate::telegram::TelegramClient;
use anyhow::{bail, Context, Result};
use grammers_client::peer::User;
use grammers_tl_types as tl;
use qrcode::render::unicode;
use qrcode::QrCode;
use std::io;
use std::time::Duration;

/// Max retries for import token before generating new QR
const MAX_IMPORT_RETRIES: u32 = 5;

pub(crate) enum QrLoginTokenExport {
    Ready { url: String, expires_at: i32 },
    Authorized(User),
    PendingMigration { dc_id: i32, token: Vec<u8> },
}

fn format_qr_login_error(err: &str) -> String {
    if is_zh() {
        format!("二维码登录失败: {}", err)
    } else {
        format!("QR login failed: {}", err)
    }
}

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
            eprintln!(
                "{}: {}",
                pick("生成二维码失败", "Failed to generate QR code"),
                e
            );
            println!("{}: {}", pick("登录链接", "Login URL"), url);
        }
    }
}

/// Try to complete login on target DC with retries
pub(crate) async fn try_import_login(
    tg: &TelegramClient,
    dc_id: i32,
    token: Vec<u8>,
) -> Result<Option<User>> {
    let client = tg.inner();
    let mut current_dc = dc_id;
    let mut current_token = token;
    let mut last_error = None::<String>;

    for attempt in 0..MAX_IMPORT_RETRIES {
        let import_request = tl::functions::auth::ImportLoginToken {
            token: current_token.clone(),
        };

        match client.invoke_in_dc(current_dc, &import_request).await {
            Ok(tl::enums::auth::LoginToken::Success(s)) => {
                return Ok(Some(handle_success(tg, s, Some(current_dc)).await?));
            }
            Ok(tl::enums::auth::LoginToken::Token(next)) => {
                // Telegram may rotate the token before returning success.
                // Continue the import flow with the refreshed token.
                current_token = next.token;
                if next.expires <= now() {
                    return Ok(None);
                }
                last_error = None;
                tokio::time::sleep(Duration::from_secs(2)).await;
                continue;
            }
            Ok(tl::enums::auth::LoginToken::MigrateTo(m2)) => {
                current_dc = m2.dc_id;
                current_token = m2.token;
                last_error = None;
                continue;
            }
            Err(e) => {
                let err_str = format!("{:?}", e);
                if err_str.contains("SESSION_PASSWORD_NEEDED") {
                    bail!(
                        "{}",
                        pick(
                            "需要两步验证。请使用: tdlr auth login add --method phone",
                            "2FA required. Use: tdlr auth login add --method phone"
                        )
                    );
                }
                if err_str.contains("AUTH_TOKEN_ALREADY_ACCEPTED") {
                    tg.set_home_dc_id(current_dc).await;
                    tokio::time::sleep(Duration::from_millis(300)).await;
                    match tg.get_me().await {
                        Ok(user) => return Ok(Some(user)),
                        Err(get_me_err) => {
                            last_error = Some(get_me_err.to_string());
                            if attempt < MAX_IMPORT_RETRIES - 1 {
                                tokio::time::sleep(Duration::from_secs(1)).await;
                                continue;
                            }
                            break;
                        }
                    }
                }
                if err_str.contains("AUTH_TOKEN_EXPIRED") || err_str.contains("AUTH_TOKEN_INVALID")
                {
                    // Token expired/invalid, need new QR
                    return Ok(None);
                }
                // Other error, retry after delay
                last_error = Some(err_str);
                if attempt < MAX_IMPORT_RETRIES - 1 {
                    tokio::time::sleep(Duration::from_secs(2)).await;
                    continue;
                }
                break;
            }
        }
    }

    if let Some(err) = last_error {
        bail!("{}", format_qr_login_error(&err));
    }

    Ok(None)
}

pub(crate) async fn export_qr_login_token(
    tg: &TelegramClient,
    api_id: i32,
    api_hash: &str,
) -> Result<QrLoginTokenExport> {
    let export_request = tl::functions::auth::ExportLoginToken {
        api_id,
        api_hash: api_hash.to_string(),
        except_ids: vec![],
    };

    let result = tg
        .inner()
        .invoke(&export_request)
        .await
        .context(pick("导出登录令牌失败", "Failed to export login token"))?;

    match result {
        tl::enums::auth::LoginToken::Token(token) => Ok(QrLoginTokenExport::Ready {
            url: format!("tg://login?token={}", base64_url::encode(&token.token)),
            expires_at: token.expires,
        }),
        tl::enums::auth::LoginToken::Success(success) => Ok(QrLoginTokenExport::Authorized(
            handle_success(tg, success, None).await?,
        )),
        tl::enums::auth::LoginToken::MigrateTo(migrate) => {
            Ok(QrLoginTokenExport::PendingMigration {
                dc_id: migrate.dc_id,
                token: migrate.token,
            })
        }
    }
}

/// Login using QR code scan
/// After successful login, returns the user. If DC migration occurred,
/// the session's home DC is automatically updated.
pub async fn login_with_qrcode(
    tg: &TelegramClient,
    api_id: i32,
    api_hash: &str,
) -> Result<grammers_client::peer::User> {
    let client = tg.inner();
    println!("\n=== {} ===", pick("二维码登录", "QR Code Login"));
    println!(
        "{}",
        pick(
            "请使用 Telegram 应用扫描二维码：",
            "Scan the QR code with your Telegram app:"
        )
    );
    println!(
        "{}\n",
        pick(
            "（打开 Telegram > 设置 > 设备 > 关联桌面设备）",
            "(Open Telegram > Settings > Devices > Link Desktop Device)"
        )
    );

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
            println!("{}", pick("正在完成登录...", "Completing login..."));
            if let Some(user) = try_import_login(tg, dc_id, token).await? {
                return Ok(user);
            }
            println!(
                "{}",
                pick(
                    "会话已过期，正在生成新的二维码...\n",
                    "Session expired, generating new QR...\n"
                )
            );
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
            .context(pick("导出登录令牌失败", "Failed to export login token"))?;

        let token = match result {
            tl::enums::auth::LoginToken::Token(t) => t,
            tl::enums::auth::LoginToken::Success(s) => {
                return handle_success(tg, s, None).await;
            }
            tl::enums::auth::LoginToken::MigrateTo(m) => {
                // QR was already scanned, try to complete
                println!("{}", pick("正在完成登录...", "Completing login..."));
                if let Some(user) = try_import_login(tg, m.dc_id, m.token).await? {
                    return Ok(user);
                }
                println!(
                    "{}",
                    pick(
                        "会话已过期，正在生成新的二维码...\n",
                        "Session expired, generating new QR...\n"
                    )
                );
                // Wait a bit before generating new QR to avoid rapid loop
                tokio::time::sleep(Duration::from_secs(2)).await;
                continue;
            }
        };

        // Step 2: Show QR code
        let url = format!("tg://login?token={}", base64_url::encode(&token.token));
        render_qr(&url);

        let expires_in = (token.expires - now()).max(1);
        if is_zh() {
            println!("等待扫描...（{} 秒后过期）", expires_in);
        } else {
            println!("Waiting for scan... (expires in {}s)", expires_in);
        }
        println!("{}", pick("按回车刷新。\n", "Press Enter to refresh.\n"));

        let deadline = tokio::time::Instant::now() + Duration::from_secs(expires_in as u64);

        // Step 3: Poll until success or MigrateTo
        loop {
            if rx.try_recv().is_ok() {
                println!("{}", pick("正在刷新...\n", "Refreshing...\n"));
                break;
            }

            if tokio::time::Instant::now() >= deadline {
                println!(
                    "{}",
                    pick("二维码已过期，正在刷新...\n", "QR expired, refreshing...\n")
                );
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
                    println!(
                        "{}",
                        pick(
                            "二维码已扫描，正在完成登录...",
                            "QR scanned! Completing login..."
                        )
                    );
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
                        bail!(
                            "{}",
                            pick(
                                "需要两步验证。请使用: tdlr auth login add --method phone",
                                "2FA required. Use: tdlr auth login add --method phone"
                            )
                        );
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
) -> Result<grammers_client::peer::User> {
    match success.authorization {
        tl::enums::auth::Authorization::Authorization(auth) => {
            if let tl::enums::User::User(raw_user) = auth.user {
                let name = raw_user
                    .first_name
                    .as_deref()
                    .unwrap_or(pick("用户", "User"));

                println!("\n✓ {}", pick("登录成功。", "Login successful!"));
                if is_zh() {
                    println!("欢迎，{}！", name);
                } else {
                    println!("Welcome, {}!", name);
                }

                // If we migrated to a different DC, update session's home DC
                if let Some(dc_id) = migrated_dc {
                    tg.set_home_dc_id(dc_id).await;
                }

                // Small delay for session sync
                tokio::time::sleep(Duration::from_millis(300)).await;

                // Now get_me should work
                Ok(tg.get_me().await?)
            } else {
                bail!("{}", pick("意外的用户类型", "Unexpected user type"));
            }
        }
        tl::enums::auth::Authorization::SignUpRequired(_) => {
            bail!(
                "{}",
                pick(
                    "需要先注册账号。请先使用官方 Telegram 应用完成注册。",
                    "Sign up required. Please register with official Telegram app first."
                )
            );
        }
    }
}

fn now() -> i32 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i32
}
