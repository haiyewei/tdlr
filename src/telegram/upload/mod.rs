//! Telegram upload functionality
//!
//! Module structure:
//! - `chat.rs` - Chat resolution (username, ID)
//! - `single.rs` - Single file upload and text message
//! - `group.rs` - Media group upload
//! - `mime.rs` - MIME type utilities

mod chat;
mod group;
mod mime;
mod single;
mod video_metadata;

pub use chat::{resolve_chat, ResolvedChat};
pub use group::{
    upload_media_group, upload_media_group_with_thumbnails, UploadMediaItem, MAX_MEDIA_GROUP_SIZE,
};
pub use mime::{is_media_group_supported, is_photo_path, is_video_path};
pub use single::{send_text, upload_file, upload_file_with_progress, upload_file_with_thumbnail};
