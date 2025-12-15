//! Expression engine using evalexpr
//!
//! # Variables available in expressions
//!
//! ## File Information
//! - `name` - File name with extension (string)
//! - `stem` - File name without extension (string)
//! - `ext` - File extension lowercase (string)
//! - `mime` - MIME type (string)
//! - `type` - File type: image/video/audio/document/archive/text/code/other (string)
//! - `path` - Full file path (string)
//! - `dir` - Parent directory name (string)
//! - `depth` - Directory depth from root path (int)
//!
//! ## File Size
//! - `size` - File size in bytes (int)
//! - `size_kb` - File size in KB (float)
//! - `size_mb` - File size in MB (float)
//! - `size_gb` - File size in GB (float)
//! - `size_str` - Human readable size (string)
//!
//! ## Date/Time
//! - `date` - Current date YYYY-MM-DD (string)
//! - `time` - Current time HH:MM:SS (string)
//! - `datetime` - Current datetime (string)
//! - `year` - Current year (int)
//! - `month` - Current month (int)
//! - `day` - Current day (int)
//! - `hour` - Current hour (int)
//! - `minute` - Current minute (int)
//! - `weekday` - Day of week: Mon/Tue/Wed/Thu/Fri/Sat/Sun (string)
//!
//! ## File Metadata
//! - `is_image` - Is image file (bool)
//! - `is_video` - Is video file (bool)
//! - `is_audio` - Is audio file (bool)
//! - `is_document` - Is document file (bool)
//! - `is_archive` - Is archive file (bool)
//! - `is_text` - Is text file (bool)
//! - `is_code` - Is code file (bool)
//! - `is_media` - Is media file (image/video/audio) (bool)
//!
//! ## Upload Context
//! - `index` - Current file index (0-based) (int)
//! - `total` - Total number of files (int)
//! - `num` - Current file number (1-based) (int)
//!
//! ## Constants
//! - `KB`, `MB`, `GB` - Size constants for comparison
//!
//! # Expression examples
//!
//! ## Caption template (string interpolation)
//! ```text
//! "{name} - {mime}"
//! name + " (" + size_str + ")"
//! if(is_video, "🎬 ", if(is_image, "🖼️ ", "📁 ")) + name
//! "[" + str::from(num) + "/" + str::from(total) + "] " + name
//! ```
//!
//! ## Routing expression (returns destination string)
//! ```text
//! if(ext == "mp4", "@videos", "me")
//! if(is_video, "@videos", if(is_image, "@photos", "me"))
//! if(size > 100 * MB, "@large", "@small")
//! if(str::contains(name, "screenshot"), "@screenshots", "me")
//! if(dir == "photos", "@photos", if(dir == "videos", "@videos", "me"))
//! if(is_media && size > 50 * MB, "@large_media", "@media")
//! ```
//!
//! # Built-in functions (from evalexpr)
//! - `str::len(s)` - String length
//! - `str::contains(s, sub)` - Check if contains substring
//! - `str::starts_with(s, prefix)` - Check prefix
//! - `str::ends_with(s, suffix)` - Check suffix
//! - `str::to_lowercase(s)` - Lowercase
//! - `str::to_uppercase(s)` - Uppercase
//! - `str::trim(s)` - Trim whitespace
//! - `str::from(v)` - Convert to string
//! - `str::substring(s, start, len)` - Substring
//! - `str::replace(s, from, to)` - Replace all occurrences
//! - `str::regex_matches(s, pattern)` - Regex match
//! - `if(cond, then, else)` - Conditional
//! - `min(a, b)`, `max(a, b)` - Min/max
//! - `floor(x)`, `ceil(x)`, `round(x)` - Rounding
//! - Math: `+`, `-`, `*`, `/`, `%`, `^`
//! - Comparison: `==`, `!=`, `<`, `>`, `<=`, `>=`
//! - Logic: `&&`, `||`, `!`

use anyhow::{anyhow, Result};
use evalexpr::*;
use std::path::Path;

/// File context for expression evaluation
#[derive(Clone)]
pub struct FileContext {
    pub name: String,
    pub stem: String,
    pub ext: String,
    pub mime: String,
    pub file_type: String,
    pub size: u64,
    pub path: String,
    pub dir: String,
    pub depth: usize,
    // Upload context
    pub index: usize,
    pub total: usize,
}

