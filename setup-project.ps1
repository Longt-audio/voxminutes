# VoxMinutes MVP Project Setup
# Run this script after install-dev-env.ps1 has completed and in a new PowerShell window

$ErrorActionPreference = "Stop"

$ProjectRoot = $PSScriptRoot

function Write-Step {
    param([string]$Message)
    Write-Host "`n[STEP] $Message" -ForegroundColor Cyan
}

function Write-Done {
    param([string]$Message)
    Write-Host "[OK] $Message" -ForegroundColor Green
}

function Set-RustMirrors {
    # Set rustup to use USTC mirror for this session
    $env:RUSTUP_DIST_SERVER = "https://mirrors.ustc.edu.cn/rustup"
    $env:RUSTUP_UPDATE_ROOT = "https://mirrors.ustc.edu.cn/rustup/rustup"

    $cargoDir = "$env:USERPROFILE\.cargo"
    $cargoConfig = Join-Path $cargoDir "config.toml"
    New-Item -ItemType Directory -Path $cargoDir -Force | Out-Null
    @"
[source.crates-io]
replace-with = 'ustc'

[source.ustc]
registry = "https://mirrors.ustc.edu.cn/crates.io-index/"

[source.tuna]
registry = "https://mirrors.tuna.tsinghua.edu.cn/crates.io-index/"
"@ | Out-File -FilePath $cargoConfig -Encoding utf8 -Force
}

Set-Location $ProjectRoot

# 1. Ensure Rust default toolchain is set
Write-Step "Configuring Rust toolchain..."
Set-RustMirrors
# Try to install with minimal profile; if mirror fails, use nightly/latest fallback
$rustInstalled = $false
try {
    rustup toolchain install stable --profile minimal
    rustup default stable
    rustup target add x86_64-pc-windows-msvc
    $rustInstalled = $true
    Write-Done "Rust toolchain configured via stable"
} catch {
    Write-Warning "Failed to install stable via mirror: $_"
}

if (-not $rustInstalled) {
    try {
        Write-Host "Trying to install via Tsinghua mirror..." -ForegroundColor Yellow
        $env:RUSTUP_DIST_SERVER = "https://mirrors.tuna.tsinghua.edu.cn/rustup"
        $env:RUSTUP_UPDATE_ROOT = "https://mirrors.tuna.tsinghua.edu.cn/rustup/rustup"
        rustup toolchain install stable --profile minimal
        rustup default stable
        rustup target add x86_64-pc-windows-msvc
        $rustInstalled = $true
        Write-Done "Rust toolchain configured via Tsinghua mirror"
    } catch {
        Write-Warning "Failed to install via Tsinghua mirror: $_"
    }
}

if (-not $rustInstalled) {
    Write-Error "Could not install Rust stable toolchain. Please install manually from https://rustup.rs and rerun."
    Read-Host -Prompt "Press Enter to exit"
    exit 1
}

# 2. Install root node_modules
Write-Step "Installing root dependencies..."
if (Test-Path "$ProjectRoot\package.json") {
    pnpm install
    Write-Done "Root dependencies installed"
} else {
    Write-Host "No root package.json found, skipping root install" -ForegroundColor Yellow
}

# 3. Install frontend dependencies
Write-Step "Installing frontend dependencies..."
Set-Location "$ProjectRoot\frontend"
if (-not (Test-Path node_modules)) {
    pnpm install
    Write-Done "Frontend dependencies installed"
} else {
    Write-Done "Frontend dependencies already present"
}

# Approve builds for native dependencies
Write-Step "Approving native builds..."
pnpm approve-builds msgpackr-extract
Write-Done "Native builds approved"

# 4. Prepare Windows sidecars (ffmpeg.exe + llama-helper.exe)
#    ⚠️ 旧的「Python backend」步骤已删除：MVP 运行时是纯 Rust，`backend/` 目录
#    在 .gitignore 里被标为「已不存在的遗留 Python 后端」，新克隆的机器上根本没有，
#    照旧脚本跑会直接 Set-Location 失败、整个 setup 中断。
Write-Step "Preparing Windows sidecars (ffmpeg + llama-helper)..."
Set-Location $ProjectRoot
& "$ProjectRoot\frontend\scripts\prepare-windows-sidecars.ps1"
if ($LASTEXITCODE -ne 0) {
    Write-Warning "侧车准备失败 —— 打包会失败，但 `pnpm tauri:dev` 开发仍可继续。"
} else {
    Write-Done "Windows sidecars ready"
}

# 5. Install Rust dependencies (verify build)
Write-Step "Verifying Rust workspace..."
Set-Location $ProjectRoot
Set-RustMirrors
# Initial check without heavy build
cargo check --workspace
Write-Done "Rust workspace checked"

Write-Host "`n==============================================" -ForegroundColor Blue
Write-Host "Project Setup Complete" -ForegroundColor Blue
Write-Host "==============================================" -ForegroundColor Blue
Write-Host "To start development:" -ForegroundColor Cyan
Write-Host "  cd frontend" -ForegroundColor White
Write-Host "  pnpm tauri:dev" -ForegroundColor White
Write-Host "`nNote: You may need to download ASR models before running." -ForegroundColor White
Write-Host "Open the app and use the model download page (Settings → local models)." -ForegroundColor White
Write-Host "`nTo build a Windows installer:" -ForegroundColor Cyan
Write-Host "  cd frontend" -ForegroundColor White
Write-Host "  pnpm tauri build" -ForegroundColor White

Read-Host -Prompt "Press Enter to exit"
