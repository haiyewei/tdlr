# `codes` Command

`codes` shows the latest messages from Telegram's `Verification Codes` dialog for the selected account.

## Usage

```bash
tdlr codes [--limit <N>] [--account <USER_ID>]
```

## Parameters

| Parameter | Description |
|------|------|
| `-n, --limit <N>` | Number of recent messages to print. Default: `10` |
| `--account <USER_ID>` | Account user ID to use. Defaults to the active account |

## Dialog selection

`tdlr codes` does not hardcode a single dialog title. It tries these candidates in order:

1. Official Telegram service user `777000`
2. Compatibility fallback user `42777`
3. A dialog named `Verification Codes`
4. A dialog named `Telegram`

The first best match from the current dialog list is used.

## Output

For each message, the command prints:

- Local timestamp
- Telegram message ID
- Message body text

If a message has no text, the command prints a placeholder such as `[media message]` or `[service message]`.

## Notes

- The account must already be logged in and authorized.
- The command only reads existing dialog history; it does not start a new login flow.
- The `Verification Codes` dialog content depends on Telegram server behavior and the account's recent login activity.

## Examples

```bash
tdlr codes
tdlr codes --limit 5
tdlr codes --account 123456789 --limit 20
```

## Reference

| File | Description |
|------|------|
| `src/cli/args/codes.rs` | CLI argument definitions for `codes` |
| `src/commands/codes.rs` | Dialog lookup and message printing |
