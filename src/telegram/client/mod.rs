//! Telegram client management
//!
//! Module structure:
//! - `instance.rs` - Single client instance (TelegramClient)
//! - `pool.rs` - Client pool for multi-account management

mod instance;
mod pool;

pub use instance::{LoginCodeDelivery, LoginCodePreference, PhoneLoginCodeState, TelegramClient};
pub use pool::{pool, ClientPool};
