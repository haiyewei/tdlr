//! Forward command entry point

use super::clone;
use super::direct;
use super::output;
use crate::cli::ForwardMode;
use crate::telegram::upload::{resolve_chat, ResolvedChat};
use crate::telegram::{pool, SessionManager};
use crate::utils::{parse_source, ChatIdentifier, TelegramLink};
use anyhow::{bail, Result};
use colored::Colorize;

pub async fn run(
    from: Vec<String>,
    from_chat: Option<String>,
    to: Option<String>,
    mode: ForwardMode,
    topic: Option<i32>,
    account: Option<i64>,
    drop_author: bool,
) -> Result<()> {
    if from.is_empty() {
        bail!("No source messages specified");
    }

    // Parse all sources
    let mut sources: Vec<TelegramLink> = Vec::new();
    for input in &from {
        match parse_source(input) {
            Ok(src) => sources.push(src),
            Err(e) => eprintln!("Warning: {}", e),
        }
    }

    if sources.is_empty() {
        bail!("No valid source messages");
    }

    // Check if any source needs --from-chat
    let needs_from_chat = sources.iter().any(|s| s.is_external());
    if needs_from_chat && from_chat.is_none() {
        bail!("--from-chat is required when using plain message IDs");
    }

    // Get client
    let client = if let Some(id) = account {
        pool().get(id).await?
    } else {
        pool().get_active().await?
    };

    if !client.is_authorized().await? {
        bail!(
            "Account {} not authorized. Please login first.",
            client.user_id
        );
    }

    // Show account info
    let account_info = SessionManager::get_account(client.user_id)?;
    let name = account_info
        .map(|a| a.display_name)
        .unwrap_or_else(|| client.user_id.to_string());
    output::print_account_header(&name, client.user_id);

    // Resolve destination chat
    let dest_str = to.as_deref().unwrap_or("");
    let dest = resolve_chat(&client, dest_str).await?;
    println!("  {} {}", "To:".cyan(), dest.name);

    let mut success = 0usize;
    let mut failed = 0usize;
    let total = sources.len();

    for (i, src) in sources.iter().enumerate() {
        let display = format!("{}:{}", src.chat.display(), src.effective_message_id());
        output::print_progress(i, total, &display);

        // Determine actual mode for this message
        let actual_mode = match mode {
            ForwardMode::Direct => ForwardMode::Direct,
            ForwardMode::Clone => ForwardMode::Clone,
            ForwardMode::Smart => {
                // Resolve source chat to check its properties
                match resolve_source_chat(&client, src, from_chat.as_deref()).await {
                    Ok(source_chat) => detect_mode_from_chat(&source_chat),
                    Err(_) => {
                        // If we can't resolve, default to direct
                        ForwardMode::Direct
                    }
                }
            }
        };

        // Show mode indicator
        let mode_str = match actual_mode {
            ForwardMode::Direct => "direct",
            ForwardMode::Clone => "clone",
            ForwardMode::Smart => unreachable!(),
        };
        print!(" [{}]", mode_str.dimmed());

        let result = match actual_mode {
            ForwardMode::Direct => {
                direct::forward_direct(
                    &client,
                    src,
                    from_chat.as_deref(),
                    &dest.input_peer,
                    topic,
                    drop_author,
                )
                .await
            }
            ForwardMode::Clone => {
                // Print newline so progress bar appears on its own line
                println!();
                clone::forward_clone(&client, src, from_chat.as_deref(), &dest, topic).await
            }
            ForwardMode::Smart => unreachable!(),
        };

        match result {
            Ok(msg_id) => {
                output::print_success(msg_id);
                success += 1;
            }
            Err(e) => {
                output::print_failure(&e.to_string());
                failed += 1;
            }
        }
    }

    output::print_summary(success, failed);
    Ok(())
}

/// Resolve source chat from TelegramLink
async fn resolve_source_chat(
    client: &crate::telegram::TelegramClient,
    src: &TelegramLink,
    from_chat: Option<&str>,
) -> Result<ResolvedChat> {
    let chat_str = match &src.chat {
        ChatIdentifier::Username(u) => u.clone(),
        ChatIdentifier::ChannelId(id) => format!("-100{}", id),
        ChatIdentifier::External => from_chat
            .ok_or_else(|| anyhow::anyhow!("--from-chat required"))?
            .to_string(),
    };
    resolve_chat(client, &chat_str).await
}

/// Detect forward mode based on resolved chat properties
/// - Public chat (has username) -> Direct
/// - Private chat with noforwards -> Clone
/// - Private chat without noforwards -> Direct
fn detect_mode_from_chat(chat: &ResolvedChat) -> ForwardMode {
    if chat.noforwards {
        // Chat has forwarding restricted, must use clone
        ForwardMode::Clone
    } else {
        // No restriction, use direct (works for both public and private)
        ForwardMode::Direct
    }
}
