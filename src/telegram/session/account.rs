//! Account info and metadata

use anyhow::Result;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

const ACCOUNTS_FILE: &str = "sessions/accounts.json";

/// Account info stored in accounts.json
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AccountInfo {
    pub user_id: i64,
    pub display_name: String,
    #[serde(default)]
    pub username: Option<String>,
}

/// Load accounts metadata from file
pub fn load_accounts() -> Result<HashMap<i64, AccountInfo>> {
    let path = PathBuf::from(ACCOUNTS_FILE);
    if !path.exists() {
        return Ok(HashMap::new());
    }
    let content = fs::read_to_string(&path)?;
    Ok(serde_json::from_str(&content)?)
}

/// Save accounts metadata to file
pub fn save_accounts(accounts: &HashMap<i64, AccountInfo>) -> Result<()> {
    super::manager::ensure_dir()?;
    let content = serde_json::to_string_pretty(accounts)?;
    fs::write(ACCOUNTS_FILE, content)?;
    Ok(())
}

/// Add or update account info
pub fn save_account(info: AccountInfo) -> Result<()> {
    let mut accounts = load_accounts()?;
    accounts.insert(info.user_id, info);
    save_accounts(&accounts)
}

/// Get account info by user_id
pub fn get_account(user_id: i64) -> Result<Option<AccountInfo>> {
    let accounts = load_accounts()?;
    Ok(accounts.get(&user_id).cloned())
}

/// Remove account from metadata
pub fn remove_account(user_id: i64) -> Result<()> {
    let mut accounts = load_accounts()?;
    accounts.remove(&user_id);
    save_accounts(&accounts)
}
