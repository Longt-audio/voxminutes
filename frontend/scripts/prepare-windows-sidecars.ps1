# VoxMinutes — Windows 侧车（externalBin）准备脚本
#
# 作用：把 `tauri build` 需要的两个侧车放到 Tauri 约定的位置与命名：
#   frontend/src-tauri/binaries/ffmpeg-x86_64-pc-windows-msvc.exe
#   frontend/src-tauri/binaries/llama-helper-x86_64-pc-windows-msvc.exe
#
# 为什么必须有：`tauri.conf.json` 的 `bundle.externalBin` 是
# `["binaries/ffmpeg", "binaries/llama-helper"]`，Tauri 会在构建时按
# `<name>-<target-triple>[.exe]` 去找文件，缺一个就直接报错退出。
# `binaries/` 目录在 .gitignore 里（二进制不入库），所以每台新机器都要跑一次。
#
# 用法（在仓库根目录）：
#   pwsh -File frontend/scripts/prepare-windows-sidecars.ps1
#
# 前置：Rust 工具链（含 x86_64-pc-windows-msvc）+ Visual Studio C++ 生成工具
#      （llama-helper 依赖 llama-cpp-sys-2，需要 MSVC 与 libclang/LLVM）。

$ErrorActionPreference = "Stop"

# 仓库根 = 本脚本的上上级目录
$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$BinariesDir = Join-Path $RepoRoot "frontend\src-tauri\binaries"
New-Item -ItemType Directory -Force -Path $BinariesDir | Out-Null

Write-Host ""
Write-Host "=== VoxMinutes: 准备 Windows 侧车 ===" -ForegroundColor Cyan
Write-Host "仓库根: $RepoRoot"
Write-Host "输出目录: $BinariesDir"
Write-Host ""

# ── 1. ffmpeg.exe ───────────────────────────────────────────────────────────
$ffmpegDst = Join-Path $BinariesDir "ffmpeg-x86_64-pc-windows-msvc.exe"
if ((Test-Path $ffmpegDst) -and (Get-Item $ffmpegDst).Length -gt 0) {
    Write-Host "[跳过] ffmpeg.exe 已存在" -ForegroundColor Green
} else {
    Write-Host "[1/2] 下载 ffmpeg (gyan.dev essentials build)..." -ForegroundColor Yellow
    $tmp = Join-Path $env:TEMP "voxminutes-ffmpeg"
    New-Item -ItemType Directory -Force -Path $tmp | Out-Null
    $zip = Join-Path $tmp "ffmpeg.zip"
    Invoke-WebRequest -Uri "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip" -OutFile $zip
    Expand-Archive -Path $zip -DestinationPath $tmp -Force
    $exe = Get-ChildItem -Path $tmp -Recurse -Filter ffmpeg.exe | Select-Object -First 1
    if (-not $exe) { throw "压缩包里没找到 ffmpeg.exe（下载地址可能已变更）" }
    Copy-Item $exe.FullName $ffmpegDst -Force
    Write-Host "[OK] ffmpeg.exe -> $ffmpegDst" -ForegroundColor Green
}

# ── 2. llama-helper.exe ─────────────────────────────────────────────────────
$helperDst = Join-Path $BinariesDir "llama-helper-x86_64-pc-windows-msvc.exe"
if ((Test-Path $helperDst) -and (Get-Item $helperDst).Length -gt 0) {
    Write-Host "[跳过] llama-helper.exe 已存在" -ForegroundColor Green
} else {
    Write-Host "[2/2] 编译 llama-helper sidecar（首次约 5~15 分钟）..." -ForegroundColor Yellow
    Push-Location $RepoRoot
    try {
        # 仓库根是 cargo workspace：产物在 <repo>/target/release/，不在 llama-helper/target/
        cargo build --release -p llama-helper
        if ($LASTEXITCODE -ne 0) { throw "cargo build -p llama-helper 失败" }
        $built = Join-Path $RepoRoot "target\release\llama-helper.exe"
        if (-not (Test-Path $built)) { throw "没找到构建产物: $built" }
        Copy-Item $built $helperDst -Force
        Write-Host "[OK] llama-helper.exe -> $helperDst" -ForegroundColor Green
    } finally {
        Pop-Location
    }
}

# ── 3. 校验 ─────────────────────────────────────────────────────────────────
Write-Host ""
Write-Host "=== 结果 ===" -ForegroundColor Cyan
$ok = $true
foreach ($f in @($ffmpegDst, $helperDst)) {
    $len = if (Test-Path $f) { (Get-Item $f).Length } else { 0 }
    $mark = if ($len -gt 0) { "[OK]" } else { "[!!]" }
    if ($len -le 0) { $ok = $false }
    Write-Host ("{0} {1,-45} {2} bytes" -f $mark, (Split-Path $f -Leaf), $len)
}

Write-Host ""
Write-Host "提示：运行时 DLL（sherpa-onnx / onnxruntime）由 src-tauri/build.rs 在" -ForegroundColor DarkGray
Write-Host "      构建时自动暂存到 src-tauri/resources-dll/，无需手工复制。" -ForegroundColor DarkGray
Write-Host ""

if (-not $ok) { exit 1 }
Write-Host "侧车准备完成，可以执行：cd frontend; pnpm tauri build" -ForegroundColor Green
