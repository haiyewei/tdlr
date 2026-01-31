# Download Command

Download files from Telegram message URLs.

## Usage

```bash
tdlr download <URLs...> [OPTIONS]
```

## Supported URL Formats

| Format | Example | Description |
|--------|---------|-------------|
| Public channel/group | `https://t.me/telegram/193` | Public channel or group message |
| Private channel | `https://t.me/c/1697797156/151` | Private channel by ID |
| Media group (public) | `https://t.me/iFreeKnow/45662/55005` | Reply or media group in public chat |
| Media group (private) | `https://t.me/c/1492447836/251015/251021` | Reply or media group in private channel |
| Comment | `https://t.me/opencfdchannel/4434?comment=360409` | Comment on a channel post |
| Forum thread | `https://t.me/myhostloc/1485524?thread=1485523` | Message in a forum topic |

## Options

| Option | Short | Description |
|--------|-------|-------------|
| `--output <DIR>` | `-o` | Output directory (default: current directory) |
| `--account <ID>` | `-a` | Account user ID to use (default: active account) |

## Examples

### Download single message
```bash
tdlr download "https://t.me/telegram/193"
```

### Download multiple messages
```bash
tdlr download "https://t.me/telegram/193" "https://t.me/telegram/194"
```

### Download to specific directory
```bash
tdlr download -o ./downloads "https://t.me/telegram/193"
```

### Download from private channel
```bash
tdlr download "https://t.me/c/1697797156/151"
```

### Use specific account
```bash
tdlr download -a 123456789 "https://t.me/telegram/193"
```

## Notes

- You must be logged in and have access to the chat to download from it
- For private channels, you must be a member of the channel
- Photos are saved as `.jpg` files
- Documents retain their original filename
- Comment downloads are not yet implemented - use direct message links instead
