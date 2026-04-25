# `auth` Command

`auth` manages Telegram account login state, the currently active account, and account checks.

## Command tree

```text
tdlr auth
├── login
│   ├── add
│   ├── list
│   ├── remove
│   └── use
├── logout
└── status
```

## `tdlr auth`

Usage:

```bash
tdlr auth <COMMAND>
```

Subcommands:

| Subcommand | Description |
|------|------|
| `login add` | Add a new account |
| `login list` | List logged-in accounts |
| `login remove` | Remove a specific account |
| `login use` | Switch the active account |
| `logout` | Log out account(s) |
| `status` | Check account status |

## `tdlr auth login add`

Add a new Telegram account.

Usage:

```bash
tdlr auth login add [--name <NAME>] [--method <phone|qr>] [--code-via <auto|app|sms>]
```

Parameters:

| Parameter | Description |
|------|------|
| `-n, --name <NAME>` | Account alias for local display only |
| `-m, --method <METHOD>` | Login method. Supports `phone` and `qr`, default is `qr` |
| `--code-via <MODE>` | Preferred phone verification code channel. Supports `auto`, `app`, and `sms`, default is `auto` |

Login methods:

| Method | Description |
|------|------|
| `qr` | Print a QR code and log in from the Telegram app |
| `phone` | Interactively request phone number, login code, and 2FA password if enabled |

Behavior:

- The login flow starts with a temporary session.
- For phone login, `--code-via app` and `--code-via sms` are preferences, not a hard override. Telegram still decides the actual initial channel.
- If `--code-via sms` is requested and Telegram exposes SMS as the next available channel, the CLI waits for the reported timeout and tries `auth.resendCode` automatically.
- The phone login flow prints the actual delivery channel, the next available channel, and the timeout returned by Telegram.
- After a successful login, the session is renamed to the final session based on the user ID.
- Account metadata is written to the local account store.
- The newly logged-in account becomes the active account automatically.
- `--name` is currently retained as a display parameter, but the final display name still comes from Telegram user info.

Examples:

```bash
tdlr auth login add
tdlr auth login add --method phone
tdlr auth login add --method phone --code-via app
tdlr auth login add --method phone --code-via sms
tdlr auth login add --name work --method qr
```

## `tdlr auth login list`

List locally saved accounts.

Usage:

```bash
tdlr auth login list
```

Output:

- User ID
- Display name
- Username, if present
- Whether the account is currently active

Notes:

- If there are no accounts, the command tells you to add one with `tdlr auth login add`.

Example:

```bash
tdlr auth login list
```

## `tdlr auth login remove`

Remove an account by user ID.

Usage:

```bash
tdlr auth login remove <ID>
```

Parameters:

| Parameter | Description |
|------|------|
| `<ID>` | Telegram user ID to remove |

Behavior:

- Removes the session and account metadata for the specified account.
- If the removed account is the active account, the active marker is cleared.
- If other accounts still exist locally, the first available account is selected automatically.

Example:

```bash
tdlr auth login remove 123456789
```

## `tdlr auth login use`

Switch the active account.

Usage:

```bash
tdlr auth login use <ID>
```

Parameters:

| Parameter | Description |
|------|------|
| `<ID>` | Telegram user ID to activate |

Behavior:

- This command only switches the current active account and does not create a new one.
- After switching, commands without an explicit `--account` use this account by default.

Example:

```bash
tdlr auth login use 123456789
```

## `tdlr auth logout`

Log out account(s).

Usage:

```bash
tdlr auth logout [--id <ID>] [--all]
```

Parameters:

| Parameter | Description |
|------|------|
| `-i, --id <ID>` | Log out a specific account. If omitted, log out the current active account |
| `--all` | Log out all accounts |

Behavior:

- `--all` removes all accounts and sessions and clears the active account.
- If neither `--id` nor `--all` is provided, the current active account is logged out.
- If the active account is logged out and other accounts still exist locally, the first available account is selected automatically.

Examples:

```bash
tdlr auth logout
tdlr auth logout --id 123456789
tdlr auth logout --all
```

## `tdlr auth status`

Check authorization state for all saved accounts in parallel.

Usage:

```bash
tdlr auth status
```

Output:

- User ID
- Whether the account is authorized
- User info or error info

Behavior:

- The command tries to create a client for every local account.
- Authorization checks and current user lookups run in parallel.
- If an account fails to load, the error is printed directly in the output.

Example:

```bash
tdlr auth status
```

## Local data location

Account data is managed by the session manager and is stored in the user's configuration directory under `tdlr`, including:

- `sessions/`
- `accounts.json`
- `.active`

## Reference

| File | Description |
|------|------|
| `src/cli/args/auth.rs` | `auth` argument definitions |
| `src/commands/auth/login/add.rs` | Add-account implementation |
| `src/telegram/auth/phone.rs` | Interactive phone login flow |
| `src/telegram/client/instance.rs` | Raw phone auth requests, resend, and sign-in completion |
| `src/commands/auth/login/list.rs` | List-account implementation |
| `src/commands/auth/login/remove.rs` | Remove-account implementation |
| `src/commands/auth/login/use_account.rs` | Switch-active-account implementation |
| `src/commands/auth/logout.rs` | Logout implementation |
| `src/commands/auth/status.rs` | Status check implementation |
| `src/telegram/session/manager.rs` | Session path and active-account management |
