# `service` Command

`service` starts a long-running process. It supports:

- `stdio` mode for local automation
- HTTP mode with structured Web endpoints

## Usage

```bash
tdlr service [--json-events] [--http-bind <HOST:PORT>]
```

## Parameters

| Parameter | Description |
|------|------|
| `--json-events` | Emit machine-readable events in `stdio` mode |
| `--http-bind <HOST:PORT>` | Start the HTTP API instead of `stdio` mode |

## `stdio` mode

### Start

```bash
tdlr service
```

### Input format

The service reads one request per input line from standard input. It supports three formats:

#### 1. Raw command line

```text
version
```

#### 2. JSON argument array

```json
["download", "--url", "https://t.me/telegram/193"]
```

#### 3. JSON request object

```json
{"id":"req-1","command":"forward -f 123 --from-chat @source -t me"}
```

or:

```json
{"id":"req-2","args":["version"]}
```

### Event output

The service emits event lines with a fixed prefix:

```text
@@TDLR_SERVICE@@ {"event":"ready","protocol":"stdio-jsonl-v1"}
```

Event types:

| Event | Description |
|------|------|
| `ready` | Service started and ready to receive commands |
| `result` | A request finished executing |
| `exit` | Service is shutting down |

### Behavior

- Normal command output still goes to `stdout`.
- Machine consumers should only parse lines with the event prefix.
- Sending `exit` or `quit` stops the service.
- `--json-events` is enabled by default.

## HTTP mode

### Start

```bash
tdlr service --http-bind 127.0.0.1:8787
```

After startup, the console prints:

```text
HTTP API listening on http://127.0.0.1:8787
```

### Protocol

- Current protocol string: `http-json-v2`
- Only `HTTP/1.x` is supported
- Request bodies are limited to 1 MB
- Connections are closed after each request

### Endpoint summary

| Method | Path | Description |
|------|------|------|
| `GET` | `/health` | Health check |
| `GET` | `/v1/health` | Health check alias |
| `GET` | `/v1/version` | Build and target information |
| `GET` | `/v1/accounts` | List saved accounts |
| `GET` | `/v1/accounts/status` | Check authorization status for each account |
| `POST` | `/v1/accounts/active` | Switch active account |
| `POST` | `/v1/accounts/logout` | Logout one account or all accounts |
| `DELETE` | `/v1/accounts/{user_id}` | Remove one saved account |
| `POST` | `/v1/auth/phone/start` | Start phone login flow |
| `POST` | `/v1/auth/phone/submit-code` | Submit login code |
| `POST` | `/v1/auth/phone/submit-password` | Submit 2FA password |
| `POST` | `/v1/auth/qr/start` | Start QR login flow |
| `GET` | `/v1/auth/flows/{flow_id}` | Poll login flow state |
| `DELETE` | `/v1/auth/flows/{flow_id}` | Cancel login flow |
| `POST` | `/v1/uploads` | Run upload with JSON request body |
| `POST` | `/v1/downloads` | Run download with JSON request body |
| `POST` | `/v1/forwards` | Run forward with JSON request body |

### `GET /health`

Example response:

```json
{"ok":true,"service":"tdlr","protocol":"http-json-v2"}
```

### `GET /v1/accounts`

Example response:

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

### Phone login flow

Start the flow:

```bash
curl -X POST http://127.0.0.1:8787/v1/auth/phone/start \
  -H "Content-Type: application/json" \
  -d '{"phone":"+8613800138000"}'
```

Successful response:

```json
{
  "ok": true,
  "flow_id": "flow-1710000000-1",
  "kind": "phone",
  "status": "waiting_for_code",
  "phone": "+8613800138000"
}
```

Submit the verification code:

```bash
curl -X POST http://127.0.0.1:8787/v1/auth/phone/submit-code \
  -H "Content-Type: application/json" \
  -d '{"flow_id":"flow-1710000000-1","code":"12345"}'
```

If the account requires 2FA, the response changes to `waiting_for_password`, and you then call:

```bash
curl -X POST http://127.0.0.1:8787/v1/auth/phone/submit-password \
  -H "Content-Type: application/json" \
  -d '{"flow_id":"flow-1710000000-1","password":"your-password"}'
```

### QR login flow

Start the flow:

```bash
curl -X POST http://127.0.0.1:8787/v1/auth/qr/start \
  -H "Content-Type: application/json" \
  -d '{}'
```

Example response:

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

Poll the flow:

```bash
curl http://127.0.0.1:8787/v1/auth/flows/flow-1710000000-2
```

When the scan completes successfully, the response changes to:

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

### Upload / download / forward endpoints

These endpoints use structured JSON instead of command arrays.

Upload example:

```bash
curl -X POST http://127.0.0.1:8787/v1/uploads \
  -H "Content-Type: application/json" \
  -d '{"path":["./videos"],"chat":"me","group":true,"thumb":["./covers"]}'
```

Download example:

```bash
curl -X POST http://127.0.0.1:8787/v1/downloads \
  -H "Content-Type: application/json" \
  -d '{"url":["https://t.me/telegram/193"],"path":"./downloads"}'
```

Forward example:

```bash
curl -X POST http://127.0.0.1:8787/v1/forwards \
  -H "Content-Type: application/json" \
  -d '{"from":["https://t.me/channel/123"],"to":"me","mode":"smart"}'
```

These operation endpoints currently return:

- `ok`
- `exit_code`
- `stdout`
- `stderr`

The request fields follow the same semantics as the CLI flags for the corresponding command.

## Limitations

- Nested `service` execution is not supported in `stdio` mode.
- `auth login add` is still not accepted through `stdio` mode because it requires exclusive `stdin`.
- Login flows in HTTP mode are stored in the service process memory and expire automatically after inactivity.
- QR login still depends on Telegram server behavior; if Telegram requires extra verification, the flow may need to fall back to phone login.

## Reference

| File | Description |
|------|------|
| `src/cli/args/service.rs` | Argument definitions |
| `src/commands/service.rs` | `stdio` mode and HTTP server entry |
| `src/commands/service_api.rs` | HTTP routing, account endpoints, and login flows |
| `src/commands/mod.rs` | Top-level command dispatch |
