//! Forward command arguments

use clap::{Args, ValueEnum};

#[derive(Debug, Clone, Copy, PartialEq, Default, ValueEnum)]
pub enum ForwardMode {
    /// Use Telegram native forward API
    Direct,
    /// Download then re-upload (for restricted content)
    Clone,
    /// Auto-detect based on chat's noforwards flag
    #[default]
    Smart,
}

#[derive(Args)]
pub struct ForwardArgs {
    /// Source message URLs or IDs
    #[arg(short, long, required = true, num_args = 1..)]
    pub from: Vec<String>,
    /// Source chat (required when using message IDs instead of URLs)
    #[arg(long)]
    pub from_chat: Option<String>,
    /// Destination chat ID or username (default: Saved Messages)
    #[arg(short, long)]
    pub to: Option<String>,
    /// Forward mode: direct/clone/smart
    #[arg(short, long, value_enum, default_value = "smart")]
    pub mode: ForwardMode,
    /// Topic ID (for forum groups)
    #[arg(long)]
    pub topic: Option<i32>,
    /// Account user ID to use (default: active account)
    #[arg(short, long)]
    pub account: Option<i64>,
    /// Drop author (send as copy without forward header, only for direct mode)
    #[arg(long)]
    pub drop_author: bool,
}
