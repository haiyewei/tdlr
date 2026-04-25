# `codes` 命令

`codes` 用于读取所选账号中 Telegram `Verification Codes` 会话的最新消息。

## 用法

```bash
tdlr codes [--limit <N>] [--account <USER_ID>]
```

## 参数

| 参数 | 说明 |
|------|------|
| `-n, --limit <N>` | 输出最近消息的条数。默认值：`10` |
| `--account <USER_ID>` | 指定要使用的账号用户 ID。默认使用当前激活账号 |

## 会话匹配顺序

`tdlr codes` 不会只写死一个会话标题，而是按以下顺序寻找候选会话：

1. 官方 Telegram 服务账号 `777000`
2. 兼容性回退账号 `42777`
3. 名称为 `Verification Codes` 的会话
4. 名称为 `Telegram` 的会话

命令会使用当前对话列表中优先级最高的第一个匹配项。

## 输出内容

每条消息会输出：

- 本地时区时间戳
- Telegram 消息 ID
- 消息正文

如果消息没有文本正文，则会输出 `[媒体消息]` 或 `[服务消息]` 之类的占位说明。

## 说明

- 账号必须已经登录并处于已授权状态。
- 该命令只读取现有会话历史，不会触发新的登录流程。
- `Verification Codes` 会话里有什么内容，仍然取决于 Telegram 服务端行为以及该账号最近的登录活动。

## 示例

```bash
tdlr codes
tdlr codes --limit 5
tdlr codes --account 123456789 --limit 20
```

## 参考

| 文件 | 说明 |
|------|------|
| `src/cli/args/codes.rs` | `codes` 命令参数定义 |
| `src/commands/codes.rs` | 会话查找与消息输出实现 |
