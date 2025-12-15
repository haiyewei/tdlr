//! Upload command arguments

use clap::Args;

#[derive(Args)]
pub struct UploadArgs {
    /// Dirs or files to upload
    #[arg(short, long, required = true, num_args = 1..)]
    pub path: Vec<String>,
    /// Chat ID or username (default: Saved Messages)
    #[arg(short, long, allow_hyphen_values = true)]
    pub chat: Option<String>,
    /// Include only specified file extensions (e.g., jpg,png,mp4)
    #[arg(short, long, num_args = 1.., value_delimiter = ',')]
    pub include: Option<Vec<String>>,
    /// Exclude specified file extensions (e.g., tmp,log)
    #[arg(short, long, num_args = 1.., value_delimiter = ',')]
    pub exclude: Option<Vec<String>>,
    /// Remove files after successful upload
    #[arg(long)]
    pub rm: bool,
    /// Topic ID (must be used with --chat for forum groups)
    #[arg(long, requires = "chat")]
    pub topic: Option<i32>,
    /// Account user ID(s) to use (default: active account)
    #[arg(short, long, action = clap::ArgAction::Append)]
    pub account: Option<Vec<i64>>,
    /// Use all accounts
    #[arg(long, conflicts_with = "account")]
    pub all_accounts: bool,
    /// Caption template (supports: {name}, {ext}, {mime}, {size}, {path})
    #[arg(long, default_value = "<code>{name}</code> - <code>{mime}</code>")]
    pub caption: String,
    /// Destination peer expression (conflicts with --chat and --topic)
    #[arg(long, conflicts_with_all = ["chat", "topic"])]
    pub to: Option<String>,
    /// Send files as media group/album (max 10 per group, photos/videos only)
    #[arg(long)]
    pub group: bool,
}
