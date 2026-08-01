# Reddit r/software / r/opensource 发布文案

## 标题

```
I built a free, offline meeting assistant for Windows that records both system audio and microphone
```

## 正文

Hi everyone,

I made **VoxMinutes**, a free and open-source Windows app for meeting transcription.

The thing that bothered me about existing tools: they either cost money, upload your audio to the cloud, or only record one side of an online meeting. VoxMinutes records **system playback + microphone at the same time**, transcribes live, and does it all offline.

What it does:

- Dual-channel recording (system audio + mic, auto-mixed)
- Real-time streaming transcription (X-ASR / SenseVoice)
- Live translation into 13 languages
- AI meeting summaries via local GGUF models or your own API
- Searchable history, export to TXT / SRT / Markdown
- Model manager with in-app downloads and local import
- UI in English / 中文 / 한국어 / 日本語

Everything runs locally. No accounts, no subscriptions, no paywall.

Built with Tauri 2 + Rust + sherpa-onnx + llama.cpp. AGPL-3.0.

- GitHub: https://github.com/Longt-audio/voxminutes
- Download: https://github.com/Longt-audio/voxminutes/releases

It's an early v0.1.x release. Windows 10/11 x64 only for now; macOS and Linux are planned. Feedback welcome!

*(Installer isn't code-signed yet; Windows may show a SmartScreen warning — click "More info → Run anyway".)*
