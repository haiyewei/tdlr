//! Single Telegram client instance

use crate::telegram::session::SessionManager;
use anyhow::Result;
use grammers_client::Client;
use grammers_mtsender::{ConnectionParams, SenderPool};
use grammers_session::storages::SqliteSession;
use grammers_session::Session;
use std::sync::Arc;
use tokio::task::JoinHandle;

/// App version from Cargo.toml
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Create connection params with custom app info
fn connection_params() -> ConnectionParams {
    ConnectionParams {
        app_version: APP_VERSION.to_string(),
        device_model: "Desktop".to_string(),
        ..ConnectionParams::default()
    }
}

/// Single Telegram client instance
pub struct TelegramClient {
    pub client: Client,
    pub user_id: i64,
    session: Arc<SqliteSession>,
    network_handle: JoinHandle<()>,
}

impl TelegramClient {
    /// Create a new client for the given user_id
    pub fn new(user_id: i64, api_id: i32) -> Result<Self> {
        SessionManager::ensure_dir()?;

        let session_path = SessionManager::session_path(user_id);
        let session = Arc::new(SqliteSession::open(session_path.to_str().unwrap())?);
        let pool =
            SenderPool::with_configuration(Arc::clone(&session), api_id, connection_params());
        let client = Client::new(&pool);

        let network_handle = {
            let runner = pool.runner;
            tokio::spawn(async move {
                runner.run().await;
            })
        };

        Ok(Self {
            client,
            user_id,
            session,
            network_handle,
        })
    }

    /// Create a new client with a temp session name (for login)
    pub fn new_temp(temp_name: &str, api_id: i32) -> Result<Self> {
        SessionManager::ensure_dir()?;

        let session_path = SessionManager::session_path_str(temp_name);
        let session = Arc::new(SqliteSession::open(session_path.to_str().unwrap())?);
        let pool =
            SenderPool::with_configuration(Arc::clone(&session), api_id, connection_params());
        let client = Client::new(&pool);

        let network_handle = {
            let runner = pool.runner;
            tokio::spawn(async move {
                runner.run().await;
            })
        };

        Ok(Self {
            client,
            user_id: 0, // Will be set after login
            session,
            network_handle,
        })
    }

    /// Get reference to the underlying client
    pub fn inner(&self) -> &Client {
        &self.client
    }

    /// Check if authorized
    pub async fn is_authorized(&self) -> Result<bool> {
        Ok(self.client.is_authorized().await?)
    }

    /// Get current user
    pub async fn get_me(&self) -> Result<grammers_client::types::User> {
        Ok(self.client.get_me().await?)
    }

    /// Get current home DC ID
    pub fn home_dc_id(&self) -> i32 {
        self.session.home_dc_id()
    }

    /// Set home DC ID (needed after DC migration during login)
    pub fn set_home_dc_id(&self, dc_id: i32) {
        self.session.set_home_dc_id(dc_id);
    }
}

impl Drop for TelegramClient {
    fn drop(&mut self) {
        self.network_handle.abort();
    }
}
