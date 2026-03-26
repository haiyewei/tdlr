# `service` 命令

`service` 用于启动长期驻留服务，目前支持两种模式：

- `stdio` 模式
- HTTP API 模式

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

事件前缀：

```text
@@TDLR_SERVICE@@
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
- `--json-events` 当前默认启用。

## HTTP 模式

### 启动

```bash
tdlr service --http-bind 127.0.0.1:8787
```

启动后控制台会输出：

```text
HTTP API listening on http://127.0.0.1:8787
```

### 接口

| 方法 | 路径 | 说明 |
|------|------|------|
| `GET` | `/health` | 健康检查 |
| `GET` | `/v1/health` | 健康检查别名 |
| `POST` | `/execute` | 执行命令 |
| `POST` | `/v1/execute` | 执行命令别名 |

### `GET /health`

返回示例：

```json
{"ok":true,"service":"tdlr","protocol":"http-json-v1"}
```

### `POST /execute`

请求体支持：

- 纯文本命令行
- JSON 参数数组
- JSON 请求对象

请求示例：

```json
{"id":"req-1","args":["version"]}
```

```json
{"id":"req-2","command":"download --url https://t.me/telegram/193 --path ./downloads"}
```

纯文本示例：

```text
version
```

响应示例：

```json
{
  "ok": true,
  "id": "req-1",
  "exit_code": 0,
  "stdout": "Version: ...\n",
  "stderr": ""
}
```

失败响应示例：

```json
{
  "ok": false,
  "id": "req-2",
  "exit_code": 1,
  "stdout": "",
  "stderr": "Error: ...\n"
}
```

### HTTP 行为说明

- 当前只支持 `HTTP/1.x`。
- 请求体最大 1 MB。
- 每个连接处理完成后会主动关闭，不保持长连接。
- HTTP 模式内部通过当前二进制的子进程执行命令，并收集 `stdout` / `stderr`。

## 限制

以下限制同时适用于两种服务模式：

- 不支持嵌套执行 `service`
- 不支持 `auth login add`

原因：

- `service` 递归调用没有意义
- `auth login add` 需要独占标准输入进行手机号、验证码或二维码交互

额外限制：

- HTTP 模式不支持 `exit` 或 `quit`

## 示例

### `stdio` 模式

```text
version
{"id":"req-1","args":["download","--url","https://t.me/telegram/193"]}
exit
```

### HTTP 模式

```bash
curl http://127.0.0.1:8787/health
```

```bash
curl -X POST http://127.0.0.1:8787/execute \
  -H "Content-Type: application/json" \
  -d '{"id":"req-1","args":["version"]}'
```

## 参考

| 文件 | 说明 |
|------|------|
| `src/cli/args/service.rs` | 参数定义 |
| `src/commands/service.rs` | `stdio` 和 HTTP 模式实现 |
| `src/commands/mod.rs` | 顶层命令分发 |
