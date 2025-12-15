//! CLI argument structures
//!
//! Module structure:
//! - `root.rs` - Root CLI and Commands enum
//! - `auth.rs` - Auth command arguments
//! - `upload.rs` - Upload command arguments

mod auth;
mod root;
mod upload;

pub use auth::{AuthCommands, LoginCommands, LoginMethod};
pub use root::{Cli, Commands};
pub use upload::UploadArgs;
