//! Version command

use anyhow::Result;
use colored::Colorize;

pub fn run() -> Result<()> {
    println!("{}: {}", "Version".cyan(), env!("TDLR_VERSION"));
    println!("{}: {}", "Rustc".cyan(), env!("RUSTC_VERSION"));
    println!(
        "{}: {}/{}",
        "Target".cyan(),
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    Ok(())
}
