//! Chat resolution utilities

use anyhow::{bail, Result};
use grammers_client::peer::Peer;
use grammers_tl_types as tl;

use crate::i18n::{is_zh, pick};
use crate::telegram::TelegramClient;

/// Resolved chat information
pub struct ResolvedChat {
    pub input_peer: tl::enums::InputPeer,
    pub name: String,
    pub peer: Option<Peer>,
    /// Whether the chat has a public username
    pub is_public: bool,
    /// Whether forwarding is restricted (noforwards flag)
    pub noforwards: bool,
}

/// Resolve chat from string (username, ID, link, or special values)
pub async fn resolve_chat(client: &TelegramClient, chat_str: &str) -> Result<ResolvedChat> {
    let chat_str = chat_str.trim();

    // Handle special values - Saved Messages
    if chat_str.is_empty() || chat_str == "me" || chat_str == "self" {
        return Ok(ResolvedChat {
            input_peer: tl::types::InputPeerSelf {}.into(),
            name: pick("收藏夹", "Saved Messages").to_string(),
            peer: None,
            is_public: false,
            noforwards: false,
        });
    }

    // Handle Telegram links
    if chat_str.starts_with("https://t.me/") || chat_str.starts_with("http://t.me/") {
        return resolve_from_link(client, chat_str).await;
    }

    // Try to resolve as username first (starts with @)
    if chat_str.starts_with('@') {
        let username = chat_str.trim_start_matches('@');
        return resolve_username(client, username).await;
    }

    // Try to parse as numeric ID
    if let Ok(id) = chat_str.parse::<i64>() {
        return resolve_by_id(client, id).await;
    }

    // Try as username without @
    resolve_username(client, chat_str).await
}

/// Resolve chat from Telegram link
async fn resolve_from_link(client: &TelegramClient, link: &str) -> Result<ResolvedChat> {
    // Parse the link path
    let path = link
        .trim_start_matches("https://t.me/")
        .trim_start_matches("http://t.me/");

    // Handle private channel links: c/CHANNEL_ID/MESSAGE_ID
    if path.starts_with("c/") {
        let parts: Vec<&str> = path.split('/').collect();
        if parts.len() >= 2 {
            if let Ok(channel_id) = parts[1].parse::<i64>() {
                // Convert to -100 format and resolve by ID
                let full_id = -1_000_000_000_000 - channel_id;
                return resolve_by_id(client, full_id).await;
            }
        }
        bail!(
            "{}",
            if is_zh() {
                format!("无效的私有频道链接: {}", link)
            } else {
                format!("Invalid private channel link: {}", link)
            }
        );
    }

    // Handle public channel/user links: @username/MESSAGE_ID or username/MESSAGE_ID
    let username = path.split('/').next().unwrap_or(path);
    let username = username.trim_start_matches('@');

    if username.is_empty() {
        bail!(
            "{}",
            if is_zh() {
                format!("无效的链接: {}", link)
            } else {
                format!("Invalid link: {}", link)
            }
        );
    }

    resolve_username(client, username).await
}

/// Resolve chat by username using high-level API
async fn resolve_username(client: &TelegramClient, username: &str) -> Result<ResolvedChat> {
    let peer = client
        .inner()
        .resolve_username(username)
        .await?
        .ok_or_else(|| {
            anyhow::anyhow!(if is_zh() {
                format!("未找到用户名 @{}", username)
            } else {
                format!("Username @{} not found", username)
            })
        })?;

    let name = peer.name().unwrap_or(pick("未知", "Unknown")).to_string();
    let input_peer = peer_to_input_peer(&peer);
    let (is_public, noforwards) = extract_chat_flags(&peer);

    Ok(ResolvedChat {
        input_peer,
        name,
        peer: Some(peer),
        is_public,
        noforwards,
    })
}

