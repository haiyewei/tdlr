use crate::cli::ForwardMode;
use crate::i18n::{current_language, is_zh, pick};
use crate::telegram::{
    auth::{export_qr_login_token, try_import_login, QrLoginTokenExport},
    pool,
    session::AccountInfo,
    SessionManager, TelegramClient,
};
use anyhow::Result;
use grammers_client::{
    client::{LoginToken, PasswordToken},
    peer::User,
    SignInError,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::process::Command;
use tokio::sync::{Mutex, RwLock};

const FLOW_TTL_SECS: u64 = 10 * 60;
const MAX_CODE_ATTEMPTS: usize = 3;

const API_HASH: &str = env!("TG_API_HASH");

static NEXT_FLOW_ID: AtomicU64 = AtomicU64::new(1);
static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug)]
struct ApiError {
    status: u16,
    message: String,
}

impl ApiError {
    fn new(status: u16, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
        }
    }

    fn bad_request(message: impl Into<String>) -> Self {
        Self::new(400, message)
    }

    fn not_found(message: impl Into<String>) -> Self {
        Self::new(404, message)
    }

    fn internal(message: impl Into<String>) -> Self {
        Self::new(500, message)
    }
}

type ApiResult<T> = std::result::Result<T, ApiError>;

#[derive(Default)]
pub(crate) struct HttpApiState {
    flows: RwLock<HashMap<String, Arc<Mutex<AuthFlow>>>>,
}

#[derive(Debug)]
struct FlowCommon {
    temp_name: String,
    updated_at: u64,
}

impl FlowCommon {
    fn new(temp_name: String) -> Self {
        Self {
            temp_name,
            updated_at: now_secs(),
        }
    }

    fn touch(&mut self) {
        self.updated_at = now_secs();
    }

    fn is_expired(&self) -> bool {
        now_secs().saturating_sub(self.updated_at) > FLOW_TTL_SECS
    }
}

enum AuthFlow {
    PhoneCode {
        common: FlowCommon,
        tg: TelegramClient,
        phone: String,
        token: LoginToken,
        invalid_attempts: usize,
    },
    PhonePassword {
        common: FlowCommon,
        tg: TelegramClient,
        phone: String,
        password_token: PasswordToken,
    },
    Qr {
        common: FlowCommon,
        tg: TelegramClient,
        login_url: String,
        expires_at: i32,
    },
    Consumed,
}

impl AuthFlow {
    fn common(&self) -> Option<&FlowCommon> {
        match self {
            Self::PhoneCode { common, .. }
            | Self::PhonePassword { common, .. }
            | Self::Qr { common, .. } => Some(common),
            Self::Consumed => None,
        }
    }
}

#[derive(Debug, Serialize)]
struct AccountSummary {
    user_id: i64,
    display_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    username: Option<String>,
    active: bool,
}

#[derive(Debug, Serialize)]
struct AccountStatusSummary {
    user_id: i64,
    display_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    username: Option<String>,
    active: bool,
    authorized: bool,
    detail: String,
}

#[derive(Debug, Serialize)]
struct AuthAccountSummary {
    user_id: i64,
    display_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    username: Option<String>,
    active: bool,
}

#[derive(Debug, Deserialize)]
struct ActivateAccountRequest {
    id: i64,
}

#[derive(Debug, Deserialize, Default)]
struct LogoutRequest {
    #[serde(default)]
    id: Option<i64>,
    #[serde(default)]
    all: bool,
}

#[derive(Debug, Deserialize)]
struct PhoneStartRequest {
    phone: String,
}

#[derive(Debug, Deserialize)]
struct PhoneCodeRequest {
    flow_id: String,
    code: String,
}

#[derive(Debug, Deserialize)]
struct PhonePasswordRequest {
    flow_id: String,
    password: String,
}

#[derive(Debug, Deserialize, Default)]
struct QrStartRequest {}

#[derive(Debug, Deserialize)]
struct UploadRequest {
    path: Vec<String>,
    #[serde(default)]
    chat: Option<String>,
    #[serde(default)]
    include: Option<Vec<String>>,
    #[serde(default)]
    exclude: Option<Vec<String>>,
    #[serde(default)]
    rm: bool,
    #[serde(default)]
    topic: Option<i32>,
    #[serde(default)]
    account: Option<Vec<i64>>,
    #[serde(default)]
    all_accounts: bool,
    #[serde(default)]
    caption: Option<String>,
    #[serde(default)]
    thumb: Option<Vec<String>>,
    #[serde(default)]
    thumb_map: Option<Vec<String>>,
    #[serde(default)]
    to: Option<String>,
    #[serde(default)]
    group: bool,
}

