//! Telegram client and authentication module

pub mod auth;
pub mod client;
pub mod session;
pub mod upload;

pub use client::{pool, ClientPool, TelegramClient};
pub use session::SessionManager;