/// Resolve chat by numeric ID (searches in dialogs)
async fn resolve_by_id(client: &TelegramClient, id: i64) -> Result<ResolvedChat> {
    // Convert -100 prefix format to raw channel_id if needed
    let target_id = if id < -1_000_000_000_000 {
        // -1002134730022 -> 2134730022
        (-id) - 1_000_000_000_000
    } else if id < 0 {
        // Negative ID without -100 prefix (legacy group)
        -id
    } else {
        id
    };

    // Fast path: try to find cached peer in session
    let possible_peers = [
        grammers_session::types::PeerId::channel(target_id),
        grammers_session::types::PeerId::chat(target_id),
        grammers_session::types::PeerId::user(target_id),
    ];

    for peer_id_opt in possible_peers {
        if let Some(peer_id) = peer_id_opt {
            if let Some(peer_ref) = client.get_peer_ref(peer_id).await {
                if let Ok(peer) = client.inner().resolve_peer(peer_ref).await {
                    let name = peer.name().unwrap_or(pick("未知", "Unknown")).to_string();
                    let input_peer = peer_to_input_peer(&peer);
                    let (is_public, noforwards) = extract_chat_flags(&peer);

                    return Ok(ResolvedChat {
                        input_peer,
                        name,
                        peer: Some(peer),
                        is_public,
                        noforwards,
                    });
                }
            }
        }
    }

    // Slow path: search through dialogs (fallback if not cached or cache invalid)
    let mut dialogs = client.inner().iter_dialogs();

    while let Some(dialog) = dialogs.next().await? {
        let peer = &dialog.peer;
        let peer_id: i64 = peer.id().bare_id();

        // Match against the normalized target_id
        if peer_id == target_id || peer_id == id || peer_id == id.abs() {
            let name = peer.name().unwrap_or(pick("未知", "Unknown")).to_string();
            let input_peer = peer_to_input_peer(peer);
            let (is_public, noforwards) = extract_chat_flags(peer);

            return Ok(ResolvedChat {
                input_peer,
                name,
                peer: Some(peer.clone()),
                is_public,
                noforwards,
            });
        }
    }

    bail!(
        "{}",
        if is_zh() {
            format!("在对话列表或缓存中未找到 ID 为 {} 的聊天", id)
        } else {
            format!("Chat with ID {} not found in dialogs or cache", id)
        }
    );
}

/// Extract is_public and noforwards flags from Peer
fn extract_chat_flags(peer: &Peer) -> (bool, bool) {
    match peer {
        Peer::User(user) => {
            // Users are considered "public" if they have a username
            let has_username = match &user.raw {
                tl::enums::User::User(u) => u.username.is_some(),
                tl::enums::User::Empty(_) => false,
            };
            (has_username, false)
        }
        Peer::Group(group) => match &group.raw {
            tl::enums::Chat::Channel(ch) => {
                let has_username = ch.username.is_some()
                    || ch
                        .usernames
                        .as_ref()
                        .map(|v| !v.is_empty())
                        .unwrap_or(false);
                (has_username, ch.noforwards)
            }
            tl::enums::Chat::ChannelForbidden(_) => (false, true),
            _ => (false, false),
        },
        Peer::Channel(channel) => {
            let has_username = channel.raw.username.is_some()
                || channel
                    .raw
                    .usernames
                    .as_ref()
                    .map(|v| !v.is_empty())
                    .unwrap_or(false);
            (has_username, channel.raw.noforwards)
        }
    }
}

/// Convert Peer to InputPeer
fn peer_to_input_peer(peer: &Peer) -> tl::enums::InputPeer {
    match peer {
        Peer::User(user) => {
            let (id, access_hash) = match &user.raw {
                tl::enums::User::User(u) => (u.id, u.access_hash.unwrap_or(0)),
                tl::enums::User::Empty(u) => (u.id, 0),
            };
            tl::types::InputPeerUser {
                user_id: id,
                access_hash,
            }
            .into()
        }
        Peer::Group(group) => {
            // Group.raw is tl::enums::Chat
            match &group.raw {
                tl::enums::Chat::Chat(c) => tl::types::InputPeerChat { chat_id: c.id }.into(),
                tl::enums::Chat::Channel(ch) => tl::types::InputPeerChannel {
                    channel_id: ch.id,
                    access_hash: ch.access_hash.unwrap_or(0),
                }
                .into(),
                tl::enums::Chat::ChannelForbidden(ch) => tl::types::InputPeerChannel {
                    channel_id: ch.id,
                    access_hash: ch.access_hash,
                }
                .into(),
                _ => {
                    let id = group.raw.id();
                    tl::types::InputPeerChat { chat_id: id }.into()
                }
            }
        }
        Peer::Channel(channel) => tl::types::InputPeerChannel {
            channel_id: channel.raw.id,
            access_hash: channel.raw.access_hash.unwrap_or(0),
        }
        .into(),
    }
}
