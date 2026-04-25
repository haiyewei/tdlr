//! Telegram client and authentication module

pub mod auth;
pub mod client;
pub mod download;
pub mod session;
pub mod upload;

pub use client::{
    pool, ClientPool, LoginCodeDelivery, LoginCodePreference, PhoneLoginCodeState, TelegramClient,
};
pub use session::SessionManager;
