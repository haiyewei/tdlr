# `upload` Command

`upload` uploads local files or directories to Telegram.

## Usage

```bash
tdlr upload --path <PATH>... [OPTIONS]
```

## Parameters

| Parameter | Description |
|------|------|
| `-p, --path <PATH>...` | File or directory path. Required and may be repeated |
| `-c, --chat <CHAT>` | Target chat ID or username. Defaults to Saved Messages |
| `-i, --include <EXT>...` | Only include the specified extensions |
| `-e, --exclude <EXT>...` | Exclude the specified extensions |
| `--rm` | Delete source files after a successful upload |
| `--topic <TOPIC>` | Topic ID for forum groups. Requires `--chat` |
| `-a, --account <USER_ID>` | Use a specific account. May be repeated |
| `--all-accounts` | Run the upload once per locally saved account |
| `--caption <HTML>` | File caption sent as raw HTML without template substitution |
| `--thumb <PATH>...` | Thumbnail or cover image file(s) or directories for video uploads |
| `--thumb-map <VIDEO=THUMB>...` | Explicitly bind a video file to a thumbnail image |
| `--to <EXPR>` | Routing expression for the destination. Conflicts with `--chat` and `--topic` |
| `--group` | Send as media groups, up to 10 items. Only supports images and videos |

## Target chat formats

`--chat` supports the following forms:

| Format | Description |
|------|------|
| empty | Saved Messages |
| `me` / `self` | Saved Messages |
| `@username` | User, group, or channel username |
| `username` | Username without `@` |
| numeric ID | Resolved from dialogs for users, groups, or channels |

## Behavior

- If `--path` points to a directory, the command recursively collects files from that directory.
- `--include` and `--exclude` filter files by extension.
- When no account is specified, the current active account is used.
- If multiple `--account` values are provided, the same upload run is executed for each account.
- `--all-accounts` loads all local accounts and uploads with each one in turn.
- `--caption` is sent as raw HTML and does not support template variables.
- `--thumb` only applies to video uploads. Each value may point to an image file or a directory that is scanned recursively for images.
- Thumbnail assignment order is: explicit `--thumb-map`, then unique file-stem match, then remaining thumbnails in input order.
- `--thumb-map` accepts either a full upload path or a unique file name / stem on the left side.
- When a video still has no assigned thumbnail after `--thumb` / `--thumb-map` resolution, `upload` tries to extract embedded cover art automatically.
- Automatic embedded cover extraction currently checks `mp4` / `mov` / `m4v` / `3gp` for `covr` first and then attached-picture tracks, and checks `mkv` / `webm` for image attachments.
- If no supported embedded artwork is found, the upload continues without a thumbnail.
- `--group` only processes images and videos. Unsupported files are skipped and counted as failures.
- `--rm` deletes processed files after the upload workflow finishes.

## Examples

```bash
tdlr upload -p ./file.txt
tdlr upload -p ./photos -c @backup_channel
tdlr upload -p ./media -i jpg,png,mp4
tdlr upload -p ./cache --rm -c me
tdlr upload -p ./videos --group -c -1001234567890
tdlr upload -p ./data -a 123456789 -a 987654321 -c me
tdlr upload -p ./media --all-accounts -c @backup
tdlr upload -p ./video.mp4 --thumb ./video-cover.jpg -c me
tdlr upload -p ./videos --thumb ./covers --group -c @backup_channel
tdlr upload -p ./videos --thumb-map "./videos/a.mp4=./covers/a.jpg" "./videos/b.mp4=./covers/b.jpg" --group -c me
```

## HTTP API

`tdlr service --http-bind ...` exposes a dedicated upload endpoint. The request body uses the same semantics as the CLI flags:

```json
{
  "path": ["./videos"],
  "thumb": ["./covers"],
  "group": true,
  "chat": "me"
}
```

Send it to `POST /v1/uploads`.

`thumb` and `thumb_map` keep the same priority as the CLI. If neither field provides a thumbnail for a video, the HTTP upload endpoint also tries embedded cover extraction before sending the file without a thumbnail.

## Routing expression: `--to`

`--to` evaluates an expression for each file and returns the target chat string.

### Available variables

| Category | Variables |
|------|------|
| File info | `name` `stem` `ext` `mime` `type` |
| Path info | `path` `dir` `depth` |
| File size | `size` `size_kb` `size_mb` `size_gb` `size_str` |
| Date and time | `date` `time` `datetime` `year` `month` `day` `hour` `minute` `weekday` |
| Type checks | `is_image` `is_video` `is_audio` `is_document` `is_archive` `is_text` `is_code` `is_media` |
| Upload context | `index` `num` `total` |
| Constants | `KB` `MB` `GB` |

### Common functions

- `if(cond, a, b)`
- `str::contains(s, sub)`
- `str::starts_with(s, prefix)`
- `str::ends_with(s, suffix)`
- `str::to_lowercase(s)`
- `str::to_uppercase(s)`
- `str::replace(s, from, to)`
- `str::regex_matches(s, pattern)`
- `min(a, b)` / `max(a, b)`
- `floor(x)` / `ceil(x)` / `round(x)`

### Examples

```bash
tdlr upload -p ./media --to 'if(is_video, "@videos", "me")'
tdlr upload -p ./sync --to 'if(size > 100 * MB, "@large_files", "@small_files")'
tdlr upload -p ./album --to 'if(dir == "photos", "@photos", "@other")'
tdlr upload -p ./media --to 'if(is_video, "-1001111111111", if(is_image, "-1002222222222", "me"))'
```

## Reference

| File | Description |
|------|------|
| `src/cli/args/upload.rs` | Argument definitions |
| `src/commands/upload/upload.rs` | Command entry point |
| `src/commands/upload/file.rs` | File collection and recursive scanning |
| `src/commands/upload/handler.rs` | Upload execution and statistics |
| `src/commands/upload/thumbnail.rs` | Thumbnail collection and video-to-thumbnail assignment |
| `src/commands/upload/expr.rs` | Routing expression implementation |
| `src/telegram/upload/chat.rs` | Destination chat resolution |
| `src/telegram/upload/embedded_thumbnail.rs` | Embedded cover extraction and temporary thumbnail preparation |
| `src/telegram/upload/group.rs` | Media group upload |
| `src/telegram/upload/single.rs` | Single-file upload |
