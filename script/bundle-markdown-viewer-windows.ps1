[CmdletBinding()]
param(
    [ValidateSet("x86_64")]
    [string]$Architecture = "x86_64",

    [string]$ReleaseVersion,
    [string]$Channel,
    [string]$OutputDir,

    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$workspaceRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Set-Location $workspaceRoot

$env:ZED_WORKSPACE = $workspaceRoot

if (-not $ReleaseVersion) {
    $ReleaseVersion = & "$workspaceRoot\script\get-crate-version.ps1" "markdown_viewer"
}

$releaseChannelFile = Join-Path $workspaceRoot "crates\markdown_viewer\RELEASE_CHANNEL"
if (-not $Channel) {
    if (Test-Path $releaseChannelFile) {
        $Channel = (Get-Content $releaseChannelFile -Raw).Trim()
    }
    else {
        $Channel = "dev"
    }
}

$targetTriple = switch ($Architecture) {
    "x86_64" { "x86_64-pc-windows-msvc" }
    default { throw "Unsupported architecture: $Architecture" }
}

$archLabel = switch ($Architecture) {
    "x86_64" { "x64" }
    default { throw "Unsupported architecture: $Architecture" }
}

$cargoTargetRoot = if ($env:CARGO_TARGET_DIR) {
    $env:CARGO_TARGET_DIR
}
else {
    Join-Path $workspaceRoot "target"
}

if (-not $OutputDir) {
    $OutputDir = Join-Path $workspaceRoot "target\dist\markdown_viewer"
}

$bundleName = "markdown-viewer-$ReleaseVersion-$Channel-windows-$archLabel"
$bundleDir = Join-Path $OutputDir $bundleName
$zipPath = Join-Path $OutputDir "$bundleName.zip"
$shaPath = "$zipPath.sha256"

if (-not $SkipBuild) {
    cargo build --release --package markdown_viewer --target $targetTriple
}

$binaryDir = Join-Path $cargoTargetRoot "$targetTriple\release"
$binaryPath = Join-Path $binaryDir "markdown_viewer.exe"
$pdbPath = Join-Path $binaryDir "markdown_viewer.pdb"

if (-not (Test-Path $binaryPath)) {
    throw "Expected built binary at '$binaryPath', but it was not found."
}

New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null
if (Test-Path $bundleDir) {
    Remove-Item -Recurse -Force $bundleDir
}
if (Test-Path $zipPath) {
    Remove-Item -Force $zipPath
}
if (Test-Path $shaPath) {
    Remove-Item -Force $shaPath
}

New-Item -ItemType Directory -Force -Path $bundleDir | Out-Null
Copy-Item $binaryPath (Join-Path $bundleDir "markdown_viewer.exe")

if (Test-Path $pdbPath) {
    Copy-Item $pdbPath (Join-Path $bundleDir "markdown_viewer.pdb")
}

Copy-Item (Join-Path $workspaceRoot "LICENSE-GPL") (Join-Path $bundleDir "LICENSE-GPL.txt")

@"
Markdown Viewer
===============

Version: $ReleaseVersion
Channel: $Channel
Architecture: $archLabel

This is a portable Windows build of the standalone Markdown Viewer.

Usage
-----
- Double-click `markdown_viewer.exe` to open an empty viewer window.
- Run `markdown_viewer.exe <PATH>` from PowerShell or CMD to open one or more Markdown files.

Notes
-----
- The application is read-only and optimized for fast local Markdown viewing.
- Markdown assets are bundled into the executable; no extra runtime files are required.
- This package includes `markdown_viewer.pdb` when available to preserve debug symbols.
"@ | Set-Content -Path (Join-Path $bundleDir "README.txt")

Compress-Archive -Path $bundleDir -DestinationPath $zipPath -CompressionLevel Optimal

$hash = (Get-FileHash -Algorithm SHA256 $zipPath).Hash.ToLowerInvariant()
"$hash *$(Split-Path -Leaf $zipPath)" | Set-Content -Path $shaPath

Write-Host "Created bundle directory: $bundleDir"
Write-Host "Created archive: $zipPath"
Write-Host "Created checksum: $shaPath"
