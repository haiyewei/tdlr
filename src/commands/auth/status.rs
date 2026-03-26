//! Status command - show all accounts status

use crate::i18n::{is_zh, pick};
use crate::telegram::pool;
use anyhow::Result;
use colored::Colorize;

pub async fn run() -> Result<()> {
    let clients = pool().get_all().await?;

    if clients.is_empty() {
        println!(
            "{}",
            pick(
                "暂无账号。请先使用 'tdlr auth login add' 添加账号。",
                "No accounts. Use 'tdlr auth login add' to add one."
            )
            .yellow()
        );
        return Ok(());
    }

    println!(
        "{} ({} accounts):\n",
        pick("账号状态", "Account Status").cyan().bold(),
        clients.len()
    );

    // Check all accounts concurrently
    let tasks: Vec<_> = clients
        .into_iter()
        .map(|client| {
            tokio::spawn(async move {
                let user_id = client.user_id;
                match client.is_authorized().await {
                    Ok(true) => match client.get_me().await {
                        Ok(user) => {
                            let username = user.username().unwrap_or("-");
                            let first_name = user.first_name().unwrap_or(pick("未知", "Unknown"));
                            (user_id, true, format!("{} (@{})", first_name, username))
                        }
                        Err(_) => (
                            user_id,
                            true,
                            pick("已授权（获取信息失败）", "Authorized (failed to get info)")
                                .to_string(),
                        ),
                    },
                    Ok(false) => (user_id, false, pick("未授权", "Not authorized").to_string()),
                    Err(e) => (
                        user_id,
                        false,
                        if is_zh() {
                            format!("错误: {}", e)
                        } else {
                            format!("Error: {}", e)
                        },
                    ),
                }
            })
        })
        .collect();

    for task in tasks {
        if let Ok((user_id, ok, msg)) = task.await {
            if ok {
                println!("  {} - {} {}", user_id, "✓".green(), msg);
            } else {
                println!("  {} - {} {}", user_id, "✗".red(), msg.red());
            }
        }
    }

    Ok(())
}
