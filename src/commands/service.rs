//! Long-lived service mode over stdin/stdout or HTTP

use crate::cli::{self, AuthCommands, Commands, LoginCommands, ServiceArgs};
use crate::i18n::{pick, set_current_language};
use anyhow::{anyhow, bail, Result};
use clap::error::ErrorKind;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::Write;
use std::process::Stdio;
use tokio::io::{self, AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::process::Command;

const EVENT_PREFIX: &str = "@@TDLR_SERVICE@@";
const MAX_HTTP_BODY_SIZE: usize = 1024 * 1024;

#[derive(Debug)]
struct ParsedRequest {
    id: Option<Value>,
    args: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct JsonRequest {
    #[serde(default)]
    id: Option<Value>,
    #[serde(default)]
    args: Option<Vec<String>>,
    #[serde(default)]
    command: Option<String>,
}

#[derive(Debug, Serialize)]
struct ServiceEvent<'a> {
    event: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<&'a Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ok: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    protocol: Option<&'a str>,
}

#[derive(Debug, Serialize)]
struct HttpCommandResponse<'a> {
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<&'a Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    exit_code: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stdout: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stderr: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<&'a str>,
}

pub async fn run(args: ServiceArgs) -> Result<()> {
    colored::control::set_override(false);

    if let Some(bind) = args.http_bind {
        return run_http(&bind).await;
    }

    run_stdio(args.json_events).await
}

async fn run_stdio(json_events: bool) -> Result<()> {
    if json_events {
        emit_event(ServiceEvent {
            event: "ready",
            id: None,
            ok: None,
            error: None,
            protocol: Some("stdio-jsonl-v1"),
        });
    }

    let stdin = BufReader::new(io::stdin());
    let mut lines = stdin.lines();

    while let Some(line) = lines.next_line().await? {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let request = match parse_request(line) {
            Ok(request) => request,
            Err(err) => {
                if json_events {
                    let message = err.to_string();
                    emit_event(ServiceEvent {
                        event: "result",
                        id: None,
                        ok: Some(false),
                        error: Some(&message),
                        protocol: None,
                    });
                } else {
                    eprintln!("{}: {}", pick("服务请求错误", "Service request error"), err);
                }
                continue;
            }
        };

        if is_exit_command(&request.args) {
            if json_events {
                emit_event(ServiceEvent {
                    event: "exit",
                    id: request.id.as_ref(),
                    ok: Some(true),
                    error: None,
                    protocol: None,
                });
            }
            break;
        }

        let (ok, error) = match execute_request(&request.args).await {
            Ok(()) => (true, None),
            Err(err) => (false, Some(err.to_string())),
        };

        if json_events {
            emit_event(ServiceEvent {
                event: "result",
                id: request.id.as_ref(),
                ok: Some(ok),
                error: error.as_deref(),
                protocol: None,
            });
        } else if let Some(error) = error {
            eprintln!("{}: {}", pick("命令执行失败", "Command failed"), error);
        }
    }

    Ok(())
}

async fn run_http(bind: &str) -> Result<()> {
    let listener = TcpListener::bind(bind).await?;
    println!(
        "{} http://{}",
        pick("HTTP API 正在监听", "HTTP API listening on"),
        bind
    );

    loop {
        let (stream, remote) = listener.accept().await?;
        tokio::spawn(async move {
            if let Err(err) = handle_http_connection(stream).await {
                eprintln!(
                    "{} {}: {}",
                    pick("HTTP 连接失败", "HTTP connection failed"),
                    remote,
                    err
                );
            }
        });
    }
}

async fn execute_request(args: &[String]) -> Result<()> {
    let parsed = match cli::parse_from(normalize_argv(args)) {
        Ok(parsed) => parsed,
        Err(err) => {
            let is_display = matches!(
                err.kind(),
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
            );
            let rendered = err.rendered();
            print!("{rendered}");
            let _ = std::io::stdout().flush();
            if is_display {
                return Ok(());
            }
            return Err(anyhow!(err.message()));
        }
    };

    set_current_language(parsed.lang);
    ensure_service_safe(&parsed.cli.command)?;
    crate::commands::execute_non_service(parsed.cli.command).await
}

