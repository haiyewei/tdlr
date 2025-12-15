//! TDLR - Telegram Downloader CLI
//!
//! A modular Rust CLI application for Telegram.

pub mod cli;
pub mod commands;
pub mod telegram;
pub mod utils;

// Re-export commonly used types
pub use cli::Cli;
