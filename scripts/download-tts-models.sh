#!/bin/bash
# ============================================================
# VoxMinutes TTS - Model Download Script (Linux/macOS)
# Downloads Supertonic 3 (en+ko+29 lang multi-speaker) + vits-piper-zh_CN-chaowen-medium (Chinese)
# ============================================================

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
MODELS_DIR="$ROOT_DIR/models"

RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info()  { echo -e "${BLUE}[INFO]${NC} $1"; }
log_ok()    { echo -e "${GREEN}[OK]${NC} $1"; }
log_warn()  { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

echo "============================================"
echo -e "${CYAN}  TTS Model Download (Sherpa-ONNX)${NC}"
echo "============================================"
echo ""

mkdir -p "$MODELS_DIR"

download_file() {
  local url="$1"
  local out="$2"
  local mirror="$3"

  if [ -f "$out" ]; then
    log_ok "Already exists: $out"
    return 0
  fi

  log_info "Downloading: $url"
  if command -v curl &> /dev/null; then
    if curl -L --progress-bar -o "$out" "$url"; then
      log_ok "Downloaded: $out"
    elif [ -n "$mirror" ]; then
      log_warn "Primary failed, trying mirror..."
      curl -L --progress-bar -o "$out" "$mirror" && log_ok "Downloaded from mirror: $out" || { log_error "Mirror also failed"; rm -f "$out"; return 1; }
    else
      log_error "Download failed"
      rm -f "$out"
      return 1
    fi
  elif command -v wget &> /dev/null; then
    if wget -O "$out" "$url"; then
      log_ok "Downloaded: $out"
    elif [ -n "$mirror" ]; then
      log_warn "Primary failed, trying mirror..."
      wget -O "$out" "$mirror" && log_ok "Downloaded from mirror: $out" || { log_error "Mirror also failed"; rm -f "$out"; return 1; }
    else
      log_error "Download failed"
      rm -f "$out"
      return 1
    fi
  else
    log_error "curl or wget required"
    return 1
  fi
}

# ===================== vits-piper-zh_CN-chaowen-medium =====================
echo ""
echo -e "${CYAN}--- vits-piper-zh_CN-chaowen-medium (Chinese) ---${NC}"

PIPER_DIR="$MODELS_DIR/vits-piper-zh_CN-chaowen-medium"
PIPER_URL="https://github.com/k2-fsa/sherpa-onnx/releases/download/tts-models/vits-piper-zh_CN-chaowen-medium.tar.bz2"
PIPER_MIRROR=""

PIPER_TMP="/tmp/vits-piper-zh_CN-chaowen-medium.tar.bz2"

download_file "$PIPER_URL" "$PIPER_TMP" "$PIPER_MIRROR" && {
  EXTRACT_DIR="$MODELS_DIR/_piper_chaowen_extract"
  rm -rf "$EXTRACT_DIR"
  mkdir -p "$EXTRACT_DIR"
  log_info "Extracting vits-piper-zh_CN-chaowen-medium..."
  tar -xjf "$PIPER_TMP" -C "$EXTRACT_DIR"
  EXTRACTED=$(ls -d "$EXTRACT_DIR"/*/ 2>/dev/null | head -1)
  if [ -n "$EXTRACTED" ]; then
    rm -rf "$PIPER_DIR"
    mv "$EXTRACTED" "$PIPER_DIR"
  fi
  rm -rf "$EXTRACT_DIR"
  rm -f "$PIPER_TMP"
}

if [ -f "$PIPER_DIR/zh_CN-chaowen-medium.onnx" ] && [ -f "$PIPER_DIR/tokens.txt" ] && [ -f "$PIPER_DIR/lexicon.txt" ]; then
  log_ok "vits-piper-zh_CN-chaowen-medium ready: $PIPER_DIR"
else
  log_warn "vits-piper-zh_CN-chaowen-medium incomplete"
fi

# ===================== matcha-icefall-zh-baker =====================
echo ""
echo -e "${CYAN}--- matcha-icefall-zh-baker (Chinese female) ---${NC}"

MATCHA_DIR="$MODELS_DIR/matcha-icefall-zh-baker"
MATCHA_URL="https://github.com/k2-fsa/sherpa-onnx/releases/download/tts-models/matcha-icefall-zh-baker.tar.bz2"
MATCHA_MIRROR=""

MATCHA_TMP="/tmp/matcha-icefall-zh-baker.tar.bz2"

download_file "$MATCHA_URL" "$MATCHA_TMP" "$MATCHA_MIRROR" && {
  EXTRACT_DIR="$MODELS_DIR/_matcha_baker_extract"
  rm -rf "$EXTRACT_DIR"
  mkdir -p "$EXTRACT_DIR"
  log_info "Extracting matcha-icefall-zh-baker..."
  tar -xjf "$MATCHA_TMP" -C "$EXTRACT_DIR"
  EXTRACTED=$(ls -d "$EXTRACT_DIR"/*/ 2>/dev/null | head -1)
  if [ -n "$EXTRACTED" ]; then
    rm -rf "$MATCHA_DIR"
    mv "$EXTRACTED" "$MATCHA_DIR"
  fi
  rm -rf "$EXTRACT_DIR"
  rm -f "$MATCHA_TMP"
}

# Download vocoder for Matcha
VOCODER_URL="https://github.com/k2-fsa/sherpa-onnx/releases/download/vocoder-models/vocos-22khz-univ.onnx"
VOCODER_PATH="$MATCHA_DIR/vocoder.onnx"
if [ ! -f "$VOCODER_PATH" ]; then
  download_file "$VOCODER_URL" "$VOCODER_PATH" ""
fi

if [ -f "$MATCHA_DIR/model-steps-3.onnx" ] && [ -f "$MATCHA_DIR/vocoder.onnx" ] && [ -f "$MATCHA_DIR/tokens.txt" ] && [ -f "$MATCHA_DIR/lexicon.txt" ]; then
  log_ok "matcha-icefall-zh-baker ready: $MATCHA_DIR"
else
  log_warn "matcha-icefall-zh-baker incomplete"
fi

# ===================== Supertonic 3 =====================
echo ""
echo -e "${CYAN}--- Supertonic 3 (en+ko Multi-speaker) ---${NC}"

SUPERTONIC_DIR="$MODELS_DIR/supertonic"
SUPERTONIC_URL="https://github.com/k2-fsa/sherpa-onnx/releases/download/tts-models/sherpa-onnx-supertonic-3-tts-int8-2026-05-11.tar.bz2"
SUPERTONIC_MIRROR="https://hf-mirror.com/csukuangfj2/sherpa-onnx-tts-models/resolve/main/sherpa-onnx-supertonic-3-tts-int8-2026-05-11.tar.bz2"

SUPERTONIC_TMP="/tmp/supertonic.tar.bz2"

download_file "$SUPERTONIC_URL" "$SUPERTONIC_TMP" "$SUPERTONIC_MIRROR" && {
  EXTRACT_DIR="$MODELS_DIR/_supertonic_extract"
  rm -rf "$EXTRACT_DIR"
  mkdir -p "$EXTRACT_DIR"
  log_info "Extracting Supertonic 3..."
  tar -xjf "$SUPERTONIC_TMP" -C "$EXTRACT_DIR"
  EXTRACTED=$(ls -d "$EXTRACT_DIR"/*/ 2>/dev/null | head -1)
  if [ -n "$EXTRACTED" ]; then
    rm -rf "$SUPERTONIC_DIR"
    mv "$EXTRACTED" "$SUPERTONIC_DIR"
  fi
  rm -rf "$EXTRACT_DIR"
  rm -f "$SUPERTONIC_TMP"
}

if [ -f "$SUPERTONIC_DIR/duration_predictor.int8.onnx" ] && [ -f "$SUPERTONIC_DIR/tts.json" ]; then
  log_ok "Supertonic 3 ready: $SUPERTONIC_DIR"
else
  log_warn "Supertonic 3 incomplete"
fi

# ===================== Summary =====================
echo ""
echo "============================================"
echo -e "${GREEN}  Download complete!${NC}"
echo "============================================"
echo ""
echo "Model directory: $MODELS_DIR"
echo "  vits-piper-zh_CN-chaowen-medium/ - Chinese TTS (single male speaker)"
echo "  matcha-icefall-zh-baker/ - Chinese TTS (single female speaker)"
echo "  supertonic/         - Supertonic 3 (31 languages multi-speaker)"
echo ""
echo "Next: start app with 'pnpm run tauri:dev'"
