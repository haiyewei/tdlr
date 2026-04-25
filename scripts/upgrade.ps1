[CmdletBinding()]
param(
    [ValidateSet("Auto", "Local", "Remote")]
    [string]$Source = $(if ($env:TDLR_INSTALL_SOURCE) { $env:TDLR_INSTALL_SOURCE } else { "Remote" }),
    [string]$InstallDir = $env:TDLR_INSTALL_DIR,
    [string]$Version = $(if ($env:TDLR_INSTALL_VERSION) { $env:TDLR_INSTALL_VERSION } else { "latest" }),
    [switch]$Proxy,
    [switch]$SkipGitHistory
)

$ErrorActionPreference = "Stop"

$BinaryName = "tdlr.exe"
$BinaryBaseName = "tdlr"
$RemoteRepoOwner = if ($env:TDLR_REMOTE_REPO_OWNER) { $env:TDLR_REMOTE_REPO_OWNER } else { "haiyewei" }
$RemoteRepoName = if ($env:TDLR_REMOTE_REPO_NAME) { $env:TDLR_REMOTE_REPO_NAME } else { "tdlr" }
$RemoteBaseUrl = $env:TDLR_REMOTE_BASE_URL
$ProxyPrefix = if ($Proxy) { "https://gh-proxy.com/" } else { "" }
$HasExplicitInstallDir = -not [string]::IsNullOrWhiteSpace($InstallDir)
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
    return (Join-Path $env:LOCALAPPDATA "Programs\tdlr\bin")
}

function Expand-TemplatePath {
    param(
        [string]$PathValue
    )

    if ([string]::IsNullOrWhiteSpace($PathValue)) {
        return $null
    }

    $expanded = [regex]::Replace($PathValue, '\$[Ee]nv:([A-Za-z_][A-Za-z0-9_]*)', {
            param($match)

            $name = $match.Groups[1].Value
            $value = [Environment]::GetEnvironmentVariable($name)
            if ([string]::IsNullOrWhiteSpace($value)) {
                return $match.Value
            }

            return $value
        })

    if ($expanded.StartsWith('$HOME')) {
        $expanded = $HOME + $expanded.Substring(5)
    }

    return [Environment]::ExpandEnvironmentVariables($expanded).Trim('"')
}

