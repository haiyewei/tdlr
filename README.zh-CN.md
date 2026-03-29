# TDLR

[English](./README.md)

`tdlr` 是一个面向 Telegram 工作流的模块化 Rust CLI。它支持账号管理、文件上传、消息下载、消息转发，以及通过 `stdio` 或 HTTP 提供长期驻留的服务模式。

## 功能

- 管理多个 Telegram 账号，并在本地切换当前激活账号。
- 上传本地文件或目录，支持扩展名过滤、路由表达式、说明文字和媒体组。
- 从 Telegram 消息链接下载文件或文本，支持文件名模板。
- 以 `direct`、`clone` 或 `smart` 模式转发消息。
- 以持久服务模式运行 `tdlr`，通过 `stdio` 或 HTTP API 传入命令。
- 使用合并后的 [`scripts/`](./scripts/) 安装脚本安装到用户目录。

## 文档导航

| 主题 | English | 中文 |
|------|------|------|
| 主页 | [README.md](./README.md) | [README.zh-CN.md](./README.zh-CN.md) |
| 文档入口 | [docs/README.md](./docs/README.md) | [docs/README.md](./docs/README.md) |
| 命令列表 | [docs/en/command-list.md](./docs/en/command-list.md) | [docs/zh-CN/command-list.md](./docs/zh-CN/command-list.md) |
| 安装 | [docs/en/install.md](./docs/en/install.md) | [docs/zh-CN/install.md](./docs/zh-CN/install.md) |
| Android | [docs/en/android.md](./docs/en/android.md) | [docs/zh-CN/android.md](./docs/zh-CN/android.md) |

## 命令

| 命令 | 说明 |
|------|------|
| [`version`](./docs/zh-CN/version.md) | 输出版本、编译器和目标平台信息 |
| [`auth`](./docs/zh-CN/auth.md) | 管理 Telegram 登录账号、当前账号和状态检查 |
| [`upload`](./docs/zh-CN/upload.md) | 将本地文件或目录上传到 Telegram |
| [`download`](./docs/zh-CN/download.md) | 从 Telegram 消息链接下载文件或文本 |
| [`forward`](./docs/zh-CN/forward.md) | 转发消息，支持克隆模式处理受限会话 |
| [`service`](./docs/zh-CN/service.md) | 启动 `stdio` 或 HTTP API 服务模式 |

## 快速开始

### 1. 安装

Linux / macOS：

```bash
curl -sSL https://raw.githubusercontent.com/haiyewei/tdlr/main/scripts/install.sh | bash
```

Windows PowerShell：

```powershell
irm https://raw.githubusercontent.com/haiyewei/tdlr/main/scripts/install.ps1 | iex
```

安装脚本会把 `tdlr` 安装到当前执行 shell 对应用户的目录里，并更新该用户的 `PATH`。如果安装 shell 实际是以 `root` 身份运行，就会安装到 `root` 的用户目录。

完整的安装、卸载、代理和源码构建说明见 [docs/zh-CN/install.md](./docs/zh-CN/install.md)。

### 2. 验证 CLI

```bash
tdlr version
```

### 3. 首次登录

```bash
tdlr auth login add
tdlr auth login list
```

### 4. 常用示例

```bash
tdlr upload -p ./media -c me
tdlr download -u "https://t.me/telegram/193" -p ./downloads
tdlr forward -f https://t.me/channel/123 -t @backup
tdlr service --http-bind 127.0.0.1:8787
```

## 从源码构建

源码构建主要面向开发、调试或自定义构建场景。

```bash
export TG_API_ID=123456
export TG_API_HASH=your_api_hash
cargo build --release
```

Windows PowerShell：

```powershell
$env:TG_API_ID = "123456"
$env:TG_API_HASH = "your_api_hash"
cargo build --release
```

## 服务模式

`tdlr service` 支持：

- 基于 `stdio` 的本地自动化命令执行。
- 通过结构化 HTTP 端点管理账号、登录流程、上传、下载、转发和健康检查。

请求格式、返回格式和限制说明见 [docs/zh-CN/service.md](./docs/zh-CN/service.md)。
