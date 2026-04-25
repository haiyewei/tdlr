# `auth` 命令

`auth` 用于管理 Telegram 账号登录状态、当前激活账号和账号检查。

## 命令树

```text
tdlr auth
├── login
│   ├── add
│   ├── list
│   ├── remove
│   └── use
├── logout
└── status
```

## `tdlr auth`

用法：

```bash
tdlr auth <COMMAND>
```

子命令：

| 子命令 | 说明 |
|------|------|
| `login add` | 添加新账号 |
| `login list` | 列出已登录账号 |
| `login remove` | 删除指定账号 |
| `login use` | 切换当前激活账号 |
| `logout` | 退出账号 |
| `status` | 检查账号状态 |

## `tdlr auth login add`

添加新 Telegram 账号。

用法：

```bash
tdlr auth login add [--name <NAME>] [--method <phone|qr>] [--code-via <auto|app|sms>]
```

参数：

| 参数 | 说明 |
|------|------|
| `-n, --name <NAME>` | 账号别名，仅用于本地显示 |
| `-m, --method <METHOD>` | 登录方式，支持 `phone` 和 `qr`，默认值为 `qr` |
| `--code-via <MODE>` | 手机号验证码通道偏好，支持 `auto`、`app`、`sms`，默认值为 `auto` |

登录方式：

| 方法 | 说明 |
|------|------|
| `qr` | 输出二维码，使用 Telegram App 扫码登录 |
| `phone` | 交互式输入手机号、验证码；如果启用 2FA，还会继续要求输入密码 |

行为说明：

- 登录过程中会先使用临时 session。
- 对手机号登录来说，`--code-via app` 和 `--code-via sms` 只是偏好，不是强制覆盖；首次验证码通道仍由 Telegram 服务端决定。
- 如果指定 `--code-via sms`，且 Telegram 返回后续可切换到 SMS，CLI 会按 Telegram 给出的等待时间自动等待，然后尝试调用 `auth.resendCode`。
- 手机号登录流程会打印 Telegram 实际使用的验证码通道、后续可切换通道，以及服务端返回的等待时间。
- 登录成功后会将 session 重命名为用户 ID 对应的正式 session。
- 账号信息会写入本地账号元数据。
- 新登录账号会自动设为当前激活账号。
- `--name` 当前只作为显示名称参数保留，最终保存的显示名仍以 Telegram 返回的用户信息为准。

示例：

```bash
tdlr auth login add
tdlr auth login add --method phone
tdlr auth login add --method phone --code-via app
tdlr auth login add --method phone --code-via sms
tdlr auth login add --name work --method qr
```

## `tdlr auth login list`

列出本地已保存的账号。

用法：

```bash
tdlr auth login list
```

输出内容：

- 用户 ID
- 显示名称
- 用户名（如果存在）
- 当前是否为激活账号

说明：

- 如果没有任何账号，会提示使用 `tdlr auth login add` 添加账号。

示例：

```bash
tdlr auth login list
```

## `tdlr auth login remove`

按用户 ID 删除账号。

用法：

```bash
tdlr auth login remove <ID>
```

参数：

| 参数 | 说明 |
|------|------|
| `<ID>` | 要删除的 Telegram 用户 ID |

行为说明：

- 会删除账号对应的 session 和账号元数据。
- 如果删除的是当前激活账号，程序会清除 active 标记。
- 如果本地还有其他账号，程序会自动切换到第一个可用账号。

示例：

```bash
tdlr auth login remove 123456789
```

## `tdlr auth login use`

切换当前激活账号。

用法：

```bash
tdlr auth login use <ID>
```

参数：

| 参数 | 说明 |
|------|------|
| `<ID>` | 要切换到的 Telegram 用户 ID |

行为说明：

- 该命令只切换当前 active 账号，不会创建新账号。
- 切换后其余未指定 `--account` 的命令都会默认使用这个账号。

示例：

```bash
tdlr auth login use 123456789
```

## `tdlr auth logout`

退出账号。

用法：

```bash
tdlr auth logout [--id <ID>] [--all]
```

参数：

| 参数 | 说明 |
|------|------|
| `-i, --id <ID>` | 指定退出某个账号；不传时默认退出当前激活账号 |
| `--all` | 退出全部账号 |

行为说明：

- `--all` 会移除全部账号和 session，并清空 active 账号。
- 不传 `--id` 和 `--all` 时，会退出当前激活账号。
- 如果退出的是当前激活账号且本地还存在其他账号，程序会自动切换到第一个可用账号。

示例：

```bash
tdlr auth logout
tdlr auth logout --id 123456789
tdlr auth logout --all
```

## `tdlr auth status`

并发检查所有已保存账号的授权状态。

用法：

```bash
tdlr auth status
```

输出内容：

- 用户 ID
- 是否已授权
- 用户信息或错误信息

行为说明：

- 该命令会尝试为本地所有账号创建客户端。
- 会并发检查授权状态和当前用户信息。
- 如果某个账号加载失败，错误会直接打印到输出。

示例：

```bash
tdlr auth status
```

## 本地数据位置

账号数据由 session 管理模块处理，默认写入用户配置目录下的 `tdlr` 子目录，包含：

- `sessions/`
- `accounts.json`
- `.active`

## 参考

| 文件 | 说明 |
|------|------|
| `src/cli/args/auth.rs` | `auth` 参数定义 |
| `src/commands/auth/login/add.rs` | 添加账号实现 |
| `src/telegram/auth/phone.rs` | 交互式手机号登录流程 |
| `src/telegram/client/instance.rs` | 原始手机号认证请求、重发验证码与登录完成逻辑 |
| `src/commands/auth/login/list.rs` | 列出账号实现 |
| `src/commands/auth/login/remove.rs` | 删除账号实现 |
| `src/commands/auth/login/use_account.rs` | 切换激活账号实现 |
| `src/commands/auth/logout.rs` | 退出账号实现 |
| `src/commands/auth/status.rs` | 状态检查实现 |
| `src/telegram/session/manager.rs` | session 路径和 active 管理 |