function Normalize-DirectoryPath {
    param(
        [string]$PathValue
    )

    $expanded = Expand-TemplatePath -PathValue $PathValue
    if ([string]::IsNullOrWhiteSpace($expanded)) {
        return $null
    }

    try {
        return [System.IO.Path]::GetFullPath($expanded).TrimEnd('\', '/')
    }
    catch {
        return $expanded.TrimEnd('\', '/')
    }
}

function Add-UniqueDirectory {
    param(
        [System.Collections.Generic.List[string]]$Directories,
        [string]$PathValue
    )

    $normalized = Normalize-DirectoryPath -PathValue $PathValue
    if ([string]::IsNullOrWhiteSpace($normalized)) {
        return
    }

    if (-not $Directories.Contains($normalized)) {
        [void]$Directories.Add($normalized)
    }
}

function Invoke-NativeCommandQuietly {
    param(
        [string]$FilePath,
        [string[]]$Arguments
    )

    $processInfo = New-Object System.Diagnostics.ProcessStartInfo
    $processInfo.FileName = $FilePath
    $processInfo.UseShellExecute = $false
    $processInfo.CreateNoWindow = $true
    $processInfo.RedirectStandardOutput = $true
    $processInfo.RedirectStandardError = $true

    if ($Arguments) {
        $escapedArguments = foreach ($argument in $Arguments) {
            if ($null -eq $argument) {
                '""'
            }
            else {
                '"' + ($argument -replace '"', '\"') + '"'
            }
        }

        $processInfo.Arguments = [string]::Join(" ", $escapedArguments)
    }

    $process = New-Object System.Diagnostics.Process
    $process.StartInfo = $processInfo

    try {
        [void]$process.Start()
        $stdout = $process.StandardOutput.ReadToEnd()
        $stderr = $process.StandardError.ReadToEnd()
        $process.WaitForExit()

        if ($process.ExitCode -ne 0) {
            return $null
        }

        if ([string]::IsNullOrWhiteSpace($stdout)) {
            return @()
        }

        return ($stdout -split "\r?\n")
    }
    finally {
        if ($process) {
            $process.Dispose()
        }
        if ($stderr) {
            $null = $stderr
        }
    }
}

function Get-DirectoriesFromPath {
    param(
        [string]$PathValue
    )

    $directories = [System.Collections.Generic.List[string]]::new()

    if ([string]::IsNullOrWhiteSpace($PathValue)) {
        return $directories
    }

    foreach ($entry in ($PathValue -split ';')) {
        if ([string]::IsNullOrWhiteSpace($entry)) {
            continue
        }

        $candidate = Join-Path $entry $BinaryName
        if (Test-Path $candidate -PathType Leaf) {
            Add-UniqueDirectory -Directories $directories -PathValue $entry
        }
    }

    return $directories
}

function Get-LegacyInstallDirsFromGit {
    $directories = [System.Collections.Generic.List[string]]::new()

    if ($SkipGitHistory) {
        return $directories
    }

    if (-not $RepoRoot -or -not (Test-Path (Join-Path $RepoRoot ".git"))) {
        return $directories
    }

    $git = Get-Command git -ErrorAction SilentlyContinue
    if (-not $git) {
        return $directories
    }

    $scriptPaths = @(
        "install/install.ps1",
        "install/install_local.ps1",
        "install/install_daily.ps1",
        "scripts/install.ps1"
    )

    $logArgs = @("-C", $RepoRoot, "log", "--format=%H", "--") + $scriptPaths
    $commits = (Invoke-NativeCommandQuietly -FilePath $git.Source -Arguments $logArgs) |
        Where-Object { -not [string]::IsNullOrWhiteSpace($_) } |
        Select-Object -Unique

    foreach ($commit in $commits) {
        foreach ($scriptPath in $scriptPaths) {
            $spec = "{0}:{1}" -f $commit, $scriptPath
            $content = Invoke-NativeCommandQuietly -FilePath $git.Source -Arguments @("-C", $RepoRoot, "show", $spec)
            if (-not $content) {
                continue
            }

            foreach ($line in $content) {
                if ($line -match '^\$Location\s*=\s*"([^"]+)"') {
                    Add-UniqueDirectory -Directories $directories -PathValue $Matches[1]
                }
            }
        }
    }

    return $directories
}

function Get-CandidateInstallDirs {
    $directories = [System.Collections.Generic.List[string]]::new()

    Add-UniqueDirectory -Directories $directories -PathValue $InstallDir

    if (-not $HasExplicitInstallDir) {
        $resolvedCommand = Get-Command $BinaryBaseName -CommandType Application -ErrorAction SilentlyContinue |
            Select-Object -First 1
        if ($resolvedCommand -and $resolvedCommand.Source) {
            Add-UniqueDirectory -Directories $directories -PathValue (Split-Path -Parent $resolvedCommand.Source)
        }

        Add-UniqueDirectory -Directories $directories -PathValue (Get-DefaultInstallDir)

        foreach ($pathValue in @(
                [Environment]::GetEnvironmentVariable("Path", "Process"),
                [Environment]::GetEnvironmentVariable("Path", "User"),
                [Environment]::GetEnvironmentVariable("Path", "Machine")
            )) {
            foreach ($directory in (Get-DirectoriesFromPath -PathValue $pathValue)) {
                Add-UniqueDirectory -Directories $directories -PathValue $directory
            }
        }

        foreach ($directory in (Get-LegacyInstallDirsFromGit)) {
            Add-UniqueDirectory -Directories $directories -PathValue $directory
        }

        Add-UniqueDirectory -Directories $directories -PathValue (Join-Path $env:SystemDrive "tdlr")
    }

    return $directories
}

function Resolve-UpgradeInstallDir {
    $matchedDirs = [System.Collections.Generic.List[string]]::new()

    foreach ($candidateDir in (Get-CandidateInstallDirs)) {
        $binaryPath = Join-Path $candidateDir $BinaryName
        if (Test-Path $binaryPath -PathType Leaf) {
            Add-UniqueDirectory -Directories $matchedDirs -PathValue $candidateDir
        }
    }

    if ($matchedDirs.Count -eq 0) {
        if ($HasExplicitInstallDir) {
            $normalized = Normalize-DirectoryPath -PathValue $InstallDir
            if ([string]::IsNullOrWhiteSpace($normalized)) {
                $normalized = $InstallDir
            }

            Write-Host "[tdlr] no existing install found in $normalized; installing there"
            return $normalized
        }

        throw "[tdlr] no installed binary found in detected directories. Use -InstallDir to target a directory or run the installer first."
    }

    if ($matchedDirs.Count -gt 1) {
        Write-Host "[tdlr] detected multiple install directories; upgrading $($matchedDirs[0])"
        foreach ($extraDir in ($matchedDirs | Select-Object -Skip 1)) {
            Write-Host "[tdlr] additional installed copy detected at $extraDir"
        }
    }
    else {
        Write-Host "[tdlr] upgrading $($matchedDirs[0])"
    }

    return $matchedDirs[0]
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

    Write-Host "[tdlr] no local binary found, building release binary with cargo build --release --bin tdlr"

    Push-Location $RepoRoot
    try {
        & $cargo.Source build --release --bin tdlr
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
            throw "[tdlr] unsupported system architecture for remote upgrade: $env:PROCESSOR_ARCHITECTURE"
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
    $tempRoot = Join-Path $env:TEMP ("tdlr-upgrade-" + [System.Guid]::NewGuid().ToString("N"))
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

function Test-FileInUseError {
    param(
        [System.Management.Automation.ErrorRecord]$ErrorRecord
    )

    $exception = $ErrorRecord.Exception
    while ($exception) {
        if ($exception -is [System.IO.IOException] -or $exception -is [System.UnauthorizedAccessException]) {
            if ($exception.Message -match 'being used by another process|another process') {
                return $true
            }
        }

        $exception = $exception.InnerException
    }

    return $false
}

function Get-LockingProcessIds {
    param(
        [string]$PathValue
    )

    if (-not (Test-Path $PathValue -PathType Leaf)) {
        return @()
    }

    $normalizedTarget = [System.IO.Path]::GetFullPath($PathValue)
    $processName = [System.IO.Path]::GetFileNameWithoutExtension($BinaryName)
    $matches = @()

    foreach ($process in (Get-Process -Name $processName -ErrorAction SilentlyContinue)) {
        $processPath = $null

        try {
            $processPath = $process.Path
        }
        catch {
            continue
        }

        if ([string]::IsNullOrWhiteSpace($processPath)) {
            continue
        }

        $normalizedProcessPath = $null
        try {
            $normalizedProcessPath = [System.IO.Path]::GetFullPath($processPath)
        }
        catch {
            $normalizedProcessPath = $processPath
        }

        if ($normalizedProcessPath -ieq $normalizedTarget) {
            $matches += $process.Id
        }
    }

    return $matches | Select-Object -Unique
}

function Install-BinaryWithRetry {
    param(
        [string]$SourcePath,
        [string]$DestinationPath
    )

    $maxAttempts = 12
    $delayMs = 500
    $lastError = $null

    for ($attempt = 1; $attempt -le $maxAttempts; $attempt++) {
        try {
            Copy-Item -LiteralPath $SourcePath -Destination $DestinationPath -Force
            return
        }
        catch {
            $lastError = $_

            if (-not (Test-FileInUseError -ErrorRecord $_)) {
                throw
            }

            if ($attempt -lt $maxAttempts) {
                Start-Sleep -Milliseconds $delayMs
                continue
            }
        }
    }

    $lockingProcessIds = Get-LockingProcessIds -PathValue $DestinationPath
    if ($lockingProcessIds.Count -gt 0) {
        throw "[tdlr] cannot update $DestinationPath because it is in use by running tdlr process(es): $($lockingProcessIds -join ', '). Close them and rerun the upgrader."
    }

    if ($lastError) {
        throw "[tdlr] cannot update $DestinationPath because it is in use by another process. Close programs using this file and rerun the upgrader."
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

$TargetInstallDir = Resolve-UpgradeInstallDir
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
    New-Item -ItemType Directory -Force -Path $TargetInstallDir | Out-Null
    $InstallPath = Join-Path $TargetInstallDir $BinaryName

    if ([System.IO.Path]::GetFullPath($SourceBinary) -ne [System.IO.Path]::GetFullPath($InstallPath)) {
        Install-BinaryWithRetry -SourcePath $SourceBinary -DestinationPath $InstallPath
    }

    Write-Host "[tdlr] upgraded $InstallPath"
    Add-InstallDirToUserPath -Entry $TargetInstallDir
    Write-Host "[tdlr] restart your terminal to pick up the persisted PATH entry in new sessions"
    Write-Host "[tdlr] run 'tdlr --help' to get started"
}
finally {
    if ($CleanupPath -and (Test-Path $CleanupPath)) {
        Remove-Item -LiteralPath $CleanupPath -Recurse -Force
    }
}
