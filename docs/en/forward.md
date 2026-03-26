# `forward` Command

`forward` forwards Telegram messages to a target dialog and supports both direct forwarding and clone forwarding.

## Usage

```bash
tdlr forward --from <SOURCE>... [OPTIONS]
```

## Parameters

| Parameter | Description |
|------|------|
| `-f, --from <SOURCE>...` | Source message URL or plain message ID. Required and may be repeated |
| `--from-chat <CHAT>` | Required when `--from` uses plain message IDs |
| `-t, --to <CHAT>` | Destination chat. Defaults to Saved Messages |
| `-m, --mode <MODE>` | Forward mode: `direct`, `clone`, or `smart`. Default is `smart` |
| `--topic <TOPIC>` | Topic ID in the destination forum group |
| `-a, --account <USER_ID>` | Use a specific account |
| `--drop-author` | Only valid in `direct` mode. Re-send without the forwarded header |

## Source input

`--from` supports two kinds of input:

| Type | Example |
|------|------|
| Telegram message URL | `https://t.me/channel/123` |
| Plain message ID | `500` |

If you use plain message IDs:

- `--from-chat` is required
- `--from-chat` can be `me`, a username, or a numeric ID

## Forward modes

| Mode | Description |
|------|------|
| `direct` | Use Telegram's native forward API |
| `clone` | Download the content first and then upload it again to the destination |
| `smart` | Inspect whether the source dialog has `noforwards`, then switch between `direct` and `clone` automatically |

## Behavior

- In `smart` mode, the source dialog is resolved first and the mode is chosen based on `noforwards`.
- `clone` mode works for restricted content because it re-uploads content instead of using the native forward API.
- Media groups in `clone` mode are downloaded first and then re-sent as albums.
- `drop-author` only matters in `direct` mode.

## Supported URL formats

| Type | Example |
|------|------|
| Public message | `https://t.me/username/123` |
| Private message | `https://t.me/c/1234567890/123` |
| Media group / reply | `https://t.me/username/123/456` |
| Comment link | `https://t.me/username/123?comment=456` |
| Topic link | `https://t.me/username/123?thread=456` |

The program prefers the following fields, in order:

- `comment_id`
- `secondary_id`
- `message_id`

as the final message ID to process.

## Examples

```bash
tdlr forward -f https://t.me/channel/123
tdlr forward -f https://t.me/channel/123 -t @backup
tdlr forward -f 500 --from-chat @source_channel -t me
tdlr forward -f https://t.me/restricted_channel/99 -m clone
tdlr forward -f https://t.me/channel/123 -m direct --drop-author
tdlr forward -f https://t.me/channel/123 -t -1001234567890 --topic 5
```

## Reference

| File | Description |
|------|------|
| `src/cli/args/forward.rs` | Argument definitions |
| `src/commands/forward/forward.rs` | Command entry point and `smart` mode dispatch |
| `src/commands/forward/direct.rs` | Native forward implementation |
| `src/commands/forward/clone.rs` | Clone forward implementation |
| `src/utils/link.rs` | Input parsing |
| `src/telegram/upload/chat.rs` | Destination chat resolution |
