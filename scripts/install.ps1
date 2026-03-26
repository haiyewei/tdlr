[CmdletBinding()]
param(
    [ValidateSet("Auto", "Local", "Remote")]
    [string]$Source = $(if ($env:TDLR_INSTALL_SOURCE) { $env:TDLR_INSTALL_SOURCE } else { "Auto" }),
    [string]$InstallDir = $env:TDLR_INSTALL_DIR,
    [string]$Version = $(if ($env:TDLR_INSTALL_VERSION) { $env:TDLR_INSTALL_VERSION } else { "latest" }),
    [switch]$Proxy
)

$ErrorActionPreference = "Stop"

$BinaryName = "tdlr.exe"
$RemoteRepoOwner = if ($env:TDLR_REMOTE_REPO_OWNER) { $env:TDLR_REMOTE_REPO_OWNER } else { "haiyewei" }
$RemoteRepoName = if ($env:TDLR_REMOTE_REPO_NAME) { $env:TDLR_REMOTE_REPO_NAME } else { "tdlr" }
$RemoteBaseUrl = $env:TDLR_REMOTE_BASE_URL
$ProxyPrefix = if ($Proxy) { "https://mirror.ghproxy.com/" } else { "" }
$ScriptPath = if ($PSCommandPath) { $PSCommandPath } else { $MyInvocation.MyCommand.Path }
$ScriptDir = if ($ScriptPath) { Split-Path -Parent $ScriptPath } else { $null }
$RepoRoot = if ($ScriptDir) {
    try {
        (Resolve-Path (Join-Path $ScriptDir "..") -ErrorAction Stop).Path
    }
    catch {
        $null
    }
}
else {
    $null
}

function Get-DefaultInstallDir {
    $cargoBin = Join-Path $HOME ".cargo\bin"
    if (Test-Path $cargoBin -PathType Container) {
        return $cargoBin
    }

    return (Join-Path $env:LOCALAPPDATA "Programs\tdlr\bin")
}

function Test-RepositoryWorkspace {
    return $RepoRoot -and (Test-Path (Join-Path $RepoRoot "Cargo.toml") -PathType Leaf)
}

function Get-LocalSourceBinary {
    $candidates = @()

    if ($ScriptDir) {
        $candidates += (Join-Path $ScriptDir $BinaryName)
    }

    if ($RepoRoot) {
        $candidates += (Join-Path $RepoRoot "target\release\$BinaryName")
        $candidates += (Join-Path $RepoRoot "target\debug\$BinaryName")
    }

    foreach ($candidate in $candidates) {
        if (Test-Path $candidate -PathType Leaf) {
            return (Resolve-Path $candidate).Path
        }
    }

    return $null
}

function Build-LocalReleaseBinary {
    $cargo = Get-Command cargo -ErrorAction SilentlyContinue
    if (-not $cargo) {
        return $null
    }

    if (-not $RepoRoot -or -not (Test-Path (Join-Path $RepoRoot "Cargo.toml") -PathType Leaf)) {
        return $null
    }

    Write-Host "[tdlr] no local binary found, building release binary with cargo build --release"

    Push-Location $RepoRoot
    try {
        & $cargo.Source build --release
    }
    finally {
        Pop-Location
    }

    $builtBinary = Join-Path $RepoRoot "target\release\$BinaryName"
    if (Test-Path $builtBinary -PathType Leaf) {
        return (Resolve-Path $builtBinary).Path
    }

    return $null
}

function Get-RemoteAssetName {
    switch ($env:PROCESSOR_ARCHITECTURE) {
        "AMD64" {
            return "tdlr-x86_64-pc-windows-msvc.zip"
        }
        default {
            throw "[tdlr] unsupported system architecture for remote install: $env:PROCESSOR_ARCHITECTURE"
        }
    }
}

function Get-RemoteDownloadUrl {
    $assetName = Get-RemoteAssetName

    if (-not [string]::IsNullOrWhiteSpace($RemoteBaseUrl)) {
        return ($RemoteBaseUrl.TrimEnd("/") + "/$assetName")
    }

    if ($Version -eq "latest") {
        return "${ProxyPrefix}https://github.com/$RemoteRepoOwner/$RemoteRepoName/releases/latest/download/$assetName"
    }

    return "${ProxyPrefix}https://github.com/$RemoteRepoOwner/$RemoteRepoName/releases/download/$Version/$assetName"
}

