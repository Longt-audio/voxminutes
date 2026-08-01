<div align="center">

<img src="docs/assets/readme/icon.png" alt="VoxMinutes" width="96" />

# VoxMinutes

**Your local meeting assistant · 100% free · Records system audio & mic together · Real-time transcription, translation & summaries — all on your device**

<sub>您的本地会议助手 · 完全免费 · 系统声音与麦克风同步录制 · 实时转写、翻译与总结，数据不出设备</sub>

<sub>로컬 회의 어시스턴트 · 완전 무료 · 시스템 오디오와 마이크 동시 녹음 · 실시간 받아쓰기, 번역, 요약 — 데이터는 기기 밖으로 나가지 않습니다</sub>

<sub>ローカル会議アシスタント · 完全無料 · システム音声とマイクを同時録音 · リアルタイム文字起こし・翻訳・要約。データはデバイスの外に出ません</sub>

**English | [中文](README.zh-CN.md) | [한국어](README.ko.md) | [日本語](README.ja.md)**

[![License: AGPL-3.0](https://img.shields.io/badge/License-AGPL--3.0-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Windows%2010%2F11-lightgrey)]()
[![Models](https://img.shields.io/badge/models-100%25%20local-green)]()
[![Price](https://img.shields.io/badge/price-100%25%20free-brightgreen)]()
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg)](https://github.com/Longt-audio/voxminutes/pulls)

<video src="docs/promotion-v2/videos/voxminutes-v2-horizontal.mp4" controls width="860" poster="docs/assets/readme/english.gif"></video>

*👆 60-second demo: dual-channel recording, real-time transcription, live translation & AI meeting minutes. Watch the [30s short version](docs/promotion-v2/videos/voxminutes-v2-horizontal-short.mp4) or the [vertical version for social media](docs/promotion-v2/videos/voxminutes-v2-vertical.mp4).*

</div>

---

## Quick Links

[Features](#features) · [Why VoxMinutes](#why-voxminutes) · [Models](#model-support) · [Download & install](#download--install) · [Build from source](#build-from-source) · [Tech stack](#tech-stack) · [Roadmap](#roadmap) · [Privacy](#privacy) · [Contributing](#contributing)

---

## What is this

VoxMinutes is a **completely free, local-first** desktop app (Windows, Tauri 2) that transcribes your meetings in real time — recording **system playback and microphone input at the same time**, something many similar tools can't do. Transcripts can be translated live into 13 languages and turned into AI meeting minutes, with every model running on your own machine. **Your audio and text never leave your device — and every feature is free to use, no subscription, no paywall.**

On first launch, a built-in onboarding wizard walks you through downloading or importing the models you need (multi-source downloads via GitHub / HuggingFace mirrors / ModelScope, plus importing local archives or GGUF files).

## Why VoxMinutes

| Pain point | Typical tools | VoxMinutes |
|------------|---------------|------------|
| Cost | Otter.ai, 讯飞听见: per-minute pricing | **100% free, no subscription, no paywall** |
| Privacy | Audio uploaded to the cloud | **Everything runs on your device** |
| Recording | Only mic OR only system sound | **Records system audio + mic together** |
| Setup | Whisper-based apps need CLI / config | **In-app model manager, no terminal needed** |
| Compatibility | Browser plugins blocked by audio permissions | **System-level capture, works with any meeting app** |

> VoxMinutes works with Zoom, Teams, Google Meet, 腾讯会议, 飞书, webinars, online lectures, and interviews — because it captures audio at the system level, not inside a specific app.

## Quick Start

1. **Download** the latest Windows installer from [Releases](https://github.com/Longt-audio/voxminutes/releases) (~50 MB).
2. **Install & launch** — the onboarding wizard helps you download an ASR model (required) and optional translation / summary models. No proxy needed in mainland China: download sources automatically fall back to hf-mirror / gh-proxy / ModelScope.
3. **Press record** — start a meeting, watch text appear in real time, translate live, and hit "Meeting Summary" when you're done.

> ⚠️ **Note for Windows:** the installer isn't code-signed yet, so SmartScreen may show a blue warning — click **More info → Run anyway**. It's safe; the source is open.

## Features

- **Real-time streaming transcription** with two local ASR engines:
  - `X-ASR` — pure streaming Chinese–English, 480 ms chunks, low latency
  - `SenseVoice` — multilingual (ZH/EN/JA/KO/Cantonese), VAD pseudo-streaming
- **Dual-channel recording** — system audio + microphone captured together, auto-mixed
- **Real-time translation** — sentence-level pipeline with token-streamed output
  - `OPUS-MT` — small and fast, Chinese ⇄ English
  - `Hy-MT2` (Tencent Hunyuan) — higher quality, **13 target languages**
- **AI meeting summaries** — offline via local GGUF LLMs (Qwen / Gemma), or remote APIs / web AI; results in a dedicated panel, exportable to Markdown
- **Translate page** — paste or type any text, source language auto-detected
- **History** — SQLite-backed, with search and inline editing of titles/segments
- **File transcription** — import audio files, re-transcribe with another engine
- **Export** — TXT / SRT / Markdown / summary Markdown
- **Model manager** — in-app downloads (multi-source, resumable, parallel) or local import, no command line needed
- **Localized UI** — English / 中文 / 한국어 / 日本語, switchable from the welcome screen

## Model support

All models are **downloaded in-app or imported by the user** — the installer ships with no models. Sources fall back in order (official → mirror), so downloads work out of the box in mainland China as well.

| Model | Purpose | Size | Sources |
|-------|---------|------|---------|
| SenseVoice (sherpa-onnx) | ASR: ZH/EN/JA/KO/Cantonese | ~854 MB | GitHub Releases / gh-proxy |
| X-ASR 480ms (sherpa-onnx) | ASR: ZH/EN streaming | ~557 MB | GitHub Releases / gh-proxy |
| OPUS-MT ZH→EN / EN→ZH | Translation (fast) | ~113 MB each | HuggingFace / hf-mirror / ModelScope |
| Hy-MT2-1.8B (Tencent Hunyuan) | Translation (quality, 13 targets) | ~1.1 GB | HuggingFace / hf-mirror |
| Qwen2.5-3B-Instruct | Summaries (smaller, faster) | ~2.1 GB | HuggingFace / hf-mirror / ModelScope |
| Qwen3-4B-Instruct-2507 | Summaries (better quality) | ~2.5 GB | HuggingFace / hf-mirror |
| Gemma-3-4B-it | Summaries (stronger English) | ~2.5 GB | HuggingFace / hf-mirror |

Notes:

- All models are Q4/int8 quantized and **run on CPU alone** (Hy-MT2 translates a sentence in ~2–4 s on an 8-thread machine); ASR real-time factor ≈ 0.25
- Models live in the app's model directory; view, delete, or import them in Settings (`.tar.bz2` / `.tar.gz` / `.zip` archives or `.gguf` files)
- Only the registered models above are supported; custom models are not supported yet

### Manual download

If in-app downloads are slow, copy the links into a download manager (IDM, aria2, etc.), then install via **Settings → Models → Import** (accepts `.tar.bz2` / `.tar.gz` / `.zip` archives, `.gguf` files, or a prepared model folder):

- **SenseVoice** (.tar.bz2): [GitHub](https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-sense-voice-zh-en-ja-ko-yue-2024-07-17.tar.bz2) / [gh-proxy mirror](https://gh-proxy.com/https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-sense-voice-zh-en-ja-ko-yue-2024-07-17.tar.bz2)
- **X-ASR 480ms** (.tar.bz2): [GitHub](https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-x-asr-480ms-streaming-zipformer-transducer-zh-en-punct-2026-06-05.tar.bz2) / [gh-proxy mirror](https://gh-proxy.com/https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-x-asr-480ms-streaming-zipformer-transducer-zh-en-punct-2026-06-05.tar.bz2)
- **OPUS-MT ZH→EN** (needs `encoder_model_int8.onnx`, `decoder_model_merged_int8.onnx`, `tokenizer.json`): [HuggingFace](https://huggingface.co/Xenova/opus-mt-zh-en) / [ModelScope](https://modelscope.cn/models/Xenova/opus-mt-zh-en); **EN→ZH**: [HuggingFace](https://huggingface.co/Xenova/opus-mt-en-zh) / [ModelScope](https://modelscope.cn/models/Xenova/opus-mt-en-zh)
- **Hy-MT2-1.8B** (.gguf): [HuggingFace](https://huggingface.co/tencent/Hy-MT2-1.8B-GGUF/resolve/main/Hy-MT2-1.8B-Q4_K_M.gguf) / [hf-mirror](https://hf-mirror.com/tencent/Hy-MT2-1.8B-GGUF/resolve/main/Hy-MT2-1.8B-Q4_K_M.gguf)
- **Qwen2.5-3B-Instruct** (.gguf): [HuggingFace](https://huggingface.co/Qwen/Qwen2.5-3B-Instruct-GGUF/resolve/main/qwen2.5-3b-instruct-q4_k_m.gguf) / [hf-mirror](https://hf-mirror.com/Qwen/Qwen2.5-3B-Instruct-GGUF/resolve/main/qwen2.5-3b-instruct-q4_k_m.gguf) / [ModelScope](https://modelscope.cn/models/Qwen/Qwen2.5-3B-Instruct-GGUF/resolve/master/qwen2.5-3b-instruct-q4_k_m.gguf)
- **Qwen3-4B-Instruct-2507** (.gguf): [HuggingFace](https://huggingface.co/bartowski/Qwen_Qwen3-4B-Instruct-2507-GGUF/resolve/main/Qwen_Qwen3-4B-Instruct-2507-Q4_K_M.gguf) / [hf-mirror](https://hf-mirror.com/bartowski/Qwen_Qwen3-4B-Instruct-2507-GGUF/resolve/main/Qwen_Qwen3-4B-Instruct-2507-Q4_K_M.gguf)
- **Gemma-3-4B-it** (.gguf): [HuggingFace](https://huggingface.co/bartowski/google_gemma-3-4b-it-GGUF/resolve/main/google_gemma-3-4b-it-Q4_K_M.gguf) / [hf-mirror](https://hf-mirror.com/bartowski/google_gemma-3-4b-it-GGUF/resolve/main/google_gemma-3-4b-it-Q4_K_M.gguf)

## UI at a glance

- **Transcribe**: recording controls + live text + inline translations (see the demo above)
- **History**: list / search / detail editing / export / AI summary
- **Translate**: type and translate, 13 target languages
- **Settings**: models (download/import/delete), audio & export, API, advanced

## Download & install

1. Grab the latest Windows installer (or portable package) from [Releases](../../releases)
2. Install and launch — the **onboarding wizard** appears automatically and guides you through downloading or importing an ASR model (required), plus translation and summary models (optional)
3. Models can also be managed anytime under **Settings → Models**

> Tip: no proxy is needed in mainland China — download sources automatically fall back to reachable mirrors (hf-mirror / gh-proxy / ModelScope).

## Build from source

Requirements: Windows 10/11 x64, Git, Node.js LTS, pnpm, Rust 1.77+, CMake, VS2022 Build Tools (macOS / Linux support planned).

```bash
git clone https://github.com/Longt-audio/voxminutes.git
cd voxminutes/frontend
pnpm install --ignore-workspace
pnpm build
pnpm tauri:dev
```

For local LLM features (Hy-MT2 translation / local summaries), also build the llama-helper sidecar (requires libclang):

```powershell
$env:LIBCLANG_PATH = "<repo root>\.tooling\llvm\bin"
cargo build -p llama-helper --release
copy target\release\llama-helper.exe frontend\src-tauri\binaries\llama-helper-x86_64-pc-windows-msvc.exe
```

See [docs/DEV_COMMANDS.md](docs/DEV_COMMANDS.md) for more development commands.

## Tech stack

| Layer | Technology |
|-------|------------|
| Desktop | Tauri 2 + Next.js (static export) + React + Tailwind CSS |
| System | Rust |
| ASR | sherpa-onnx (SenseVoice / X-ASR, ONNX Runtime) |
| Translation | OPUS-MT (ONNX Runtime) / Hy-MT2 (llama.cpp sidecar) |
| Summaries | llama.cpp sidecar (GGUF, Qwen / Gemma) + OpenAI-compatible remote APIs |
| Database | SQLite |

## Roadmap

| Version | Goal |
|---------|------|
| v0.1.0 (current MVP) | Real-time & file transcription, dual-engine translation, local summaries, history & export, model download/import, onboarding |
| v0.2.0 | TTS, subtitle overlay window, selection translation |
| v0.3.0 | Push-to-talk live interpreting, live summaries, speaker diarization |
| Future (paid) | Cloud high-accuracy ASR, team workspace |

> 💡 Want macOS / Linux, TTS, or speaker diarization sooner? Open an issue or vote on existing ones — roadmap priorities follow community demand.

## Privacy

Recording, transcription, translation, and summarization all happen on your device. Unless you explicitly configure a remote summary API, the app exchanges no content with any server.

## License

This project is licensed under **AGPL-3.0** — see [LICENSE](LICENSE).

## Contributing

Issues and pull requests are welcome. Please make sure `cargo check --workspace`, `cargo test`, and `cd frontend && pnpm build` pass before submitting.

- 🐛 Report bugs & request features via [Issues](https://github.com/Longt-audio/voxminutes/issues)
- 💬 Ask questions & share ideas in [Discussions](https://github.com/Longt-audio/voxminutes/discussions)
- ⭐ If VoxMinutes helps you, **star the repo** — it helps more people discover it!

---

**VoxMinutes** — your voice, your data.
