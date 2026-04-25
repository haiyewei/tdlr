//! Verification codes command arguments

use clap::Args;

#[derive(Debug, Args)]
pub struct CodesArgs {
    /// Number of recent messages to show
    #[arg(long, short = 'n', default_value_t = 10)]
    pub limit: usize,
    /// Account user ID to use
    #[arg(long)]
    pub account: Option<i64>,
}
