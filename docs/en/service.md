# `service` Command

`service` starts a long-running process. It currently supports two modes:

- `stdio` mode
- HTTP API mode

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

Event prefix:

```text
@@TDLR_SERVICE@@
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
- `--json-events` is currently enabled by default.

## HTTP mode

### Start

```bash
tdlr service --http-bind 127.0.0.1:8787
```

After startup, the console prints:

```text
HTTP API listening on http://127.0.0.1:8787
```

### Endpoints

| Method | Path | Description |
|------|------|------|
| `GET` | `/health` | Health check |
| `GET` | `/v1/health` | Health check alias |
| `POST` | `/execute` | Execute a command |
| `POST` | `/v1/execute` | Execute a command alias |

### `GET /health`

Example response:

```json
{"ok":true,"service":"tdlr","protocol":"http-json-v1"}
```

### `POST /execute`

The request body supports:

- Plain-text command lines
- JSON argument arrays
- JSON request objects

Request examples:

```json
{"id":"req-1","args":["version"]}
```

```json
{"id":"req-2","command":"download --url https://t.me/telegram/193 --path ./downloads"}
```

Plain-text example:

```text
version
```

Successful response example:

```json
{
  "ok": true,
  "id": "req-1",
  "exit_code": 0,
  "stdout": "Version: ...\n",
  "stderr": ""
}
```

Failure response example:

```json
{
  "ok": false,
  "id": "req-2",
  "exit_code": 1,
  "stdout": "",
  "stderr": "Error: ...\n"
}
```

### HTTP behavior

- Only `HTTP/1.x` is supported at the moment.
- Request bodies are limited to 1 MB.
- Connections are closed after each request. Keep-alive is not used.
- HTTP mode runs commands through a child process of the current binary and captures `stdout` and `stderr`.

## Limitations

The following limitations apply to both service modes:

- Nested `service` execution is not supported
- `auth login add` is not supported

Reasons:

- Recursive `service` invocation is not meaningful
- `auth login add` needs exclusive `stdin` access for phone, code, password, or QR interaction

Additional limitation:

- HTTP mode does not support `exit` or `quit`

## Examples

### `stdio` mode

```text
version
{"id":"req-1","args":["download","--url","https://t.me/telegram/193"]}
exit
```

### HTTP mode

```bash
curl http://127.0.0.1:8787/health
```

```bash
curl -X POST http://127.0.0.1:8787/execute \
  -H "Content-Type: application/json" \
  -d '{"id":"req-1","args":["version"]}'
```

## Reference

| File | Description |
|------|------|
| `src/cli/args/service.rs` | Argument definitions |
| `src/commands/service.rs` | `stdio` and HTTP mode implementation |
| `src/commands/mod.rs` | Top-level command dispatch |