function Get-RemoteBinary {
    $url = Get-RemoteDownloadUrl
    $tempRoot = Join-Path $env:TEMP ("tdlr-install-" + [System.Guid]::NewGuid().ToString("N"))
    $zipPath = Join-Path $tempRoot "package.zip"
    $extractDir = Join-Path $tempRoot "extract"

    New-Item -ItemType Directory -Force -Path $extractDir | Out-Null

    Write-Host "[tdlr] downloading $url"
    Invoke-WebRequest -Uri $url -OutFile $zipPath -UseBasicParsing
    Expand-Archive -Path $zipPath -DestinationPath $extractDir -Force

    $binary = Get-ChildItem -Path $extractDir -Filter $BinaryName -Recurse -File | Select-Object -First 1
    if (-not $binary) {
        throw "[tdlr] binary $BinaryName not found in the downloaded package."
    }

    return @{
        BinaryPath = $binary.FullName
        CleanupPath = $tempRoot
    }
}

function Test-PathEntry {
    param(
        [string]$PathValue,
        [string]$Entry
    )

    if ([string]::IsNullOrWhiteSpace($PathValue)) {
        return $false
    }

    foreach ($segment in ($PathValue -split ";")) {
        if ($segment.TrimEnd("\") -eq $Entry.TrimEnd("\")) {
            return $true
        }
    }

    return $false
}

function Add-InstallDirToUserPath {
    param(
        [string]$Entry
    )

    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")

    if (-not (Test-PathEntry -PathValue $userPath -Entry $Entry)) {
        $newUserPath = if ([string]::IsNullOrWhiteSpace($userPath)) {
            $Entry
        }
        else {
            "$userPath;$Entry"
        }

        [Environment]::SetEnvironmentVariable("Path", $newUserPath, "User")
        Write-Host "[tdlr] added install directory to the user PATH"
    }
    else {
        Write-Host "[tdlr] install directory already exists in the user PATH"
    }

    if (-not (Test-PathEntry -PathValue $env:Path -Entry $Entry)) {
        $env:Path = "$Entry;$env:Path"
        Write-Host "[tdlr] updated PATH for the current PowerShell session"
    }
}

function Resolve-ExecutionMode {
    switch ($Source.ToLowerInvariant()) {
        "local" { return "local" }
        "remote" { return "remote" }
        "auto" {
            $hasScriptBinary = $ScriptDir -and (Test-Path (Join-Path $ScriptDir $BinaryName) -PathType Leaf)
            $hasRepoRoot = $RepoRoot -and (Test-Path (Join-Path $RepoRoot "Cargo.toml") -PathType Leaf)
            $hasBuiltBinary = $RepoRoot -and (Test-Path (Join-Path $RepoRoot "target\release\$BinaryName") -PathType Leaf)

            if ($hasScriptBinary -or $hasRepoRoot -or $hasBuiltBinary) {
                return "local"
            }

            return "remote"
        }
        default {
            throw "[tdlr] unsupported install source mode: $Source"
        }
    }
}

if ([string]::IsNullOrWhiteSpace($InstallDir)) {
    $InstallDir = Get-DefaultInstallDir
}

$InstallMode = Resolve-ExecutionMode
$SourceBinary = $null
$CleanupPath = $null

if ($InstallMode -eq "local") {
    $scriptBinary = if ($ScriptDir) {
        $candidate = Join-Path $ScriptDir $BinaryName
        if (Test-Path $candidate -PathType Leaf) {
            (Resolve-Path $candidate).Path
        }
    }
    else {
        $null
    }

    if ($scriptBinary) {
        $SourceBinary = $scriptBinary
    }
    elseif (Test-RepositoryWorkspace) {
        $builtBinary = Build-LocalReleaseBinary
        if ($builtBinary) {
            $SourceBinary = $builtBinary
        }
        else {
            $SourceBinary = Get-LocalSourceBinary
        }
    }
    else {
        $SourceBinary = Get-LocalSourceBinary
    }

    if (-not $SourceBinary) {
        throw "[tdlr] no local binary found next to the script or in target/release."
    }
}
else {
    $remote = Get-RemoteBinary
    $SourceBinary = $remote.BinaryPath
    $CleanupPath = $remote.CleanupPath
}

try {
    New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
    $InstallPath = Join-Path $InstallDir $BinaryName

    if ([System.IO.Path]::GetFullPath($SourceBinary) -ne [System.IO.Path]::GetFullPath($InstallPath)) {
        Copy-Item -LiteralPath $SourceBinary -Destination $InstallPath -Force
    }

    Write-Host "[tdlr] installed to $InstallPath"
    Add-InstallDirToUserPath -Entry $InstallDir
    Write-Host "[tdlr] restart your terminal to pick up the persisted PATH entry in new sessions"
    Write-Host "[tdlr] run 'tdlr --help' to get started"
}
finally {
    if ($CleanupPath -and (Test-Path $CleanupPath)) {
        Remove-Item -LiteralPath $CleanupPath -Recurse -Force
    }
}
