# Forward 命令

将 Telegram 消息从某个对话转发到另一个对话，且完美支持突破**禁止转发**限制的群组/频道。

## 基本用法

```bash
tdlr forward -f <消息URL或ID> [选项]
```

## 参数

| 参数 | 短参数 | 说明 |
|------|--------|------|
| `--from` | `-f` | 来源消息的 URL 或 ID（必需，可多个） |
| `--from-chat` | | 来源对话（当 `--from` 提供的是消息 ID 而非 URL 时必需使用该参数） |
| `--to` | `-t` | 目标聊天 ID 或用户名（默认：Saved Messages） |
| `--mode` | `-m` | 转发模式：`direct` / `clone` / `smart`（默认：`smart`） |
| `--topic` | | 话题 ID（用于发往目标群组特定话题时使用） |
| `--drop-author` | | 不附带原作者名称（作为副本发送，仅对 `direct` 原生转发模式生效） |
| `--account` | `-a` | 指定使用的账户 ID（默认：当前激活账户） |

## 转发模式说明 (--mode)

支持三种不同的转发模式，能够无缝处理并突破群组或频道的**禁止转发**（`restricted content`/`noforwards`）限制：

- **smart** (默认): 智能检测。程序会自动检查来源对话（Chat）的 `noforwards` 安全限制标志。如果不受限制，则使用速度最快的原生转发 (Direct)；如果该频道/群组限制了转发与保存，则自动降级采用克隆 (Clone) 模式拉取内容。
- **direct**: 原生转发。使用 Telegram 的原生 Forward API。速度极快，自带来源追溯和来源标签。
- **clone**: 克隆（突破限制）模式。针对禁止转发的内容，程序会在内存/本地先将内容下载，然后再作为全新的文件将其重新上传发送到目标位置。

## Chat ID 格式

`--to` 和 `--from-chat` 参数支持多种格式，会自动通过 Telegram API 解析正确的类型：

| 格式 | 说明 |
|------|------|
| 空 / `me` / `self` | Saved Messages (收藏夹) |
| `@username` | 用户名（用户/群组/频道） |
| `username` | 用户名（不带@） |
| 数字 ID | 自动从对话列表中查找匹配的用户/群组/频道 |

无需手动区分用户 ID、群组 ID 或频道 ID，程序会自动进行智能识别。

## 支持的输入源格式 (`--from`)

`-f` 可以接受纯数字的 Message ID，也可以接受你在各处复制出来的标准 Telegram 链接。程序会自动识别如下常见的分享链接，并准确提取到对应的那条内容本身（即使是附着在主消息下的评论）：

- `https://t.me/username/123` (公开频道/群组消息)
- `https://t.me/c/1234567890/123` (私密频道/群组消息)
- `https://t.me/username/123/456` (媒体组中的某张特定图片/回复消息)
- `https://t.me/username/123?comment=456` (频道消息下的某条特定评论)
- `https://t.me/username/123?thread=456` (话题内的特定消息)

## 示例

### 基础转发（使用消息链接）

直接填入 Telegram 客户端复制出来的消息链接是最简单的一种方式：

```bash
# 转发单条消息到 Saved Messages
tdlr forward -f https://t.me/channel/123

# 转发多条消息到指定用户
tdlr forward -f https://t.me/channel/123 https://t.me/channel/124 -t @username

# 转发私密群组的消息到另一个群组
tdlr forward -f https://t.me/c/12345/678 -t 987654321
```

### 使用消息 ID 与源对话信息

如果你只有消息数字 ID 而没有完整链接，你需要显式指明来源对话：

```bash
# 从指定频道转发消息 ID 为 500 的消息到 Saved Messages
tdlr forward -f 500 --from-chat @my_channel
```

### 转发到话题版块

如果想将消息发往开启了论坛主题的主群组下的特定板块里，使用 `--topic` 参数：

```bash
# 将消息链接转发到指定群组的对应话题 (Topic 5) 中
tdlr forward -f https://t.me/channel/123 -t 1234567890 --topic 5
```

### 处理受限/禁止转发的内容（克隆）

由于命令行默认工作在 `smart` 模式下，因此当你想要保存防盗和防转发素材时，依然像普通的转发同等操作即可：

```bash
# 如果来源频道限制了转发/保存，将自动降级为克隆（流式下载并上传）模式
tdlr forward -f https://t.me/restricted_channel/99
```

当然你也可以强制介入、手动切换到你指定的模式下：

```bash
# 强制使用直接转发模式（且不附带原作者名字 / 作为全新副本发送）
tdlr forward -f https://t.me/channel/123 -m direct --drop-author

# 强制使用克隆模式（适用于源内容并未受限，但是你想将其转为独立文件发送以抹去所有源关联时）
tdlr forward -f https://t.me/channel/123 -m clone
```
