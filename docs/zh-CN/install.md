# 安装

安装脚本已经合并为一套统一入口：

- Unix: `scripts/install.sh`
- Windows: `scripts/install.ps1`

默认行为：

- 优先自动判断本地源码安装还是远程发布包安装
- 安装到用户目录，而不是系统目录
- 写入用户级 PATH，而不是机器级 PATH
- 不再要求 `sudo` 或管理员权限

默认安装目录：

- Linux / macOS: 优先 `~/.cargo/bin`，否则 `~/.local/bin`
- Windows: 优先 `%USERPROFILE%\.cargo\bin`，否则 `%LOCALAPPDATA%\Programs\tdlr\bin`

## Linux 发布变体

Linux 发布包现在分为两类：

- `*-unknown-linux-gnu`：给 Ubuntu、Debian、Fedora、RHEL 这类常见发行版使用
- `*-unknown-linux-musl`：给 Alpine 和最小化 Docker 基底镜像使用，更适合做自包含部署

Unix 安装脚本会在 Linux 上自动识别当前环境是 `glibc` 还是 `musl`，也可以手动指定：

```bash
bash scripts/install.sh --target x86_64-unknown-linux-musl
```

---

## 1. 稳定版安装

### Linux / macOS

```bash
curl -sSL https://raw.githubusercontent.com/haiyewei/tdlr/main/scripts/install.sh | bash
```

指定版本：

```bash
curl -sSL https://raw.githubusercontent.com/haiyewei/tdlr/main/scripts/install.sh | bash -s -- --version v0.1.0
```

使用代理：

```bash
curl -sSL https://raw.githubusercontent.com/haiyewei/tdlr/main/scripts/install.sh | bash -s -- --proxy
```

### Windows (PowerShell)

```powershell
irm https://raw.githubusercontent.com/haiyewei/tdlr/main/scripts/install.ps1 | iex
```

指定版本：

```powershell
$Version = "v0.1.0"; irm https://raw.githubusercontent.com/haiyewei/tdlr/main/scripts/install.ps1 | iex
```

使用代理：

```powershell
$Proxy = $true; irm https://raw.githubusercontent.com/haiyewei/tdlr/main/scripts/install.ps1 | iex
```

---

## 2. 每日构建版

每日构建版可以直接通过统一脚本指定 `daily`。

### Linux / macOS

```bash
curl -sSL https://raw.githubusercontent.com/haiyewei/tdlr/main/scripts/install.sh | bash -s -- --source remote --version daily
```

### Windows (PowerShell)

```powershell
$Source = "Remote"; $Version = "daily"; irm https://raw.githubusercontent.com/haiyewei/tdlr/main/scripts/install.ps1 | iex
```

---

## 3. 本地源码安装

如果你想从源码编译并安装：

### 环境准备

- [Rust 和 Cargo](https://rustup.rs/) (`>= 1.70.0`)

### Linux / macOS

```bash
git clone https://github.com/haiyewei/tdlr.git
cd tdlr
bash scripts/install.sh --source local
```

### Windows (PowerShell)

```powershell
git clone https://github.com/haiyewei/tdlr.git
cd tdlr
.\scripts\install.ps1 -Source Local
```

如果从源码编译，还需要先设置 Telegram API 凭据：

```bash
export TG_API_ID=123456
export TG_API_HASH=your_api_hash
```

Windows PowerShell:

```powershell
$env:TG_API_ID = "123456"
$env:TG_API_HASH = "your_api_hash"
```

---

## 手动下载

如果你不想使用脚本，也可以从 [Releases](https://github.com/haiyewei/tdlr/releases) 页面手动下载对应平台的二进制包：

| 平台 | 文件 |
|------|------|
| Linux x86_64 (GNU libc) | `tdlr-x86_64-unknown-linux-gnu.tar.gz` |
| Linux ARM64 (GNU libc) | `tdlr-aarch64-unknown-linux-gnu.tar.gz` |
| Linux x86_64 (musl / Alpine / 最小 Docker) | `tdlr-x86_64-unknown-linux-musl.tar.gz` |
| Linux ARM64 (musl / Alpine / 最小 Docker) | `tdlr-aarch64-unknown-linux-musl.tar.gz` |
| macOS x86_64 | `tdlr-x86_64-apple-darwin.tar.gz` |
| macOS ARM64 | `tdlr-aarch64-apple-darwin.tar.gz` |
| Windows x86_64 (MSVC) | `tdlr-x86_64-pc-windows-msvc.zip` |

---

## 验证安装

安装完成后执行：

```bash
tdlr --version
```
