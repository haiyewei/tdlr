[CmdletBinding()]
param(
    [string]$InstallDir = $env:TDLR_INSTALL_DIR,
    [switch]$KeepPath,
    [switch]$SkipGitHistory,
    [switch]$RemoveUserData,
    [switch]$KeepUserData
)

$ErrorActionPreference = "Stop"

$BinaryName = "tdlr.exe"
$BinaryBaseName = "tdlr"
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

if ($RemoveUserData -and $KeepUserData) {
    throw "Cannot specify both -RemoveUserData and -KeepUserData."
}

function Get-DefaultInstallDir {
    return (Join-Path $env:LOCALAPPDATA "Programs\tdlr\bin")
}

function Get-DefaultUserDataDir {
    $appData = [Environment]::GetFolderPath([Environment+SpecialFolder]::ApplicationData)
    if (-not [string]::IsNullOrWhiteSpace($appData)) {
        return Join-Path $appData "tdlr"
    }

    if (-not [string]::IsNullOrWhiteSpace($env:XDG_CONFIG_HOME)) {
        return Join-Path $env:XDG_CONFIG_HOME "tdlr"
    }

    return Join-Path $HOME ".config\tdlr"
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
    $commits = (& $git.Source @logArgs 2>$null) |
        Where-Object { -not [string]::IsNullOrWhiteSpace($_) } |
        Select-Object -Unique

    foreach ($commit in $commits) {
        foreach ($scriptPath in $scriptPaths) {
            $spec = "{0}:{1}" -f $commit, $scriptPath
            $content = & $git.Source -C $RepoRoot show $spec 2>$null
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

function Test-OwnedInstallDir {
    param(
        [string]$DirectoryPath
    )

    $leaf = Split-Path -Leaf $DirectoryPath
    $parent = Split-Path -Parent $DirectoryPath

    if ($leaf -eq "tdlr") {
        return $true
    }

    if ($leaf -eq "bin" -and (Split-Path -Leaf $parent) -eq "tdlr") {
        return $true
    }

    return $false
}

function Test-PathEntry {
    param(
        [string]$PathValue,
        [string]$Entry
    )

    if ([string]::IsNullOrWhiteSpace($PathValue) -or [string]::IsNullOrWhiteSpace($Entry)) {
        return $false
    }

    $normalizedEntry = (Normalize-DirectoryPath -PathValue $Entry)
    foreach ($segment in ($PathValue -split ';')) {
        $normalizedSegment = Normalize-DirectoryPath -PathValue $segment
        if ($normalizedSegment -and $normalizedSegment -ieq $normalizedEntry) {
            return $true
        }
    }

    return $false
}

function Remove-PathEntry {
    param(
        [string]$PathValue,
        [string]$Entry
    )

    if ([string]::IsNullOrWhiteSpace($PathValue) -or [string]::IsNullOrWhiteSpace($Entry)) {
        return $PathValue
    }

    $normalizedEntry = Normalize-DirectoryPath -PathValue $Entry
    $kept = foreach ($segment in ($PathValue -split ';')) {
        if ([string]::IsNullOrWhiteSpace($segment)) {
            continue
        }

        $normalizedSegment = Normalize-DirectoryPath -PathValue $segment
        if ($normalizedSegment -and $normalizedSegment -ieq $normalizedEntry) {
            continue
        }

        $segment
    }

    return ($kept -join ';')
}

function Remove-EmptyDirectoryIfOwned {
    param(
        [string]$DirectoryPath
    )

    if (-not (Test-Path $DirectoryPath -PathType Container)) {
        return
    }

    if (-not (Test-OwnedInstallDir -DirectoryPath $DirectoryPath)) {
        return
    }

    $children = Get-ChildItem -LiteralPath $DirectoryPath -Force
    if ($children.Count -gt 0) {
        return
    }

    Remove-Item -LiteralPath $DirectoryPath -Force
    Write-Host "[tdlr] removed empty directory $DirectoryPath"

    $parent = Split-Path -Parent $DirectoryPath
    if ((Split-Path -Leaf $DirectoryPath) -eq "bin" -and (Split-Path -Leaf $parent) -eq "tdlr") {
        if (Test-Path $parent -PathType Container) {
            $parentChildren = Get-ChildItem -LiteralPath $parent -Force
            if ($parentChildren.Count -eq 0) {
                Remove-Item -LiteralPath $parent -Force
                Write-Host "[tdlr] removed empty directory $parent"
            }
        }
    }
}

function Supports-InteractivePrompt {
    try {
        return [Environment]::UserInteractive -and -not [Console]::IsInputRedirected -and -not [Console]::IsOutputRedirected
    }
    catch {
        return $false
    }
}

function Should-RemoveUserData {
    param(
        [string]$DirectoryPath
    )

    if ([string]::IsNullOrWhiteSpace($DirectoryPath) -or -not (Test-Path $DirectoryPath -PathType Container)) {
        return $false
    }

    if ($RemoveUserData) {
        return $true
    }

    if ($KeepUserData) {
        Write-Host "[tdlr] preserved user data at $DirectoryPath"
        return $false
    }

    if (-not (Supports-InteractivePrompt)) {
        Write-Host "[tdlr] preserved user data at $DirectoryPath (non-interactive mode). Use -RemoveUserData to delete it."
        return $false
    }

    $answer = Read-Host "[tdlr] remove user data at '$DirectoryPath'? This deletes auth sessions and account metadata [y/N]"
    return $answer -match '^(?i)y(?:es)?$'
}

function Remove-UserDataDirectory {
    param(
        [string]$DirectoryPath
    )

    if ([string]::IsNullOrWhiteSpace($DirectoryPath) -or -not (Test-Path $DirectoryPath -PathType Container)) {
        return
    }

    Remove-Item -LiteralPath $DirectoryPath -Recurse -Force
    Write-Host "[tdlr] removed user data directory $DirectoryPath"
}

function Get-CandidateInstallDirs {
    $directories = [System.Collections.Generic.List[string]]::new()

    Add-UniqueDirectory -Directories $directories -PathValue $InstallDir

    if (-not $HasExplicitInstallDir) {
        Add-UniqueDirectory -Directories $directories -PathValue (Get-DefaultInstallDir)

        $resolvedCommand = Get-Command $BinaryBaseName -CommandType Application -ErrorAction SilentlyContinue |
            Select-Object -First 1
        if ($resolvedCommand -and $resolvedCommand.Source) {
            Add-UniqueDirectory -Directories $directories -PathValue (Split-Path -Parent $resolvedCommand.Source)
        }

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

$candidateDirs = Get-CandidateInstallDirs
if ($candidateDirs.Count -eq 0) {
    Write-Host "[tdlr] no candidate install directories found."
    exit 0
}

$removedAnyBinary = $false
foreach ($candidateDir in $candidateDirs) {
    $binaryPath = Join-Path $candidateDir $BinaryName
    if (Test-Path $binaryPath -PathType Leaf) {
        Remove-Item -LiteralPath $binaryPath -Force
        Write-Host "[tdlr] removed $binaryPath"
        $removedAnyBinary = $true
    }

    Remove-EmptyDirectoryIfOwned -DirectoryPath $candidateDir
}

if (-not $KeepPath) {
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    foreach ($candidateDir in $candidateDirs) {
        $userPath = Remove-PathEntry -PathValue $userPath -Entry $candidateDir
    }
    [Environment]::SetEnvironmentVariable("Path", $userPath, "User")

    $processPath = [Environment]::GetEnvironmentVariable("Path", "Process")
    foreach ($candidateDir in $candidateDirs) {
        $processPath = Remove-PathEntry -PathValue $processPath -Entry $candidateDir
    }
    $env:Path = $processPath

    $machinePath = [Environment]::GetEnvironmentVariable("Path", "Machine")
    if (-not [string]::IsNullOrWhiteSpace($machinePath)) {
        $newMachinePath = $machinePath
        foreach ($candidateDir in $candidateDirs) {
            $newMachinePath = Remove-PathEntry -PathValue $newMachinePath -Entry $candidateDir
        }

        if ($newMachinePath -ne $machinePath) {
            try {
                [Environment]::SetEnvironmentVariable("Path", $newMachinePath, "Machine")
                Write-Host "[tdlr] removed matching machine PATH entries"
            }
            catch {
                Write-Host "[tdlr] detected legacy machine PATH entries but could not update them without elevation"
            }
        }
    }

    Write-Host "[tdlr] removed matching PATH entries from user and current-session PATH"
}

$userDataDir = Normalize-DirectoryPath -PathValue (Get-DefaultUserDataDir)
if ($userDataDir -and (Test-Path $userDataDir -PathType Container)) {
    if (Should-RemoveUserData -DirectoryPath $userDataDir) {
        Remove-UserDataDirectory -DirectoryPath $userDataDir
    }
}
elseif ($RemoveUserData -and $userDataDir) {
    Write-Host "[tdlr] no user data directory found at $userDataDir"
}

if ($removedAnyBinary) {
    Write-Host "[tdlr] uninstall complete"
}
else {
    Write-Host "[tdlr] no installed binary found in detected directories."
}
