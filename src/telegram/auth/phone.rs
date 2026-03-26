//! Phone number login method

use crate::i18n::pick;
use anyhow::{bail, Context, Result};
use grammers_client::Client;
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

/// Login using phone number and verification code
pub async fn login_with_phone(
    client: &Client,
    api_hash: &str,
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

    if crate::i18n::is_zh() {
        println!("正在为 {} 请求登录验证码...", phone);
    } else {
        println!("Requesting login code for {}...", phone);
    }

    let token = tokio::time::timeout(
        Duration::from_secs(30),
        client.request_login_code(phone, api_hash),
    )
    .await
    .context(pick("请求超时", "Request timed out"))?
    .context(pick("请求登录验证码失败", "Failed to request login code"))?;

    println!("✓ {}", pick("验证码已发送。", "Login code sent!"));

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

        let result = tokio::time::timeout(Duration::from_secs(30), client.sign_in(&token, &code))
            .await
            .context(pick("登录超时", "Sign in timed out"))?;

        match result {
            Ok(user) => break user,
            Err(grammers_client::SignInError::PasswordRequired(password_token)) => {
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
                    client.check_password(password_token, password.trim()),
                )
                .await
                .context(pick("密码校验超时", "Password check timed out"))?
                .context(pick("密码验证失败", "Password verification failed"))?;
            }
            Err(grammers_client::SignInError::InvalidCode) => {
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
                    if crate::i18n::is_zh() {
                        format!("请重新输入验证码（第 {}/{} 次）：", code_attempt, MAX_CODE_ATTEMPTS)
                    } else {
                        format!(
                            "Enter the verification code again (attempt {}/{}): ",
                            code_attempt,
                            MAX_CODE_ATTEMPTS
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
                if crate::i18n::is_zh() {
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
        if crate::i18n::is_zh() {
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
