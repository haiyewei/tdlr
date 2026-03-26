use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

use grammers_client::{session::storages::MemorySession, tl, Client, SenderPool};

fn main() {
    println!("cargo:rerun-if-changed=.env");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=TG_API_ID");
    println!("cargo:rerun-if-env-changed=TG_API_HASH");
    println!("cargo:rerun-if-env-changed=BUILD_TYPE");

    // 设置构建时间
    let build_date = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    println!("cargo:rustc-env=BUILD_DATE={}", build_date);

    // 获取 rustc 版本
    let rustc_version = Command::new("rustc")
        .arg("--version")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());
    println!("cargo:rustc-env=RUSTC_VERSION={}", rustc_version);

    let build_type = std::env::var("BUILD_TYPE").unwrap_or_else(|_| "local".to_string());

    // 获取 git hash
    let git_hash = Command::new("git")
        .args(&["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let version = env!("CARGO_PKG_VERSION");

    // 正式版：版本号 + 构建时间
    // 每日版：daily-哈希 + 构建时间
    // 本地版：版本号-local + 构建时间
    let display_version = match build_type.as_str() {
        "release" => format!("{} (build {})", version, build_date),
        "daily" => format!("daily-{} (build {})", git_hash, build_date),
        _ => format!("{}-local (build {})", version, build_date),
    };

    println!("cargo:rustc-env=TDLR_VERSION={}", display_version);

    let dotenv_values = load_dotenv(Path::new(".env"));

    for (key, value) in &dotenv_values {
        if key != "TG_API_ID" && key != "TG_API_HASH" {
            println!("cargo:rustc-env={}={}", key, value);
        }
    }

    let api_id = parse_api_id(&resolve_required_env("TG_API_ID", &dotenv_values));
    let api_hash = resolve_required_env("TG_API_HASH", &dotenv_values);

    validate_api_credentials(api_id, &api_hash);

    println!("cargo:rustc-env=TG_API_ID={}", api_id);
    println!("cargo:rustc-env=TG_API_HASH={}", api_hash);
}

fn load_dotenv(path: &Path) -> HashMap<String, String> {
    let mut values = HashMap::new();

    if !path.exists() {
        return values;
    }

    let content = fs::read_to_string(path).unwrap_or_else(|err| {
        panic!("failed to read {}: {}", path.display(), err);
    });

    for (line_no, raw_line) in content.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let (key, value) = line.split_once('=').unwrap_or_else(|| {
            panic!(
                "invalid .env entry at line {}: expected KEY=VALUE",
                line_no + 1
            )
        });

        values.insert(
            key.trim().to_string(),
            normalize_dotenv_value(value.trim()).to_string(),
        );
    }

    values
}

fn normalize_dotenv_value(value: &str) -> &str {
    value
        .strip_prefix('"')
        .and_then(|inner| inner.strip_suffix('"'))
        .or_else(|| {
            value
                .strip_prefix('\'')
                .and_then(|inner| inner.strip_suffix('\''))
        })
        .unwrap_or(value)
}

fn resolve_required_env(key: &str, dotenv_values: &HashMap<String, String>) -> String {
    std::env::var(key)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| dotenv_values.get(key).cloned())
        .unwrap_or_else(|| {
            panic!(
                "missing required Telegram API credential {key}. Set it in the environment or in .env before building."
            )
        })
}

fn parse_api_id(api_id: &str) -> i32 {
    let parsed = api_id.parse::<i32>().unwrap_or_else(|_| {
        panic!("invalid TG_API_ID: expected a positive integer");
    });

    if parsed <= 0 {
        panic!("invalid TG_API_ID: expected a positive integer");
    }

    parsed
}

fn validate_api_credentials(api_id: i32, api_hash: &str) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap_or_else(|err| panic!("failed to create validation runtime: {}", err));

    runtime.block_on(async {
        let session = Arc::new(MemorySession::default());
        let pool = SenderPool::new(Arc::clone(&session), api_id);
        let handle = pool.handle.clone();
        let client = Client::new(handle.clone());

        let runner = tokio::spawn(async move {
            pool.runner.run().await;
        });

        let request = tl::functions::auth::ExportLoginToken {
            api_id,
            api_hash: api_hash.to_string(),
            except_ids: Vec::new(),
        };

        let result = tokio::time::timeout(Duration::from_secs(20), client.invoke(&request)).await;
        handle.quit();
        let _ = runner.await;

        match result {
            Ok(Ok(_)) => {}
            Ok(Err(err)) => panic!(
                "failed to validate TG_API_ID/TG_API_HASH with Telegram: {}",
                err
            ),
            Err(_) => panic!(
                "timed out while validating TG_API_ID/TG_API_HASH with Telegram"
            ),
        }
    });
}
