//! Hello command

use anyhow::Result;
use colored::Colorize;

pub fn run(name: &str) -> Result<()> {
    println!("Hello, {}!", name.green());
    Ok(())
}
