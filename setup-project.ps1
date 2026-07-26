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

# 4. Install backend dependencies
Write-Step "Installing Python backend dependencies..."
Set-Location "$ProjectRoot\backend"
if (-not (Test-Path venv)) {
    python -m venv venv
    Write-Done "Python virtual environment created"
} else {
    Write-Done "Python virtual environment already exists"
}

.\venv\Scripts\Activate.ps1
python -m pip config set global.index-url https://pypi.tuna.tsinghua.edu.cn/simple
python -m pip config set global.trusted-host pypi.tuna.tsinghua.edu.cn

python -m pip install --upgrade pip
pip install -r requirements.txt
Write-Done "Python backend dependencies installed"

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
Write-Host "Run the model download script when available." -ForegroundColor White

Read-Host -Prompt "Press Enter to exit"
