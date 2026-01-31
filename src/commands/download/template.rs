//! Filename template engine for downloads
//!
//! Supports Go-like template syntax with variables and functions.
//!
//! ## Variables
//! - `DialogID` - Telegram dialog/chat ID
//! - `MessageID` - Telegram message ID
//! - `MessageDate` - Message timestamp
//! - `FileName` - Original filename
//! - `FileCaption` - Message caption/text
//! - `FileSize` - Human readable file size
//! - `DownloadDate` - Download timestamp
//!
//! ## Functions
//! - `upper STRING` - Convert to uppercase
//! - `lower STRING` - Convert to lowercase
//! - `replace STRING PAIRS...` - Replace substrings
//! - `filenamify STRING [MaxLength]` - Sanitize filename
//! - `formatDate TIMESTAMP [format]` - Format timestamp
//! - `now` - Current timestamp
//! - `rand MIN MAX` - Random number

use chrono::{Local, TimeZone};
use regex::Regex;
use std::sync::LazyLock;

/// Default filename template
pub const DEFAULT_TEMPLATE: &str = "{{ .DialogID }}_{{ .MessageID }}_{{ filenamify .FileName }}";

/// Template context with all available variables
#[derive(Clone)]
pub struct TemplateContext {
    pub dialog_id: i64,
    pub message_id: i32,
    pub message_date: i64,
    pub file_name: String,
    pub file_caption: String,
    pub file_size: String,
    pub download_date: i64,
}

impl TemplateContext {
    pub fn new(dialog_id: i64, message_id: i32, file_name: String) -> Self {
        Self {
            dialog_id,
            message_id,
            message_date: 0,
            file_name,
            file_caption: String::new(),
            file_size: String::new(),
            download_date: Local::now().timestamp(),
        }
    }

    #[allow(dead_code)]
    pub fn with_message_date(mut self, ts: i64) -> Self {
        self.message_date = ts;
        self
    }

    #[allow(dead_code)]
    pub fn with_caption(mut self, caption: String) -> Self {
        self.file_caption = caption;
        self
    }

    pub fn with_size(mut self, size: u64) -> Self {
        self.file_size = format_size(size);
        self
    }
}

// Regex patterns for template parsing
static VAR_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\{\{\s*\.(\w+)\s*\}\}").unwrap());

static FUNC_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\{\{\s*(\w+)\s+([^}]+)\s*\}\}").unwrap());

static NESTED_FUNC_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\{\{\s*(\w+)\s+\(([^)]+)\)\s*\}\}").unwrap());

/// Render a template with the given context
pub fn render(template: &str, ctx: &TemplateContext) -> String {
    let mut result = template.to_string();

    // Process nested functions first (e.g., {{ lower (replace .FileName ...) }})
    result = process_nested_functions(&result, ctx);

    // Process simple functions (e.g., {{ filenamify .FileName 32 }})
    result = process_functions(&result, ctx);

    // Process variables (e.g., {{ .DialogID }})
    result = process_variables(&result, ctx);

    // Get extension from original filename
    let ext = get_extension(&ctx.file_name);

    // Ensure result has extension
    if !ext.is_empty() && !result.ends_with(&format!(".{}", ext)) {
        result = format!("{}.{}", result, ext);
    }

    result
}

/// Process simple variable substitutions
fn process_variables(template: &str, ctx: &TemplateContext) -> String {
    VAR_PATTERN
        .replace_all(template, |caps: &regex::Captures| {
            let var_name = &caps[1];
            get_variable(var_name, ctx)
        })
        .to_string()
}

/// Process function calls
fn process_functions(template: &str, ctx: &TemplateContext) -> String {
    FUNC_PATTERN
        .replace_all(template, |caps: &regex::Captures| {
            let func_name = &caps[1];
            let args_str = &caps[2];
            execute_function(func_name, args_str, ctx)
        })
        .to_string()
}

/// Process nested function calls
fn process_nested_functions(template: &str, ctx: &TemplateContext) -> String {
    NESTED_FUNC_PATTERN
        .replace_all(template, |caps: &regex::Captures| {
            let outer_func = &caps[1];
            let inner_expr = &caps[2];

            // Execute inner expression first
            let inner_result = if inner_expr.contains(' ') {
                // It's a function call
                let parts: Vec<&str> = inner_expr.splitn(2, ' ').collect();
                if parts.len() == 2 {
                    execute_function(parts[0], parts[1], ctx)
                } else {
                    inner_expr.to_string()
                }
            } else if inner_expr.starts_with('.') {
                // It's a variable
                get_variable(&inner_expr[1..], ctx)
            } else {
                inner_expr.to_string()
            };

            // Execute outer function with inner result
            execute_function(outer_func, &format!("\"{}\"", inner_result), ctx)
        })
        .to_string()
}

/// Get variable value
fn get_variable(name: &str, ctx: &TemplateContext) -> String {
    match name {
        "DialogID" => ctx.dialog_id.to_string(),
        "MessageID" => ctx.message_id.to_string(),
        "MessageDate" => ctx.message_date.to_string(),
        "FileName" => ctx.file_name.clone(),
        "FileCaption" => ctx.file_caption.clone(),
        "FileSize" => ctx.file_size.clone(),
        "DownloadDate" => ctx.download_date.to_string(),
        _ => format!("{{{{ .{} }}}}", name), // Unknown variable, keep as-is
    }
}

