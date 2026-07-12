//! Show recent messages from the Verification Codes dialog.

use crate::i18n::{is_zh, pick};
use crate::telegram::pool;
use anyhow::{bail, Result};
use chrono::Local;
use grammers_session::types::PeerRef;

fn dialog_score(peer_id: i64, name: Option<&str>) -> Option<u8> {
    if peer_id == 777_000 {
        return Some(0);
    }
    if peer_id == 42_777 {
        return Some(1);
    }

    let normalized = name.unwrap_or_default().trim().to_ascii_lowercase();
    if normalized == "verification codes" {
        return Some(2);
    }
    if normalized == "telegram" {
        return Some(3);
    }

    None
}

fn message_body(message: &grammers_client::message::Message) -> String {
    let text = message.text().trim();
    if !text.is_empty() {
        return text.to_string();
    }
    if message.media().is_some() {
        return pick("[媒体消息]", "[media message]").to_string();
    }
    if message.action().is_some() {
        return pick("[服务消息]", "[service message]").to_string();
    }
    pick("[空消息]", "[empty message]").to_string()
}

pub async fn run(limit: usize, account: Option<i64>) -> Result<()> {
    if limit == 0 {
        bail!(
            "{}",
            pick("--limit 必须大于 0", "--limit must be greater than 0")
        );
    }

    let client = if let Some(id) = account {
        pool().get(id).await?
    } else {
        pool().get_active().await?
    };

    if !client.is_authorized().await? {
        bail!(
            "{}",
            if is_zh() {
                format!(
                    "账号 {} 未授权。请先使用 'tdlr auth login add' 登录。",
                    client.user_id
                )
            } else {
                format!(
                    "Account {} is not authorized. Login first with 'tdlr auth login add'.",
                    client.user_id
                )
            }
        );
    }

    let mut dialogs = client.inner().iter_dialogs();
    let mut target: Option<(u8, String, i64, PeerRef)> = None;

    while let Some(dialog) = dialogs.next().await? {
        let peer = dialog.peer();
        let name = peer.name().unwrap_or(pick("未知", "Unknown")).to_string();
        let bare_id = peer.id().bare_id_unchecked();

        let Some(score) = dialog_score(bare_id, Some(&name)) else {
            continue;
        };

        let candidate = (score, name, bare_id, dialog.peer_ref());
        match &target {
            Some((best_score, ..)) if *best_score <= score => {}
            _ => target = Some(candidate),
        }
    }

    let Some((_, dialog_name, peer_id, peer_ref)) = target else {
        bail!(
            "{}",
            pick(
                "未找到 Verification Codes 会话。请先确保该会话在当前账号的对话列表中出现过。",
                "Could not find the Verification Codes dialog. Make sure it has appeared in the current account's dialog list."
            )
        );
    };

    println!(
        "{}",
        if is_zh() {
            format!("会话: {} ({})", dialog_name, peer_id)
        } else {
            format!("Dialog: {} ({})", dialog_name, peer_id)
        }
    );

    let mut messages = client.inner().iter_messages(peer_ref).limit(limit);
    let mut count = 0usize;

    while let Some(message) = messages.next().await? {
        count += 1;
        let timestamp = message
            .date()
            .with_timezone(&Local)
            .format("%Y-%m-%d %H:%M:%S");
        println!();
        println!("[{}] #{}", timestamp, message.id());
        println!("{}", message_body(&message));
    }

    if count == 0 {
        println!(
            "{}",
            pick(
                "该会话暂无可显示的消息。",
                "No messages are available in this dialog."
            )
        );
    }

    Ok(())
}
