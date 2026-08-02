<div align="center">

<img src="docs/assets/readme/icon.png" alt="VoxMinutes" width="96" />

# VoxMinutes

**您的本地会议助手 · 完全免费 · 系统声音与麦克风同步录制 · 实时转写、13 语言翻译与 AI 总结，数据不出设备**

**[English](README.md) | 中文 | [한국어](README.ko.md) | [日本語](README.ja.md)**

[![License: AGPL-3.0](https://img.shields.io/badge/License-AGPL--3.0-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Windows%2010%2F11-lightgrey)]()
[![Models](https://img.shields.io/badge/models-100%25%20local-green)]()
[![Price](https://img.shields.io/badge/price-100%25%20free-brightgreen)]()
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg)](https://github.com/Longt-audio/voxminutes/pulls)

<img src="docs/assets/readme/demo.gif" alt="VoxMinutes 实时转写与翻译演示" width="860" />

*👆 动图演示：双路录音、实时转写、实时翻译与 AI 会议纪要。想看带声音的完整演示？[60 秒视频](docs/promotion-v2/videos/voxminutes-v2-horizontal.mp4) 或 [30 秒精华版](docs/promotion-v2/videos/voxminutes-v2-horizontal-short.mp4)。*

</div>

---

## 快速导航

[功能特性](#功能特性) · [为什么选择 VoxMinutes](#为什么选择-voxminutes) · [模型支持](#模型支持) · [下载与安装](#下载与安装) · [从源码构建](#从源码构建) · [技术栈](#技术栈) · [路线图](#路线图) · [隐私](#隐私) · [参与贡献](#参与贡献)

---

## 这是什么

VoxMinutes 是一款**完全免费、本地优先**的桌面应用（Windows，Tauri 2）：一边开会一边实时转写，同时把系统播放的声音和麦克风输入**两路音频同时录下**——很多同类工具只支持麦克风。转写结果可以实时翻译成 13 种语言、一键生成 AI 会议纪要，全部模型都在你自己的电脑上运行，**音频与文本从不出设备，所有功能完全免费，无订阅、无付费墙**。

首次启动时，内置的新手指引会带你下载或导入所需模型（支持 GitHub、HuggingFace 镜像、ModelScope 多源下载，也支持本地压缩包 / GGUF 文件导入）。

## 为什么选择 VoxMinutes

| 痛点 | 常见工具 | VoxMinutes |
|------|----------|------------|
| 费用 | Otter.ai、讯飞听见：按分钟收费 | **完全免费，无订阅、无付费墙** |
| 隐私 | 音频上传云端 | **一切都在你的设备上运行** |
| 录音 | 只能录麦克风或只能录系统声音 | **系统声音 + 麦克风同时录制** |
| 上手难度 | Whisper 类工具需要命令行/配置 | **应用内模型管家，无需终端** |
| 兼容性 | 浏览器插件受音频权限限制 | **系统级采集，适配任何会议软件** |

> VoxMinutes 适用于 Zoom、腾讯会议、飞书、Teams、Google Meet、网课、访谈等场景——因为它在系统层面采集音频，不依赖特定应用。

## 快速开始

1. **下载**最新 Windows 安装包（约 50 MB）：[Releases](https://github.com/Longt-audio/voxminutes/releases)
2. **安装并启动**——新手指引会帮你下载 ASR 模型（必装）以及可选的翻译/总结模型。中国大陆无需代理：下载源会自动回退到 hf-mirror / gh-proxy / ModelScope。
3. **按下录音**——开会时看文字实时出现，随时实时翻译，结束后点「Meeting Summary」一键生成纪要。

> ⚠️ **Windows 提示：** 安装包暂未代码签名，SmartScreen 可能出现蓝色警告——点击**更多信息 → 仍要运行**即可。源码公开，请放心。

## 功能特性

- **实时流式转写**：两种本地 ASR 引擎可选
  - `X-ASR`（中英双语纯流式，480ms chunk，低延迟）
  - `SenseVoice`（中/英/日/韩/粤多语言，VAD 伪流式）
- **双路录音**：系统播放 + 麦克风同时录制，自动混流
- **实时翻译**：句级流水线，译文逐字流式显示
  - `OPUS-MT`：轻量快速，中英互译
  - `Hy-MT2`（腾讯混元）：更高质量，**13 种目标语言**
- **AI 会议纪要**：本地 GGUF 大模型（Qwen / Gemma）离线总结，也可用远程 API 或网页 AI；结果独立面板展示，可导出 Markdown
- **翻译页**：文本即输即译，自动识别源语言
- **历史记录**：SQLite 存储，支持搜索、标题/段落行内编辑
- **文件转写**：导入音频文件离线转写、重新识别
- **导出**：TXT / SRT / Markdown / 会议总结 Markdown
- **模型管家**：应用内下载（多源 + 断点续传 + 多模型并行）或本地导入，无需命令行
- **多语言界面**：English / 中文 / 한국어 / 日本語，欢迎页即可切换

## 模型支持

所有模型均为**应用内下载或用户自行导入**，安装包不内置任何模型。下载源按序自动回退（官方源 → 国内镜像），国内网络默认可用。

| 模型 | 用途 | 大小 | 下载源 |
|------|------|------|--------|
| SenseVoice（sherpa-onnx） | ASR：中/英/日/韩/粤 | ~854 MB | GitHub Releases / gh-proxy |
| X-ASR 480ms（sherpa-onnx） | ASR：中英纯流式 | ~557 MB | GitHub Releases / gh-proxy |
| OPUS-MT 中→英 / 英→中 | 翻译（快速） | 各 ~113 MB | HuggingFace / hf-mirror / ModelScope |
| Hy-MT2-1.8B（腾讯混元） | 翻译（高质量，13 种目标语言） | ~1.1 GB | HuggingFace / hf-mirror |
| Qwen2.5-3B-Instruct | 会议总结（较小较快） | ~2.1 GB | HuggingFace / hf-mirror / ModelScope |
| Qwen3-4B-Instruct-2507 | 会议总结（质量更好） | ~2.5 GB | HuggingFace / hf-mirror |
| Gemma-3-4B-it | 会议总结（英文较强） | ~2.5 GB | HuggingFace / hf-mirror |

说明：

- 全部模型为 Q4/int8 量化，**纯 CPU 即可运行**（约 8 线程的机器上 Hy-MT2 实时翻译约 2~4 秒/句）；ASR 实时转写 RTF ≈ 0.25
- 模型文件存放在应用模型目录，可在设置页查看、删除、导入（`.tar.bz2` / `.tar.gz` / `.zip` 压缩包或 `.gguf` 文件）
- 仅支持上表注册的模型，暂不支持自定义模型

### 手动下载

应用内下载较慢时，可把链接复制到下载器（迅雷 / IDM / aria2），完成后在「设置 → 模型」点「导入」安装（支持 `.tar.bz2` / `.tar.gz` / `.zip` 压缩包、`.gguf` 文件、已备齐文件的模型文件夹）：

- **SenseVoice**（.tar.bz2）：[GitHub](https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-sense-voice-zh-en-ja-ko-yue-2024-07-17.tar.bz2) / [gh-proxy 镜像](https://gh-proxy.com/https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-sense-voice-zh-en-ja-ko-yue-2024-07-17.tar.bz2)
- **X-ASR 480ms**（.tar.bz2）：[GitHub](https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-x-asr-480ms-streaming-zipformer-transducer-zh-en-punct-2026-06-05.tar.bz2) / [gh-proxy 镜像](https://gh-proxy.com/https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-x-asr-480ms-streaming-zipformer-transducer-zh-en-punct-2026-06-05.tar.bz2)
- **OPUS-MT 中→英**（需 `encoder_model_int8.onnx`、`decoder_model_merged_int8.onnx`、`tokenizer.json` 三个文件）：[HuggingFace](https://huggingface.co/Xenova/opus-mt-zh-en) / [ModelScope](https://modelscope.cn/models/Xenova/opus-mt-zh-en)；**英→中**：[HuggingFace](https://huggingface.co/Xenova/opus-mt-en-zh) / [ModelScope](https://modelscope.cn/models/Xenova/opus-mt-en-zh)
- **Hy-MT2-1.8B**（.gguf）：[HuggingFace](https://huggingface.co/tencent/Hy-MT2-1.8B-GGUF/resolve/main/Hy-MT2-1.8B-Q4_K_M.gguf) / [hf-mirror 镜像](https://hf-mirror.com/tencent/Hy-MT2-1.8B-GGUF/resolve/main/Hy-MT2-1.8B-Q4_K_M.gguf)
- **Qwen2.5-3B-Instruct**（.gguf）：[HuggingFace](https://huggingface.co/Qwen/Qwen2.5-3B-Instruct-GGUF/resolve/main/qwen2.5-3b-instruct-q4_k_m.gguf) / [hf-mirror 镜像](https://hf-mirror.com/Qwen/Qwen2.5-3B-Instruct-GGUF/resolve/main/qwen2.5-3b-instruct-q4_k_m.gguf) / [ModelScope](https://modelscope.cn/models/Qwen/Qwen2.5-3B-Instruct-GGUF/resolve/master/qwen2.5-3b-instruct-q4_k_m.gguf)
- **Qwen3-4B-Instruct-2507**（.gguf）：[HuggingFace](https://huggingface.co/bartowski/Qwen_Qwen3-4B-Instruct-2507-GGUF/resolve/main/Qwen_Qwen3-4B-Instruct-2507-Q4_K_M.gguf) / [hf-mirror 镜像](https://hf-mirror.com/bartowski/Qwen_Qwen3-4B-Instruct-2507-GGUF/resolve/main/Qwen_Qwen3-4B-Instruct-2507-Q4_K_M.gguf)
- **Gemma-3-4B-it**（.gguf）：[HuggingFace](https://huggingface.co/bartowski/google_gemma-3-4b-it-GGUF/resolve/main/google_gemma-3-4b-it-Q4_K_M.gguf) / [hf-mirror 镜像](https://hf-mirror.com/bartowski/google_gemma-3-4b-it-GGUF/resolve/main/google_gemma-3-4b-it-Q4_K_M.gguf)

## 界面预览

- **实时转录**：录音控制 + 实时文本 + 内联译文（见上方演示视频）
- **历史记录**：列表 / 搜索 / 详情编辑 / 导出 / AI 总结
- **翻译**：即输即译，13 种目标语言
- **设置**：模型（下载/导入/删除）、音频与导出、API、高级

## 下载与安装

1. 从 [Releases](../../releases) 下载最新 Windows 安装包（或便携包）
2. 安装并启动，**新手指引**会自动弹出，引导你下载或导入 ASR 模型（必装）、翻译与总结模型（可选）
3. 模型之后也可以随时在「设置 → 模型」中下载 / 导入 / 删除

> 提示：中国大陆用户无需任何代理，下载源会自动回退到可用镜像（hf-mirror / gh-proxy / ModelScope）。

## 从源码构建

环境：Windows 10/11 x64、Git、Node.js LTS、pnpm、Rust 1.77+、CMake、VS2022 Build Tools（macOS / Linux 支持计划中）。

```bash
git clone https://github.com/Longt-audio/voxminutes.git
cd voxminutes/frontend
pnpm install --ignore-workspace
pnpm build
pnpm tauri:dev
```

如需本地 LLM 功能（Hy-MT2 翻译 / 本地会议总结），还要编译 llama-helper sidecar（需要 libclang）：

```powershell
$env:LIBCLANG_PATH = "<项目根>\.tooling\llvm\bin"
cargo build -p llama-helper --release
copy target\release\llama-helper.exe frontend\src-tauri\binaries\llama-helper-x86_64-pc-windows-msvc.exe
```

更多开发命令见 [docs/DEV_COMMANDS.md](docs/DEV_COMMANDS.md)。

## 技术栈

| 层 | 技术 |
|----|------|
| 桌面框架 | Tauri 2 + Next.js（静态导出）+ React + Tailwind CSS |
| 系统层 | Rust |
| ASR | sherpa-onnx（SenseVoice / X-ASR，ONNX Runtime） |
| 机器翻译 | OPUS-MT（ONNX Runtime）/ Hy-MT2（llama.cpp sidecar） |
| 会议总结 | llama.cpp sidecar（GGUF，Qwen / Gemma）+ OpenAI 兼容远程 API |
| 数据库 | SQLite |

## 路线图

| 版本 | 目标 |
|------|------|
| v0.1.0（当前 MVP） | 实时/离线转写、双引擎翻译、本地会议总结、历史与导出、模型下载/导入、首启引导 |
| v0.2.0 | TTS 语音合成、字幕悬浮窗、划词翻译 |
| v0.3.0 | 按键实时传译、实时摘要、说话人识别 |
| 未来（付费版） | 云端高精度 ASR、团队协作 |

> 💡 想要 macOS / Linux、TTS 或说话人分离更快落地？去提 issue 或在已有 issue 里投票——路线图优先级会跟随社区需求。

## 隐私

音频、转写、翻译与总结全部在你的设备上完成；除非你主动配置远程总结 API，应用不与任何服务器交换你的内容。

## 开源协议

本项目采用 **AGPL-3.0** 协议，详见 [LICENSE](LICENSE)。

## 参与贡献

欢迎 Issue 与 Pull Request。提交前请确保 `cargo check --workspace`、`cargo test` 与 `cd frontend && pnpm build` 通过。

- 🐛 报 Bug / 提需求：[Issues](https://github.com/Longt-audio/voxminutes/issues)
- 💬 提问 / 交流想法：[Discussions](https://github.com/Longt-audio/voxminutes/discussions)
- ⭐ 如果 VoxMinutes 对你有帮助，**点个 Star**——这能帮更多人发现它！

---

**VoxMinutes** — 你的声音，你的数据。