impl FileContext {
    pub fn from_path_with_context(path: &Path, index: usize, total: usize) -> Self {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("file")
            .to_string();

        let stem = path
            .file_stem()
            .and_then(|n| n.to_str())
            .unwrap_or("file")
            .to_string();

        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        let mime = guess_mime(&ext);
        let file_type = get_file_type(&ext);
        let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        let path_str = path.display().to_string();

        let dir = path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        let depth = path.components().count().saturating_sub(1);

        Self {
            name,
            stem,
            ext,
            mime,
            file_type,
            size,
            path: path_str,
            dir,
            depth,
            index,
            total,
        }
    }

    /// Build evalexpr context with all variables
    pub fn to_eval_context(&self) -> HashMapContext {
        let now = chrono::Local::now();

        let mut ctx = HashMapContext::new();

        // File info - strings
        let _ = ctx.set_value("name".into(), Value::String(self.name.clone()));
        let _ = ctx.set_value("stem".into(), Value::String(self.stem.clone()));
        let _ = ctx.set_value("ext".into(), Value::String(self.ext.clone()));
        let _ = ctx.set_value("mime".into(), Value::String(self.mime.clone()));
        let _ = ctx.set_value("type".into(), Value::String(self.file_type.clone()));
        let _ = ctx.set_value("path".into(), Value::String(self.path.clone()));
        let _ = ctx.set_value("dir".into(), Value::String(self.dir.clone()));
        let _ = ctx.set_value("depth".into(), Value::Int(self.depth as i64));

        // Size variables
        let _ = ctx.set_value("size".into(), Value::Int(self.size as i64));
        let _ = ctx.set_value("size_kb".into(), Value::Float(self.size as f64 / 1024.0));
        let _ = ctx.set_value(
            "size_mb".into(),
            Value::Float(self.size as f64 / (1024.0 * 1024.0)),
        );
        let _ = ctx.set_value(
            "size_gb".into(),
            Value::Float(self.size as f64 / (1024.0 * 1024.0 * 1024.0)),
        );
        let _ = ctx.set_value("size_str".into(), Value::String(format_size(self.size)));

        // Date/time variables
        let _ = ctx.set_value(
            "date".into(),
            Value::String(now.format("%Y-%m-%d").to_string()),
        );
        let _ = ctx.set_value(
            "time".into(),
            Value::String(now.format("%H:%M:%S").to_string()),
        );
        let _ = ctx.set_value(
            "datetime".into(),
            Value::String(now.format("%Y-%m-%d %H:%M:%S").to_string()),
        );
        let _ = ctx.set_value(
            "year".into(),
            Value::Int(now.format("%Y").to_string().parse().unwrap_or(2025)),
        );
        let _ = ctx.set_value(
            "month".into(),
            Value::Int(now.format("%m").to_string().parse().unwrap_or(1)),
        );
        let _ = ctx.set_value(
            "day".into(),
            Value::Int(now.format("%d").to_string().parse().unwrap_or(1)),
        );
        let _ = ctx.set_value(
            "hour".into(),
            Value::Int(now.format("%H").to_string().parse().unwrap_or(0)),
        );
        let _ = ctx.set_value(
            "minute".into(),
            Value::Int(now.format("%M").to_string().parse().unwrap_or(0)),
        );
        let _ = ctx.set_value(
            "weekday".into(),
            Value::String(now.format("%a").to_string()),
        );

        // File type booleans
        let _ = ctx.set_value("is_image".into(), Value::Boolean(self.file_type == "image"));
        let _ = ctx.set_value("is_video".into(), Value::Boolean(self.file_type == "video"));
        let _ = ctx.set_value("is_audio".into(), Value::Boolean(self.file_type == "audio"));
        let _ = ctx.set_value(
            "is_document".into(),
            Value::Boolean(self.file_type == "document"),
        );
        let _ = ctx.set_value(
            "is_archive".into(),
            Value::Boolean(self.file_type == "archive"),
        );
        let _ = ctx.set_value("is_text".into(), Value::Boolean(self.file_type == "text"));
        let _ = ctx.set_value("is_code".into(), Value::Boolean(self.file_type == "code"));
        let _ = ctx.set_value(
            "is_media".into(),
            Value::Boolean(matches!(
                self.file_type.as_str(),
                "image" | "video" | "audio"
            )),
        );

        // Upload context
        let _ = ctx.set_value("index".into(), Value::Int(self.index as i64));
        let _ = ctx.set_value("total".into(), Value::Int(self.total as i64));
        let _ = ctx.set_value("num".into(), Value::Int((self.index + 1) as i64));

        // Constants for size comparison
        let _ = ctx.set_value("KB".into(), Value::Int(1024));
        let _ = ctx.set_value("MB".into(), Value::Int(1024 * 1024));
        let _ = ctx.set_value("GB".into(), Value::Int(1024 * 1024 * 1024));

        ctx
    }

