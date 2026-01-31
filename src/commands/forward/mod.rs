//! Forward commands
//!
//! Module structure:
//! - `forward.rs` - Command entry point
//! - `direct.rs` - Direct forward using Telegram API
//! - `clone.rs` - Clone forward (download + re-upload)
//! - `output.rs` - Output formatting utilities

mod clone;
mod direct;
mod forward;
mod output;

pub use forward::run;
