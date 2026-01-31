//! Telegram link parsing
//!
//! Supported URL formats:
//! - https://t.me/username/123                  (public channel/group)
//! - https://t.me/c/1234567890/123              (private channel by ID)
//! - https://t.me/username/123/456              (media group / reply)
//! - https://t.me/c/1234567890/123/456          (private media group / reply)
//! - https://t.me/username/123?comment=456      (comment in channel)
//! - https://t.me/username/123?thread=456       (forum topic thread)

use anyhow::{bail, Result};
use regex::Regex;
use std::sync::LazyLock;

/// Chat identifier type
#[derive(Debug, Clone)]
pub enum ChatIdentifier {
    /// Public username (e.g., "telegram")
    Username(String),
    /// Private channel ID (e.g., 1234567890)
    ChannelId(i64),
    /// Use external chat reference (for plain message IDs)
    External,
}

impl ChatIdentifier {
    /// Convert to string for display
    pub fn display(&self) -> String {
        match self {
            ChatIdentifier::Username(u) => format!("@{}", u),
            ChatIdentifier::ChannelId(id) => format!("c/{}", id),
            ChatIdentifier::External => "(external)".to_string(),
        }
    }
}

/// Parsed Telegram message link
#[derive(Debug, Clone)]
pub struct TelegramLink {
    /// Chat identifier (username or channel_id for private)
    pub chat: ChatIdentifier,
    /// Primary message ID
    pub message_id: i32,
    /// Secondary message ID (for media groups, replies, or comments)
    pub secondary_id: Option<i32>,
    /// Thread/topic ID (from ?thread= parameter)
    pub thread_id: Option<i32>,
    /// Comment ID (from ?comment= parameter)
    pub comment_id: Option<i32>,
}

// Regex patterns for URL parsing
static PUBLIC_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    // https://t.me/username/123 or https://t.me/username/123/456
    Regex::new(r"^https?://t\.me/([a-zA-Z][a-zA-Z0-9_]{3,})/(\d+)(?:/(\d+))?").unwrap()
});

static PRIVATE_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    // https://t.me/c/1234567890/123 or https://t.me/c/1234567890/123/456
    Regex::new(r"^https?://t\.me/c/(\d+)/(\d+)(?:/(\d+))?").unwrap()
});

static QUERY_THREAD: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[?&]thread=(\d+)").unwrap());

static QUERY_COMMENT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[?&]comment=(\d+)").unwrap());

/// Parse a Telegram message URL
pub fn parse_link(url: &str) -> Result<TelegramLink> {
    let url = url.trim();

    // Extract query parameters first
    let thread_id = QUERY_THREAD
        .captures(url)
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().parse().ok());

    let comment_id = QUERY_COMMENT
        .captures(url)
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().parse().ok());

    // Try private channel pattern first (t.me/c/...)
    if let Some(caps) = PRIVATE_PATTERN.captures(url) {
        let channel_id: i64 = caps.get(1).unwrap().as_str().parse()?;
        let message_id: i32 = caps.get(2).unwrap().as_str().parse()?;
        let secondary_id: Option<i32> = caps.get(3).and_then(|m| m.as_str().parse().ok());

        return Ok(TelegramLink {
            chat: ChatIdentifier::ChannelId(channel_id),
            message_id,
            secondary_id,
            thread_id,
            comment_id,
        });
    }

    // Try public channel/group pattern
    if let Some(caps) = PUBLIC_PATTERN.captures(url) {
        let username = caps.get(1).unwrap().as_str().to_string();
        let message_id: i32 = caps.get(2).unwrap().as_str().parse()?;
        let secondary_id: Option<i32> = caps.get(3).and_then(|m| m.as_str().parse().ok());

        return Ok(TelegramLink {
            chat: ChatIdentifier::Username(username),
            message_id,
            secondary_id,
            thread_id,
            comment_id,
        });
    }

    bail!("Invalid Telegram URL format: {}", url);
}

/// Parse a source string (URL or plain message ID)
/// Returns External chat identifier for plain message IDs
pub fn parse_source(input: &str) -> Result<TelegramLink> {
    let input = input.trim();

    // Try as plain message ID first
    if let Ok(id) = input.parse::<i32>() {
        return Ok(TelegramLink {
            chat: ChatIdentifier::External,
            message_id: id,
            secondary_id: None,
            thread_id: None,
            comment_id: None,
        });
    }

    // Otherwise parse as URL
    parse_link(input)
}

impl TelegramLink {
    /// Get the effective message ID to download/forward
    /// Priority: comment_id > secondary_id > message_id
    pub fn effective_message_id(&self) -> i32 {
        self.comment_id
            .or(self.secondary_id)
            .unwrap_or(self.message_id)
    }

    /// Check if this is a comment link
    pub fn is_comment(&self) -> bool {
        self.comment_id.is_some()
    }

    /// Check if this is a thread/topic link
    #[allow(dead_code)]
    pub fn is_thread(&self) -> bool {
        self.thread_id.is_some()
    }

    /// Check if chat is external (plain message ID)
    pub fn is_external(&self) -> bool {
        matches!(self.chat, ChatIdentifier::External)
    }
}
