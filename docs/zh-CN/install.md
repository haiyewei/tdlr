# 安装

安装脚本已经合并为一套统一入口：

- Unix: `scripts/install.sh` / `scripts/upgrade.sh` / `scripts/uninstall.sh`
- Windows: `scripts/install.ps1` / `scripts/upgrade.ps1` / `scripts/uninstall.ps1`

默认行为：

- 优先自动判断本地源码安装还是远程发布包安装
- 安装到用户目录，而不是系统目录
- 写入用户级 PATH，而不是机器级 PATH
- 不再要求 `sudo` 或管理员权限
- 升级脚本会自动识别当前安装目录，默认升级到最新远程发布包
- 卸载脚本会自动识别当前安装目录，并在仓库内运行时结合 Git 历史识别旧版系统级安装路径

默认安装目录：

- Linux / macOS: `~/.local/bin`
- Windows: `%LOCALAPPDATA%\Programs\tdlr\bin`

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
curl -sSL https://raw.githubusercontent.com/haiyewei/tdlr/main/scripts/install.sh | bash -s -- --version v0.2.5
```

使用代理：

```bash
curl -sSL https://raw.githubusercontent.com/haiyewei/tdlr/main/scripts/install.sh | bash -s -- --proxy
```

说明：

- `--proxy` 会把远程发布包下载地址改写为 `https://gh-proxy.com/https://github.com/...`
- 初始安装脚本本身仍然是从 `raw.githubusercontent.com` 拉取

### Windows (PowerShell)

```powershell
irm https://raw.githubusercontent.com/haiyewei/tdlr/main/scripts/install.ps1 | iex
```

指定版本：

```powershell
$Version = "v0.2.5"; irm https://raw.githubusercontent.com/haiyewei/tdlr/main/scripts/install.ps1 | iex
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

## 4. 升级

升级脚本默认会：

- 优先识别当前已经安装的 `tdlr` 所在目录，然后原地覆盖升级
- 默认使用远程发布包升级到最新版本
- 支持 `--proxy`、`--install-dir`、`--source local` 等参数

如果没有检测到现有安装，可以显式传 `--install-dir` 或 `-InstallDir`，把升级脚本当作“安装到指定目录”的入口使用。

### Linux / macOS

```bash
curl -sSL https://raw.githubusercontent.com/haiyewei/tdlr/main/scripts/upgrade.sh | bash
```

使用代理：

```bash
curl -sSL https://raw.githubusercontent.com/haiyewei/tdlr/main/scripts/upgrade.sh | bash -s -- --proxy
```

在仓库内从本地源码升级：

```bash
git clone https://github.com/haiyewei/tdlr.git
cd tdlr
bash scripts/upgrade.sh --source local
```

### Windows (PowerShell)

```powershell
irm https://raw.githubusercontent.com/haiyewei/tdlr/main/scripts/upgrade.ps1 | iex
```

使用代理：

```powershell
$Proxy = $true; irm https://raw.githubusercontent.com/haiyewei/tdlr/main/scripts/upgrade.ps1 | iex
```

在仓库内从本地源码升级：

```powershell
git clone https://github.com/haiyewei/tdlr.git
cd tdlr
.\scripts\upgrade.ps1 -Source Local
```

---

## 5. 卸载

卸载脚本会删除当前用户安装目录里的二进制文件，也会检查 `PATH` 中已经存在的安装位置；如果脚本是在仓库里运行，还会结合 Git 历史中的旧安装脚本去识别旧版系统级安装目录，例如 `/usr/local/bin` 和 `C:\tdlr`。

如果你已经知道准确的安装目录，也可以在 Unix 下传 `--install-dir`，或在 PowerShell 下传 `-InstallDir`，把卸载范围限制在指定路径。

### Linux / macOS

```bash
curl -sSL https://raw.githubusercontent.com/haiyewei/tdlr/main/scripts/uninstall.sh | bash
```

如果希望同时启用基于 Git 历史的旧路径识别，建议在仓库里运行：

```bash
git clone https://github.com/haiyewei/tdlr.git
cd tdlr
bash scripts/uninstall.sh
```

### Windows (PowerShell)

```powershell
irm https://raw.githubusercontent.com/haiyewei/tdlr/main/scripts/uninstall.ps1 | iex
```

如果希望同时启用基于 Git 历史的旧路径识别，建议在仓库里运行：

```powershell
git clone https://github.com/haiyewei/tdlr.git
cd tdlr
.\scripts\uninstall.ps1
```

如果 Windows 里仍然存在旧版机器级 PATH 项，请以管理员身份重新运行 PowerShell 卸载脚本，这样它才能顺带移除机器级 PATH。

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
