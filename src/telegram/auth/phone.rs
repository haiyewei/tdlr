//! Phone number login method

use crate::i18n::{is_zh, pick};
use crate::telegram::{
    LoginCodeDelivery, LoginCodePreference, PhoneLoginCodeState, TelegramClient,
};
use anyhow::{bail, Context, Result};
use grammers_client::SignInError;
use std::io::{self, Write};
use std::time::Duration;

const MAX_CODE_ATTEMPTS: usize = 3;

fn sanitize_auth_code(input: &str) -> String {
    input
        .trim()
        .chars()
        .filter(|ch| {
            !ch.is_whitespace()
                && !matches!(
                    ch,
                    '\u{200B}' | '\u{200C}' | '\u{200D}' | '\u{2060}' | '\u{FEFF}' | '-'
                )
        })
        .collect()
}

fn delivery_label(via: LoginCodeDelivery) -> &'static str {
    match via {
        LoginCodeDelivery::App => pick("应用内验证码", "in-app code"),
        LoginCodeDelivery::Sms => "SMS",
        LoginCodeDelivery::Call => pick("语音电话", "voice call"),
        LoginCodeDelivery::FlashCall => pick("闪拨来电", "flash call"),
        LoginCodeDelivery::MissedCall => pick("未接来电", "missed call"),
        LoginCodeDelivery::FragmentSms => pick("片段短信", "fragment SMS"),
        LoginCodeDelivery::FirebaseSms => pick("Firebase SMS", "Firebase SMS"),
        LoginCodeDelivery::EmailCode => pick("邮箱验证码", "email code"),
        LoginCodeDelivery::SetUpEmailRequired => pick("需要先设置邮箱", "email setup required"),
        LoginCodeDelivery::SmsWord => pick("短信单词验证码", "SMS word code"),
        LoginCodeDelivery::SmsPhrase => pick("短信短语验证码", "SMS phrase code"),
    }
}

fn preference_matches(preference: LoginCodePreference, sent_via: LoginCodeDelivery) -> bool {
    match preference {
        LoginCodePreference::Auto => true,
        LoginCodePreference::App => sent_via == LoginCodeDelivery::App,
        LoginCodePreference::Sms => sent_via == LoginCodeDelivery::Sms,
    }
}

fn print_delivery_status(state: &PhoneLoginCodeState) {
    println!(
        "{}",
        if is_zh() {
            format!("当前验证码通道: {}", delivery_label(state.sent_via))
        } else {
            format!(
                "Current delivery channel: {}",
                delivery_label(state.sent_via)
            )
        }
    );

    if let Some(next_via) = state.next_via {
        if let Some(timeout) = state.timeout {
            println!(
                "{}",
                if is_zh() {
                    format!(
                        "后续可切换到: {}（约 {} 秒后可请求）",
                        delivery_label(next_via),
                        timeout
                    )
                } else {
                    format!(
                        "Next available channel: {} (requestable in about {}s)",
                        delivery_label(next_via),
                        timeout
                    )
                }
            );
        } else {
            println!(
                "{}",
                if is_zh() {
                    format!("后续可切换到: {}", delivery_label(next_via))
                } else {
                    format!("Next available channel: {}", delivery_label(next_via))
                }
            );
        }
    }
}

async fn try_switch_to_sms(
    tg: &TelegramClient,
    current: PhoneLoginCodeState,
) -> Result<PhoneLoginCodeState> {
    if current.sent_via == LoginCodeDelivery::Sms {
        return Ok(current);
    }

    if current.next_via != Some(LoginCodeDelivery::Sms) {
        println!(
            "{}",
            pick(
                "Telegram 当前没有提供可切换的 SMS 验证码，将继续使用现有通道。",
                "Telegram does not currently offer SMS as the next delivery channel; continuing with the current channel."
            )
        );
        return Ok(current);
    }

    let wait_secs = current.timeout.unwrap_or(0).max(0) as u64;
    if wait_secs > 0 {
        println!(
            "{}",
            if is_zh() {
                format!(
                    "Telegram 要求先等待 {} 秒，然后才能请求 SMS 验证码。",
                    wait_secs
                )
            } else {
                format!(
                    "Telegram requires waiting {}s before requesting SMS delivery.",
                    wait_secs
                )
            }
        );
        tokio::time::sleep(Duration::from_secs(wait_secs)).await;
    }

    println!(
        "{}",
        pick("正在请求 SMS 验证码...", "Requesting SMS delivery...")
    );

    let resent = tokio::time::timeout(Duration::from_secs(30), tg.resend_login_code(&current))
        .await
        .context(pick("请求 SMS 超时", "SMS resend timed out"))?;

    match resent {
        Ok(state) => {
            print_delivery_status(&state);
            if state.sent_via != LoginCodeDelivery::Sms {
                println!(
                    "{}",
                    if is_zh() {
                        format!(
                            "Telegram 实际改为通过 {} 发送验证码。",
                            delivery_label(state.sent_via)
                        )
                    } else {
                        format!(
                            "Telegram switched delivery to {} instead of SMS.",
                            delivery_label(state.sent_via)
                        )
                    }
                );
            }
            Ok(state)
        }
        Err(err) => {
            println!(
                "{}",
                if is_zh() {
                    format!("请求 SMS 失败，将继续使用当前通道: {}", err)
                } else {
                    format!("Failed to request SMS; continuing with the current channel: {err}")
                }
            );
            Ok(current)
        }
    }
}

