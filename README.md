# TDLR

[简体中文](./README.zh-CN.md)

`tdlr` is a modular Rust CLI for Telegram workflows. It supports account management, file upload, message download, message forwarding, and long-running service mode over `stdio` or HTTP.

## Features

- Manage multiple Telegram accounts and switch the active account locally.
- Upload files or directories with extension filters, routing expressions, captions, media groups, and automatic embedded video cover fallback.
- Download files or text from Telegram message links with filename templates.
- Forward messages in `direct`, `clone`, or `smart` mode.
- Run `tdlr` as a persistent service and send commands over `stdio` or an HTTP API.
- Install into user-owned directories with the merged scripts in [`scripts/`](./scripts/).

## Documentation

| Topic | English | 中文 |
|------|------|------|
| Home | [README](./README.md) | [README.zh-CN](./README.zh-CN.md) |
| Documentation index | [docs/README.md](./docs/README.md) | [docs/README.md](./docs/README.md) |
| Command list | [docs/en/command-list.md](./docs/en/command-list.md) | [docs/zh-CN/command-list.md](./docs/zh-CN/command-list.md) |
| Install | [docs/en/install.md](./docs/en/install.md) | [docs/zh-CN/install.md](./docs/zh-CN/install.md) |
| Android | [docs/en/android.md](./docs/en/android.md) | [docs/zh-CN/android.md](./docs/zh-CN/android.md) |

## Commands

| Command | Description |
|------|------|
| [`version`](./docs/en/version.md) | Print build, compiler, and target information |
| [`auth`](./docs/en/auth.md) | Manage Telegram logins, active account, and account status |
| [`upload`](./docs/en/upload.md) | Upload local files or directories to Telegram |
| [`download`](./docs/en/download.md) | Download files or text from Telegram message links |
| [`forward`](./docs/en/forward.md) | Forward messages, including clone mode for restricted chats |
| [`service`](./docs/en/service.md) | Start persistent `stdio` or HTTP API service mode |

## Quick Start

### 1. Install

Linux / macOS:

```bash
curl -sSL https://raw.githubusercontent.com/haiyewei/tdlr/main/scripts/install.sh | bash
```

Windows PowerShell:

```powershell
irm https://raw.githubusercontent.com/haiyewei/tdlr/main/scripts/install.ps1 | iex
```

The installer writes `tdlr` into the current shell user's directory and updates that user's `PATH`. If the install shell runs as `root`, it installs into `root`'s user directory.

Full install, uninstall, proxy, and source-build instructions: [docs/en/install.md](./docs/en/install.md)

### 2. Verify the CLI

```bash
tdlr version
```

### 3. First login

```bash
tdlr auth login add
tdlr auth login list
```

### 4. Common examples

```bash
tdlr upload -p ./media -c me
tdlr download -u "https://t.me/telegram/193" -p ./downloads
tdlr forward -f https://t.me/channel/123 -t @backup
tdlr service --http-bind 127.0.0.1:8787
```

If a video upload does not specify `--thumb` or `--thumb-map`, `tdlr` now tries to reuse embedded cover art from supported containers before falling back to no thumbnail.

## Build From Source

Building from source is mainly for development, local patching, or custom builds.

```bash
export TG_API_ID=123456
export TG_API_HASH=your_api_hash
cargo build --release
```

Windows PowerShell:

```powershell
$env:TG_API_ID = "123456"
$env:TG_API_HASH = "your_api_hash"
cargo build --release
```

## Service Mode

`tdlr service` supports:

- `stdio` command execution for local automation.
- typed HTTP endpoints for accounts, login flows, uploads, downloads, forwards, and health checks.

See [docs/en/service.md](./docs/en/service.md) for request formats, response formats, and limitations.
