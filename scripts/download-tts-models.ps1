# ============================================================
# VoxMinutes TTS - 模型下载脚本 (Windows PowerShell)
# 下载 Supertonic 3 (en+ko+29语 多音色) 和 vits-piper-zh_CN-chaowen-medium (中文)
# ============================================================

$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectRoot = Split-Path -Parent $ScriptDir
$ModelsDir = Join-Path $ProjectRoot "models"

Write-Host "============================================" -ForegroundColor Cyan
Write-Host "  TTS 模型下载脚本 (Sherpa-ONNX)" -ForegroundColor Cyan
Write-Host "============================================" -ForegroundColor Cyan
Write-Host ""

function Download-File {
    param([string]$Url, [string]$OutFile, [string]$MirrorUrl)
    
    if (Test-Path $OutFile) {
        Write-Host "[SKIP] Already exists: $OutFile" -ForegroundColor Green
        return $true
    }
    
    Write-Host "[DOWNLOAD] $Url" -ForegroundColor Blue
    try {
        $ProgressPreference = 'SilentlyContinue'
        Invoke-WebRequest -Uri $Url -OutFile $OutFile -ErrorAction Stop
        Write-Host "[OK] Downloaded: $OutFile" -ForegroundColor Green
        return $true
    } catch {
        Write-Host "[RETRY] Mirror: $MirrorUrl" -ForegroundColor Yellow
        try {
            $ProgressPreference = 'SilentlyContinue'
            Invoke-WebRequest -Uri $MirrorUrl -OutFile $OutFile -ErrorAction Stop
            Write-Host "[OK] Downloaded from mirror: $OutFile" -ForegroundColor Green
            return $true
        } catch {
            Write-Host "[FAIL] Failed to download: $Url" -ForegroundColor Red
            Remove-Item $OutFile -ErrorAction SilentlyContinue
            return $false
        }
    }
}

function Extract-TarBz2 {
    param([string]$Archive, [string]$DestDir)
    
    $TarFile = $Archive -replace '\.bz2$', '.tar'
    
    # Decompress bz2
    Write-Host "[EXTRACT] Decompressing $Archive..." -ForegroundColor Blue
    $memStream = New-Object System.IO.MemoryStream
    $fileStream = [System.IO.File]::OpenRead($Archive)
    $decompressor = New-Object System.IO.Compression.BZip2Stream($fileStream, [System.IO.Compression.CompressionMode]::Decompress)
    $decompressor.CopyTo($memStream)
    $decompressor.Close()
    $fileStream.Close()
    
    # Write tar
    [System.IO.File]::WriteAllBytes($TarFile, $memStream.ToArray())
    $memStream.Close()
    
    # Extract tar
    Write-Host "[EXTRACT] Extracting $TarFile to $DestDir..." -ForegroundColor Blue
    New-Item -ItemType Directory -Force -Path $DestDir | Out-Null
    
    # Use tar command if available (Windows 10 1803+)
    $tarExe = Get-Command tar.exe -ErrorAction SilentlyContinue
    if ($tarExe) {
        & tar -xf $TarFile -C $DestDir
    } else {
        # Fallback: manual tar extraction
        $bytes = [System.IO.File]::ReadAllBytes($TarFile)
        $pos = 0
        while ($pos -lt $bytes.Length) {
            $name = [System.Text.Encoding]::UTF8.GetString($bytes, $pos, 100).TrimEnd([char]0)
            $pos += 100
            $mode = [System.Text.Encoding]::UTF8.GetString($bytes, $pos, 8).TrimEnd([char]0)
            $pos += 8
            $uid = [System.Text.Encoding]::UTF8.GetString($bytes, $pos, 8).TrimEnd([char]0)
            $pos += 8
            $gid = [System.Text.Encoding]::UTF8.GetString($bytes, $pos, 8).TrimEnd([char]0)
            $pos += 8
            $sizeStr = [System.Text.Encoding]::UTF8.GetString($bytes, $pos, 12).TrimEnd([char]0)
            $size = [Convert]::ToInt64($sizeStr.Trim(), 8)
            $pos += 12
            $mtimeStr = [System.Text.Encoding]::UTF8.GetString($bytes, $pos, 12).TrimEnd([char]0)
            $pos += 12
            $chksum = [System.Text.Encoding]::UTF8.GetString($bytes, $pos, 8).TrimEnd([char]0)
            $pos += 8
            $typeflag = [char]$bytes[$pos]
            $pos += 1
            $linkname = [System.Text.Encoding]::UTF8.GetString($bytes, $pos, 100).TrimEnd([char]0)
            $pos += 100
            
            # Skip padding to 512 byte boundary
            $pos = [Math]::Ceiling($pos / 512.0) * 512
            
            if ($typeflag -eq '0' -or $typeflag -eq [char]0) {
                $destPath = Join-Path $DestDir $name
                $parentDir = Split-Path $destPath -Parent
                if ($parentDir) { New-Item -ItemType Directory -Force -Path $parentDir | Out-Null }
                
                if ($size -gt 0) {
                    [System.IO.File]::WriteAllBytes($destPath, $bytes[$pos..($pos + $size - 1)])
                } else {
                    New-Item -ItemType Directory -Force -Path $destPath | Out-Null
                }
                $pos += $size
                $pos = [Math]::Ceiling($pos / 512.0) * 512
            } elseif ($typeflag -eq '5') {
                New-Item -ItemType Directory -Force -Path (Join-Path $DestDir $name) | Out-Null
            } else {
                if ($size -gt 0) {
                    $pos += $size
                    $pos = [Math]::Ceiling($pos / 512.0) * 512
                }
            }
        }
    }
    
    # Clean up tar file
    Remove-Item $TarFile -Force
    Write-Host "[OK] Extracted to $DestDir" -ForegroundColor Green
}

