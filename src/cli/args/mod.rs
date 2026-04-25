//! CLI argument structures
//!
//! Module structure:
//! - `root.rs` - Root CLI and Commands enum
//! - `auth.rs` - Auth command arguments
//! - `upload.rs` - Upload command arguments
//! - `download.rs` - Download command arguments
//! - `forward.rs` - Forward command arguments
//! - `service.rs` - Service mode arguments

mod auth;
mod codes;
mod download;
mod forward;
mod root;
mod service;
mod upload;

pub use auth::{AuthCommands, LoginCodeVia, LoginCommands, LoginMethod};
pub use codes::CodesArgs;
pub use download::DownloadArgs;
pub use forward::{ForwardArgs, ForwardMode};
pub use root::{Cli, Commands};
pub use service::ServiceArgs;
pub use upload::UploadArgs;