#[derive(Debug, Deserialize)]
struct DownloadRequest {
    url: Vec<String>,
    #[serde(default = "default_download_path")]
    path: String,
    #[serde(default)]
    include: Option<Vec<String>>,
    #[serde(default)]
    exclude: Option<Vec<String>>,
    #[serde(default)]
    template: Option<String>,
    #[serde(default)]
    account: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct ForwardRequest {
    from: Vec<String>,
    #[serde(default)]
    from_chat: Option<String>,
    #[serde(default)]
    to: Option<String>,
    #[serde(default)]
    mode: ForwardMode,
    #[serde(default)]
    topic: Option<i32>,
    #[serde(default)]
    account: Option<i64>,
    #[serde(default)]
    drop_author: bool,
}

fn default_download_path() -> String {
    ".".to_string()
}

fn api_id() -> i32 {
    env!("TG_API_ID").parse().expect("Invalid TG_API_ID")
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn next_flow_id() -> String {
    format!(
        "flow-{}-{}",
        now_secs(),
        NEXT_FLOW_ID.fetch_add(1, Ordering::Relaxed)
    )
}

fn next_temp_name() -> String {
    format!(
        "temp_service_{}_{}",
        now_secs(),
        NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed)
    )
}

fn sanitize_auth_code(input: &str) -> String {
    input
        .trim()
        .chars()
        .filter(|ch| {
            !ch.is_whitespace()
                && !matches!(
                    ch,
                    '\u{200B}' | '\u{200C}' | '\u{200D}' | '\u{2060}' | '\u{FEFF}' | '-'
                )
        })
        .collect()
}

fn cleanup_temp_session(temp_name: &str) {
    let dir = SessionManager::sessions_dir().join(temp_name);
    if dir.exists() {
        let _ = fs::remove_dir_all(&dir);
        return;
    }

    let session = SessionManager::sessions_dir()
        .join(temp_name)
        .join(format!("{}.session", temp_name));
    if session.exists() {
        let _ = fs::remove_file(session);
    }
}

async fn finalize_authenticated_flow(
    temp_name: String,
    tg: TelegramClient,
    user: User,
) -> Result<AuthAccountSummary> {
    let user_id = user.raw.id();
    let display_name = user
        .first_name()
        .unwrap_or(pick("用户", "User"))
        .to_string();
    let username = user.username().map(|value| value.to_string());

    drop(tg);
    tokio::time::sleep(Duration::from_millis(100)).await;

    let temp_session = SessionManager::session_path_str(&temp_name);
    let temp_dir = SessionManager::sessions_dir().join(&temp_name);
    let final_path = SessionManager::session_path(user_id);
    if final_path.exists() {
        let _ = fs::remove_file(&final_path);
    }
    fs::rename(&temp_session, &final_path)?;
    let _ = fs::remove_dir(&temp_dir);

    SessionManager::save_account(AccountInfo {
        user_id,
        display_name: display_name.clone(),
        username: username.clone(),
    })?;
    SessionManager::set_active(user_id)?;

    Ok(AuthAccountSummary {
        user_id,
        display_name,
        username,
        active: true,
    })
}

async fn execute_child_command(
    args: &[String],
) -> Result<(std::process::ExitStatus, String, String)> {
    let current_exe = std::env::current_exe()?;
    let mut command = Command::new(current_exe);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("TDLR_LANG", current_language().selector())
        .env("NO_COLOR", "1")
        .env("CLICOLOR", "0")
        .env("CLICOLOR_FORCE", "0");

    let output = command.output().await?;
    Ok((
        output.status,
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    ))
}

fn push_multi_value_flag(args: &mut Vec<String>, flag: &str, values: &[String]) {
    if values.is_empty() {
        return;
    }

    args.push(flag.to_string());
    args.extend(values.iter().cloned());
}

fn push_single_value_flag(args: &mut Vec<String>, flag: &str, value: &str) {
    args.push(format!("{flag}={value}"));
}

fn build_upload_args(request: &UploadRequest) -> Vec<String> {
    let mut args = vec!["upload".to_string()];
    push_multi_value_flag(&mut args, "--path", &request.path);

    if let Some(chat) = &request.chat {
        push_single_value_flag(&mut args, "--chat", chat);
    }
    if let Some(include) = &request.include {
        push_multi_value_flag(&mut args, "--include", include);
    }
    if let Some(exclude) = &request.exclude {
        push_multi_value_flag(&mut args, "--exclude", exclude);
    }
    if request.rm {
        args.push("--rm".to_string());
    }
    if let Some(topic) = request.topic {
        args.push("--topic".to_string());
        args.push(topic.to_string());
    }
    if let Some(account) = &request.account {
        for user_id in account {
            args.push("--account".to_string());
            args.push(user_id.to_string());
        }
    }
    if request.all_accounts {
        args.push("--all-accounts".to_string());
    }
    if let Some(caption) = &request.caption {
        args.push("--caption".to_string());
        args.push(caption.clone());
    }
    if let Some(thumb) = &request.thumb {
        push_multi_value_flag(&mut args, "--thumb", thumb);
    }
    if let Some(thumb_map) = &request.thumb_map {
        push_multi_value_flag(&mut args, "--thumb-map", thumb_map);
    }
    if let Some(to) = &request.to {
        push_single_value_flag(&mut args, "--to", to);
    }
    if request.group {
        args.push("--group".to_string());
    }

    args
}

fn build_download_args(request: &DownloadRequest) -> Vec<String> {
    let mut args = vec!["download".to_string()];
    push_multi_value_flag(&mut args, "--url", &request.url);
    args.push("--path".to_string());
    args.push(request.path.clone());

    if let Some(include) = &request.include {
        push_multi_value_flag(&mut args, "--include", include);
    }
    if let Some(exclude) = &request.exclude {
        push_multi_value_flag(&mut args, "--exclude", exclude);
    }
    if let Some(template) = &request.template {
        args.push("--template".to_string());
        args.push(template.clone());
    }
    if let Some(account) = request.account {
        args.push("--account".to_string());
        args.push(account.to_string());
    }

    args
}

fn build_forward_args(request: &ForwardRequest) -> Vec<String> {
    let mut args = vec!["forward".to_string()];
    push_multi_value_flag(&mut args, "--from", &request.from);

    if let Some(from_chat) = &request.from_chat {
        push_single_value_flag(&mut args, "--from-chat", from_chat);
    }
    if let Some(to) = &request.to {
        push_single_value_flag(&mut args, "--to", to);
    }
    args.push("--mode".to_string());
    args.push(
        match request.mode {
            ForwardMode::Direct => "direct",
            ForwardMode::Clone => "clone",
            ForwardMode::Smart => "smart",
        }
        .to_string(),
    );
    if let Some(topic) = request.topic {
        args.push("--topic".to_string());
        args.push(topic.to_string());
    }
    if let Some(account) = request.account {
        args.push("--account".to_string());
        args.push(account.to_string());
    }
    if request.drop_author {
        args.push("--drop-author".to_string());
    }

    args
}

fn child_response(
    status: std::process::ExitStatus,
    stdout: String,
    stderr: String,
) -> (u16, Value) {
    let ok = status.success();
    let code = if ok { 200 } else { 400 };
    (
        code,
        json!({
            "ok": ok,
            "exit_code": status.code().unwrap_or(-1),
            "stdout": stdout,
            "stderr": stderr,
        }),
    )
}

fn api_error_response(err: ApiError) -> (u16, Value) {
    (
        err.status,
        json!({
            "ok": false,
            "error": err.message,
        }),
    )
}

fn parse_json_body<T>(body: &[u8]) -> ApiResult<T>
where
    T: for<'de> Deserialize<'de>,
{
    let text = std::str::from_utf8(body).map_err(|_| {
        ApiError::bad_request(pick(
            "请求体必须是有效的 UTF-8",
            "request body must be valid UTF-8",
        ))
    })?;
    let payload = if text.trim().is_empty() {
        "{}"
    } else {
        text.trim()
    };
    serde_json::from_str(payload).map_err(|err| {
        ApiError::bad_request(if is_zh() {
            format!("无效的 JSON 请求: {}", err)
        } else {
            format!("invalid JSON request: {}", err)
        })
    })
}

fn path_segments(path: &str) -> Vec<&str> {
    let path = path.split('?').next().unwrap_or(path).trim_matches('/');
    if path.is_empty() {
        Vec::new()
    } else {
        path.split('/').collect()
    }
}

impl HttpApiState {
    pub(crate) fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    async fn insert_flow(&self, flow_id: String, flow: AuthFlow) {
        self.flows
            .write()
            .await
            .insert(flow_id, Arc::new(Mutex::new(flow)));
    }