async fn handle_http_connection(stream: TcpStream) -> Result<()> {
    let (read_half, mut write_half) = stream.into_split();
    let mut reader = BufReader::new(read_half);

    let mut request_line = String::new();
    if reader.read_line(&mut request_line).await? == 0 {
        return Ok(());
    }

    let request_line = request_line.trim_end_matches(['\r', '\n']);
    let mut parts = request_line.split_whitespace();
    let method = parts
        .next()
        .ok_or_else(|| anyhow!(pick("缺少 HTTP 方法", "missing HTTP method")))?
        .to_string();
    let path = parts
        .next()
        .ok_or_else(|| anyhow!(pick("缺少 HTTP 路径", "missing HTTP path")))?
        .to_string();
    let version = parts
        .next()
        .ok_or_else(|| anyhow!(pick("缺少 HTTP 版本", "missing HTTP version")))?
        .to_string();

    let mut content_length = 0usize;
    loop {
        let mut line = String::new();
        let n = reader.read_line(&mut line).await?;
        if n == 0 {
            break;
        }

        let line = line.trim_end_matches(['\r', '\n']);
        if line.is_empty() {
            break;
        }

        if let Some((name, value)) = line.split_once(':') {
            if name.trim().eq_ignore_ascii_case("content-length") {
                content_length = value.trim().parse().map_err(|_| {
                    anyhow!(pick("无效的 Content-Length", "invalid Content-Length"))
                })?;
            }
        }
    }

    let response = if !version.starts_with("HTTP/1.") {
        http_json_response(
            505,
            json!({ "ok": false, "error": pick("不支持的 HTTP 版本", "unsupported HTTP version") }),
        )
    } else if content_length > MAX_HTTP_BODY_SIZE {
        http_json_response(
            413,
            json!({ "ok": false, "error": pick("请求体过大", "request body too large") }),
        )
    } else {
        let mut body = vec![0u8; content_length];
        if content_length > 0 {
            reader.read_exact(&mut body).await?;
        }
        route_http_request(&method, &path, &body).await
    };

    write_half.write_all(response.as_bytes()).await?;
    write_half.shutdown().await?;
    Ok(())
}

async fn route_http_request(method: &str, path: &str, body: &[u8]) -> String {
    match (method, path) {
        ("GET", "/health") | ("GET", "/v1/health") => http_json_response(
            200,
            json!({
                "ok": true,
                "service": "tdlr",
                "protocol": "http-json-v1"
            }),
        ),
        ("POST", "/execute") | ("POST", "/v1/execute") => handle_http_execute(body).await,
        _ => http_json_response(
            404,
            json!({ "ok": false, "error": pick("未找到", "not found") }),
        ),
    }
}

async fn handle_http_execute(body: &[u8]) -> String {
    let body_text = match std::str::from_utf8(body) {
        Ok(text) => text.trim(),
        Err(_) => {
            return http_json_response(
                400,
                json!({ "ok": false, "error": pick("请求体必须是有效的 UTF-8", "request body must be valid UTF-8") }),
            );
        }
    };

    let request = match parse_request(body_text) {
        Ok(request) => request,
        Err(err) => {
            return http_json_response(
                400,
                json!({
                    "ok": false,
                    "error": err.to_string()
                }),
            );
        }
    };

    if let Err(err) = validate_http_request(&request.args) {
        let message = err.to_string();
        let payload = HttpCommandResponse {
            ok: false,
            id: request.id.as_ref(),
            exit_code: None,
            stdout: None,
            stderr: None,
            error: Some(&message),
        };
        return http_json_response(400, json!(payload));
    }

    match execute_request_via_child(&request).await {
        Ok((status, stdout, stderr)) => {
            let payload = HttpCommandResponse {
                ok: status.success(),
                id: request.id.as_ref(),
                exit_code: Some(status.code().unwrap_or(-1)),
                stdout: Some(&stdout),
                stderr: Some(&stderr),
                error: None,
            };
            let code = if status.success() { 200 } else { 400 };
            http_json_response(code, json!(payload))
        }
        Err(err) => {
            let message = err.to_string();
            let payload = HttpCommandResponse {
                ok: false,
                id: request.id.as_ref(),
                exit_code: None,
                stdout: None,
                stderr: None,
                error: Some(&message),
            };
            http_json_response(500, json!(payload))
        }
    }
}

async fn execute_request_via_child(
    request: &ParsedRequest,
) -> Result<(std::process::ExitStatus, String, String)> {
    let current_exe = std::env::current_exe()?;
    let mut command = Command::new(current_exe);
    let current_lang = crate::i18n::current_language();
    command
        .args(child_args(&request.args))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("TDLR_LANG", current_lang.selector())
        .env("NO_COLOR", "1")
        .env("CLICOLOR", "0")
        .env("CLICOLOR_FORCE", "0");

    let output = command.output().await?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    Ok((output.status, stdout, stderr))
}

fn validate_http_request(args: &[String]) -> Result<()> {
    ensure_service_safe_args(args)?;

    if is_exit_command(args) {
        bail!(
            "{}",
            pick(
                "HTTP API 不支持 exit 或 quit 命令",
                "HTTP API does not support exit or quit commands"
            )
        );
    }

    Ok(())
}

fn ensure_service_safe(command: &Commands) -> Result<()> {
    match command {
        Commands::Service(_) => bail!(
            "{}",
            pick(
                "服务模式不支持嵌套的 service 命令",
                "service mode does not support nested service commands"
            )
        ),
        Commands::Auth(AuthCommands::Login(LoginCommands::Add { .. })) => bail!(
            "{}",
            pick(
                "服务模式不支持 'auth login add'，因为登录交互需要独占 stdin",
                "service mode does not support 'auth login add' because login prompts require exclusive stdin"
            )
        ),
        _ => Ok(()),
    }
}