    /// Build simple variable map for template substitution
    pub fn to_vars(&self) -> std::collections::HashMap<String, String> {
        let now = chrono::Local::now();
        let mut vars = std::collections::HashMap::new();

        // File info
        vars.insert("name".to_string(), self.name.clone());
        vars.insert("stem".to_string(), self.stem.clone());
        vars.insert("ext".to_string(), self.ext.clone());
        vars.insert("EXT".to_string(), self.ext.to_uppercase());
        vars.insert("mime".to_string(), self.mime.clone());
        vars.insert("type".to_string(), self.file_type.clone());
        vars.insert("path".to_string(), self.path.clone());
        vars.insert("dir".to_string(), self.dir.clone());
        vars.insert("depth".to_string(), self.depth.to_string());

        // Size
        vars.insert("size".to_string(), format_size(self.size));
        vars.insert("size_bytes".to_string(), self.size.to_string());
        vars.insert(
            "size_kb".to_string(),
            format!("{:.2}", self.size as f64 / 1024.0),
        );
        vars.insert(
            "size_mb".to_string(),
            format!("{:.2}", self.size as f64 / (1024.0 * 1024.0)),
        );

        // Date/time
        vars.insert("date".to_string(), now.format("%Y-%m-%d").to_string());
        vars.insert("time".to_string(), now.format("%H:%M:%S").to_string());
        vars.insert(
            "datetime".to_string(),
            now.format("%Y-%m-%d %H:%M:%S").to_string(),
        );
        vars.insert("year".to_string(), now.format("%Y").to_string());
        vars.insert("month".to_string(), now.format("%m").to_string());
        vars.insert("day".to_string(), now.format("%d").to_string());
        vars.insert("weekday".to_string(), now.format("%a").to_string());

        // Upload context
        vars.insert("index".to_string(), self.index.to_string());
        vars.insert("total".to_string(), self.total.to_string());
        vars.insert("num".to_string(), (self.index + 1).to_string());

        vars
    }
}

/// Evaluate an expression and return string result
pub fn eval_expr(expr: &str, ctx: &FileContext) -> Result<String> {
    let eval_ctx = ctx.to_eval_context();

    match eval_with_context(expr, &eval_ctx) {
        Ok(value) => Ok(value_to_string(&value)),
        Err(e) => Err(anyhow!("Expression error: {}", e)),
    }
}

/// Evaluate a routing expression (returns destination string)
pub fn eval_routing(expr: &str, ctx: &FileContext) -> String {
    match eval_expr(expr, ctx) {
        Ok(result) => result,
        Err(e) => {
            eprintln!("Warning: routing expression error: {}", e);
            "me".to_string() // Default to Saved Messages on error
        }
    }
}

/// Evaluate caption - supports both simple {var} templates and evalexpr expressions
pub fn eval_caption(template: &str, ctx: &FileContext) -> String {
    // If template contains {var} patterns, use simple substitution
    if template.contains('{') && template.contains('}') {
        let vars = ctx.to_vars();
        eval_template(template, &vars)
    } else {
        // Otherwise treat as evalexpr expression
        match eval_expr(template, ctx) {
            Ok(result) => result,
            Err(_) => template.to_string(),
        }
    }
}

/// Simple template evaluation with {var} substitution
pub fn eval_template(template: &str, vars: &std::collections::HashMap<String, String>) -> String {
    let mut result = template.to_string();

    // Simple {var} substitution
    for (key, value) in vars {
        result = result.replace(&format!("{{{}}}", key), value);
    }

    // Handle escape sequences
    result = result.replace("\\n", "\n");
    result = result.replace("\\t", "\t");

    result
}

/// Convert evalexpr Value to String
fn value_to_string(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Int(i) => i.to_string(),
        Value::Float(f) => {
            if f.fract() == 0.0 {
                format!("{:.0}", f)
            } else {
                format!("{:.2}", f)
            }
        }
        Value::Boolean(b) => b.to_string(),
        Value::Tuple(t) => t.iter().map(value_to_string).collect::<Vec<_>>().join(", "),
        Value::Empty => String::new(),
    }
}

