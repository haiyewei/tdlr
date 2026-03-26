# `forward` 命令

`forward` 用于将 Telegram 消息转发到目标会话，支持直接转发和克隆转发。

## 用法

```bash
tdlr forward --from <SOURCE>... [OPTIONS]
```

## 参数

| 参数 | 说明 |
|------|------|
| `-f, --from <SOURCE>...` | 来源消息 URL 或纯消息 ID，必填，可多个 |
| `--from-chat <CHAT>` | 当 `--from` 使用纯消息 ID 时必填 |
| `-t, --to <CHAT>` | 目标聊天，默认 Saved Messages |
| `-m, --mode <MODE>` | 转发模式：`direct` / `clone` / `smart`，默认 `smart` |
| `--topic <TOPIC>` | 目标论坛群组的话题 ID |
| `-a, --account <USER_ID>` | 使用指定账号 |
| `--drop-author` | 仅 `direct` 模式可用，作为拷贝发送，不带转发头 |

## 输入源

`--from` 支持两类输入：

| 类型 | 示例 |
|------|------|
| Telegram 消息 URL | `https://t.me/channel/123` |
| 纯消息 ID | `500` |

如果使用纯消息 ID：

- 必须同时传 `--from-chat`
- `--from-chat` 可以是 `me`、用户名或数字 ID

## 转发模式

| 模式 | 说明 |
|------|------|
| `direct` | 使用 Telegram 原生转发 API |
| `clone` | 先下载内容，再重新上传到目标会话 |
| `smart` | 自动检查来源会话是否设置 `noforwards`，然后在 `direct` 和 `clone` 间切换 |

## 行为说明

- `smart` 模式下，程序会先解析来源会话，再根据 `noforwards` 选择模式。
- `clone` 模式对禁止转发内容有效，因为它会重新上传内容，而不是调用 Telegram 原生转发。
- 媒体组在 `clone` 模式下会先全部下载，再按相册重新发送。
- `drop-author` 只在 `direct` 模式下有意义。

## 支持的 URL 形式

| 类型 | 示例 |
|------|------|
| 公开消息 | `https://t.me/username/123` |
| 私有消息 | `https://t.me/c/1234567890/123` |
| 媒体组 / 回复 | `https://t.me/username/123/456` |
| 评论链接 | `https://t.me/username/123?comment=456` |
| 话题链接 | `https://t.me/username/123?thread=456` |

程序会优先使用：

- `comment_id`
- `secondary_id`
- `message_id`

作为最终要处理的消息 ID。

## 示例

```bash
tdlr forward -f https://t.me/channel/123
tdlr forward -f https://t.me/channel/123 -t @backup
tdlr forward -f 500 --from-chat @source_channel -t me
tdlr forward -f https://t.me/restricted_channel/99 -m clone
tdlr forward -f https://t.me/channel/123 -m direct --drop-author
tdlr forward -f https://t.me/channel/123 -t -1001234567890 --topic 5
```

## 参考

| 文件 | 说明 |
|------|------|
| `src/cli/args/forward.rs` | 参数定义 |
| `src/commands/forward/forward.rs` | 命令入口和 `smart` 模式分发 |
| `src/commands/forward/direct.rs` | 原生转发实现 |
| `src/commands/forward/clone.rs` | 克隆转发实现 |
| `src/utils/link.rs` | 输入源解析 |
| `src/telegram/upload/chat.rs` | 目标聊天解析 |
