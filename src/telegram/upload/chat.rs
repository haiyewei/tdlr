//! Chat resolution utilities

use anyhow::{bail, Result};
use grammers_client::types::Peer;
use grammers_client::Client;
use grammers_tl_types as tl;

/// Resolved chat information
pub struct ResolvedChat {
    pub input_peer: tl::enums::InputPeer,
    pub name: String,
    pub peer: Option<Peer>,
}

/// Resolve chat from string (username, ID, or special values)
pub async fn resolve_chat(client: &Client, chat_str: &str) -> Result<ResolvedChat> {
    let chat_str = chat_str.trim();

    // Handle special values - Saved Messages
    if chat_str.is_empty() || chat_str == "me" || chat_str == "self" {
        return Ok(ResolvedChat {
            input_peer: tl::types::InputPeerSelf {}.into(),
            name: "Saved Messages".to_string(),
            peer: None,
        });
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

/// Resolve chat by username using high-level API
async fn resolve_username(client: &Client, username: &str) -> Result<ResolvedChat> {
    let peer = client
        .resolve_username(username)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Username @{} not found", username))?;

    let name = peer.name().unwrap_or("Unknown").to_string();
    let input_peer = peer_to_input_peer(&peer);

    Ok(ResolvedChat {
        input_peer,
        name,
        peer: Some(peer),
    })
}

/// Resolve chat by numeric ID (searches in dialogs)
async fn resolve_by_id(client: &Client, id: i64) -> Result<ResolvedChat> {
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

    let mut dialogs = client.iter_dialogs();

    while let Some(dialog) = dialogs.next().await? {
        let peer = &dialog.peer;
        let peer_id: i64 = peer.id().bare_id();

        // Match against the normalized target_id
        if peer_id == target_id || peer_id == id || peer_id == id.abs() {
            let name = peer.name().unwrap_or("Unknown").to_string();
            let input_peer = peer_to_input_peer(peer);

            return Ok(ResolvedChat {
                input_peer,
                name,
                peer: Some(peer.clone()),
            });
        }
    }

    bail!("Chat with ID {} not found in dialogs", id);
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
