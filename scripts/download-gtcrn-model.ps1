# ============================================================
# VoxMinutes - GTCRN Speech Enhancement Model Download
# Downloads the GTCRN deep learning denoiser model for sherpa-onnx
# Used by the audio preprocessing pipeline to suppress noise
# before feeding audio to ASR recognition.
# ============================================================

$ErrorActionPreference = "Stop"
[System.Net.ServicePointManager]::SecurityProtocol = [System.Net.SecurityProtocolType]::Tls12

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectRoot = Split-Path -Parent $ScriptDir
$ModelsDir = Join-Path $ProjectRoot "models"

Write-Host "============================================" -ForegroundColor Cyan
Write-Host "  GTCRN Speech Enhancement Model Download" -ForegroundColor Cyan
Write-Host "============================================" -ForegroundColor Cyan
Write-Host ""

$GtcrnDir = Join-Path $ModelsDir "sherpa-onnx-gtcrn-simple"
$GtcrnUrl = "https://github.com/k2-fsa/sherpa-onnx/releases/download/speech-enhancement-models/gtcrn_simple.onnx"
$GtcrnOnnx = Join-Path $GtcrnDir "gtcrn_simple.onnx"

# Already downloaded?
if (Test-Path $GtcrnOnnx) {
    $header = [System.IO.File]::ReadAllBytes($GtcrnOnnx)[0..3]
    if ($header[0] -eq 0x08) {
        $sizeKB = [math]::Round((Get-Item $GtcrnOnnx).Length / 1KB, 1)
        Write-Host "[SKIP] GTCRN model already exists and valid: $GtcrnOnnx ($sizeKB KB)" -ForegroundColor Green
        exit 0
    }
    Write-Host "[WARN] Existing file is invalid, re-downloading..." -ForegroundColor Yellow
    Remove-Item $GtcrnOnnx -Force
}

Write-Host "--- GTCRN Speech Enhancement Model ---" -ForegroundColor Cyan
Write-Host "Model: gtcrn_simple.onnx (lightweight real-time denoiser)" -ForegroundColor Gray
Write-Host "Size: ~110 KB" -ForegroundColor Gray
Write-Host "Expected noise reduction: >20 dB" -ForegroundColor Gray
Write-Host "Frame size: 12.5 ms (supports real-time streaming)" -ForegroundColor Gray
Write-Host ""

New-Item -ItemType Directory -Force -Path $GtcrnDir | Out-Null

try {
    Write-Host "[DOWNLOAD] $GtcrnUrl" -ForegroundColor Blue
    $ProgressPreference = 'SilentlyContinue'
    Invoke-WebRequest -Uri $GtcrnUrl -OutFile $GtcrnOnnx -TimeoutSec 60 -ErrorAction Stop
} catch {
    Write-Host "[FAIL] GitHub download failed: $_" -ForegroundColor Red
    exit 1
}

# Verify ONNX header
$bytes = [System.IO.File]::ReadAllBytes($GtcrnOnnx)
if ($bytes.Length -lt 100 -or $bytes[0] -ne 0x08) {
    Write-Host "[FAIL] Downloaded file is not a valid ONNX model" -ForegroundColor Red
    Remove-Item $GtcrnOnnx -Force
    exit 1
}

$sizeKB = [math]::Round($bytes.Length / 1KB, 1)
Write-Host "[OK] GTCRN model ready: $GtcrnOnnx ($sizeKB KB)" -ForegroundColor Green
Write-Host ""
Write-Host "The application will auto-detect and use GTCRN for noise suppression."
Write-Host "If the model is missing, RNNoise is used as fallback (10-15 dB)."
Write-Host "Next: pnpm run tauri:dev" -ForegroundColor Cyan
