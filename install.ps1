# tv-mcp installer (Windows)
# Usage: irm https://raw.githubusercontent.com/FrontToBackCulture/tv-mcp/main/install.ps1 | iex

$ErrorActionPreference = "Stop"

$Repo = "FrontToBackCulture/tv-mcp"
$InstallDir = Join-Path $env:USERPROFILE ".tv-mcp\bin"
$BinaryName = "tv-mcp.exe"
$BinaryPath = Join-Path $InstallDir $BinaryName

# Detect architecture
$Arch = if ([Environment]::Is64BitOperatingSystem) { "x86_64" } else {
  Write-Error "Unsupported architecture: 32-bit Windows is not supported"
  exit 1
}
$Target = "$Arch-pc-windows-msvc"
$AssetName = "tv-mcp-$Target.exe"

Write-Host "tv-mcp installer"
Write-Host "  Platform: Windows $Arch"
Write-Host "  Target:   $Target"
Write-Host ""

# Fetch latest release
$LatestUrl = "https://api.github.com/repos/$Repo/releases/latest"
try {
  $Release = Invoke-RestMethod -Uri $LatestUrl -UseBasicParsing
} catch {
  Write-Error "Failed to fetch latest release from GitHub: $_"
  exit 1
}

$Asset = $Release.assets | Where-Object { $_.name -eq $AssetName } | Select-Object -First 1
if (-not $Asset) {
  Write-Error "No prebuilt binary found for $Target in release $($Release.tag_name)"
  exit 1
}

Write-Host "Downloading $($Release.tag_name) from release..."
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
Invoke-WebRequest -Uri $Asset.browser_download_url -OutFile $BinaryPath -UseBasicParsing

# Unblock (Windows SmartScreen / zone identifier)
Unblock-File -Path $BinaryPath

Write-Host ""
Write-Host "Installed: $BinaryPath"
& $BinaryPath --version
Write-Host ""
Write-Host "Done! tv-mcp will be auto-registered next time you open TV Client."
Write-Host "Or register manually: claude mcp add tv-mcp `"$BinaryPath`""