/// Execute a template function
fn execute_function(name: &str, args_str: &str, ctx: &TemplateContext) -> String {
    let args = parse_args(args_str, ctx);

    match name {
        "upper" => args.first().map(|s| s.to_uppercase()).unwrap_or_default(),
        "lower" => args.first().map(|s| s.to_lowercase()).unwrap_or_default(),
        "snakecase" => args.first().map(|s| to_snake_case(s)).unwrap_or_default(),
        "camelcase" => args.first().map(|s| to_camel_case(s)).unwrap_or_default(),
        "kebabcase" => args.first().map(|s| to_kebab_case(s)).unwrap_or_default(),
        "replace" => {
            if args.len() >= 3 {
                let mut result = args[0].clone();
                let pairs: Vec<_> = args[1..].chunks(2).collect();
                for pair in pairs {
                    if pair.len() == 2 {
                        result = result.replace(&pair[0], &pair[1]);
                    }
                }
                result
            } else {
                args.first().cloned().unwrap_or_default()
            }
        }
        "repeat" => {
            if args.len() >= 2 {
                let n: usize = args[1].parse().unwrap_or(1);
                args[0].repeat(n)
            } else {
                args.first().cloned().unwrap_or_default()
            }
        }
        "rand" => {
            if args.len() >= 2 {
                let min: i64 = args[0].parse().unwrap_or(0);
                let max: i64 = args[1].parse().unwrap_or(100);
                use std::time::{SystemTime, UNIX_EPOCH};
                let seed = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos() as i64;
                let range = max - min + 1;
                (min + (seed.abs() % range)).to_string()
            } else {
                "0".to_string()
            }
        }
        "now" => Local::now().timestamp().to_string(),
        "formatDate" => {
            if args.is_empty() {
                return String::new();
            }
            let ts: i64 = args[0].parse().unwrap_or(0);
            let format = args.get(1).map(|s| s.as_str()).unwrap_or("20060102150405");
            format_timestamp(ts, format)
        }
        "filenamify" => {
            if args.is_empty() {
                return String::new();
            }
            let max_len: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(255);
            filenamify(&args[0], max_len)
        }
        _ => format!("{{{{ {} {} }}}}", name, args_str), // Unknown function
    }
}

/// Parse function arguments
fn parse_args(args_str: &str, ctx: &TemplateContext) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_string = false;
    let mut string_char = '"';

    for c in args_str.chars() {
        match c {
            '"' | '\'' | '`' if !in_string => {
                in_string = true;
                string_char = c;
            }
            c if c == string_char && in_string => {
                in_string = false;
                args.push(current.clone());
                current.clear();
            }
            ' ' if !in_string => {
                if !current.is_empty() {
                    // Check if it's a variable reference
                    let value = if current.starts_with('.') {
                        get_variable(&current[1..], ctx)
                    } else {
                        current.clone()
                    };
                    args.push(value);
                    current.clear();
                }
            }
            _ => current.push(c),
        }
    }

    // Handle last argument
    if !current.is_empty() {
        let value = if current.starts_with('.') {
            get_variable(&current[1..], ctx)
        } else {
            current
        };
        args.push(value);
    }

    args
}

/// Sanitize filename
fn filenamify(s: &str, max_len: usize) -> String {
    let invalid_chars = ['/', '\\', ':', '*', '?', '"', '<', '>', '|', '\0'];
    let mut result: String = s
        .chars()
        .map(|c| {
            if invalid_chars.contains(&c) || c.is_control() {
                '_'
            } else {
                c
            }
        })
        .collect();

    // Remove extension for length calculation
    let ext = get_extension(&result);
    let name_without_ext = if !ext.is_empty() {
        result
            .strip_suffix(&format!(".{}", ext))
            .unwrap_or(&result)
            .to_string()
    } else {
        result.clone()
    };

    // Truncate if needed (accounting for extension)
    let ext_len = if ext.is_empty() { 0 } else { ext.len() + 1 };
    let max_name_len = max_len.saturating_sub(ext_len);

    if name_without_ext.len() > max_name_len {
        result = name_without_ext.chars().take(max_name_len).collect();
    } else {
        result = name_without_ext;
    }

    result
}

/// Format timestamp using Go-style format
fn format_timestamp(ts: i64, format: &str) -> String {
    let dt = Local
        .timestamp_opt(ts, 0)
        .single()
        .unwrap_or_else(Local::now);

    // Convert Go format to chrono format
    let chrono_format = format
        .replace("2006", "%Y")
        .replace("01", "%m")
        .replace("02", "%d")
        .replace("15", "%H")
        .replace("04", "%M")
        .replace("05", "%S");

    dt.format(&chrono_format).to_string()
}

/// Convert to snake_case
fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            result.push('_');
        }
        result.push(c.to_lowercase().next().unwrap_or(c));
    }
    result.replace(' ', "_").replace('-', "_")
}

/// Convert to camelCase
fn to_camel_case(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = false;
    for (i, c) in s.chars().enumerate() {
        if c == ' ' || c == '_' || c == '-' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(c.to_uppercase().next().unwrap_or(c));
            capitalize_next = false;
        } else if i == 0 {
            result.push(c.to_lowercase().next().unwrap_or(c));
        } else {
            result.push(c);
        }
    }
    result
}

/// Convert to kebab-case
fn to_kebab_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            result.push('-');
        }
        result.push(c.to_lowercase().next().unwrap_or(c));
    }
    result.replace(' ', "-").replace('_', "-")
}

/// Get file extension
fn get_extension(filename: &str) -> String {
    filename
        .rsplit('.')
        .next()
        .filter(|ext| ext.len() < filename.len())
        .unwrap_or("")
        .to_lowercase()
}

/// Format file size
fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2}GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2}MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2}KB", bytes as f64 / KB as f64)
    } else {
        format!("{}B", bytes)
    }
}
