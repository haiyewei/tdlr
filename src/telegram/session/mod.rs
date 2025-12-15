//! Session and account management
//!
//! Module structure:
//! - `account.rs` - Account info and metadata
//! - `manager.rs` - Session file management
//! - `active.rs` - Active account tracking

mod account;
mod active;
mod manager;

pub use account::AccountInfo;
pub use manager::SessionManager;
