//! Client pool for multi-account management

use super::instance::TelegramClient;
use crate::telegram::session::SessionManager;
use anyhow::{bail, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Client pool for managing multiple active accounts (keyed by user_id)
pub struct ClientPool {
    clients: RwLock<HashMap<i64, Arc<TelegramClient>>>,
    api_id: i32,
}

impl ClientPool {
    /// Create a new client pool
    pub fn new(api_id: i32) -> Self {
        Self {
            clients: RwLock::new(HashMap::new()),
            api_id,
        }
    }

    /// Get or create a client for the given user_id
    pub async fn get(&self, user_id: i64) -> Result<Arc<TelegramClient>> {
        // Check if already exists
        {
            let clients = self.clients.read().await;
            if let Some(client) = clients.get(&user_id) {
                return Ok(Arc::clone(client));
            }
        }

        // Create new client
        if !SessionManager::exists(user_id) {
            bail!("Account {} not found", user_id);
        }

        let client = Arc::new(TelegramClient::new(user_id, self.api_id)?);

        // Store in pool
        {
            let mut clients = self.clients.write().await;
            clients.insert(user_id, Arc::clone(&client));
        }

        Ok(client)
    }

    /// Get the active account's client
    pub async fn get_active(&self) -> Result<Arc<TelegramClient>> {
        let user_id =
            SessionManager::get_active()?.ok_or_else(|| anyhow::anyhow!("No active account"))?;
        self.get(user_id).await
    }

    /// Get all clients for all accounts
    pub async fn get_all(&self) -> Result<Vec<Arc<TelegramClient>>> {
        let ids = SessionManager::list_user_ids()?;
        self.get_many(&ids).await
    }

    /// Get clients for specified user_ids only
    pub async fn get_many(&self, user_ids: &[i64]) -> Result<Vec<Arc<TelegramClient>>> {
        let mut result = Vec::new();

        for &user_id in user_ids {
            match self.get(user_id).await {
                Ok(client) => result.push(client),
                Err(e) => eprintln!("Failed to load {}: {}", user_id, e),
            }
        }

        Ok(result)
    }

    /// Remove a client from the pool
    pub async fn remove(&self, user_id: i64) {
        let mut clients = self.clients.write().await;
        clients.remove(&user_id);
    }

    /// Clear all clients from the pool
    pub async fn clear(&self) {
        let mut clients = self.clients.write().await;
        clients.clear();
    }

    /// Get number of active clients
    pub async fn len(&self) -> usize {
        self.clients.read().await.len()
    }

    /// Check if pool is empty
    pub async fn is_empty(&self) -> bool {
        self.clients.read().await.is_empty()
    }
}

/// Global client pool instance
static POOL: std::sync::OnceLock<ClientPool> = std::sync::OnceLock::new();

/// Get the global client pool
pub fn pool() -> &'static ClientPool {
    POOL.get_or_init(|| {
        let api_id: i32 = env!("TG_API_ID").parse().expect("Invalid TG_API_ID");
        ClientPool::new(api_id)
    })
}
