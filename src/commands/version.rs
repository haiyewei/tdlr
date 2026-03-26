//! Version command

use crate::i18n::pick;
use anyhow::Result;
use colored::Colorize;

pub fn run() -> Result<()> {
    println!(
        "{}: {}",
        pick("版本", "Version").cyan(),
        env!("TDLR_VERSION")
    );
    println!("{}: {}", "Rustc".cyan(), env!("RUSTC_VERSION"));
    println!(
        "{}: {}/{}",
        pick("目标平台", "Target").cyan(),
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    Ok(())
}
