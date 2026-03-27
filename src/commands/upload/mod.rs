//! Upload commands
//!
//! Module structure:
//! - `upload.rs` - Command entry point
//! - `file.rs` - File collection and filtering
//! - `expr.rs` - Expression engine for captions and routing
//! - `handler.rs` - Upload handlers (single/group)
//! - `output.rs` - Output formatting utilities
//! - `thumbnail.rs` - Thumbnail assignment for video uploads

pub mod expr;
mod file;
mod handler;
mod output;
mod thumbnail;
mod upload;

pub use upload::run;
