# Installation

The install scripts have been merged into a single entry point per platform:

- Unix: `scripts/install.sh` / `scripts/upgrade.sh` / `scripts/uninstall.sh`
- Windows: `scripts/install.ps1` / `scripts/upgrade.ps1` / `scripts/uninstall.ps1`

Default behavior:

- Automatically detect whether to install from local source or a remote release package
- Install into user-owned directories instead of system directories
- Update user-level `PATH` instead of machine-level `PATH`
- Do not require `sudo` or administrator privileges
- The upgrade scripts detect the current install directory and upgrade to the latest remote release by default
- The uninstall scripts detect current install locations and legacy system-wide locations from Git history when available

Default install directories:

- Linux / macOS: `~/.local/bin`
- Windows: `%LOCALAPPDATA%\Programs\tdlr\bin`

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
curl -sSL https://raw.githubusercontent.com/haiyewei/tdlr/main/scripts/install.sh | bash -s -- --version v0.2.6
```

Use proxy mode:

```bash
curl -sSL https://raw.githubusercontent.com/haiyewei/tdlr/main/scripts/install.sh | bash -s -- --proxy
```

Notes:

- `--proxy` rewrites remote release asset downloads to `https://gh-proxy.com/https://github.com/...`
- The installer script itself is still fetched from `raw.githubusercontent.com`

### Windows (PowerShell)

```powershell
irm https://raw.githubusercontent.com/haiyewei/tdlr/main/scripts/install.ps1 | iex
```

Install a specific version:

```powershell
$Version = "v0.2.6"; irm https://raw.githubusercontent.com/haiyewei/tdlr/main/scripts/install.ps1 | iex
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

## 4. Upgrade

By default, the upgrade scripts:

- Detect the directory of the current `tdlr` installation and replace it in place
- Upgrade to the latest remote release package
- Support flags such as `--proxy`, `--install-dir`, and `--source local`

If no existing install is detected, pass `--install-dir` on Unix or `-InstallDir` on PowerShell to use the upgrader as an installer for a specific directory.

### Linux / macOS

```bash
curl -sSL https://raw.githubusercontent.com/haiyewei/tdlr/main/scripts/upgrade.sh | bash
```

Use proxy mode:

```bash
curl -sSL https://raw.githubusercontent.com/haiyewei/tdlr/main/scripts/upgrade.sh | bash -s -- --proxy
```

Upgrade from local source inside the repository:

```bash
git clone https://github.com/haiyewei/tdlr.git
cd tdlr
bash scripts/upgrade.sh --source local
```

### Windows (PowerShell)

```powershell
irm https://raw.githubusercontent.com/haiyewei/tdlr/main/scripts/upgrade.ps1 | iex
```

Use proxy mode:

```powershell
$Proxy = $true; irm https://raw.githubusercontent.com/haiyewei/tdlr/main/scripts/upgrade.ps1 | iex
```

Upgrade from local source inside the repository:

```powershell
git clone https://github.com/haiyewei/tdlr.git
cd tdlr
.\scripts\upgrade.ps1 -Source Local
```

---

## 5. Uninstall

The uninstall scripts remove binaries from the current user install directory, inspect `PATH` for existing installs, and also look at historical installer paths from Git history when they run inside the repository. That covers previous system-wide installs such as `/usr/local/bin` and `C:\tdlr`.

If you already know the exact install directory, pass `--install-dir` on Unix or `-InstallDir` on PowerShell to limit the uninstall scope to that location only.

### Linux / macOS

```bash
curl -sSL https://raw.githubusercontent.com/haiyewei/tdlr/main/scripts/uninstall.sh | bash
```

Run from the repository to include Git-history-based legacy path detection:

```bash
git clone https://github.com/haiyewei/tdlr.git
cd tdlr
bash scripts/uninstall.sh
```

### Windows (PowerShell)

```powershell
irm https://raw.githubusercontent.com/haiyewei/tdlr/main/scripts/uninstall.ps1 | iex
```

Run from the repository to include Git-history-based legacy path detection:

```powershell
git clone https://github.com/haiyewei/tdlr.git
cd tdlr
.\scripts\uninstall.ps1
```

If a legacy machine-level PATH entry still exists on Windows, rerun the PowerShell script in an elevated session so it can remove that machine PATH entry too.

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
