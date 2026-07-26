#!/bin/bash
# Build ASR C# service for macOS and Windows (self-contained)
# Usage: ./build-asr-service.sh [mac|win]

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ASR_SERVICE_DIR="$SCRIPT_DIR/../asr-service"
OUTPUT_DIR="$SCRIPT_DIR/frontend/src-tauri/binaries"

mkdir -p "$OUTPUT_DIR"

PLATFORM="${1:-mac}"

echo "Building ASR Service for $PLATFORM..."

if [ "$PLATFORM" = "mac" ]; then
    RID="osx-x64"
    RID_ARM="osx-arm64"
    EXE_NAME="AsrService"

    cd "$ASR_SERVICE_DIR"
    dotnet publish -c Release -r "$RID" --self-contained true -p:PublishSingleFile=true -p:PublishTrimmed=true -o "$OUTPUT_DIR/asr-service-x86_64"
    cp "$OUTPUT_DIR/asr-service-x86_64/$EXE_NAME" "$OUTPUT_DIR/$EXE_NAME-x86_64-apple-darwin"

    dotnet publish -c Release -r "$RID_ARM" --self-contained true -p:PublishSingleFile=true -p:PublishTrimmed=true -o "$OUTPUT_DIR/asr-service-arm64"
    cp "$OUTPUT_DIR/asr-service-arm64/$EXE_NAME" "$OUTPUT_DIR/$EXE_NAME-aarch64-apple-darwin"

    echo "macOS binaries built:"
    ls -la "$OUTPUT_DIR/$EXE_NAME"-*-apple-darwin

elif [ "$PLATFORM" = "win" ]; then
    RID="win-x64"
    EXE_NAME="AsrService.exe"

    cd "$ASR_SERVICE_DIR"
    dotnet publish -c Release -r "$RID" --self-contained true -p:PublishSingleFile=true -p:PublishTrimmed=true -o "$OUTPUT_DIR/asr-service-win"
    cp "$OUTPUT_DIR/asr-service-win/$EXE_NAME" "$OUTPUT_DIR/$EXE_NAME-x86_64-pc-windows-msvc"

    echo "Windows binary built:"
    ls -la "$OUTPUT_DIR/$EXE_NAME"-x86_64-pc-windows-msvc

else
    echo "Unknown platform: $PLATFORM (use 'mac' or 'win')"
    exit 1
fi

echo "Done! Binaries in $OUTPUT_DIR"
