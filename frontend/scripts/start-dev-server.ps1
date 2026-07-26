<#
.SYNOPSIS
    安全启动前端开发服务器，处理 Windows TIME_WAIT 端口竞态问题
.DESCRIPTION
    1. 杀掉占用端口 3118 的进程
    2. 轮询等待端口完全释放（处理 TIME_WAIT）
    3. 启动 pnpm exec serve
#>

param(
    [int]$Port = 3118,
    [int]$WaitTimeoutSec = 30
)

$ErrorActionPreference = "Stop"

# ── 1. 杀掉占用端口的进程 ──────────────────────────────────
Write-Host "[start-dev-server] 检查端口 ${Port}..."
$connections = Get-NetTCPConnection -LocalPort $Port -ErrorAction SilentlyContinue
if ($connections) {
    $pids = $connections | Select-Object -ExpandProperty OwningProcess -Unique
    foreach ($procId in $pids) {
        $proc = Get-Process -Id $procId -ErrorAction SilentlyContinue
        if ($proc) {
            Write-Host "[start-dev-server] 终止进程: $($proc.ProcessName) (PID: $procId)"
            Stop-Process -Id $procId -Force -ErrorAction SilentlyContinue
        }
    }
    Write-Host "[start-dev-server] 已发送终止信号，等待端口释放..."
} else {
    Write-Host "[start-dev-server] 端口 ${Port} 当前无占用"
}

# ── 2. 轮询等待端口完全释放（含 TIME_WAIT） ─────────────────
$elapsed = 0
$intervalMs = 500
do {
    $stillUsed = Get-NetTCPConnection -LocalPort $Port -ErrorAction SilentlyContinue
    if (-not $stillUsed) {
        Write-Host "[start-dev-server] 端口 ${Port} 已释放 (等待 ${elapsed}s)"
        break
    }
    Start-Sleep -Milliseconds $intervalMs
    $elapsed += $intervalMs / 1000.0
} while ($elapsed -lt $WaitTimeoutSec)

if ($elapsed -ge $WaitTimeoutSec) {
    Write-Host "[start-dev-server] ⚠ 等待超时 (${WaitTimeoutSec}s)，端口可能仍被占用，但仍尝试启动..."
    $stillRemaining = Get-NetTCPConnection -LocalPort $Port -ErrorAction SilentlyContinue
    if ($stillRemaining) {
        Write-Host "[start-dev-server]   残留连接状态: $($stillRemaining.State -join ', ')"
    }
}

# ── 3. 启动静态文件服务器（使用 pnpm build 的 out/ 输出） ───
$outDir = Join-Path $PSScriptRoot "..\out"
if (-not (Test-Path $outDir)) {
    Write-Host "[start-dev-server] ⚠ out/ 目录不存在，先运行 pnpm build"
    exit 1
}
Write-Host "[start-dev-server] 启动静态文件服务器: pnpm exec serve out -l ${Port} --no-clipboard"
pnpm exec serve out -l $Port --no-clipboard
