//! Root CLI and Commands enum

use super::auth::AuthCommands;
use super::upload::UploadArgs;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "tdlr")]
#[command(author, version, about = "TDLR - Telegram Downloader CLI")]
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
}
