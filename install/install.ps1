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
        Write-Host "Unsupported system architecture: $env:PROCESSOR_ARCHITECTURE" -ForegroundColor Red
        exit 1
    }
}

# set version
if (!$Version)
{
    $Version = (Invoke-RestMethod -Uri "https://api.github.com/repos/$Owner/$Repo/releases/latest").tag_name
}
Write-Host "Target version: $Version" -ForegroundColor Blue

# build download URL
$URL = "${PROXY_PREFIX}https://github.com/$Owner/$Repo/releases/download/$Version/${Repo}_Windows_$Arch.zip"
Write-Host "Downloading $Repo from $URL" -ForegroundColor Blue

# download and extract
Invoke-WebRequest -Uri $URL -OutFile "$Repo.zip"
if (-not(Test-Path "$Repo.zip"))
{
    Write-Host "Download $URL failed" -ForegroundColor Red
    exit 1
}

# extract to $LOCATION
Expand-Archive -Path "$Repo.zip" -DestinationPath "$Location" -Force

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

# remove zip file
Remove-Item "$Repo.zip"

Write-Host "$Repo installed successfully! Location: $Location" -ForegroundColor Green
Write-Host "Run '$Repo --help' to get started" -ForegroundColor Green
