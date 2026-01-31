//! CLI argument structures
//!
//! Module structure:
//! - `root.rs` - Root CLI and Commands enum
//! - `auth.rs` - Auth command arguments
//! - `upload.rs` - Upload command arguments
//! - `download.rs` - Download command arguments
//! - `forward.rs` - Forward command arguments

mod auth;
mod download;
mod forward;
mod root;
mod upload;

pub use auth::{AuthCommands, LoginCommands, LoginMethod};
pub use download::DownloadArgs;
pub use forward::{ForwardArgs, ForwardMode};
pub use root::{Cli, Commands};
pub use upload::UploadArgs;