async fn apply_delivery_preference(
    tg: &TelegramClient,
    state: PhoneLoginCodeState,
    preference: LoginCodePreference,
) -> Result<PhoneLoginCodeState> {
    match preference {
        LoginCodePreference::Auto => Ok(state),
        LoginCodePreference::App => {
            if !preference_matches(preference, state.sent_via) {
                println!(
                    "{}",
                    if is_zh() {
                        format!(
                            "已请求应用内验证码，但 Telegram 当前实际通过 {} 发送。",
                            delivery_label(state.sent_via)
                        )
                    } else {
                        format!(
                            "Requested an in-app code, but Telegram is currently using {}.",
                            delivery_label(state.sent_via)
                        )
                    }
                );
            }
            Ok(state)
        }
        LoginCodePreference::Sms => try_switch_to_sms(tg, state).await,
    }
}

/// Login using phone number and verification code
pub async fn login_with_phone(
    tg: &TelegramClient,
    api_hash: &str,
    code_via: LoginCodePreference,
) -> Result<grammers_client::peer::User> {
    println!("\n=== {} ===", pick("手机号登录", "Phone Login"));

    print!(
        "{}",
        pick(
            "请输入手机号（带国家代码，例如 +8613800138000）：",
            "Enter your phone number (with country code, e.g. +8613800138000): "
        )
    );
    io::stdout().flush()?;

    let mut phone = String::new();
    io::stdin().read_line(&mut phone)?;
    let phone = phone.trim();

    if phone.is_empty() {
        bail!("{}", pick("手机号不能为空", "Phone number cannot be empty"));
    }

    if is_zh() {
        println!("正在为 {} 请求登录验证码...", phone);
    } else {
        println!("Requesting login code for {}...", phone);
    }

    let state = tokio::time::timeout(Duration::from_secs(30), tg.send_login_code(phone, api_hash))
        .await
        .context(pick("请求超时", "Request timed out"))?
        .context(pick("请求登录验证码失败", "Failed to request login code"))?;

    println!("✓ {}", pick("验证码已发送。", "Login code sent!"));
    print_delivery_status(&state);
    let state = apply_delivery_preference(tg, state, code_via).await?;

    print!(
        "{}",
        pick("请输入验证码：", "Enter the verification code: ")
    );
    io::stdout().flush()?;

    let mut code = String::new();
    io::stdin().read_line(&mut code)?;
    let mut code = sanitize_auth_code(&code);

    if code.is_empty() {
        bail!(
            "{}",
            pick("验证码不能为空", "Verification code cannot be empty")
        );
    }

    let mut code_attempt = 1usize;
    let user = loop {
        println!("{}", pick("正在登录...", "Signing in..."));

        let result = tokio::time::timeout(
            Duration::from_secs(30),
            tg.sign_in_with_phone_code(&state, &code),
        )
        .await
        .context(pick("登录超时", "Sign in timed out"))?;

        match result {
            Ok(user) => break user,
            Err(SignInError::PasswordRequired(password_token)) => {
                println!("\n{}", pick("已启用两步验证。", "2FA is enabled."));
                print!(
                    "{}",
                    pick("请输入两步验证密码：", "Enter your 2FA password: ")
                );
                io::stdout().flush()?;

                let mut password = String::new();
                io::stdin().read_line(&mut password)?;

                break tokio::time::timeout(
                    Duration::from_secs(30),
                    tg.inner().check_password(password_token, password.trim()),
                )
                .await
                .context(pick("密码校验超时", "Password check timed out"))?
                .context(pick("密码验证失败", "Password verification failed"))?;
            }
            Err(SignInError::InvalidCode) => {
                if code_attempt >= MAX_CODE_ATTEMPTS {
                    bail!(
                        "{}",
                        pick(
                            "验证码无效，已超过最大重试次数。",
                            "Invalid verification code, maximum retry attempts exceeded."
                        )
                    );
                }

                println!(
                    "{}",
                    pick(
                        "验证码无效。请重新输入收到的验证码。",
                        "Invalid verification code. Please enter the code you received again."
                    )
                );

                code_attempt += 1;
                print!(
                    "{}",
                    if is_zh() {
                        format!(
                            "请重新输入验证码（第 {}/{} 次）：",
                            code_attempt, MAX_CODE_ATTEMPTS
                        )
                    } else {
                        format!(
                            "Enter the verification code again (attempt {}/{}): ",
                            code_attempt, MAX_CODE_ATTEMPTS
                        )
                    }
                );
                io::stdout().flush()?;

                let mut retry_code = String::new();
                io::stdin().read_line(&mut retry_code)?;
                let retry_code = sanitize_auth_code(&retry_code);

                if retry_code.is_empty() {
                    bail!(
                        "{}",
                        pick("验证码不能为空", "Verification code cannot be empty")
                    );
                }

                code = retry_code;
                continue;
            }
            Err(e) => bail!(
                "{}",
                if is_zh() {
                    format!("登录失败: {}", e)
                } else {
                    format!("Login failed: {}", e)
                }
            ),
        }
    };

    println!("\n✓ {}", pick("登录成功。", "Login successful!"));
    println!(
        "{}",
        if is_zh() {
            format!(
                "欢迎，{}！",
                user.first_name().unwrap_or(pick("用户", "User"))
            )
        } else {
            format!("Welcome, {}!", user.first_name().unwrap_or("User"))
        }
    );
    Ok(user)
}