# ===================== Download vits-piper-zh_CN-chaowen-medium =====================
Write-Host "`n--- vits-piper-zh_CN-chaowen-medium (中文) ---" -ForegroundColor Cyan
$PiperDir = Join-Path $ModelsDir "vits-piper-zh_CN-chaowen-medium"
$PiperUrl = "https://github.com/k2-fsa/sherpa-onnx/releases/download/tts-models/vits-piper-zh_CN-chaowen-medium.tar.bz2"
$PiperMirror = ""

$PiperTemp = Join-Path $env:TEMP "vits-piper-zh_CN-chaowen-medium.tar.bz2"

if (Download-File -Url $PiperUrl -OutFile $PiperTemp -MirrorUrl $PiperMirror) {
    $ExtractDir = Join-Path $ModelsDir "_piper_chaowen_extract"
    Remove-Item $ExtractDir -Recurse -Force -ErrorAction SilentlyContinue
    New-Item -ItemType Directory -Force -Path $ExtractDir | Out-Null
    
    Extract-TarBz2 -Archive $PiperTemp -DestDir $ExtractDir
    
    # Move extracted files to vits-piper-zh_CN-chaowen-medium/
    $ExtractedModelDir = Get-ChildItem $ExtractDir -Directory | Select-Object -First 1
    if ($ExtractedModelDir) {
        Remove-Item $PiperDir -Recurse -Force -ErrorAction SilentlyContinue
        Move-Item $ExtractedModelDir.FullName $PiperDir -Force
    }
    Remove-Item $ExtractDir -Recurse -Force -ErrorAction SilentlyContinue
    Remove-Item $PiperTemp -Force
}

# Verify vits-piper-zh_CN-chaowen-medium
$PiperOk = (Test-Path (Join-Path $PiperDir "zh_CN-chaowen-medium.onnx")) -and (Test-Path (Join-Path $PiperDir "tokens.txt")) -and (Test-Path (Join-Path $PiperDir "lexicon.txt"))
if ($PiperOk) { Write-Host "[OK] vits-piper-zh_CN-chaowen-medium 模型就绪: $PiperDir" -ForegroundColor Green }
else { Write-Host "[WARN] vits-piper-zh_CN-chaowen-medium 模型不完整，请检查" -ForegroundColor Yellow }

# ===================== Download matcha-icefall-zh-baker =====================
Write-Host "`n--- matcha-icefall-zh-baker (中文女声) ---" -ForegroundColor Cyan
$MatchaDir = Join-Path $ModelsDir "matcha-icefall-zh-baker"
$MatchaUrl = "https://github.com/k2-fsa/sherpa-onnx/releases/download/tts-models/matcha-icefall-zh-baker.tar.bz2"
$MatchaMirror = ""

$MatchaTemp = Join-Path $env:TEMP "matcha-icefall-zh-baker.tar.bz2"

