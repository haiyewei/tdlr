//! Status command - show all accounts status

use crate::telegram::pool;
use anyhow::Result;
use colored::Colorize;

pub async fn run() -> Result<()> {
    let clients = pool().get_all().await?;

    if clients.is_empty() {
        println!(
            "{}",
            "No accounts. Use 'tdlr auth login add' to add one.".yellow()
        );
        return Ok(());
    }

    println!(
        "{} ({} accounts):\n",
        "Account Status".cyan().bold(),
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
                            let first_name = user.first_name().unwrap_or("Unknown");
                            (user_id, true, format!("{} (@{})", first_name, username))
                        }
                        Err(_) => (user_id, true, "Authorized (failed to get info)".to_string()),
                    },
                    Ok(false) => (user_id, false, "Not authorized".to_string()),
                    Err(e) => (user_id, false, format!("Error: {}", e)),
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
