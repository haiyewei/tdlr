//! Telegram client management
//!
//! Module structure:
//! - `instance.rs` - Single client instance (TelegramClient)
//! - `pool.rs` - Client pool for multi-account management

mod instance;
mod pool;

pub use instance::TelegramClient;
pub use pool::{pool, ClientPool};