if (Download-File -Url $MatchaUrl -OutFile $MatchaTemp -MirrorUrl $MatchaMirror) {
    $ExtractDir = Join-Path $ModelsDir "_matcha_baker_extract"
    Remove-Item $ExtractDir -Recurse -Force -ErrorAction SilentlyContinue
    New-Item -ItemType Directory -Force -Path $ExtractDir | Out-Null
    
    Extract-TarBz2 -Archive $MatchaTemp -DestDir $ExtractDir
    
    $ExtractedModelDir = Get-ChildItem $ExtractDir -Directory | Select-Object -First 1
    if ($ExtractedModelDir) {
        Remove-Item $MatchaDir -Recurse -Force -ErrorAction SilentlyContinue
        Move-Item $ExtractedModelDir.FullName $MatchaDir -Force
    }
    Remove-Item $ExtractDir -Recurse -Force -ErrorAction SilentlyContinue
    Remove-Item $MatchaTemp -Force
}

# Download vocoder for Matcha
$VocoderUrl = "https://github.com/k2-fsa/sherpa-onnx/releases/download/vocoder-models/vocos-22khz-univ.onnx"
$VocoderPath = Join-Path $MatchaDir "vocoder.onnx"
if (-not (Test-Path $VocoderPath)) {
    Download-File -Url $VocoderUrl -OutFile $VocoderPath -MirrorUrl ""
}

# Verify matcha-icefall-zh-baker
$MatchaOk = (Test-Path (Join-Path $MatchaDir "model-steps-3.onnx")) -and (Test-Path (Join-Path $MatchaDir "vocoder.onnx")) -and (Test-Path (Join-Path $MatchaDir "tokens.txt")) -and (Test-Path (Join-Path $MatchaDir "lexicon.txt"))
if ($MatchaOk) { Write-Host "[OK] matcha-icefall-zh-baker 模型就绪: $MatchaDir" -ForegroundColor Green }
else { Write-Host "[WARN] matcha-icefall-zh-baker 模型不完整，请检查" -ForegroundColor Yellow }

# ===================== Download Supertonic 3 =====================
Write-Host "`n--- Supertonic 3 (en+ko+29语 多音色) ---" -ForegroundColor Cyan
$SupertonicDir = Join-Path $ModelsDir "supertonic"
$SupertonicUrl = "https://github.com/k2-fsa/sherpa-onnx/releases/download/tts-models/sherpa-onnx-supertonic-3-tts-int8-2026-05-11.tar.bz2"
$SupertonicMirror = "https://hf-mirror.com/csukuangfj2/sherpa-onnx-tts-models/resolve/main/sherpa-onnx-supertonic-3-tts-int8-2026-05-11.tar.bz2"

$SupertonicTemp = Join-Path $env:TEMP "supertonic.tar.bz2"

if (Download-File -Url $SupertonicUrl -OutFile $SupertonicTemp -MirrorUrl $SupertonicMirror) {
    $ExtractDir = Join-Path $ModelsDir "_supertonic_extract"
    Remove-Item $ExtractDir -Recurse -Force -ErrorAction SilentlyContinue
    New-Item -ItemType Directory -Force -Path $ExtractDir | Out-Null
    
    Extract-TarBz2 -Archive $SupertonicTemp -DestDir $ExtractDir
    
    $ExtractedModelDir = Get-ChildItem $ExtractDir -Directory | Select-Object -First 1
    if ($ExtractedModelDir) {
        Remove-Item $SupertonicDir -Recurse -Force -ErrorAction SilentlyContinue
        Move-Item $ExtractedModelDir.FullName $SupertonicDir -Force
    }
    Remove-Item $ExtractDir -Recurse -Force -ErrorAction SilentlyContinue
    Remove-Item $SupertonicTemp -Force
}

# Verify Supertonic
$StOk = (Test-Path (Join-Path $SupertonicDir "duration_predictor.int8.onnx")) -and (Test-Path (Join-Path $SupertonicDir "tts.json"))
if ($StOk) { Write-Host "[OK] Supertonic 3 模型就绪: $SupertonicDir" -ForegroundColor Green }
else { Write-Host "[WARN] Supertonic 3 模型不完整，请检查" -ForegroundColor Yellow }

# ===================== Summary =====================
Write-Host "`n============================================" -ForegroundColor Cyan
Write-Host "  下载完成！" -ForegroundColor Green
Write-Host "============================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "模型目录: $ModelsDir"
Write-Host "  vits-piper-zh_CN-chaowen-medium/ - 中文 TTS (单音色男声)"
Write-Host "  matcha-icefall-zh-baker/ - 中文 TTS (单音色女声)"
Write-Host "  supertonic/         - Supertonic 3 (31 语种多音色)"
Write-Host ""
Write-Host "下一步: 启动应用 pnpm run tauri:dev" -ForegroundColor Cyan
