# Hacker News Show HN 发布文案

## 标题

```
Show HN: VoxMinutes – free, offline meeting assistant for Windows
```

## 正文

I built VoxMinutes, a free, open-source, local-first meeting assistant for Windows. It records system audio and microphone together, then transcribes, translates, and summarizes entirely offline.

**Why:** Existing tools either upload audio to the cloud, charge by the minute, or only capture one audio source. I wanted a self-hosted alternative for online meetings and interviews.

**Stack:**

- Desktop: Tauri 2 + Next.js + React
- System / audio: Rust
- ASR: sherpa-onnx (X-ASR, SenseVoice)
- Translation: OPUS-MT (ONNX) / Hy-MT2 (llama.cpp sidecar)
- Summaries: local GGUF models (Qwen / Gemma) or OpenAI-compatible API
- Database: SQLite

**Highlights:**

- Dual-channel recording (system playback + mic, auto-mixed)
- Real-time streaming transcription
- Live translation into 13 languages
- Local AI meeting summaries
- Model manager with multi-source downloads and local import
- CPU-only, quantized models
- UI in EN / ZH / JA / KO

License: AGPL-3.0

Repo: https://github.com/Longt-audio/voxminutes
Download: https://github.com/Longt-audio/voxminutes/releases

It's an early v0.1.x release. Windows 10/11 x64 only; macOS/Linux are planned. I’d love feedback on accuracy, UX, and the model-download flow.
