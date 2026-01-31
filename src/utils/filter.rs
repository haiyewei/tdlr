//! File extension filter utilities

use std::path::Path;

/// File extension filter for include/exclude patterns
pub struct ExtFilter {
    include: Option<Vec<String>>,
    exclude: Option<Vec<String>>,
}

impl ExtFilter {
    /// Create a new filter with optional include/exclude lists
    pub fn new(include: Option<Vec<String>>, exclude: Option<Vec<String>>) -> Self {
        Self {
            include: include.map(|v| v.iter().map(|s| normalize_ext(s)).collect()),
            exclude: exclude.map(|v| v.iter().map(|s| normalize_ext(s)).collect()),
        }
    }

    /// Check if extension should be included
    pub fn should_include(&self, ext: &str) -> bool {
        let ext = normalize_ext(ext);

        // If include list is specified, extension must be in it
        if let Some(ref include) = self.include {
            if !include.iter().any(|e| e == &ext) {
                return false;
            }
        }

        // If exclude list is specified, extension must not be in it
        if let Some(ref exclude) = self.exclude {
            if exclude.iter().any(|e| e == &ext) {
                return false;
            }
        }

        true
    }

    /// Check if a file path passes the filter
    pub fn matches_path(&self, path: &Path) -> bool {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        self.should_include(ext)
    }
}

/// Normalize extension string (remove leading dot, lowercase)
fn normalize_ext(s: &str) -> String {
    s.trim_start_matches('.').to_lowercase()
}

/// Get extension from filename
pub fn get_extension(filename: &str) -> String {
    filename
        .rsplit('.')
        .next()
        .filter(|ext| ext.len() < filename.len())
        .unwrap_or("")
        .to_lowercase()
}
