# `upload` 命令

`upload` 用于将本地文件或目录上传到 Telegram。

## 用法

```bash
tdlr upload --path <PATH>... [OPTIONS]
```

## 参数

| 参数 | 说明 |
|------|------|
| `-p, --path <PATH>...` | 文件或目录路径，必填，可多个 |
| `-c, --chat <CHAT>` | 目标聊天 ID 或用户名，默认 Saved Messages |
| `-i, --include <EXT>...` | 仅包含指定扩展名 |
| `-e, --exclude <EXT>...` | 排除指定扩展名 |
| `--rm` | 上传成功后删除源文件 |
| `--topic <TOPIC>` | 论坛群组的话题 ID，要求同时传 `--chat` |
| `-a, --account <USER_ID>` | 使用指定账号，可重复传多次 |
| `--all-accounts` | 使用所有账号依次执行上传 |
| `--caption <HTML>` | 文件说明，直接按 HTML 发送，不做模板替换 |
| `--thumb <PATH>...` | 视频上传时使用的封面图片文件或目录 |
| `--thumb-map <VIDEO=THUMB>...` | 显式指定某个视频对应哪个封面图 |
| `--to <EXPR>` | 目标路由表达式，和 `--chat` / `--topic` 冲突 |
| `--group` | 以媒体组发送，最多 10 个，仅支持图片和视频 |

## 目标聊天格式

`--chat` 支持以下形式：

| 格式 | 说明 |
|------|------|
| 空值 | Saved Messages |
| `me` / `self` | Saved Messages |
| `@username` | 用户、群组或频道用户名 |
| `username` | 不带 `@` 的用户名 |
| 数字 ID | 会自动解析对话列表中的用户、群组或频道 |

## 行为说明

- 如果 `--path` 指向目录，程序会递归收集目录中的文件。
- `--include` 和 `--exclude` 按扩展名过滤文件。
- 未显式指定账号时，使用当前激活账号。
- 指定多个 `--account` 时，会对每个账号分别执行同一轮上传。
- `--all-accounts` 会加载本地所有账号并依次上传。
- `--caption` 当前是原样 HTML，不支持模板变量替换。
- `--thumb` 只对视频上传生效；每个值都可以是图片文件，也可以是会被递归扫描的目录。
- 封面分配顺序是：显式 `--thumb-map`，其次按文件 stem 唯一匹配，最后按剩余输入顺序兜底。
- `--thumb-map` 左侧既可以写完整视频路径，也可以写唯一的文件名或 stem。
- `--group` 只会处理图片和视频；不支持媒体组的文件会被跳过并计入失败统计。
- `--rm` 会在全部上传流程结束后删除已处理文件。

## 示例

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

`tdlr service --http-bind ...` 会暴露专用上传端点，请求体直接使用与 CLI 对应的字段语义：

```json
{
  "path": ["./videos"],
  "thumb": ["./covers"],
  "group": true,
  "chat": "me"
}
```

把这个 JSON 发送到 `POST /v1/uploads` 即可。

## 路由表达式 `--to`

`--to` 使用表达式计算每个文件的目标聊天字符串。

### 可用变量

| 类别 | 变量 |
|------|------|
| 文件信息 | `name` `stem` `ext` `mime` `type` |
| 路径信息 | `path` `dir` `depth` |
| 文件大小 | `size` `size_kb` `size_mb` `size_gb` `size_str` |
| 日期时间 | `date` `time` `datetime` `year` `month` `day` `hour` `minute` `weekday` |
| 类型判断 | `is_image` `is_video` `is_audio` `is_document` `is_archive` `is_text` `is_code` `is_media` |
| 上传上下文 | `index` `num` `total` |
| 常量 | `KB` `MB` `GB` |

### 常用函数

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

### 示例

```bash
tdlr upload -p ./media --to 'if(is_video, "@videos", "me")'
tdlr upload -p ./sync --to 'if(size > 100 * MB, "@large_files", "@small_files")'
tdlr upload -p ./album --to 'if(dir == "photos", "@photos", "@other")'
tdlr upload -p ./media --to 'if(is_video, "-1001111111111", if(is_image, "-1002222222222", "me"))'
```

## 参考

| 文件 | 说明 |
|------|------|
| `src/cli/args/upload.rs` | 参数定义 |
| `src/commands/upload/upload.rs` | 命令入口 |
| `src/commands/upload/file.rs` | 文件收集和递归扫描 |
| `src/commands/upload/handler.rs` | 上传执行和统计 |
| `src/commands/upload/thumbnail.rs` | 封面文件收集和视频封面分配 |
| `src/commands/upload/expr.rs` | 路由表达式实现 |
| `src/telegram/upload/chat.rs` | 目标聊天解析 |
| `src/telegram/upload/group.rs` | 媒体组上传 |
| `src/telegram/upload/single.rs` | 单文件上传 |
