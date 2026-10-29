# Build Conbind for Windows (MSI + NSIS installers and a portable zip).
#
# Usage: scripts\build-windows.ps1 [-Release]
#   (default)   local build: dev bundle ID, no updater signing key needed
#   -Release    full release build (needs TAURI_SIGNING_PRIVATE_KEY)
#
# Needs Visual Studio C++ Build Tools, WebView2, Node.js, pnpm, Rust and bun.
[CmdletBinding()]
param(
  [switch]$Release
)

$ErrorActionPreference = 'Stop'
Set-Location (Join-Path $PSScriptRoot '..')

foreach ($tool in 'pnpm', 'cargo', 'bun') {
  if (-not (Get-Command $tool -ErrorAction SilentlyContinue)) {
    $hint = if ($tool -eq 'bun') { ' bun builds the ext-builder sidecar; install it from https://bun.sh' } else { '' }
    throw "$tool is required but was not found on PATH.$hint"
  }
}

function Invoke-Step([scriptblock]$Command) {
  & $Command
  if ($LASTEXITCODE -ne 0) { throw "Command failed with exit code $LASTEXITCODE" }
}

Invoke-Step { pnpm install --frozen-lockfile }
if ($Release) {
  Invoke-Step { pnpm build }
} else {
  Invoke-Step { pnpm build --local }
}

$releaseDir = 'conbind-launcher/src-tauri/target/release'

# Portable zip: the bare executable plus the resources it loads at runtime.
$portableRoot = Join-Path $releaseDir 'portable'
$portableDir = Join-Path $portableRoot 'conbind'
if (Test-Path $portableRoot) { Remove-Item -Recurse -Force $portableRoot }
New-Item -ItemType Directory -Force -Path $portableDir | Out-Null
Copy-Item (Join-Path $releaseDir 'conbind.exe') $portableDir
foreach ($resource in 'built-in-features', 'resources') {
  $source = Join-Path 'conbind-launcher/src-tauri' $resource
  if (Test-Path $source) { Copy-Item -Recurse $source $portableDir }
}
$arch = if ($env:PROCESSOR_ARCHITECTURE -eq 'ARM64') { 'arm64' } else { 'x64' }
$zipPath = Join-Path $releaseDir "conbind_windows_${arch}_portable.zip"
Compress-Archive -Path (Join-Path $portableDir '*') -DestinationPath $zipPath -Force
Remove-Item -Recurse -Force $portableRoot

Write-Host ''
Write-Host 'Build output:'
Get-ChildItem -Path (Join-Path $releaseDir 'bundle') -Recurse -Include *.msi, *.exe -Depth 1 | ForEach-Object { $_.FullName }
Write-Host $zipPath
