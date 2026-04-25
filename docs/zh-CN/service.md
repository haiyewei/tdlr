# `service` 命令

`service` 用于启动长期驻留服务，支持：

- 面向本地自动化的 `stdio` 模式
- 提供结构化 Web 端点的 HTTP 模式

## 用法

```bash
tdlr service [--json-events] [--http-bind <HOST:PORT>]
```

## 参数

| 参数 | 说明 |
|------|------|
| `--json-events` | 在 `stdio` 模式下输出机器可读事件 |
| `--http-bind <HOST:PORT>` | 启动 HTTP API，而不是 `stdio` 模式 |

## `stdio` 模式

### 启动

```bash
tdlr service
```

### 输入格式

服务逐行读取标准输入，每行一条请求。支持三种格式：

#### 1. 原始命令行

```text
version
```

#### 2. JSON 参数数组

```json
["download", "--url", "https://t.me/telegram/193"]
```

#### 3. JSON 请求对象

```json
{"id":"req-1","command":"forward -f 123 --from-chat @source -t me"}
```

或者：

```json
{"id":"req-2","args":["version"]}
```

### 事件输出

服务会输出带固定前缀的事件行：

```text
@@TDLR_SERVICE@@ {"event":"ready","protocol":"stdio-jsonl-v1"}
```

事件类型：

| 事件 | 说明 |
|------|------|
| `ready` | 服务已启动，可以接收命令 |
| `result` | 某条请求执行完成 |
| `exit` | 服务退出 |

### 行为说明

- 普通命令输出仍然会写到 `stdout`。
- 机器消费方应只解析带事件前缀的行。
- 发送 `exit` 或 `quit` 会结束服务。
- `--json-events` 默认启用。

## HTTP 模式

### 启动

```bash
tdlr service --http-bind 127.0.0.1:8787
```

启动后控制台会输出：

```text
HTTP API listening on http://127.0.0.1:8787
```

### 协议说明

- 当前协议标识：`http-json-v2`
- 目前只支持 `HTTP/1.x`
- 请求体最大 1 MB
- 每个请求完成后会主动关闭连接

### 接口总览

| 方法 | 路径 | 说明 |
|------|------|------|
| `GET` | `/health` | 健康检查 |
| `GET` | `/v1/health` | 健康检查别名 |
| `GET` | `/v1/version` | 版本、编译器和目标平台信息 |
| `GET` | `/v1/accounts` | 列出本地保存的账号 |
| `GET` | `/v1/accounts/status` | 检查每个账号的授权状态 |
| `POST` | `/v1/accounts/active` | 切换当前激活账号 |
| `POST` | `/v1/accounts/logout` | 退出一个账号或全部账号 |
| `DELETE` | `/v1/accounts/{user_id}` | 删除一个本地保存的账号 |
| `POST` | `/v1/auth/phone/start` | 开始手机号登录流程 |
| `POST` | `/v1/auth/phone/resend` | 请求下一个可用的手机号验证码通道 |
| `POST` | `/v1/auth/phone/submit-code` | 提交短信/应用验证码 |
| `POST` | `/v1/auth/phone/submit-password` | 提交两步验证密码 |
| `POST` | `/v1/auth/qr/start` | 开始二维码登录流程 |
| `GET` | `/v1/auth/flows/{flow_id}` | 查询登录流程状态 |
| `DELETE` | `/v1/auth/flows/{flow_id}` | 取消登录流程 |
| `POST` | `/v1/uploads` | 用 JSON 请求体执行上传 |
| `POST` | `/v1/downloads` | 用 JSON 请求体执行下载 |
| `POST` | `/v1/forwards` | 用 JSON 请求体执行转发 |

### `GET /health`

返回示例：

```json
{"ok":true,"service":"tdlr","protocol":"http-json-v2"}
```

### `GET /v1/accounts`

返回示例：

```json
{
  "ok": true,
  "accounts": [
    {
      "user_id": 123456789,
      "display_name": "Alice",
      "username": "alice",
      "active": true
    }
  ]
}
```

### 手机号登录流程

开始流程：

```bash
curl -X POST http://127.0.0.1:8787/v1/auth/phone/start \
  -H "Content-Type: application/json" \
  -d '{"phone":"+8613800138000","code_via":"sms"}'
```

