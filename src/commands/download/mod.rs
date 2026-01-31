//! Download commands
//!
//! Module structure:
//! - `download.rs` - Command entry point
//! - `handler.rs` - Download handlers
//! - `output.rs` - Output formatting utilities
//! - `template.rs` - Filename template engine

mod download;
mod handler;
mod output;
pub mod template;

pub use download::run;

// Exports for JNI/Android integration
#[cfg(target_os = "android")]
pub use crate::utils::{parse_link, ExtFilter, TelegramLink};
#[cfg(target_os = "android")]
pub use handler::{download_link, DownloadContext, DownloadStats};
