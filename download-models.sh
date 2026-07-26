#!/bin/bash
# VoxMinutes MVP ASR Model Download Script (Bash)
# Downloads:
#   - SenseVoice model (for offline file transcription and VAD pseudo-streaming)
#   - X-ASR streaming model (for pure streaming ASR)

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
MODELS_DIR="${1:-$SCRIPT_DIR/models}"
mkdir -p "$MODELS_DIR"

log_info() { echo -e "\033[0;34m[INFO]\033[0m $1"; }
log_ok()   { echo -e "\033[0;32m[OK]\033[0m $1"; }
log_warn() { echo -e "\033[0;33m[WARN]\033[0m $1"; }

download_model() {
    local name="$1"
    local url="$2"
    local check_files="$3"
    local target_dir="$MODELS_DIR/$name"
    local needs_download=false

    for f in $check_files; do
        if [ ! -f "$target_dir/$f" ]; then
            needs_download=true
            break
        fi
    done

    if [ "$needs_download" = false ]; then
        log_ok "$name already exists: $target_dir"
        return
    fi

    log_info "Downloading $name from GitHub..."
    log_info "URL: $url"

    local tmp_file="/tmp/$name.tar.bz2"
    rm -f "$tmp_file"

    if command -v curl >/dev/null 2>&1; then
        curl -L --progress-bar -o "$tmp_file" "$url"
    elif command -v wget >/dev/null 2>&1; then
        wget -O "$tmp_file" "$url"
    else
        echo "ERROR: curl or wget required"
        exit 1
    fi

    log_info "Extracting to $MODELS_DIR ..."
    rm -rf "$target_dir"
    tar -xjf "$tmp_file" -C "$MODELS_DIR"
    rm -f "$tmp_file"

    for f in $check_files; do
        if [ ! -f "$target_dir/$f" ]; then
            echo "ERROR: Model file incomplete: $f missing in $target_dir"
            exit 1
        fi
    done

    log_ok "$name downloaded and extracted to $target_dir"
}

log_info "VoxMinutes ASR Model Download"
log_info "Models directory: $MODELS_DIR"

# SenseVoice model
download_model \
    "sherpa-onnx-sense-voice-zh-en-ja-ko-yue-2024-07-17" \
    "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-sense-voice-zh-en-ja-ko-yue-2024-07-17.tar.bz2" \
    "model.onnx tokens.txt"

# X-ASR streaming model
download_model \
    "sherpa-onnx-x-asr-480ms-streaming-zipformer-transducer-zh-en-punct-2026-06-05" \
    "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-x-asr-480ms-streaming-zipformer-transducer-zh-en-punct-2026-06-05.tar.bz2" \
    "encoder.onnx decoder.onnx joiner.onnx tokens.txt"

log_ok "All ASR models are ready."
