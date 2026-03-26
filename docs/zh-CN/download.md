# `download` 命令

`download` 用于从 Telegram 消息链接下载文件或文本内容。

## 用法

```bash
tdlr download --url <URL>... [OPTIONS]
```

## 参数

| 参数 | 说明 |
|------|------|
| `-u, --url <URL>...` | Telegram 消息链接，必填，可多个 |
| `-p, --path <PATH>` | 输出目录，默认当前目录 |
| `-i, --include <EXT>...` | 仅包含指定扩展名 |
| `-e, --exclude <EXT>...` | 排除指定扩展名 |
| `-t, --template <TEMPLATE>` | 输出文件名模板 |
| `-a, --account <USER_ID>` | 使用指定账号 |

## 支持的链接格式

| 类型 | 示例 |
|------|------|
| 公开频道/群组消息 | `https://t.me/telegram/193` |
| 私有频道/群组消息 | `https://t.me/c/1697797156/151` |
| 公开媒体组或回复 | `https://t.me/username/123/456` |
| 私有媒体组或回复 | `https://t.me/c/1234567890/123/456` |
| 评论链接 | `https://t.me/username/123?comment=456` |
| 话题链接 | `https://t.me/username/123?thread=456` |

## 行为说明

- 程序会先解析所有 URL，再逐条下载。
- 未指定账号时使用当前激活账号。
- 如果输出目录不存在，会自动创建。
- 图片默认保存为 `.jpg`。
- 文档优先保留原始文件名。
- 文本消息会保存为 `.txt` 文件。
- 模板渲染完成后，如果缺少扩展名，会自动补回原始扩展名。

## 当前限制

- 评论链接当前不支持下载，程序会直接提示失败。
- 纯消息 ID 当前不支持下载，必须使用完整 URL。

## 示例

```bash
tdlr download -u "https://t.me/telegram/193"
tdlr download -u "https://t.me/telegram/193" "https://t.me/telegram/194" -p ./downloads
tdlr download -u "https://t.me/c/1697797156/151" -a 123456789
tdlr download -u "https://t.me/telegram/193" -i jpg,png
tdlr download -u "https://t.me/telegram/193" -t "{{ .DialogID }}_{{ .MessageID }}_{{ filenamify .FileName }}"
```

## 文件名模板

默认模板：

```text
{{ .DialogID }}_{{ .MessageID }}_{{ filenamify .FileName }}
```

### 可用变量

| 变量 | 说明 |
|------|------|
| `DialogID` | 会话 ID |
| `MessageID` | 消息 ID |
| `MessageDate` | 消息时间戳 |
| `FileName` | 原始文件名 |
| `FileCaption` | 消息文字或说明 |
| `FileSize` | 人类可读文件大小 |
| `DownloadDate` | 下载时间戳 |

### 可用函数

| 函数 | 说明 |
|------|------|
| `upper STRING` | 转大写 |
| `lower STRING` | 转小写 |
| `snakecase STRING` | 转 snake_case |
| `camelcase STRING` | 转 camelCase |
| `kebabcase STRING` | 转 kebab-case |
| `replace STRING FROM TO ...` | 成对替换字符串 |
| `repeat STRING N` | 重复字符串 |
| `rand MIN MAX` | 生成随机数 |
| `now` | 当前时间戳 |
| `formatDate TIMESTAMP [FORMAT]` | 格式化时间戳 |
| `filenamify STRING [MAX_LEN]` | 清洗非法文件名字符 |

### 示例

```bash
tdlr download -u "https://t.me/telegram/193" -t "{{ upper .FileName }}"
tdlr download -u "https://t.me/telegram/193" -t "{{ formatDate .DownloadDate 2006-01-02 }}_{{ filenamify .FileName }}"
tdlr download -u "https://t.me/telegram/193" -t "{{ lower (replace .FileName \" \" \"_\") }}"
```

说明：

- `formatDate` 使用 Go 风格格式模板，例如 `20060102150405`。
- `filenamify` 会替换非法字符，并在需要时截断长度。

## 参考

| 文件 | 说明 |
|------|------|
| `src/cli/args/download.rs` | 参数定义 |
| `src/commands/download/download.rs` | 命令入口 |
| `src/commands/download/handler.rs` | 下载执行和统计 |
| `src/commands/download/template.rs` | 文件名模板实现 |
| `src/utils/link.rs` | Telegram 链接解析 |
| `src/telegram/download/message.rs` | 消息拉取 |
| `src/telegram/download/file.rs` | 文件下载 |
