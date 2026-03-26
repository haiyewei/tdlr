# `download` Command

`download` downloads files or text content from Telegram message links.

## Usage

```bash
tdlr download --url <URL>... [OPTIONS]
```

## Parameters

| Parameter | Description |
|------|------|
| `-u, --url <URL>...` | Telegram message links. Required and may be repeated |
| `-p, --path <PATH>` | Output directory. Defaults to the current directory |
| `-i, --include <EXT>...` | Only include the specified extensions |
| `-e, --exclude <EXT>...` | Exclude the specified extensions |
| `-t, --template <TEMPLATE>` | Output filename template |
| `-a, --account <USER_ID>` | Use a specific account |

## Supported link formats

| Type | Example |
|------|------|
| Public channel or group message | `https://t.me/telegram/193` |
| Private channel or group message | `https://t.me/c/1697797156/151` |
| Public media group or reply | `https://t.me/username/123/456` |
| Private media group or reply | `https://t.me/c/1234567890/123/456` |
| Comment link | `https://t.me/username/123?comment=456` |
| Topic link | `https://t.me/username/123?thread=456` |

## Behavior

- The command parses all URLs first and then downloads them one by one.
- When no account is specified, the current active account is used.
- The output directory is created automatically if it does not exist.
- Images are saved as `.jpg` by default.
- Documents keep their original filename whenever possible.
- Text messages are saved as `.txt` files.
- If the rendered template has no extension, the original extension is appended automatically.

## Current limitations

- Comment links are not supported for download yet and fail immediately.
- Plain message IDs are not supported yet. A full URL is required.

## Examples

```bash
tdlr download -u "https://t.me/telegram/193"
tdlr download -u "https://t.me/telegram/193" "https://t.me/telegram/194" -p ./downloads
tdlr download -u "https://t.me/c/1697797156/151" -a 123456789
tdlr download -u "https://t.me/telegram/193" -i jpg,png
tdlr download -u "https://t.me/telegram/193" -t "{{ .DialogID }}_{{ .MessageID }}_{{ filenamify .FileName }}"
```

## Filename template

Default template:

```text
{{ .DialogID }}_{{ .MessageID }}_{{ filenamify .FileName }}
```

### Available variables

| Variable | Description |
|------|------|
| `DialogID` | Dialog ID |
| `MessageID` | Message ID |
| `MessageDate` | Message timestamp |
| `FileName` | Original filename |
| `FileCaption` | Message text or caption |
| `FileSize` | Human-readable file size |
| `DownloadDate` | Download timestamp |

### Available functions

| Function | Description |
|------|------|
| `upper STRING` | Convert to uppercase |
| `lower STRING` | Convert to lowercase |
| `snakecase STRING` | Convert to snake_case |
| `camelcase STRING` | Convert to camelCase |
| `kebabcase STRING` | Convert to kebab-case |
| `replace STRING FROM TO ...` | Replace substrings in pairs |
| `repeat STRING N` | Repeat a string |
| `rand MIN MAX` | Generate a random number |
| `now` | Current timestamp |
| `formatDate TIMESTAMP [FORMAT]` | Format a timestamp |
| `filenamify STRING [MAX_LEN]` | Sanitize invalid filename characters |

### Examples

```bash
tdlr download -u "https://t.me/telegram/193" -t "{{ upper .FileName }}"
tdlr download -u "https://t.me/telegram/193" -t "{{ formatDate .DownloadDate 2006-01-02 }}_{{ filenamify .FileName }}"
tdlr download -u "https://t.me/telegram/193" -t "{{ lower (replace .FileName \" \" \"_\") }}"
```

Notes:

- `formatDate` uses Go-style time layouts such as `20060102150405`.
- `filenamify` replaces invalid characters and truncates when needed.

## Reference

| File | Description |
|------|------|
| `src/cli/args/download.rs` | Argument definitions |
| `src/commands/download/download.rs` | Command entry point |
| `src/commands/download/handler.rs` | Download execution and statistics |
| `src/commands/download/template.rs` | Filename template implementation |
| `src/utils/link.rs` | Telegram link parsing |
| `src/telegram/download/message.rs` | Message fetch |
| `src/telegram/download/file.rs` | File download |
