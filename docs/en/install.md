# Installation

The install scripts have been merged into a single entry point per platform:

- Unix: `scripts/install.sh`
- Windows: `scripts/install.ps1`

Default behavior:

- Automatically detect whether to install from local source or a remote release package
- Install into user-owned directories instead of system directories
- Update user-level `PATH` instead of machine-level `PATH`
- Do not require `sudo` or administrator privileges

Default install directories:

- Linux / macOS: prefer `~/.cargo/bin`, otherwise `~/.local/bin`
- Windows: prefer `%USERPROFILE%\.cargo\bin`, otherwise `%LOCALAPPDATA%\Programs\tdlr\bin`

## Linux release variants

Linux releases are published in two variants:

- `*-unknown-linux-gnu`: for common distributions such as Ubuntu, Debian, Fedora, RHEL, and most standard server images
- `*-unknown-linux-musl`: for Alpine and minimal Docker base images where a more self-contained binary is preferred

The Unix installer auto-detects `glibc` or `musl` on Linux. You can also force a specific release asset:

```bash
bash scripts/install.sh --target x86_64-unknown-linux-musl
```

---

## 1. Stable install

### Linux / macOS

```bash
curl -sSL https://raw.githubusercontent.com/haiyewei/tdlr/main/scripts/install.sh | bash
```

Install a specific version:

```bash
curl -sSL https://raw.githubusercontent.com/haiyewei/tdlr/main/scripts/install.sh | bash -s -- --version v0.1.0
```

Use proxy mode:

```bash
curl -sSL https://raw.githubusercontent.com/haiyewei/tdlr/main/scripts/install.sh | bash -s -- --proxy
```

### Windows (PowerShell)

```powershell
irm https://raw.githubusercontent.com/haiyewei/tdlr/main/scripts/install.ps1 | iex
```

Install a specific version:

```powershell
$Version = "v0.1.0"; irm https://raw.githubusercontent.com/haiyewei/tdlr/main/scripts/install.ps1 | iex
```

Use proxy mode:

```powershell
$Proxy = $true; irm https://raw.githubusercontent.com/haiyewei/tdlr/main/scripts/install.ps1 | iex
```

---

## 2. Daily build

Daily builds can be installed through the same unified scripts with `daily`.

### Linux / macOS

```bash
curl -sSL https://raw.githubusercontent.com/haiyewei/tdlr/main/scripts/install.sh | bash -s -- --source remote --version daily
```

### Windows (PowerShell)

```powershell
$Source = "Remote"; $Version = "daily"; irm https://raw.githubusercontent.com/haiyewei/tdlr/main/scripts/install.ps1 | iex
```

---

## 3. Local source install

If you want to build and install from source:

### Prerequisites

- [Rust and Cargo](https://rustup.rs/) (`>= 1.70.0`)

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

If you build from source, export Telegram API credentials before running Cargo:

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

## Manual download

If you do not want to use the scripts, download a platform package manually from [Releases](https://github.com/haiyewei/tdlr/releases):

| Platform | File |
|------|------|
| Linux x86_64 (GNU libc) | `tdlr-x86_64-unknown-linux-gnu.tar.gz` |
| Linux ARM64 (GNU libc) | `tdlr-aarch64-unknown-linux-gnu.tar.gz` |
| Linux x86_64 (musl / Alpine / minimal Docker) | `tdlr-x86_64-unknown-linux-musl.tar.gz` |
| Linux ARM64 (musl / Alpine / minimal Docker) | `tdlr-aarch64-unknown-linux-musl.tar.gz` |
| macOS x86_64 | `tdlr-x86_64-apple-darwin.tar.gz` |
| macOS ARM64 | `tdlr-aarch64-apple-darwin.tar.gz` |
| Windows x86_64 (MSVC) | `tdlr-x86_64-pc-windows-msvc.zip` |

---

## Verify installation

After installation, run:

```bash
tdlr --version
```
