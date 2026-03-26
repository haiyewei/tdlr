//! TDLR - Telegram Downloader CLI
//!
//! A modular Rust CLI application for Telegram.

pub mod cli;
pub mod commands;
pub mod i18n;
pub mod telegram;
pub mod utils;

// JNI bindings for Android
#[cfg(target_os = "android")]
pub mod jni;

// Re-export commonly used types
pub use cli::Cli;