fn ensure_service_safe_args(args: &[String]) -> Result<()> {
    match child_args(args) {
        [service, ..] if service == "service" => {
            bail!(
                "{}",
                pick(
                    "服务模式不支持嵌套的 service 命令",
                    "service mode does not support nested service commands"
                )
            )
        }
        [auth, login, add, ..] if auth == "auth" && login == "login" && add == "add" => bail!(
            "{}",
            pick(
                "服务模式不支持 'auth login add'，因为登录交互需要独占 stdin",
                "service mode does not support 'auth login add' because login prompts require exclusive stdin"
            )
        ),
        [] => bail!("{}", pick("空命令", "empty command")),
        _ => Ok(()),
    }
}

fn normalize_argv(args: &[String]) -> Vec<String> {
    if matches!(args.first(), Some(first) if first == "tdlr") {
        args.to_vec()
    } else {
        let mut argv = Vec::with_capacity(args.len() + 1);
        argv.push("tdlr".to_string());
        argv.extend(args.iter().cloned());
        argv
    }
}

fn child_args(args: &[String]) -> &[String] {
    if matches!(args.first(), Some(first) if first == "tdlr") {
        &args[1..]
    } else {
        args
    }
}

fn parse_request(line: &str) -> Result<ParsedRequest> {
    if line.starts_with('{') {
        return parse_json_request(line);
    }

    if line.starts_with('[') {
        let args: Vec<String> = serde_json::from_str(line).map_err(|err| {
            anyhow!(
                "{}",
                if crate::i18n::is_zh() {
                    format!("无效的 JSON 参数数组: {}", err)
                } else {
                    format!("invalid JSON args array: {}", err)
                }
            )
        })?;
        return build_request(None, args);
    }

    build_request(None, split_command_line(line)?)
}

fn parse_json_request(line: &str) -> Result<ParsedRequest> {
    let request: JsonRequest = serde_json::from_str(line).map_err(|err| {
        anyhow!(
            "{}",
            if crate::i18n::is_zh() {
                format!("无效的 JSON 请求: {}", err)
            } else {
                format!("invalid JSON request: {}", err)
            }
        )
    })?;

    let args = match (request.args, request.command) {
        (Some(args), None) => args,
        (None, Some(command)) => split_command_line(&command)?,
        (Some(_), Some(_)) => bail!(
            "{}",
            pick(
                "JSON 请求不能同时包含 'args' 和 'command'",
                "JSON request cannot contain both 'args' and 'command'"
            )
        ),
        (None, None) => bail!(
            "{}",
            pick(
                "JSON 请求必须包含 'args' 或 'command' 之一",
                "JSON request must contain either 'args' or 'command'"
            )
        ),
    };

    build_request(request.id, args)
}

fn build_request(id: Option<Value>, args: Vec<String>) -> Result<ParsedRequest> {
    if args.is_empty() {
        bail!("{}", pick("空命令", "empty command"));
    }

    Ok(ParsedRequest { id, args })
}

fn is_exit_command(args: &[String]) -> bool {
    matches!(args, [single] if single == "exit" || single == "quit")
        || matches!(args, [first, second] if first == "tdlr" && (second == "exit" || second == "quit"))
}

fn emit_event(event: ServiceEvent<'_>) {
    let line = json!(event).to_string();
    println!("{EVENT_PREFIX} {line}");
    let _ = std::io::stdout().flush();
}

fn http_json_response(status: u16, body: Value) -> String {
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        413 => "Payload Too Large",
        500 => "Internal Server Error",
        505 => "HTTP Version Not Supported",
        _ => "OK",
    };

    let body = body.to_string();
    format!(
        "HTTP/1.1 {} {}\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        status,
        reason,
        body.len(),
        body
    )
}

fn split_command_line(input: &str) -> Result<Vec<String>> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut chars = input.chars().peekable();
    let mut quote: Option<char> = None;

    while let Some(ch) = chars.next() {
        match quote {
            Some(q) if ch == q => {
                quote = None;
            }
            Some('"') if ch == '\\' => match chars.peek().copied() {
                Some('"') | Some('\\') => {
                    let escaped = chars.next().ok_or_else(|| {
                        anyhow!(pick(
                            "引号字符串末尾存在转义符",
                            "trailing escape in quoted string"
                        ))
                    })?;
                    current.push(escaped);
                }
                Some(_) => current.push(ch),
                None => bail!(
                    "{}",
                    pick(
                        "引号字符串末尾存在转义符",
                        "trailing escape in quoted string"
                    )
                ),
            },
            Some(_) => current.push(ch),
            None if ch.is_whitespace() => {
                if !current.is_empty() {
                    args.push(std::mem::take(&mut current));
                }
            }
            None if ch == '"' || ch == '\'' => {
                quote = Some(ch);
            }
            None if ch == '\\' => {
                let escaped = chars.next().ok_or_else(|| {
                    anyhow!(pick("命令末尾存在转义符", "trailing escape in command"))
                })?;
                current.push(escaped);
            }
            None => current.push(ch),
        }
    }

    if quote.is_some() {
        bail!("{}", pick("引号字符串未闭合", "unterminated quoted string"));
    }

    if !current.is_empty() {
        args.push(current);
    }

    Ok(args)
}
