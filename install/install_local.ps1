param(
    [Boolean]$SkipCompilation = $False
)

$ErrorActionPreference = "Stop"

$Repo = "tdlr"
$Location = "$Env:SystemDrive\tdlr"

# check if run as admin
if (-not ([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]"Administrator"))
{
    Write-Host "Please run this script as Administrator to install to system location" -ForegroundColor Red
    exit 1
}

# compile if not skipped
if (-not $SkipCompilation)
{
    # check if cargo is installed
    if (-not (Get-Command "cargo" -ErrorAction SilentlyContinue))
    {
        Write-Host "cargo could not be found, please install Rust first." -ForegroundColor Red
        exit 1
    }

    Write-Host "Compiling $Repo..." -ForegroundColor Blue
    cargo build --release
}

$Executable = "target\release\${Repo}.exe"

if (-not (Test-Path "$Executable"))
{
    Write-Host "Executable not found at $Executable. Compilation might have failed." -ForegroundColor Red
    exit 1
}

Write-Host "Installing $Repo to $Location..." -ForegroundColor Blue

if (-not (Test-Path "$Location"))
{
    New-Item -ItemType Directory -Force -Path "$Location" | Out-Null
}

Copy-Item -Path "$Executable" -Destination "$Location\${Repo}.exe" -Force

# add to PATH if not already
$PathEnv = [Environment]::GetEnvironmentVariable("Path", [EnvironmentVariableTarget]::Machine)
if (-not($PathEnv -like "*$Location*"))
{
    Write-Host "Adding $Location to Path Environment variable..." -ForegroundColor Blue

    $NewPath = $PathEnv + ";$Location"
    [Environment]::SetEnvironmentVariable("Path", $NewPath, [EnvironmentVariableTarget]::Machine)
    [Environment]::SetEnvironmentVariable("Path", $NewPath, [EnvironmentVariableTarget]::Process)

    Write-Host "Note: Updates to PATH might not be visible until you restart your terminal" -ForegroundColor Yellow
}

Write-Host "$Repo compiled and installed successfully! Location: $Location\${Repo}.exe" -ForegroundColor Green
Write-Host "Run '$Repo --help' to get started" -ForegroundColor Green
