# StatsWidget - Windows Build Script
# Run this from an ADMIN PowerShell or regular PowerShell:
#   powershell -ExecutionPolicy Bypass -File build.ps1
#
# Output:
#   - release/StatsWidget.exe          (portable, send this to friends)
#   - release/StatsWidget_Setup.exe    (NSIS installer)

$ErrorActionPreference = "Stop"
$projectRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $projectRoot

Write-Host "=== StatsWidget Build Script ===" -ForegroundColor Cyan

# ---------- 1. Check prerequisites ----------
Write-Host "`n[1/5] Checking prerequisites..." -ForegroundColor Yellow

if (-not (Get-Command node -ErrorAction SilentlyContinue)) {
    Write-Host "ERROR: Node.js not found. Install from https://nodejs.org" -ForegroundColor Red
    exit 1
}
Write-Host "  Node.js : $(node --version)"

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host "ERROR: Rust/Cargo not found. Install from https://rustup.rs" -ForegroundColor Red
    exit 1
}
Write-Host "  Rust    : $(cargo --version)"

if (-not (Get-Command npm -ErrorAction SilentlyContinue)) {
    Write-Host "ERROR: npm not found." -ForegroundColor Red
    exit 1
}
Write-Host "  npm     : $(npm --version)"

# ---------- 2. Install dependencies ----------
Write-Host "`n[2/5] Installing npm dependencies..." -ForegroundColor Yellow
npm install

# ---------- 3. Build the frontend ----------
Write-Host "`n[3/5] Building frontend (Vite + TypeScript)..." -ForegroundColor Yellow
npm run build

# ---------- 4. Build Tauri (Rust + bundle) ----------
Write-Host "`n[4/5] Building Tauri app (this may take several minutes on first run)..." -ForegroundColor Yellow
npx tauri build 2>&1 | ForEach-Object { Write-Host "  $_" }

# ---------- 5. Collect output ----------
Write-Host "`n[5/5] Collecting outputs..." -ForegroundColor Yellow

$releaseDir = Join-Path $projectRoot "release"
if (-not (Test-Path $releaseDir)) {
    New-Item -ItemType Directory -Path $releaseDir | Out-Null
}

# The standalone exe (Tauri embeds all web assets into the binary)
$builtExe = Join-Path $projectRoot "src-tauri\target\release\tauri-app.exe"
if (Test-Path $builtExe) {
    Copy-Item $builtExe (Join-Path $releaseDir "StatsWidget.exe") -Force
    Write-Host "  [OK] Portable EXE : release/StatsWidget.exe" -ForegroundColor Green
} else {
    Write-Host "  [!!] Standalone exe not found at: $builtExe" -ForegroundColor Red
    Write-Host "       Check src-tauri/target/release/ for the .exe" -ForegroundColor Red
}

# The NSIS installer
$nsisInstaller = Join-Path $projectRoot "src-tauri\target\release\bundle\nsis\StatsWidget_0.1.0_x64-setup.exe"
if (Test-Path $nsisInstaller) {
    Copy-Item $nsisInstaller (Join-Path $releaseDir "StatsWidget_Setup.exe") -Force
    Write-Host "  [OK] NSIS Setup  : release/StatsWidget_Setup.exe" -ForegroundColor Green
} else {
    # Try glob for any NSIS output
    $nsisDir = Join-Path $projectRoot "src-tauri\target\release\bundle\nsis"
    if (Test-Path $nsisDir) {
        Get-ChildItem $nsisDir -Filter "*.exe" | ForEach-Object {
            Copy-Item $_.FullName (Join-Path $releaseDir $_.Name) -Force
            Write-Host "  [OK] NSIS Setup  : release/$($_.Name)" -ForegroundColor Green
        }
    } else {
        Write-Host "  [!!] NSIS installer not found." -ForegroundColor Red
    }
}

# ---------- Done ----------
Write-Host "`n=== Build complete ===" -ForegroundColor Cyan
Write-Host ""
Write-Host "  Portable EXE : release/StatsWidget.exe"
Write-Host "     (Send this directly to friends)"
Write-Host "     (Requires WebView2 -- preinstalled on Win 10 1803+ / Win 11)"
Write-Host ""
Write-Host "  NSIS Installer : release/StatsWidget_Setup.exe"
Write-Host "     (Standard installer with start menu shortcut)"
Write-Host ""
Write-Host "Press any key to open the release folder..."
$null = $Host.UI.RawUI.ReadKey("NoEcho,IncludeKeyDown")
Invoke-Item $releaseDir
