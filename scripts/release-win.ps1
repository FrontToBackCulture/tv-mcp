# tv-mcp Windows release
# Usage: .\scripts\release-win.ps1 v0.10.31
#
# Uploads Windows binary to an EXISTING GitHub release.
# Run release-mac.sh on Mac first to create the release.

param(
  [Parameter(Mandatory=$true)][string]$Version
)

$ErrorActionPreference = "Stop"

Push-Location (Join-Path $PSScriptRoot "..")

try {
  # Verify version matches Cargo.toml
  $cargoVersion = "v" + ((Get-Content Cargo.toml | Select-String '^version = ').ToString() -replace '.*"([^"]+)".*','$1')
  if ($Version -ne $cargoVersion) {
    Write-Error "Arg version $Version doesn't match Cargo.toml version $cargoVersion. Run 'git pull' first."
    exit 1
  }

  # Confirm release exists
  try {
    gh release view $Version --json tagName | Out-Null
  } catch {
    Write-Error "Release $Version does not exist. Run release-mac.sh on Mac first."
    exit 1
  }

  # Build Windows binary (always cross-compile to x86_64 so the artifact arch
  # matches the asset name regardless of host arch — important on Parallels
  # Windows running on Apple Silicon, which is ARM64 by default).
  Write-Host "Ensuring x86_64-pc-windows-msvc target is installed..."
  rustup target add x86_64-pc-windows-msvc | Out-Null

  Write-Host "Building Windows (x86_64) binary..."
  cargo build --release --target x86_64-pc-windows-msvc

  $artifact = "tv-mcp-x86_64-pc-windows-msvc.exe"
  Copy-Item "target\x86_64-pc-windows-msvc\release\tv-mcp.exe" $artifact -Force

  # Upload
  Write-Host "Uploading $artifact to release $Version..."
  gh release upload $Version $artifact --clobber

  Remove-Item $artifact -Force

  Write-Host ""
  Write-Host "✅ Windows release done. Both artifacts are now on the release page." -ForegroundColor Green
} finally {
  Pop-Location
}
