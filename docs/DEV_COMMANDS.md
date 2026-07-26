# VoxMinutes 开发常用命令

> 所有命令默认在项目根目录 `D:\AI_CODING\realtime_transcription\voxminutes` 下执行（Git Bash 或 PowerShell 均可，除非特别说明）。

## 日常运行（改代码后重启应用）

### 改了前端 UI（frontend/src 下的 .tsx/.ts/.css）

⚠️ **必须先重新构建前端，再启动**。`tauri dev` 加载的是静态导出 `frontend/out/`，不是 Next.js 热更新，只重启应用看不到 UI 改动。

```bash
cd frontend
pnpm build          # 重新生成 out/（约 1-2 分钟）
pnpm tauri:dev      # 启动应用
```

### 只改了 Rust 后端（frontend/src-tauri/src）

```bash
cd frontend
pnpm tauri:dev
```

如果应用已在 `tauri dev` 模式下运行，保存 Rust 文件后它会**自动重编译并重启**，无需手动操作。

### 快速验证 Rust 能否编译（不启动应用）

```bash
cargo check --workspace        # 项目根目录执行
```

## 首次环境准备（新机器只需一次）

```bash
# 1. 前端依赖（注意必须加 --ignore-workspace，根目录有 pnpm-workspace.yaml）
cd frontend
pnpm install --ignore-workspace

# 2. 下载 ASR 模型（SenseVoice + X-ASR，约 1.5GB，到 models/）
cd ..
powershell -File download-models.ps1        # Windows
# bash download-models.sh                   # Linux/macOS

# 3. 前端静态构建
cd frontend
pnpm build

# 4. 启动
pnpm tauri:dev
```

## 会议总结：本地模型（llama-helper）

会议总结的"本地模型"方式由 `llama-helper`（llama.cpp sidecar）提供。编译它需要 libclang（llama-cpp-sys-2 的 bindgen）：

- 本仓库采用免安装方案：官方 LLVM Windows 安装包用 7-Zip 解压到 `.tooling/llvm/`（已被 .gitignore 忽略，勿提交）。
- 手动编译：

```powershell
$env:LIBCLANG_PATH = "D:\AI_CODING\realtime_transcription\voxminutes\.tooling\llvm\bin"
cargo build -p llama-helper --release   # 项目根目录
```

- `frontend/build-gpu.bat` 已内置自动探测（未设置 `LIBCLANG_PATH` 且 `.tooling\llvm` 存在时自动使用）。
- 构建产物需复制为 `frontend/src-tauri/binaries/llama-helper-x86_64-pc-windows-msvc.exe`（已注册 `externalBin`，开发和打包都从这里解析；`build-gpu.bat` 会自动复制）。
- 总结用 GGUF 模型在应用内「设置 → 模型下载」中获取（Qwen2.5-3B / Qwen3-4B-2507 / Gemma-3-4B-it，均 Q4_K_M，2~2.5GB），不随仓库分发。
- llama-helper 同时承载 Hy-MT2 翻译引擎推理（`translation/llm.rs` 经共享 `llama_sidecar` 调用）；其 generate 协议支持可选 `repeat_penalty` / `frequency_penalty`（Hy-MT2 使用 1.15 / 0.05）。

## 备用启动命令

偶发情况下 `pnpm tauri:dev` 的 pnpm.ps1 包装器在后台/某些终端里会崩溃，可绕过：

```bash
cd frontend
PATH="$PWD/node_modules/.bin:$PATH" node scripts/tauri-auto.js dev
```

## 打包发布

```bash
cd frontend
pnpm tauri:build
```

## 日志位置

| 日志 | 路径 |
|------|------|
| 应用运行日志（Rust + 前端转发） | `logs/app_*.log`（项目根目录，保留最近 30 个） |
| tauri dev 控制台输出 | 启动时的终端 |
| 录音文件 | `C:\Users\<用户名>\recordings\` |
| SQLite 数据库 | `%APPDATA%\com.voxminutes.app\meeting_minutes.sqlite` |

## 常用排障

| 症状 | 处理 |
|------|------|
| UI 改动没生效 | 忘了 `pnpm build`，重新构建后再启动 |
| 端口 3118 被占用 | `start-dev-server.ps1` 会自动清理；不行就手动 `taskkill //F //IM node.exe` |
| crates 下载慢/失败 | 已配置 USTC 镜像（`.cargo/config.toml`），删 `~/.cargo/registry` 重试 |
| GitHub 模型下载失败 | 应用内/脚本会自动回退 gh-proxy.com 镜像 |
| 录音存不进历史 | 看 `logs/app_*.log` 里 `api_save_transcript` 相关报错 |
