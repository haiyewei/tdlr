# Build script for Android targets
# Requires: Android NDK, Rust targets installed

param(
    [string]$NdkPath = $env:ANDROID_NDK_HOME,
    [string]$Target = "all"
)

if (-not $NdkPath) {
    Write-Error "ANDROID_NDK_HOME not set. Please set it to your NDK path."
    Write-Host "Example: `$env:ANDROID_NDK_HOME = 'C:\Android\ndk\26.1.10909125'"
    exit 1
}

# NDK toolchain bin path
$ToolchainBin = "$NdkPath\toolchains\llvm\prebuilt\windows-x86_64\bin"
if (-not (Test-Path $ToolchainBin)) {
    Write-Error "NDK toolchain not found at: $ToolchainBin"
    exit 1
}

# Add toolchain to PATH
$env:PATH = "$ToolchainBin;$env:PATH"

# Targets to build
$Targets = @(
    "aarch64-linux-android",    # arm64-v8a
    "armv7-linux-androideabi",  # armeabi-v7a
    "x86_64-linux-android",     # x86_64
    "i686-linux-android"        # x86
)

if ($Target -ne "all") {
    $Targets = @($Target)
}

# Install targets if needed
foreach ($t in $Targets) {
    Write-Host "Checking target: $t"
    rustup target add $t 2>$null
}

# Build each target
foreach ($t in $Targets) {
    Write-Host "`n=== Building for $t ===" -ForegroundColor Cyan
    cargo build --release --target $t --features android
    
    if ($LASTEXITCODE -ne 0) {
        Write-Error "Build failed for $t"
        exit 1
    }
}

# Copy outputs to jniLibs structure
$OutputDir = "target\android\jniLibs"
New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

$AbiMap = @{
    "aarch64-linux-android" = "arm64-v8a"
    "armv7-linux-androideabi" = "armeabi-v7a"
    "x86_64-linux-android" = "x86_64"
    "i686-linux-android" = "x86"
}

foreach ($t in $Targets) {
    $Abi = $AbiMap[$t]
    $SrcLib = "target\$t\release\libtdlr_core.so"
    $DstDir = "$OutputDir\$Abi"
    
    if (Test-Path $SrcLib) {
        New-Item -ItemType Directory -Force -Path $DstDir | Out-Null
        Copy-Item $SrcLib (Join-Path $DstDir "libtdlr.so")
        Write-Host "Copied: $DstDir\libtdlr.so" -ForegroundColor Green
    }
}

Write-Host "`nBuild complete! Libraries are in: $OutputDir" -ForegroundColor Green
