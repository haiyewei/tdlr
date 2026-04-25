# 命令列表

本文档给出 `tdlr` 当前命令树总览，并链接到每个命令的独立详细文档。

## 顶层命令

```text
tdlr
├── version
├── auth
│   ├── login
│   │   ├── add
│   │   ├── list
│   │   ├── remove
│   │   └── use
│   ├── logout
│   └── status
├── codes
├── upload
├── download
├── forward
└── service
```

## 顶层说明

| 命令 | 说明 |
|------|------|
| [`version`](./version.md) | 输出版本、Rust 编译器版本和当前目标平台 |
| [`auth`](./auth.md) | 管理 Telegram 登录账号、激活账号和账号状态 |
| [`codes`](./codes.md) | 显示 `Verification Codes` 会话的最近消息 |
| [`upload`](./upload.md) | 将本地文件或目录上传到 Telegram |
| [`download`](./download.md) | 从 Telegram 消息链接下载文件或文本 |
| [`forward`](./forward.md) | 将 Telegram 消息转发到其他会话，支持克隆模式 |
| [`service`](./service.md) | 启动长期驻留服务，支持 `stdio` 和 HTTP API |

## `auth` 子命令

| 命令 | 说明 |
|------|------|
| [`auth login add`](./auth.md#tdlr-auth-login-add) | 添加新账号，支持手机号登录和二维码登录 |
| [`auth login list`](./auth.md#tdlr-auth-login-list) | 列出已登录账号 |
| [`auth login remove`](./auth.md#tdlr-auth-login-remove) | 按用户 ID 删除账号 |
| [`auth login use`](./auth.md#tdlr-auth-login-use) | 切换当前激活账号 |
| [`auth logout`](./auth.md#tdlr-auth-logout) | 退出当前账号、指定账号或全部账号 |
| [`auth status`](./auth.md#tdlr-auth-status) | 并发检查所有账号的授权状态 |

## 相关文档

| 文档 | 说明 |
|------|------|
| [version](./version.md) | `version` 命令完整说明 |
| [auth](./auth.md) | `auth` 命令完整说明 |
| [codes](./codes.md) | `codes` 命令完整说明 |
| [upload](./upload.md) | `upload` 命令完整说明 |
| [download](./download.md) | `download` 命令完整说明 |
| [forward](./forward.md) | `forward` 命令完整说明 |
| [service](./service.md) | `service` 命令完整说明 |
| [安装](./install.md) | 安装脚本、安装模式和用户级安装目录 |
| [Android 集成指南](./android.md) | Android JNI 集成方式 |
