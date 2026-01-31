//! JNI bindings for Android integration
//!
//! This module provides Java Native Interface (JNI) bindings to expose
//! tdlr functionality to Android applications.
//!
//! ## Usage in Kotlin/Java
//!
//! ```kotlin
//! class TdlrNative {
//!     companion object {
//!         init {
//!             System.loadLibrary("tdlr")
//!         }
//!     }
//!
//!     external fun initRuntime(): Long
//!     external fun destroyRuntime(handle: Long)
//!     external fun download(handle: Long, url: String, outputPath: String, accountId: Long): String
//!     external fun getVersion(): String
//! }
//! ```

#![cfg(target_os = "android")]

use jni::objects::{JClass, JString};
use jni::sys::{jlong, jstring};
use jni::JNIEnv;
use std::ffi::CString;
use std::sync::Arc;
use tokio::runtime::Runtime;

/// Tokio runtime handle stored as opaque pointer
struct RuntimeHandle {
    runtime: Runtime,
}

/// Initialize the Tokio runtime
/// Returns a handle that must be passed to other functions
#[no_mangle]
pub extern "system" fn Java_com_tdlr_TdlrNative_initRuntime(_env: JNIEnv, _class: JClass) -> jlong {
    let runtime = match Runtime::new() {
        Ok(rt) => rt,
        Err(_) => return 0,
    };

    let handle = Box::new(RuntimeHandle { runtime });
    Box::into_raw(handle) as jlong
}

/// Destroy the Tokio runtime
#[no_mangle]
pub extern "system" fn Java_com_tdlr_TdlrNative_destroyRuntime(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
) {
    if handle != 0 {
        unsafe {
            let _ = Box::from_raw(handle as *mut RuntimeHandle);
        }
    }
}

/// Get library version
#[no_mangle]
pub extern "system" fn Java_com_tdlr_TdlrNative_getVersion<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
) -> jstring {
    let version = env!("CARGO_PKG_VERSION");
    let output = env
        .new_string(version)
        .expect("Failed to create Java string");
    output.into_raw()
}

/// Download file from Telegram URL
/// Returns JSON result: {"success": true, "path": "..."} or {"success": false, "error": "..."}
#[no_mangle]
pub extern "system" fn Java_com_tdlr_TdlrNative_download<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    handle: jlong,
    url: JString<'local>,
    output_path: JString<'local>,
    account_id: jlong,
) -> jstring {
    let result = download_impl(&mut env, handle, url, output_path, account_id);
    let json = match result {
        Ok(path) => format!(r#"{{"success":true,"path":"{}"}}"#, escape_json(&path)),
        Err(e) => format!(r#"{{"success":false,"error":"{}"}}"#, escape_json(&e)),
    };

    env.new_string(&json)
        .expect("Failed to create Java string")
        .into_raw()
}

fn download_impl(
    env: &mut JNIEnv,
    handle: jlong,
    url: JString,
    output_path: JString,
    account_id: jlong,
) -> Result<String, String> {
    if handle == 0 {
        return Err("Runtime not initialized".to_string());
    }

    let url: String = env
        .get_string(&url)
        .map_err(|e| format!("Invalid URL string: {}", e))?
        .into();

    let output_path: String = env
        .get_string(&output_path)
        .map_err(|e| format!("Invalid output path: {}", e))?
        .into();

    let runtime_handle = unsafe { &*(handle as *const RuntimeHandle) };

    // Run async download in the runtime
    runtime_handle
        .runtime
        .block_on(async { do_download(&url, &output_path, account_id as i64).await })
}

async fn do_download(url: &str, output_path: &str, account_id: i64) -> Result<String, String> {
    use crate::commands::download::{
        download_link, parse_link, DownloadContext, DownloadStats, ExtFilter,
    };
    use crate::telegram::TelegramClient;
    use std::path::Path;

    // Parse the URL
    let link = parse_link(url).ok_or_else(|| format!("Invalid Telegram URL: {}", url))?;

    // Get API credentials from environment
    let api_id: i32 = std::env::var("TELEGRAM_API_ID")
        .map_err(|_| "TELEGRAM_API_ID not set")?
        .parse()
        .map_err(|_| "Invalid TELEGRAM_API_ID")?;

    // Create client
    let tg = TelegramClient::new(account_id, api_id)
        .map_err(|e| format!("Failed to create client: {}", e))?;

    if !tg.is_authorized().await.unwrap_or(false) {
        return Err("Account not authorized".to_string());
    }

    // Create filter and context
    let filter = ExtFilter::new(None, None);
    let output_dir = Path::new(output_path);
    let template = "{{ .DialogID }}_{{ .MessageID }}_{{ filenamify .FileName }}";

    let ctx = DownloadContext {
        client: tg.inner(),
        output_dir,
        filter: &filter,
        template,
    };

    // Download
    let mut stats = DownloadStats::default();
    download_link(&ctx, &link, &mut stats)
        .await
        .map_err(|e| format!("Download failed: {}", e))?;

    if stats.success > 0 {
        Ok(output_path.to_string())
    } else {
        Err("No files downloaded".to_string())
    }
}

/// Escape string for JSON
fn escape_json(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

/// Check if an account session exists
#[no_mangle]
pub extern "system" fn Java_com_tdlr_TdlrNative_hasSession<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    account_id: jlong,
) -> bool {
    use crate::telegram::SessionManager;
    let path = SessionManager::session_path(account_id);
    path.exists()
}

/// Set session directory path
#[no_mangle]
pub extern "system" fn Java_com_tdlr_TdlrNative_setSessionDir<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    path: JString<'local>,
) -> bool {
    let path: String = match env.get_string(&path) {
        Ok(s) => s.into(),
        Err(_) => return false,
    };

    std::env::set_var("TDLR_SESSION_DIR", path);
    true
}

/// Set API credentials
#[no_mangle]
pub extern "system" fn Java_com_tdlr_TdlrNative_setApiCredentials<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    api_id: JString<'local>,
    api_hash: JString<'local>,
) -> bool {
    let api_id: String = match env.get_string(&api_id) {
        Ok(s) => s.into(),
        Err(_) => return false,
    };

    let api_hash: String = match env.get_string(&api_hash) {
        Ok(s) => s.into(),
        Err(_) => return false,
    };

    std::env::set_var("TELEGRAM_API_ID", api_id);
    std::env::set_var("TELEGRAM_API_HASH", api_hash);
    true
}