    async fn get_flow(&self, flow_id: &str) -> Option<Arc<Mutex<AuthFlow>>> {
        self.flows.read().await.get(flow_id).cloned()
    }

    async fn remove_flow(&self, flow_id: &str) {
        self.flows.write().await.remove(flow_id);
    }

    async fn cancel_flow(&self, flow_id: &str) -> ApiResult<Value> {
        let flow =
            self.flows.write().await.remove(flow_id).ok_or_else(|| {
                ApiError::not_found(pick("登录流程不存在", "login flow not found"))
            })?;

        let mut guard = flow.lock().await;
        match std::mem::replace(&mut *guard, AuthFlow::Consumed) {
            AuthFlow::PhoneCode { common, .. }
            | AuthFlow::PhonePassword { common, .. }
            | AuthFlow::Qr { common, .. } => {
                cleanup_temp_session(&common.temp_name);
            }
            AuthFlow::Consumed => {}
        }

        Ok(json!({
            "ok": true,
            "flow_id": flow_id,
            "status": "cancelled",
        }))
    }

    async fn cleanup_expired_flows(&self) {
        let snapshot: Vec<(String, Arc<Mutex<AuthFlow>>)> = self
            .flows
            .read()
            .await
            .iter()
            .map(|(flow_id, flow)| (flow_id.clone(), Arc::clone(flow)))
            .collect();

        for (flow_id, flow) in snapshot {
            let expired = {
                let guard = flow.lock().await;
                guard
                    .common()
                    .map(|common| common.is_expired())
                    .unwrap_or(false)
            };
            if expired {
                let _ = self.cancel_flow(&flow_id).await;
            }
        }
    }
}

fn list_accounts_payload() -> ApiResult<Value> {
    let accounts =
        SessionManager::list_accounts().map_err(|err| ApiError::internal(err.to_string()))?;
    let active = SessionManager::get_active().map_err(|err| ApiError::internal(err.to_string()))?;
    let payload: Vec<_> = accounts
        .into_iter()
        .map(|account| AccountSummary {
            active: active == Some(account.user_id),
            user_id: account.user_id,
            display_name: account.display_name,
            username: account.username,
        })
        .collect();
    Ok(json!({
        "ok": true,
        "accounts": payload,
    }))
}

async fn account_status_payload() -> ApiResult<Value> {
    let accounts =
        SessionManager::list_accounts().map_err(|err| ApiError::internal(err.to_string()))?;
    let active = SessionManager::get_active().map_err(|err| ApiError::internal(err.to_string()))?;
    let mut status = Vec::with_capacity(accounts.len());

    for account in accounts {
        let detail;
        let mut authorized = false;
        let mut username = account.username.clone();

        match pool().get(account.user_id).await {
            Ok(client) => match client.is_authorized().await {
                Ok(true) => {
                    authorized = true;
                    detail = match client.get_me().await {
                        Ok(user) => {
                            username = user.username().map(|value| value.to_string()).or(username);
                            user.first_name()
                                .unwrap_or(pick("未知", "Unknown"))
                                .to_string()
                        }
                        Err(_) => pick("已授权（获取信息失败）", "Authorized (failed to get info)")
                            .to_string(),
                    };
                }
                Ok(false) => {
                    detail = pick("未授权", "Not authorized").to_string();
                }
                Err(err) => {
                    detail = if is_zh() {
                        format!("错误: {}", err)
                    } else {
                        format!("Error: {}", err)
                    };
                }
            },
            Err(err) => {
                detail = err.to_string();
            }
        }

        status.push(AccountStatusSummary {
            user_id: account.user_id,
            display_name: account.display_name,
            username,
            active: active == Some(account.user_id),
            authorized,
            detail,
        });
    }

    Ok(json!({
        "ok": true,
        "accounts": status,
    }))
}

async fn activate_account(body: &[u8]) -> ApiResult<(u16, Value)> {
    let request: ActivateAccountRequest = parse_json_body(body)?;
    SessionManager::set_active(request.id).map_err(|err| ApiError::bad_request(err.to_string()))?;
    let account = SessionManager::get_account(request.id)
        .map_err(|err| ApiError::internal(err.to_string()))?
        .map(|value| value.display_name)
        .unwrap_or_else(|| request.id.to_string());
    Ok((
        200,
        json!({
            "ok": true,
            "user_id": request.id,
            "display_name": account,
        }),
    ))
}

async fn remove_account(user_id: i64) -> ApiResult<(u16, Value)> {
    if !SessionManager::exists(user_id) {
        return Err(ApiError::not_found(if is_zh() {
            format!("未找到账号 {}", user_id)
        } else {
            format!("Account {} not found", user_id)
        }));
    }

    let was_active = SessionManager::get_active()
        .map_err(|err| ApiError::internal(err.to_string()))?
        == Some(user_id);
    let display_name = SessionManager::get_account(user_id)
        .map_err(|err| ApiError::internal(err.to_string()))?
        .map(|value| value.display_name)
        .unwrap_or_else(|| user_id.to_string());

    SessionManager::remove(user_id).map_err(|err| ApiError::internal(err.to_string()))?;
    pool().remove(user_id).await;

    let mut switched_to = None;
    if was_active {
        SessionManager::clear_active();
        if let Some(next_id) = SessionManager::list_user_ids()
            .map_err(|err| ApiError::internal(err.to_string()))?
            .first()
            .copied()
        {
            SessionManager::set_active(next_id)
                .map_err(|err| ApiError::internal(err.to_string()))?;
            switched_to = Some(next_id);
        }
    }

    Ok((
        200,
        json!({
            "ok": true,
            "user_id": user_id,
            "display_name": display_name,
            "switched_to": switched_to,
        }),
    ))
}

async fn logout_accounts(body: &[u8]) -> ApiResult<(u16, Value)> {
    let request: LogoutRequest = parse_json_body(body)?;
    if request.all {
        let ids =
            SessionManager::list_user_ids().map_err(|err| ApiError::internal(err.to_string()))?;
        for user_id in &ids {
            SessionManager::remove(*user_id).map_err(|err| ApiError::internal(err.to_string()))?;
        }
        SessionManager::clear_active();
        pool().clear().await;
        return Ok((
            200,
            json!({
                "ok": true,
                "count": ids.len(),
            }),
        ));
    }

    let user_id = match request.id {
        Some(value) => value,
        None => SessionManager::get_active()
            .map_err(|err| ApiError::internal(err.to_string()))?
            .ok_or_else(|| {
                ApiError::bad_request(pick(
                    "当前没有活跃账号。请指定 user_id 或使用 all=true。",
                    "No active account. Specify a user_id or set all=true.",
                ))
            })?,
    };

    remove_account(user_id).await
}

async fn start_phone_login(state: Arc<HttpApiState>, body: &[u8]) -> ApiResult<(u16, Value)> {
    let request: PhoneStartRequest = parse_json_body(body)?;
    let phone = request.phone.trim();
    if phone.is_empty() {
        return Err(ApiError::bad_request(pick(
            "手机号不能为空",
            "phone number cannot be empty",
        )));
    }

    let temp_name = next_temp_name();
    let tg = TelegramClient::new_temp(&temp_name, api_id())
        .await
        .map_err(|err| ApiError::internal(err.to_string()))?;

    let token = tokio::time::timeout(
        Duration::from_secs(30),
        tg.inner().request_login_code(phone, API_HASH),
    )
    .await
    .map_err(|_| ApiError::new(504, pick("请求超时", "request timed out")))?
    .map_err(|err| ApiError::bad_request(err.to_string()))?;

    let flow_id = next_flow_id();
    state
        .insert_flow(
            flow_id.clone(),
            AuthFlow::PhoneCode {
                common: FlowCommon::new(temp_name),
                tg,
                phone: phone.to_string(),
                token,
                invalid_attempts: 0,
            },
        )
        .await;

    Ok((
        200,
        json!({
            "ok": true,
            "flow_id": flow_id,
            "kind": "phone",
            "status": "waiting_for_code",
            "phone": phone,
        }),
    ))
}

async fn submit_phone_code(state: Arc<HttpApiState>, body: &[u8]) -> ApiResult<(u16, Value)> {
    let request: PhoneCodeRequest = parse_json_body(body)?;
    let code = sanitize_auth_code(&request.code);
    if code.is_empty() {
        return Err(ApiError::bad_request(pick(
            "验证码不能为空",
            "verification code cannot be empty",
        )));
    }

    let flow = state
        .get_flow(&request.flow_id)
        .await
        .ok_or_else(|| ApiError::not_found(pick("登录流程不存在", "login flow not found")))?;

    let mut guard = flow.lock().await;
    let current = std::mem::replace(&mut *guard, AuthFlow::Consumed);
    match current {
        AuthFlow::PhoneCode {
            mut common,
            tg,
            phone,
            token,
            invalid_attempts,
        } => {
            common.touch();
            let sign_in =
                tokio::time::timeout(Duration::from_secs(30), tg.inner().sign_in(&token, &code))
                    .await
                    .map_err(|_| ApiError::new(504, pick("登录超时", "sign in timed out")))?;

            match sign_in {
                Ok(user) => {
                    drop(guard);
                    state.remove_flow(&request.flow_id).await;
                    let account = finalize_authenticated_flow(common.temp_name, tg, user)
                        .await
                        .map_err(|err| ApiError::internal(err.to_string()))?;
                    Ok((
                        200,
                        json!({
                            "ok": true,
                            "flow_id": request.flow_id,
                            "kind": "phone",
                            "status": "completed",
                            "account": account,
                        }),
                    ))
                }
                Err(SignInError::PasswordRequired(password_token)) => {
                    let hint = password_token.hint().map(|value| value.to_string());
                    *guard = AuthFlow::PhonePassword {
                        common,
                        tg,
                        phone,
                        password_token,
                    };
                    Ok((
                        200,
                        json!({
                            "ok": true,
                            "flow_id": request.flow_id,
                            "kind": "phone",
                            "status": "waiting_for_password",
                            "hint": hint,
                        }),
                    ))
                }
                Err(SignInError::InvalidCode) => {
                    let used_attempts = invalid_attempts + 1;
                    if used_attempts >= MAX_CODE_ATTEMPTS {
                        drop(guard);
                        state.remove_flow(&request.flow_id).await;
                        cleanup_temp_session(&common.temp_name);
                        Err(ApiError::bad_request(pick(
                            "验证码无效，已超过最大重试次数。",
                            "invalid verification code, maximum retry attempts exceeded.",
                        )))
                    } else {
                        let remaining = MAX_CODE_ATTEMPTS - used_attempts;
                        *guard = AuthFlow::PhoneCode {
                            common,
                            tg,
                            phone,
                            token,
                            invalid_attempts: used_attempts,
                        };
                        Err(ApiError::new(
                            400,
                            if is_zh() {
                                format!("验证码无效，还可重试 {} 次。", remaining)
                            } else {
                                format!(
                                    "Invalid verification code. {remaining} attempt(s) remaining."
                                )
                            },
                        ))
                    }
                }
                Err(err) => {
                    drop(guard);
                    state.remove_flow(&request.flow_id).await;
                    cleanup_temp_session(&common.temp_name);
                    Err(ApiError::bad_request(err.to_string()))
                }
            }
        }
        other => {
            *guard = other;
            Err(ApiError::bad_request(pick(
                "当前流程不处于等待验证码状态",
                "flow is not waiting for a verification code",
            )))
        }
    }
}

async fn submit_phone_password(state: Arc<HttpApiState>, body: &[u8]) -> ApiResult<(u16, Value)> {
    let request: PhonePasswordRequest = parse_json_body(body)?;
    if request.password.trim().is_empty() {
        return Err(ApiError::bad_request(pick(
            "两步验证密码不能为空",
            "2FA password cannot be empty",
        )));
    }

    let flow = state
        .get_flow(&request.flow_id)
        .await
        .ok_or_else(|| ApiError::not_found(pick("登录流程不存在", "login flow not found")))?;

    let mut guard = flow.lock().await;
    let current = std::mem::replace(&mut *guard, AuthFlow::Consumed);
    match current {
        AuthFlow::PhonePassword {
            mut common,
            tg,
            phone,
            password_token,
        } => {
            common.touch();
            let result = tokio::time::timeout(
                Duration::from_secs(30),
                tg.inner()
                    .check_password(password_token, request.password.trim()),
            )
            .await
            .map_err(|_| ApiError::new(504, pick("密码校验超时", "password check timed out")))?;

            match result {
                Ok(user) => {
                    drop(guard);
                    state.remove_flow(&request.flow_id).await;
                    let account = finalize_authenticated_flow(common.temp_name, tg, user)
                        .await
                        .map_err(|err| ApiError::internal(err.to_string()))?;
                    Ok((
                        200,
                        json!({
                            "ok": true,
                            "flow_id": request.flow_id,
                            "kind": "phone",
                            "status": "completed",
                            "account": account,
                        }),
                    ))
                }
                Err(SignInError::InvalidPassword(next_password_token)) => {
                    let hint = next_password_token.hint().map(|value| value.to_string());
                    *guard = AuthFlow::PhonePassword {
                        common,
                        tg,
                        phone,
                        password_token: next_password_token,
                    };
                    Err(ApiError::new(
                        400,
                        if let Some(hint) = hint {
                            if is_zh() {
                                format!("两步验证密码错误。提示: {}", hint)
                            } else {
                                format!("Invalid 2FA password. Hint: {}", hint)
                            }
                        } else {
                            pick("两步验证密码错误", "invalid 2FA password").to_string()
                        },
                    ))
                }
                Err(err) => {
                    drop(guard);
                    state.remove_flow(&request.flow_id).await;
                    cleanup_temp_session(&common.temp_name);
                    Err(ApiError::bad_request(err.to_string()))
                }
            }
        }
        other => {
            *guard = other;
            Err(ApiError::bad_request(pick(
                "当前流程不处于等待两步验证密码状态",
                "flow is not waiting for a 2FA password",
            )))
        }
    }
}

async fn start_qr_login(state: Arc<HttpApiState>, body: &[u8]) -> ApiResult<(u16, Value)> {
    let _: QrStartRequest = parse_json_body(body)?;
    let temp_name = next_temp_name();
    let tg = TelegramClient::new_temp(&temp_name, api_id())
        .await
        .map_err(|err| ApiError::internal(err.to_string()))?;

    match export_qr_login_token(&tg, api_id(), API_HASH)
        .await
        .map_err(|err| ApiError::bad_request(err.to_string()))?
    {
        QrLoginTokenExport::Ready { url, expires_at } => {
            let flow_id = next_flow_id();
            state
                .insert_flow(
                    flow_id.clone(),
                    AuthFlow::Qr {
                        common: FlowCommon::new(temp_name),
                        tg,
                        login_url: url.clone(),
                        expires_at,
                    },
                )
                .await;
            Ok((
                200,
                json!({
                    "ok": true,
                    "flow_id": flow_id,
                    "kind": "qr",
                    "status": "waiting_for_scan",
                    "login_url": url,
                    "expires_at": expires_at,
                }),
            ))
        }
        QrLoginTokenExport::Authorized(user) => {
            let account = finalize_authenticated_flow(temp_name, tg, user)
                .await
                .map_err(|err| ApiError::internal(err.to_string()))?;
            Ok((
                200,
                json!({
                    "ok": true,
                    "kind": "qr",
                    "status": "completed",
                    "account": account,
                }),
            ))
        }
        QrLoginTokenExport::PendingMigration { dc_id, token } => {
            match try_import_login(&tg, dc_id, token)
                .await
                .map_err(|err| ApiError::bad_request(err.to_string()))?
            {
                Some(user) => {
                    let account = finalize_authenticated_flow(temp_name, tg, user)
                        .await
                        .map_err(|err| ApiError::internal(err.to_string()))?;
                    Ok((
                        200,
                        json!({
                            "ok": true,
                            "kind": "qr",
                            "status": "completed",
                            "account": account,
                        }),
                    ))
                }
                None => match export_qr_login_token(&tg, api_id(), API_HASH)
                    .await
                    .map_err(|err| ApiError::bad_request(err.to_string()))?
                {
                    QrLoginTokenExport::Ready { url, expires_at } => {
                        let flow_id = next_flow_id();
                        state
                            .insert_flow(
                                flow_id.clone(),
                                AuthFlow::Qr {
                                    common: FlowCommon::new(temp_name),
                                    tg,
                                    login_url: url.clone(),
                                    expires_at,
                                },
                            )
                            .await;
                        Ok((
                            200,
                            json!({
                                "ok": true,
                                "flow_id": flow_id,
                                "kind": "qr",
                                "status": "waiting_for_scan",
                                "login_url": url,
                                "expires_at": expires_at,
                            }),
                        ))
                    }
                    QrLoginTokenExport::Authorized(user) => {
                        let account = finalize_authenticated_flow(temp_name, tg, user)
                            .await
                            .map_err(|err| ApiError::internal(err.to_string()))?;
                        Ok((
                            200,
                            json!({
                                "ok": true,
                                "kind": "qr",
                                "status": "completed",
                                "account": account,
                            }),
                        ))
                    }
                    QrLoginTokenExport::PendingMigration { .. } => {
                        cleanup_temp_session(&temp_name);
                        Err(ApiError::bad_request(pick(
                            "二维码登录初始化失败，请重试",
                            "failed to initialize QR login, please retry",
                        )))
                    }
                },
            }
        }
    }
}

async fn get_flow_status(state: Arc<HttpApiState>, flow_id: &str) -> ApiResult<(u16, Value)> {
    let flow = state
        .get_flow(flow_id)
        .await
        .ok_or_else(|| ApiError::not_found(pick("登录流程不存在", "login flow not found")))?;

    let mut guard = flow.lock().await;
    let current = std::mem::replace(&mut *guard, AuthFlow::Consumed);
    match current {
        AuthFlow::PhoneCode {
            mut common,
            tg,
            phone,
            token,
            invalid_attempts,
        } => {
            common.touch();
            *guard = AuthFlow::PhoneCode {
                common,
                tg,
                phone: phone.clone(),
                token,
                invalid_attempts,
            };
            Ok((
                200,
                json!({
                    "ok": true,
                    "flow_id": flow_id,
                    "kind": "phone",
                    "status": "waiting_for_code",
                    "phone": phone,
                    "remaining_attempts": MAX_CODE_ATTEMPTS.saturating_sub(invalid_attempts),
                }),
            ))
        }
        AuthFlow::PhonePassword {
            mut common,
            tg,
            phone,
            password_token,
        } => {
            let hint = password_token.hint().map(|value| value.to_string());
            common.touch();
            *guard = AuthFlow::PhonePassword {
                common,
                tg,
                phone: phone.clone(),
                password_token,
            };
            Ok((
                200,
                json!({
                    "ok": true,
                    "flow_id": flow_id,
                    "kind": "phone",
                    "status": "waiting_for_password",
                    "phone": phone,
                    "hint": hint,
                }),
            ))
        }
        AuthFlow::Qr {
            mut common,
            tg,
            login_url,
            expires_at,
        } => {
            common.touch();
            let step = export_qr_login_token(&tg, api_id(), API_HASH)
                .await
                .map_err(|err| ApiError::bad_request(err.to_string()))?;

            match step {
                QrLoginTokenExport::Ready { url, expires_at } => {
                    *guard = AuthFlow::Qr {
                        common,
                        tg,
                        login_url: url.clone(),
                        expires_at,
                    };
                    Ok((
                        200,
                        json!({
                            "ok": true,
                            "flow_id": flow_id,
                            "kind": "qr",
                            "status": "waiting_for_scan",
                            "login_url": url,
                            "expires_at": expires_at,
                        }),
                    ))
                }
                QrLoginTokenExport::Authorized(user) => {
                    drop(guard);
                    state.remove_flow(flow_id).await;
                    let account = finalize_authenticated_flow(common.temp_name, tg, user)
                        .await
                        .map_err(|err| ApiError::internal(err.to_string()))?;
                    Ok((
                        200,
                        json!({
                            "ok": true,
                            "flow_id": flow_id,
                            "kind": "qr",
                            "status": "completed",
                            "account": account,
                        }),
                    ))
                }
                QrLoginTokenExport::PendingMigration { dc_id, token } => {
                    match try_import_login(&tg, dc_id, token)
                        .await
                        .map_err(|err| ApiError::bad_request(err.to_string()))?
                    {
                        Some(user) => {
                            drop(guard);
                            state.remove_flow(flow_id).await;
                            let account = finalize_authenticated_flow(common.temp_name, tg, user)
                                .await
                                .map_err(|err| ApiError::internal(err.to_string()))?;
                            Ok((
                                200,
                                json!({
                                    "ok": true,
                                    "flow_id": flow_id,
                                    "kind": "qr",
                                    "status": "completed",
                                    "account": account,
                                }),
                            ))
                        }
                        None => match export_qr_login_token(&tg, api_id(), API_HASH)
                            .await
                            .map_err(|err| ApiError::bad_request(err.to_string()))?
                        {
                            QrLoginTokenExport::Ready { url, expires_at } => {
                                *guard = AuthFlow::Qr {
                                    common,
                                    tg,
                                    login_url: url.clone(),
                                    expires_at,
                                };
                                Ok((
                                    200,
                                    json!({
                                        "ok": true,
                                        "flow_id": flow_id,
                                        "kind": "qr",
                                        "status": "waiting_for_scan",
                                        "login_url": url,
                                        "expires_at": expires_at,
                                    }),
                                ))
                            }
                            QrLoginTokenExport::Authorized(user) => {
                                drop(guard);
                                state.remove_flow(flow_id).await;
                                let account =
                                    finalize_authenticated_flow(common.temp_name, tg, user)
                                        .await
                                        .map_err(|err| ApiError::internal(err.to_string()))?;
                                Ok((
                                    200,
                                    json!({
                                        "ok": true,
                                        "flow_id": flow_id,
                                        "kind": "qr",
                                        "status": "completed",
                                        "account": account,
                                    }),
                                ))
                            }
                            QrLoginTokenExport::PendingMigration { .. } => {
                                *guard = AuthFlow::Qr {
                                    common,
                                    tg,
                                    login_url: login_url.clone(),
                                    expires_at,
                                };
                                Ok((
                                    200,
                                    json!({
                                        "ok": true,
                                        "flow_id": flow_id,
                                        "kind": "qr",
                                        "status": "waiting_for_scan",
                                        "login_url": login_url,
                                        "expires_at": expires_at,
                                    }),
                                ))
                            }
                        },
                    }
                }
            }
        }
        AuthFlow::Consumed => {
            *guard = AuthFlow::Consumed;
            Err(ApiError::not_found(pick(
                "登录流程不存在",
                "login flow not found",
            )))
        }
    }
}

async fn run_upload(body: &[u8]) -> ApiResult<(u16, Value)> {
    let request: UploadRequest = parse_json_body(body)?;
    let args = build_upload_args(&request);
    let (status, stdout, stderr) = execute_child_command(&args)
        .await
        .map_err(|err| ApiError::internal(err.to_string()))?;
    Ok(child_response(status, stdout, stderr))
}

async fn run_download(body: &[u8]) -> ApiResult<(u16, Value)> {
    let request: DownloadRequest = parse_json_body(body)?;
    let args = build_download_args(&request);
    let (status, stdout, stderr) = execute_child_command(&args)
        .await
        .map_err(|err| ApiError::internal(err.to_string()))?;
    Ok(child_response(status, stdout, stderr))
}

async fn run_forward(body: &[u8]) -> ApiResult<(u16, Value)> {
    let request: ForwardRequest = parse_json_body(body)?;
    let args = build_forward_args(&request);
    let (status, stdout, stderr) = execute_child_command(&args)
        .await
        .map_err(|err| ApiError::internal(err.to_string()))?;
    Ok(child_response(status, stdout, stderr))
}

pub(crate) async fn route_http_request(
    state: Arc<HttpApiState>,
    method: &str,
    path: &str,
    body: &[u8],
) -> (u16, Value) {
    state.cleanup_expired_flows().await;

    let segments = path_segments(path);
    let response = match (method, segments.as_slice()) {
        ("GET", ["health"]) | ("GET", ["v1", "health"]) => Ok((
            200,
            json!({
                "ok": true,
                "service": "tdlr",
                "protocol": "http-json-v2",
            }),
        )),
        ("GET", ["version"]) | ("GET", ["v1", "version"]) => Ok((
            200,
            json!({
                "ok": true,
                "version": env!("TDLR_VERSION"),
                "rustc": env!("RUSTC_VERSION"),
                "target": {
                    "os": std::env::consts::OS,
                    "arch": std::env::consts::ARCH,
                }
            }),
        )),
        ("GET", ["v1", "accounts"]) => list_accounts_payload().map(|payload| (200, payload)),
        ("GET", ["v1", "accounts", "status"]) => {
            account_status_payload().await.map(|payload| (200, payload))
        }
        ("POST", ["v1", "accounts", "active"]) => activate_account(body).await,
        ("POST", ["v1", "accounts", "logout"]) => logout_accounts(body).await,
        ("DELETE", ["v1", "accounts", id]) => match id.parse::<i64>() {
            Ok(user_id) => remove_account(user_id).await,
            Err(_) => Err(ApiError::bad_request(pick(
                "无效的账号 ID",
                "invalid account id",
            ))),
        },
        ("POST", ["v1", "auth", "phone", "start"]) => {
            start_phone_login(Arc::clone(&state), body).await
        }
        ("POST", ["v1", "auth", "phone", "submit-code"]) => {
            submit_phone_code(Arc::clone(&state), body).await
        }
        ("POST", ["v1", "auth", "phone", "submit-password"]) => {
            submit_phone_password(Arc::clone(&state), body).await
        }
        ("POST", ["v1", "auth", "qr", "start"]) => start_qr_login(Arc::clone(&state), body).await,
        ("GET", ["v1", "auth", "flows", flow_id]) => {
            get_flow_status(Arc::clone(&state), flow_id).await
        }
        ("DELETE", ["v1", "auth", "flows", flow_id]) => state
            .cancel_flow(flow_id)
            .await
            .map(|payload| (200, payload)),
        ("POST", ["v1", "uploads"]) => run_upload(body).await,
        ("POST", ["v1", "downloads"]) => run_download(body).await,
        ("POST", ["v1", "forwards"]) => run_forward(body).await,
        _ => Err(ApiError::not_found(pick("未找到", "not found"))),
    };

    match response {
        Ok(value) => value,
        Err(err) => api_error_response(err),
    }
}
