//! Message fetching from Telegram

use crate::i18n::is_zh;
use crate::telegram::upload::{resolve_chat, ResolvedChat};
use crate::telegram::TelegramClient;
use anyhow::{bail, Result};
use grammers_client::Client;
use grammers_tl_types as tl;

/// Content extracted from a message
pub enum MessageContent {
    Photo(tl::types::Photo, String),
    Document(tl::types::Document, String),
    /// Text message content
    Text(String),
}

/// Fetch message and extract content using raw API
/// Supports media groups (albums) - will fetch all media in the group
/// Also supports text-only messages
pub async fn fetch_message(
    tg: &TelegramClient,
    chat_str: &str,
    message_id: i32,
) -> Result<(ResolvedChat, Vec<MessageContent>)> {
    let client = tg.inner();
    let resolved = resolve_chat(tg, chat_str).await?;
    let input_peer = resolved.input_peer.clone();

    // First, get the target message to check for grouped_id
    let result = client
        .invoke(&tl::functions::messages::GetHistory {
            peer: input_peer.clone(),
            offset_id: message_id + 1,
            offset_date: 0,
            add_offset: 0,
            limit: 1,
            max_id: message_id + 1,
            min_id: message_id - 1,
            hash: 0,
        })
        .await?;

    let messages = extract_messages(result);

    // Find the target message and check for grouped_id
    let mut grouped_id: Option<i64> = None;
    for msg in &messages {
        if let tl::enums::Message::Message(m) = msg {
            if m.id == message_id {
                grouped_id = m.grouped_id;
                break;
            }
        }
    }

    // If message has grouped_id, fetch all messages in the group
    let all_messages = if let Some(gid) = grouped_id {
        fetch_media_group(client, &input_peer, message_id, gid).await?
    } else {
        messages
    };

    // Extract content from all messages
    let mut content_list = Vec::new();

    for msg in all_messages {
        if let tl::enums::Message::Message(m) = msg {
            // For grouped messages, include all with same grouped_id
            // For single message, only include the target
            let should_include = match grouped_id {
                Some(gid) => m.grouped_id == Some(gid),
                None => m.id == message_id,
            };

            if should_include {
                if let Some(media) = m.media {
                    extract_media(&media, &m.message, &mut content_list);
                } else if !m.message.is_empty() {
                    // Text-only message
                    content_list.push(MessageContent::Text(m.message.clone()));
                }
            }
        }
    }

    if content_list.is_empty() && grouped_id.is_none() {
        bail!(
            "{}",
            if is_zh() {
                format!("未找到消息 {}，或消息内容为空", message_id)
            } else {
                format!("Message {} not found or empty", message_id)
            }
        );
    }

    Ok((resolved, content_list))
}

/// Fetch all messages in a media group
async fn fetch_media_group(
    client: &Client,
    input_peer: &tl::enums::InputPeer,
    message_id: i32,
    _grouped_id: i64,
) -> Result<Vec<tl::enums::Message>> {
    // Media groups typically have up to 10 items
    // Fetch messages around the target to get the whole group
    let result = client
        .invoke(&tl::functions::messages::GetHistory {
            peer: input_peer.clone(),
            offset_id: message_id + 10,
            offset_date: 0,
            add_offset: 0,
            limit: 20, // Fetch enough to cover the group
            max_id: message_id + 10,
            min_id: message_id - 10,
            hash: 0,
        })
        .await?;

    Ok(extract_messages(result))
}

/// Extract messages from API response
fn extract_messages(result: tl::enums::messages::Messages) -> Vec<tl::enums::Message> {
    match result {
        tl::enums::messages::Messages::Messages(m) => m.messages,
        tl::enums::messages::Messages::Slice(m) => m.messages,
        tl::enums::messages::Messages::ChannelMessages(m) => m.messages,
        tl::enums::messages::Messages::NotModified(_) => vec![],
    }
}

/// Extract media from MessageMedia
fn extract_media(
    media: &tl::enums::MessageMedia,
    caption: &str,
    content_list: &mut Vec<MessageContent>,
) {
    match media {
        tl::enums::MessageMedia::Photo(photo_media) => {
            if let Some(tl::enums::Photo::Photo(photo)) = &photo_media.photo {
                content_list.push(MessageContent::Photo(photo.clone(), caption.to_string()));
            }
        }
        tl::enums::MessageMedia::Document(doc_media) => {
            if let Some(tl::enums::Document::Document(doc)) = &doc_media.document {
                content_list.push(MessageContent::Document(doc.clone(), caption.to_string()));
            }
        }
        _ => {}
    }
}

// Keep old name for backward compatibility
pub type MessageMedia = MessageContent;