成功返回：

```json
{
  "ok": true,
  "flow_id": "flow-1710000000-1",
  "kind": "phone",
  "status": "waiting_for_code",
  "phone": "+8613800138000",
  "requested_via": "sms",
  "sent_via": "app",
  "next_via": "sms",
  "timeout": 60,
  "remaining_attempts": 3
}
```

说明：

- `code_via` 支持 `auto`、`app`、`sms`。
- `code_via` 只是偏好，不是强制要求；首次验证码通道仍由 Telegram 服务端决定。
- `sent_via`、`next_via` 和 `timeout` 直接反映 Telegram 当前返回的验证码投递状态。

需要时可请求下一个可用通道：

```bash
curl -X POST http://127.0.0.1:8787/v1/auth/phone/resend \
  -H "Content-Type: application/json" \
  -d '{"flow_id":"flow-1710000000-1"}'
```

提交验证码：

```bash
curl -X POST http://127.0.0.1:8787/v1/auth/phone/submit-code \
  -H "Content-Type: application/json" \
  -d '{"flow_id":"flow-1710000000-1","code":"12345"}'
```

如果账号启用了两步验证，返回状态会变成 `waiting_for_password`，此时继续调用：

```bash
curl -X POST http://127.0.0.1:8787/v1/auth/phone/submit-password \
  -H "Content-Type: application/json" \
  -d '{"flow_id":"flow-1710000000-1","password":"your-password"}'
```

### 二维码登录流程

开始流程：

```bash
curl -X POST http://127.0.0.1:8787/v1/auth/qr/start \
  -H "Content-Type: application/json" \
  -d '{}'
```

返回示例：

```json
{
  "ok": true,
  "flow_id": "flow-1710000000-2",
  "kind": "qr",
  "status": "waiting_for_scan",
  "login_url": "tg://login?token=...",
  "expires_at": 1710000032
}
```

轮询状态：

```bash
curl http://127.0.0.1:8787/v1/auth/flows/flow-1710000000-2
```

扫码成功后，返回会变成：

```json
{
  "ok": true,
  "flow_id": "flow-1710000000-2",
  "kind": "qr",
  "status": "completed",
  "account": {
    "user_id": 123456789,
    "display_name": "Alice",
    "username": "alice",
    "active": true
  }
}
```

### 上传 / 下载 / 转发端点

这些端点不再使用命令数组，而是使用结构化 JSON。

上传示例：

```bash
curl -X POST http://127.0.0.1:8787/v1/uploads \
  -H "Content-Type: application/json" \
  -d '{"path":["./videos"],"chat":"me","group":true,"thumb":["./covers"]}'
```

下载示例：

```bash
curl -X POST http://127.0.0.1:8787/v1/downloads \
  -H "Content-Type: application/json" \
  -d '{"url":["https://t.me/telegram/193"],"path":"./downloads"}'
```

转发示例：

```bash
curl -X POST http://127.0.0.1:8787/v1/forwards \
  -H "Content-Type: application/json" \
  -d '{"from":["https://t.me/channel/123"],"to":"me","mode":"smart"}'
```

这些操作型端点当前返回：

- `ok`
- `exit_code`
- `stdout`
- `stderr`

请求字段语义与对应 CLI 命令参数保持一致。

对于 `POST /v1/uploads`，封面行为也与 CLI 保持一致：优先使用显式 `thumb` / `thumb_map`，否则服务会尝试提取视频内嵌封面，最后再回退为无封面上传。

## 限制

- `stdio` 模式仍然不支持嵌套执行 `service`。
- `stdio` 模式仍然不接受 `auth login add`，因为它需要独占 `stdin`。
- HTTP 登录流程状态保存在服务进程内存里，长时间不活动会自动过期。
- 二维码登录是否需要额外验证仍取决于 Telegram 服务端行为；遇到额外校验时，可能需要改走手机号登录。

## 参考

| 文件 | 说明 |
|------|------|
| `src/cli/args/service.rs` | 参数定义 |
| `src/commands/service.rs` | `stdio` 模式和 HTTP 服务入口 |
| `src/commands/service_api.rs` | HTTP 路由、账号端点和登录流程 |
| `src/commands/mod.rs` | 顶层命令分发 |
