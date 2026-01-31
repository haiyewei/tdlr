//! Telegram download functionality
//!
//! Module structure:
//! - `message.rs` - Message fetching
//! - `file.rs` - File download with progress

mod file;
mod message;

pub use file::{
    download_document, download_document_with_progress, download_photo,
    download_photo_with_progress, DocumentInfo, DownloadResult, PhotoInfo,
};
pub use message::{fetch_message, MessageContent, MessageMedia};
