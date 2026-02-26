use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=.env");
    println!("cargo:rerun-if-changed=build.rs");

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

    // 读取 .env 文件
    let env_path = Path::new(".env");
    if env_path.exists() {
        let content = fs::read_to_string(env_path).unwrap();
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((key, value)) = line.split_once('=') {
                println!("cargo:rustc-env={}={}", key.trim(), value.trim());
            }
        }
    }
}
