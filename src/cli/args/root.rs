//! Root CLI and Commands enum

use super::auth::AuthCommands;
use super::download::DownloadArgs;
use super::forward::ForwardArgs;
use super::upload::UploadArgs;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "tdlr")]
#[command(author, version = env!("TDLR_VERSION"), about = "TDLR - Telegram Downloader CLI")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Say hello to someone
    Hello {
        /// Name to greet
        #[arg(short, long, default_value = "World")]
        name: String,
    },
    /// Show version information
    Version,
    /// Manage Telegram authentication
    #[command(subcommand)]
    Auth(AuthCommands),
    /// Upload files/dirs to Telegram
    Upload(UploadArgs),
    /// Download files from Telegram message URLs
    Download(DownloadArgs),
    /// Forward messages from one chat to another
    Forward(ForwardArgs),
}
