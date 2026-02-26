//! Direct forward using Telegram native API

use crate::telegram::upload::resolve_chat;
use crate::utils::{ChatIdentifier, TelegramLink};
use anyhow::{bail, Result};
use grammers_tl_types as tl;

/// Forward message using Telegram native forward API
pub async fn forward_direct(
    tg: &crate::telegram::TelegramClient,
    src: &TelegramLink,
    from_chat: Option<&str>,
    to_peer: &tl::enums::InputPeer,
    topic: Option<i32>,
    drop_author: bool,
) -> Result<i32> {
    let client = tg.inner();
    // Resolve source chat
    let from_peer = match &src.chat {
        ChatIdentifier::Username(username) => {
            let resolved = resolve_chat(tg, username).await?;
            resolved.input_peer
        }
        ChatIdentifier::ChannelId(id) => {
            let chat_str = format!("-100{}", id);
            let resolved = resolve_chat(tg, &chat_str).await?;
            resolved.input_peer
        }
        ChatIdentifier::External => {
            let chat_str = from_chat.ok_or_else(|| anyhow::anyhow!("--from-chat required"))?;
            let resolved = resolve_chat(tg, chat_str).await?;
            resolved.input_peer
        }
    };

    // Generate random ID
    let random_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos() as i64;

    let request = tl::functions::messages::ForwardMessages {
        silent: false,
        background: false,
        with_my_score: false,
        drop_author,
        drop_media_captions: false,
        noforwards: false,
        from_peer,
        id: vec![src.effective_message_id()],
        random_id: vec![random_id],
        to_peer: to_peer.clone(),
        top_msg_id: topic,
        schedule_date: None,
        send_as: None,
        quick_reply_shortcut: None,
        video_timestamp: None,
        allow_paid_stars: None,
        allow_paid_floodskip: false,
        reply_to: None,
        suggested_post: None,
        effect: None,
        schedule_repeat_period: None,
    };

    let result = client.invoke(&request).await?;
    extract_message_id(result)
}

/// Extract message ID from Updates response
fn extract_message_id(result: tl::enums::Updates) -> Result<i32> {
    match result {
        tl::enums::Updates::Updates(updates) => {
            for update in &updates.updates {
                if let tl::enums::Update::MessageId(msg_id) = update {
                    return Ok(msg_id.id);
                }
            }
            for update in updates.updates {
                match update {
                    tl::enums::Update::NewMessage(m) => {
                        if let tl::enums::Message::Message(msg) = m.message {
                            return Ok(msg.id);
                        }
                    }
                    tl::enums::Update::NewChannelMessage(m) => {
                        if let tl::enums::Message::Message(msg) = m.message {
                            return Ok(msg.id);
                        }
                    }
                    _ => {}
                }
            }
            bail!("No message ID in response");
        }
        tl::enums::Updates::UpdateShort(_) => bail!("Unexpected UpdateShort response"),
        tl::enums::Updates::Combined(c) => {
            for update in c.updates {
                if let tl::enums::Update::MessageId(msg_id) = update {
                    return Ok(msg_id.id);
                }
            }
            bail!("No message ID in Combined response");
        }
        tl::enums::Updates::UpdateShortMessage(m) => Ok(m.id),
        tl::enums::Updates::UpdateShortChatMessage(m) => Ok(m.id),
        tl::enums::Updates::TooLong => bail!("Updates too long"),
        tl::enums::Updates::UpdateShortSentMessage(m) => Ok(m.id),
    }
}
