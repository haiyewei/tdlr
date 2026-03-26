//! TDLR CLI entry point

use anyhow::Result;
use clap::Parser;
use tdlr_core::{commands, Cli};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    commands::execute(cli.command).await
}
