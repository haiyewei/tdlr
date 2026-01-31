//! Progress bar utilities for file transfers

use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use std::sync::Arc;

/// Create a progress bar for file transfers (download/upload)
/// Returns a progress bar that displays on stderr with consistent styling
pub fn create_progress_bar(total_size: u64) -> Result<ProgressBar> {
    let pb = ProgressBar::with_draw_target(
        Some(total_size),
        indicatif::ProgressDrawTarget::stderr_with_hz(10),
    );
    pb.set_style(
        ProgressStyle::default_bar()
            .template(
                "  {spinner:.green} [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})",
            )?
            .progress_chars("█▓░"),
    );
    pb.enable_steady_tick(std::time::Duration::from_millis(100));
    Ok(pb)
}

/// Create an optional progress bar (None if size is 0)
pub fn create_progress_bar_opt(total_size: u64) -> Result<Option<ProgressBar>> {
    if total_size > 0 {
        Ok(Some(create_progress_bar(total_size)?))
    } else {
        Ok(None)
    }
}

/// Create an Arc-wrapped progress bar for shared access
pub fn create_shared_progress_bar(total_size: u64) -> Result<Arc<ProgressBar>> {
    Ok(Arc::new(create_progress_bar(total_size)?))
}
