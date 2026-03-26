//! Service mode arguments

use clap::Args;

#[derive(Args)]
pub struct ServiceArgs {
    /// Print machine-readable lifecycle events for stdio clients
    #[arg(long, default_value_t = true)]
    pub json_events: bool,
    /// Serve HTTP API on the given host:port instead of stdio mode
    #[arg(long)]
    pub http_bind: Option<String>,
}
