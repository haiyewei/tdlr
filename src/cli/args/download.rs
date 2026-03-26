//! Download command arguments

use clap::Args;

#[derive(Debug, Args)]
pub struct DownloadArgs {
    /// Telegram message URLs to download
    #[arg(short, long, required = true, num_args = 1..)]
    pub url: Vec<String>,
    /// Output directory (default: current directory)
    #[arg(short, long, default_value = ".")]
    pub path: String,
    /// Include only specified file extensions (e.g., jpg,png,mp4)
    #[arg(short, long, num_args = 1.., value_delimiter = ',')]
    pub include: Option<Vec<String>>,
    /// Exclude specified file extensions (e.g., tmp,log)
    #[arg(short, long, num_args = 1.., value_delimiter = ',')]
    pub exclude: Option<Vec<String>>,
    /// Filename template (e.g., "{{ .DialogID }}_{{ .MessageID }}_{{ .FileName }}")
    #[arg(short, long)]
    pub template: Option<String>,
    /// Account user ID to use (default: active account)
    #[arg(short, long)]
    pub account: Option<i64>,
}
