# Command List

This document gives an overview of the current `tdlr` command tree and links to the dedicated documentation for each command.

## Top-level commands

```text
tdlr
├── version
├── auth
│   ├── login
│   │   ├── add
│   │   ├── list
│   │   ├── remove
│   │   └── use
│   ├── logout
│   └── status
├── upload
├── download
├── forward
└── service
```

## Top-level overview

| Command | Description |
|------|------|
| [`version`](./version.md) | Print version, Rust compiler, and current target platform |
| [`auth`](./auth.md) | Manage Telegram login accounts, active account, and account status |
| [`upload`](./upload.md) | Upload local files or directories to Telegram |
| [`download`](./download.md) | Download files or text from Telegram message links |
| [`forward`](./forward.md) | Forward Telegram messages to another chat, including clone mode |
| [`service`](./service.md) | Start a long-running service with `stdio` or HTTP API mode |

## `auth` subcommands

| Command | Description |
|------|------|
| [`auth login add`](./auth.md#tdlr-auth-login-add) | Add a new account with phone or QR login |
| [`auth login list`](./auth.md#tdlr-auth-login-list) | List saved login accounts |
| [`auth login remove`](./auth.md#tdlr-auth-login-remove) | Remove an account by user ID |
| [`auth login use`](./auth.md#tdlr-auth-login-use) | Switch the active account |
| [`auth logout`](./auth.md#tdlr-auth-logout) | Log out the current account, a specific account, or all accounts |
| [`auth status`](./auth.md#tdlr-auth-status) | Check authorization status for all accounts in parallel |

## Related documents

| Document | Description |
|------|------|
| [version](./version.md) | Full `version` command reference |
| [auth](./auth.md) | Full `auth` command reference |
| [upload](./upload.md) | Full `upload` command reference |
| [download](./download.md) | Full `download` command reference |
| [forward](./forward.md) | Full `forward` command reference |
| [service](./service.md) | Full `service` command reference |
| [install](./install.md) | Install scripts, install modes, and user-level paths |
| [Android integration](./android.md) | Android JNI integration guide |
