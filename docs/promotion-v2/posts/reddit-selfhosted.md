# Reddit r/selfhosted 发布文案

## 标题

```
VoxMinutes – free, open-source, fully local meeting assistant: records system audio + mic together, real-time transcription, translation & summaries (Windows)
```

## 正文

Hi r/selfhosted!

I built **VoxMinutes**, a 100% free, local-first meeting assistant for Windows (Tauri 2 + Rust, AGPL-3.0).

**Why:** Most meeting transcription tools either charge per minute, require uploading your audio to the cloud, or can only capture your mic OR system sound — not both. I wanted something that records an online meeting (the other side's voice + my own) and transcribes it live, entirely offline.

**What it does:**

- **Dual-channel recording** — system playback + microphone captured simultaneously and auto-mixed. Great for online meetings, lectures, interviews.
- **Real-time streaming transcription** with two local ASR engines:
  - X-ASR — streaming Chinese–English, 480 ms chunks, low latency
  - SenseVoice — multilingual (ZH/EN/JA/KO/Cantonese)
- **Live translation** — sentence-level pipeline; Hy-MT2 (Tencent Hunyuan) supports 13 target languages, plus a lightweight OPUS-MT option
- **AI meeting minutes** — generated offline by local GGUF LLMs (Qwen / Gemma), or via your own API
- **History & export** — SQLite-backed, searchable, export to TXT / SRT / Markdown
- **Runs on CPU alone** — all models are quantized; 8 GB RAM recommended, no GPU needed

**Privacy:** your audio and transcripts never leave your device. No accounts, no subscriptions, no paywall — every feature is free.

First launch includes an onboarding wizard that walks you through downloading models in-app (multi-source with mirror fallback, resumable, parallel) or importing local files — no command line needed. UI available in English / 中文 / 한국어 / 日本語.

- GitHub: https://github.com/Longt-audio/voxminutes
- Download (Windows installer): https://github.com/Longt-audio/voxminutes/releases

It's an early v0.1.x release — macOS/Linux support is planned. Feedback, bug reports and feature requests are very welcome!

*(Note: the installer isn't code-signed yet, so Windows may show a SmartScreen warning — click "More info → Run anyway".)*
