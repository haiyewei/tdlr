param(
    [String]$Version,
    [Boolean]$Proxy = $False
)

$Owner = "haiyewei"
$Repo = "tdlr"
$Location = "$Env:SystemDrive\tdlr"

$ErrorActionPreference = "Stop"

# check if run as admin
if (-not ([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]"Administrator"))
{
    Write-Host "Please run this script as Administrator" -ForegroundColor Red
    exit 1
}

# use proxy if argument is passed
$PROXY_PREFIX = ""
if ($Proxy)
{
    $PROXY_PREFIX = "https://mirror.ghproxy.com/"
    Write-Host "Using GitHub proxy: $PROXY_PREFIX" -ForegroundColor Blue
}

# Set download ARCH based on system architecture (only 64bit supported)
$Arch = ""
switch ($env:PROCESSOR_ARCHITECTURE)
{
    "AMD64" {
        $Arch = "64bit"
    }
    default {
        Write-Host "Unsupported system architecture: $env:PROCESSOR_ARCHITECTURE. Only x86_64 is supported on Windows." -ForegroundColor Red
        exit 1
    }
}

# set version
if (!$Version)
{
    Write-Host "Fetching latest version..." -ForegroundColor Blue
    try {
        $Version = (Invoke-RestMethod -Uri "https://api.github.com/repos/$Owner/$Repo/releases/latest").tag_name
    } catch {
        Write-Host "Failed to fetch latest version from GitHub API" -ForegroundColor Red
        exit 1
    }
}
Write-Host "Target version: $Version" -ForegroundColor Blue

# build download URL
$URL = "${PROXY_PREFIX}https://github.com/$Owner/$Repo/releases/download/$Version/${Repo}_Windows_$Arch.zip"
Write-Host "Downloading $Repo from $URL" -ForegroundColor Blue

# Create temporary file for download
$TempFile = [System.IO.Path]::GetTempFileName() + ".zip"

try {
    # download and extract
    Invoke-WebRequest -Uri $URL -OutFile $TempFile -UseBasicParsing
    
    if (-not(Test-Path $TempFile))
    {
        Write-Host "Download $URL failed" -ForegroundColor Red
        exit 1
    }

    # ensure $LOCATION exists
    if (-not (Test-Path $Location)) {
        New-Item -ItemType Directory -Path $Location -Force | Out-Null
    }

    # extract to $LOCATION
    Write-Host "Extracting to $Location..." -ForegroundColor Blue
    Expand-Archive -Path $TempFile -DestinationPath $Location -Force
}
finally {
    # remove temp file
    if (Test-Path $TempFile) {
        Remove-Item $TempFile -Force
    }
}

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

Write-Host "$Repo installed successfully! Location: $Location" -ForegroundColor Green
Write-Host "Run '$Repo --help' to get started" -ForegroundColor Green

