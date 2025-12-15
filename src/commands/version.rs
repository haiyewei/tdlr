//! Version command

use anyhow::Result;
use colored::Colorize;

pub fn run() -> Result<()> {
    println!("{}: {}", "Version".cyan(), env!("CARGO_PKG_VERSION"));
    println!("{}: {}", "BuildDate".cyan(), env!("BUILD_DATE"));
    println!("{}: {}", "Rustc".cyan(), env!("RUSTC_VERSION"));
    println!(
        "{}: {}/{}",
        "Target".cyan(),
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    Ok(())
}
