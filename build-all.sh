#!/bin/bash
# ============================================================
# VoxMinutes ASR Service - 一键编译脚本 (Sherpa-ONNX 版)
# 用途: 编译 C# ASR 微服务为跨平台可执行文件
# 依赖: .NET 8 SDK
# ============================================================

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ASR_SERVICE_DIR="$SCRIPT_DIR/asr-service"
OUTPUT_DIR="$SCRIPT_DIR/voxminutes-main/frontend/src-tauri/binaries"
MODELS_DIR="$ASR_SERVICE_DIR/Models"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info()  { echo -e "${BLUE}[INFO]${NC} $1"; }
log_ok()    { echo -e "${GREEN}[OK]${NC} $1"; }
log_warn()  { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

echo "============================================"
echo "  VoxMinutes ASR Service 编译脚本"
echo "============================================"

# 1. 检查 .NET SDK
log_info "检查 .NET SDK..."
if ! command -v dotnet &> /dev/null; then
    log_error "未找到 dotnet 命令！请先安装 .NET 8 SDK："
    echo ""
    echo "  macOS:  brew install dotnet-sdk"
    echo "  Linux:  wget https://dot.net/v1/dotnet-install.sh && chmod +x dotnet-install.sh && ./dotnet-install.sh"
    echo "  Windows: https://dotnet.microsoft.com/download/dotnet/8.0"
    echo ""
    exit 1
fi

DOTNET_VERSION=$(dotnet --version)
log_ok "dotnet 版本: $DOTNET_VERSION"

# 2. 创建输出目录
mkdir -p "$OUTPUT_DIR"
mkdir -p "$OUTPUT_DIR/Models"
log_ok "输出目录: $OUTPUT_DIR"

# 3. 检查模型文件
log_info "检查模型文件..."
MODELS_OK=true
if [ -d "$MODELS_DIR/sense-voice" ]; then
    log_ok "  ✓ sense-voice"
else
    log_warn "  ✗ sense-voice (未找到，运行 ./download-models.sh 下载)"
    MODELS_OK=false
fi

# 4. 确定平台
ARCH=$(uname -m)
OS=$(uname)
log_info "当前平台: $OS $ARCH"

# 5. 编译 C# 服务
log_info "编译 C# ASR 服务..."

cd "$ASR_SERVICE_DIR"

# 恢复依赖
dotnet restore
log_ok "依赖恢复完成"

# 构建配置
if [ "$OS" = "Darwin" ] && [ "$ARCH" = "arm64" ]; then
    RID="osx-arm64"
    BIN_SUFFIX="aarch64-apple-darwin"
elif [ "$OS" = "Darwin" ]; then
    RID="osx-x64"
    BIN_SUFFIX="x86_64-apple-darwin"
elif [ "$OS" = "Linux" ]; then
    RID="linux-x64"
    BIN_SUFFIX="x86_64-unknown-linux-gnu"
else
    RID="win-x64"
    BIN_SUFFIX="x86_64-pc-windows-msvc"
fi

log_info "目标 RID: $RID"

# 发布 (self-contained, 单文件)
dotnet publish \
    -c Release \
    -r "$RID" \
    --self-contained true \
    -p:PublishSingleFile=true \
    -p:PublishTrimmed=false \
    -p:IncludeNativeLibrariesForSelfExtract=true \
    -o "$OUTPUT_DIR"

log_ok "编译完成"

# 6. 重命名并放置模型
cd "$OUTPUT_DIR"

if [ "$OS" = "Darwin" ]; then
    mv AsrService "AsrService-$BIN_SUFFIX"
    FINAL_BIN="AsrService-$BIN_SUFFIX"
elif [ "$OS" = "Linux" ]; then
    mv AsrService "AsrService-$BIN_SUFFIX"
    FINAL_BIN="AsrService-$BIN_SUFFIX"
else
    mv AsrService.exe "AsrService-$BIN_SUFFIX.exe"
    FINAL_BIN="AsrService-$BIN_SUFFIX.exe"
fi

# 7. 复制模型到二进制目录（运行时查找）
log_info "复制模型文件..."
if [ -d "$MODELS_DIR/sense-voice" ]; then
    mkdir -p "$OUTPUT_DIR/Models"
    cp -r "$MODELS_DIR/sense-voice" "$OUTPUT_DIR/Models/"
fi

# 8. 验证
log_info "验证输出..."
if [ -f "$OUTPUT_DIR/$FINAL_BIN" ]; then
    SIZE=$(du -h "$OUTPUT_DIR/$FINAL_BIN" | cut -f1)
    MODEL_COUNT=$(ls -d "$OUTPUT_DIR"/Models/*/ 2>/dev/null | wc -l | tr -d ' ')
    log_ok "ASR 服务编译成功！"
    log_ok "  二进制: $FINAL_BIN ($SIZE)"
    log_ok "  模型数量: $MODEL_COUNT"
    log_ok "  输出目录: $OUTPUT_DIR/"
else
    log_error "编译失败！"
    ls -la "$OUTPUT_DIR/"
    exit 1
fi

echo ""
echo "============================================"
echo -e "${GREEN}  编译成功！${NC}"
echo "============================================"
echo ""
echo "下一步:"
echo "  1. 测试运行: $OUTPUT_DIR/$FINAL_BIN"
echo "  2. 健康检查: curl http://localhost:5000/api/v1/asr/health"
echo "  3. 编译 Tauri: cd $SCRIPT_DIR/voxminutes-main/frontend && pnpm tauri dev"
echo ""
