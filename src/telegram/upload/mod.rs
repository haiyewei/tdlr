//! Telegram upload functionality
//!
//! Module structure:
//! - `chat.rs` - Chat resolution (username, ID)
//! - `single.rs` - Single file upload
//! - `group.rs` - Media group upload
//! - `mime.rs` - MIME type utilities

mod chat;
mod group;
mod mime;
mod single;

pub use chat::{resolve_chat, ResolvedChat};
pub use group::{upload_media_group, MAX_MEDIA_GROUP_SIZE};
pub use mime::is_media_group_supported;
pub use single::upload_file;
