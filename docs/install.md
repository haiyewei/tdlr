# 安装

## Linux / macOS

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

## Windows (PowerShell)

以管理员身份运行：

```powershell
irm https://raw.githubusercontent.com/haiyewei/tdlr/main/install/install.ps1 | iex
```

指定版本：
```powershell
$Version = "v0.1.0"; irm https://raw.githubusercontent.com/haiyewei/tdlr/main/install/install.ps1 | iex
```

## 手动下载

从 [Releases](https://github.com/haiyewei/tdlr/releases) 下载对应平台的二进制文件：

| 平台 | 文件 |
|------|------|
| Linux x86_64 | `tdlr_Linux_64bit.tar.gz` |
| Linux ARM64 | `tdlr_Linux_arm64.tar.gz` |
| macOS x86_64 | `tdlr_MacOS_64bit.tar.gz` |
| macOS ARM64 | `tdlr_MacOS_arm64.tar.gz` |
| Windows x86_64 | `tdlr_Windows_64bit.zip` |

## 验证安装

```bash
tdlr --version
```
