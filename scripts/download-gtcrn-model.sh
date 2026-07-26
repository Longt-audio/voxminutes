#!/usr/bin/env bash
# ============================================================
# VoxMinutes - GTCRN Speech Enhancement Model Download (Unix)
# ============================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
MODELS_DIR="$PROJECT_ROOT/models"
GTCRN_DIR="$MODELS_DIR/sherpa-onnx-gtcrn-simple"

GTCRN_URL="https://github.com/k2-fsa/sherpa-onnx/releases/download/speech-enhancement-models/sherpa-onnx-gtcrn-simple.tar.bz2"
GTCRN_MIRROR="https://hf-mirror.com/csukuangfj2/sherpa-onnx-speech-enhancement-models/resolve/main/sherpa-onnx-gtcrn-simple.tar.bz2"

echo "============================================"
echo "  GTCRN Speech Enhancement Model Download"
echo "============================================"
echo ""

download_file() {
    local url="$1" out="$2" mirror="$3"
    if [ -f "$out" ]; then
        echo "[SKIP] Already exists: $out"
        return 0
    fi
    echo "[DOWNLOAD] $url"
    if curl -L --progress-bar -o "$out" "$url"; then
        echo "[OK] Downloaded: $out"
        return 0
    fi
    echo "[RETRY] Mirror: $mirror"
    if curl -L --progress-bar -o "$out" "$mirror"; then
        echo "[OK] Downloaded from mirror: $out"
        return 0
    fi
    echo "[FAIL] Failed to download: $url"
    rm -f "$out"
    return 1
}

echo "--- GTCRN Speech Enhancement Model ---"
echo "Model: gtcrn_simple.onnx (lightweight real-time denoiser)"
echo "Size: ~500 KB"
echo "Expected noise reduction: >20 dB"
echo "Frame size: 12.5 ms (supports real-time streaming)"
echo ""

GTCRN_TEMP="/tmp/gtcrn_simple_$$.tar.bz2"

if download_file "$GTCRN_URL" "$GTCRN_TEMP" "$GTCRN_MIRROR"; then
    EXTRACT_DIR="$MODELS_DIR/_gtcrn_extract"
    rm -rf "$EXTRACT_DIR"
    mkdir -p "$EXTRACT_DIR"

    echo "[EXTRACT] Extracting..."
    tar -xjf "$GTCRN_TEMP" -C "$EXTRACT_DIR"

    # Move extracted directory to final location
    EXTRACTED_DIR=$(find "$EXTRACT_DIR" -mindepth 1 -maxdepth 1 -type d | head -1)
    if [ -n "$EXTRACTED_DIR" ]; then
        rm -rf "$GTCRN_DIR"
        mv "$EXTRACTED_DIR" "$GTCRN_DIR"
    fi

    rm -rf "$EXTRACT_DIR"
    rm -f "$GTCRN_TEMP"
fi

GTCRN_ONNX="$GTCRN_DIR/gtcrn_simple.onnx"
if [ -f "$GTCRN_ONNX" ]; then
    echo "[OK] GTCRN model ready: $GTCRN_ONNX"
else
    echo "[WARN] GTCRN model not found. App will fall back to RNNoise."
fi

echo ""
echo "============================================"
echo "  Download Complete!"
echo "============================================"