/// Get file type category from extension
fn get_file_type(ext: &str) -> String {
    match ext.to_lowercase().as_str() {
        "jpg" | "jpeg" | "png" | "gif" | "webp" | "svg" | "bmp" | "ico" | "tiff" | "heic"
        | "raw" | "cr2" | "nef" => "image",
        "mp4" | "mkv" | "avi" | "mov" | "webm" | "flv" | "wmv" | "m4v" | "3gp" | "mts" | "m2ts" => {
            "video"
        }
        "mp3" | "wav" | "ogg" | "flac" | "aac" | "m4a" | "wma" | "opus" | "aiff" | "ape" => "audio",
        "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "odt" | "ods" | "odp"
        | "rtf" | "epub" => "document",
        "zip" | "rar" | "7z" | "tar" | "gz" | "bz2" | "xz" | "iso" | "dmg" | "cab" => "archive",
        "txt" | "md" | "csv" | "log" | "ini" | "cfg" | "conf" => "text",
        "rs" | "py" | "js" | "java" | "c" | "cpp" | "h" | "hpp" | "go" | "rb" | "php" | "swift"
        | "kt" | "scala" | "html" | "css" | "scss" | "sass" | "less" | "json" | "xml" | "yaml"
        | "yml" | "toml" | "sh" | "bash" | "zsh" | "fish" | "bat" | "ps1" | "sql" | "r" | "lua"
        | "perl" | "vue" | "jsx" | "tsx" | "svelte" => "code",
        _ => "other",
    }
    .to_string()
}

/// Guess MIME type from extension
fn guess_mime(ext: &str) -> String {
    match ext.to_lowercase().as_str() {
        // Images
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "bmp" => "image/bmp",
        "ico" => "image/x-icon",
        "tiff" => "image/tiff",
        "heic" => "image/heic",
        // Videos
        "mp4" => "video/mp4",
        "mkv" => "video/x-matroska",
        "avi" => "video/x-msvideo",
        "mov" => "video/quicktime",
        "webm" => "video/webm",
        "flv" => "video/x-flv",
        "wmv" => "video/x-ms-wmv",
        "ts" => "video/mp2t",
        // Audio
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "ogg" => "audio/ogg",
        "flac" => "audio/flac",
        "aac" => "audio/aac",
        "m4a" => "audio/mp4",
        "opus" => "audio/opus",
        // Documents
        "pdf" => "application/pdf",
        "doc" => "application/msword",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "xls" => "application/vnd.ms-excel",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "ppt" => "application/vnd.ms-powerpoint",
        "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        // Archives
        "zip" => "application/zip",
        "rar" => "application/vnd.rar",
        "7z" => "application/x-7z-compressed",
        "tar" => "application/x-tar",
        "gz" => "application/gzip",
        // Text/Code
        "txt" => "text/plain",
        "html" | "htm" => "text/html",
        "css" => "text/css",
        "js" => "text/javascript",
        "json" => "application/json",
        "xml" => "application/xml",
        "md" => "text/markdown",
        "csv" => "text/csv",
        "yaml" | "yml" => "text/yaml",
        // Default
        _ => "application/octet-stream",
    }
    .to_string()
}

/// Format file size to human readable
fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_ctx() -> FileContext {
        FileContext {
            name: "video.mp4".to_string(),
            stem: "video".to_string(),
            ext: "mp4".to_string(),
            mime: "video/mp4".to_string(),
            file_type: "video".to_string(),
            size: 100 * 1024 * 1024,
            path: "/tmp/video.mp4".to_string(),
            dir: "tmp".to_string(),
            depth: 2,
            index: 0,
            total: 5,
        }
    }

    #[test]
    fn test_simple_expr() {
        let ctx = test_ctx();
        assert_eq!(eval_expr("name", &ctx).unwrap(), "video.mp4");
        assert_eq!(eval_expr("ext", &ctx).unwrap(), "mp4");
        assert_eq!(eval_expr("type", &ctx).unwrap(), "video");
    }

    #[test]
    fn test_boolean_vars() {
        let ctx = test_ctx();
        assert_eq!(eval_expr("is_video", &ctx).unwrap(), "true");
        assert_eq!(eval_expr("is_image", &ctx).unwrap(), "false");
        assert_eq!(eval_expr("is_media", &ctx).unwrap(), "true");
    }

    #[test]
    fn test_upload_context() {
        let ctx = test_ctx();
        assert_eq!(eval_expr("index", &ctx).unwrap(), "0");
        assert_eq!(eval_expr("num", &ctx).unwrap(), "1");
        assert_eq!(eval_expr("total", &ctx).unwrap(), "5");
    }

    #[test]
    fn test_conditional() {
        let ctx = test_ctx();
        let result = eval_expr(r#"if(is_video, "@videos", "me")"#, &ctx).unwrap();
        assert_eq!(result, "@videos");
    }

    #[test]
    fn test_size_comparison() {
        let ctx = test_ctx();
        let result = eval_expr("if(size > 50 * MB, \"large\", \"small\")", &ctx).unwrap();
        assert_eq!(result, "large");
    }
}
