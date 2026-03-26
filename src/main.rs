//! TDLR CLI entry point

use anyhow::Result;
use tdlr_core::{cli, commands, i18n};

#[tokio::main]
async fn main() -> Result<()> {
    let parsed = cli::parse_env();
    i18n::set_current_language(parsed.lang);
    commands::execute(parsed.cli.command).await
}
