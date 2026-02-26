# 安装

你可以根据需要选择不同的安装方式：**正式版**（推荐）、**每日构建版**（尝鲜新功能）或**本地源码编译**。

---

## 1. 正式版安装 (Stable Release)

最稳定的发行版本，适合生产和日常稳定使用。

### Linux / macOS

```bash
curl -sSL https://raw.githubusercontent.com/haiyewei/tdlr/main/install/install.sh | sudo bash
```

指定版本：
```bash
curl -sSL https://raw.githubusercontent.com/haiyewei/tdlr/main/install/install.sh | sudo bash -s -- --version v0.1.0
```

使用代理（中国大陆）：
```bash
curl -sSL https://raw.githubusercontent.com/haiyewei/tdlr/main/install/install.sh | sudo bash -s -- --proxy
```

### Windows (PowerShell)

**请以管理员身份运行 PowerShell：**

```powershell
irm https://raw.githubusercontent.com/haiyewei/tdlr/main/install/install.ps1 | iex
```

指定版本：
```powershell
$Version = "v0.1.0"; irm https://raw.githubusercontent.com/haiyewei/tdlr/main/install/install.ps1 | iex
```

---

## 2. 每日构建版 (Daily Build)

每天 UTC 0点基于最新代码自动构建的测试版本。包含最新修复和尚在测试的新功能，适合希望尽早体验或参与反馈的用户。

### Linux / macOS

```bash
curl -sSL https://raw.githubusercontent.com/haiyewei/tdlr/main/install/install_daily.sh | sudo bash
```

### Windows (PowerShell)

**请以管理员身份运行 PowerShell：**

```powershell
irm https://raw.githubusercontent.com/haiyewei/tdlr/main/install/install_daily.ps1 | iex
```

---

## 3. 本地源码编译 (Local Build / Git Clone)

如果你想从源码自行编译 `tdlr`，或者希望参与贡献代码。

### 环境准备

- [Rust 和 Cargo](https://rustup.rs/) (>= 1.70.0)

### 编译与安装

项目中内置了自动化编译和安装的脚本。

#### Linux / macOS

```bash
# 1. 克隆代码仓库
git clone https://github.com/haiyewei/tdlr.git
cd tdlr

# 2. 运行本地安装脚本进行编译并安装到设备
bash install/install_local.sh
```

#### Windows (PowerShell)

**请以管理员身份运行 PowerShell：**

```powershell
# 1. 克隆代码仓库
git clone https://github.com/haiyewei/tdlr.git
cd tdlr

# 2. 运行本地安装脚本进行编译并安装到设备
.\install\install_local.ps1
```

---

## 手动下载

如果你不想使用一键脚本，可以从 [Releases](https://github.com/haiyewei/tdlr/releases) 页面手动下载对应平台的二进制文件：

| 平台 | 文件 |
|------|------|
| Linux x86_64 | `tdlr_Linux_64bit.tar.gz` |
| Linux ARM64 | `tdlr_Linux_arm64.tar.gz` |
| macOS x86_64 | `tdlr_MacOS_64bit.tar.gz` |
| macOS ARM64 | `tdlr_MacOS_arm64.tar.gz` |
| Windows x86_64 | `tdlr_Windows_64bit.zip` |

---

## 验证安装

无论使用哪种安装方式，完成后可以在终端执行以下命令验证安装：

```bash
tdlr --version
```
