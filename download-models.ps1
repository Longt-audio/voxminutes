# VoxMinutes MVP ASR Model Download Script (PowerShell)
# Downloads:
#   - SenseVoice model (for offline file transcription and VAD pseudo-streaming)
#   - X-ASR streaming model (for pure streaming ASR)
#
# Usage: .\download-models.ps1 [-ModelsDir "D:\realtime_transcription\voxminutes\models"]

param(
    [string]$ModelsDir = "$PSScriptRoot\models"
)

$ErrorActionPreference = "Stop"

function Write-Info { param([string]$Message) Write-Host "[INFO] $Message" -ForegroundColor Cyan }
function Write-Ok   { param([string]$Message) Write-Host "[OK] $Message" -ForegroundColor Green }
function Write-Warn { param([string]$Message) Write-Host "[WARN] $Message" -ForegroundColor Yellow }

New-Item -ItemType Directory -Path $ModelsDir -Force | Out-Null

$models = @(
    @{
        Name = "sherpa-onnx-sense-voice-zh-en-ja-ko-yue-2024-07-17"
        Url = "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-sense-voice-zh-en-ja-ko-yue-2024-07-17.tar.bz2"
        CheckFiles = @("model.onnx", "tokens.txt")
    },
    @{
        Name = "sherpa-onnx-x-asr-480ms-streaming-zipformer-transducer-zh-en-punct-2026-06-05"
        Url = "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-x-asr-480ms-streaming-zipformer-transducer-zh-en-punct-2026-06-05.tar.bz2"
        CheckFiles = @("encoder.onnx", "decoder.onnx", "joiner.onnx", "tokens.txt")
    }
)

foreach ($model in $models) {
    $targetDir = Join-Path $ModelsDir $model.Name
    $needsDownload = $false

    foreach ($checkFile in $model.CheckFiles) {
        if (-not (Test-Path (Join-Path $targetDir $checkFile))) {
            $needsDownload = $true
            break
        }
    }

    if (-not $needsDownload) {
        Write-Ok "$($model.Name) already exists: $targetDir"
        continue
    }

    Write-Info "Downloading $($model.Name) from GitHub..."
    Write-Info "URL: $($model.Url)"

    $tmpFile = Join-Path $env:TEMP "$($model.Name).tar.bz2"
    if (Test-Path $tmpFile) { Remove-Item -Force $tmpFile }

    try {
        Invoke-WebRequest -Uri $model.Url -OutFile $tmpFile -UseBasicParsing
    } catch {
        Write-Warn "Failed to download from GitHub: $_"
        Write-Info "Trying mirror (gitee not available, please ensure GitHub access)..."
        throw
    }

    if (-not (Test-Path $tmpFile)) {
        throw "Download failed: $tmpFile not found"
    }

    Write-Info "Extracting to $ModelsDir ..."
    if (Test-Path $targetDir) { Remove-Item -Recurse -Force $targetDir }

    # Prefer tar if available (Windows 10 1803+ includes bsdtar)
    $tar = Get-Command tar -ErrorAction SilentlyContinue
    if ($tar) {
        & tar -xjf $tmpFile -C $ModelsDir
    } else {
        # Fallback to 7z if available
        $7z = Get-Command 7z -ErrorAction SilentlyContinue
        if ($7z) {
            & 7z x $tmpFile -o"$ModelsDir" -y
        } else {
            throw "Neither tar nor 7z found. Please install 7-Zip or use WSL to extract .tar.bz2 files."
        }
    }

    Remove-Item -Force $tmpFile

    foreach ($checkFile in $model.CheckFiles) {
        if (-not (Test-Path (Join-Path $targetDir $checkFile))) {
            throw "Model file incomplete: $checkFile missing in $targetDir"
        }
    }

    Write-Ok "$($model.Name) downloaded and extracted to $targetDir"
}

Write-Host ""
Write-Host "==============================================" -ForegroundColor Green
Write-Host "All ASR models are ready." -ForegroundColor Green
Write-Host "Models directory: $ModelsDir" -ForegroundColor Green
Write-Host "==============================================" -ForegroundColor Green

Read-Host -Prompt "Press Enter to exit"
