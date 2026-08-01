# Product Hunt 发布文案

## Product Name

VoxMinutes

## Tagline

Free, local-first meeting assistant for Windows. Records system audio + mic together; transcribes, translates & summarizes offline.

## Description

VoxMinutes is a completely free, open-source, local-first desktop app for Windows that turns your meetings into searchable text — without sending any audio or transcript to the cloud.

Most transcription tools either charge per minute, require cloud uploads, or can only record your microphone. VoxMinutes records **both system playback and your microphone at the same time**, so it works for Zoom, Teams, Google Meet, webinars, online lectures, and interviews.

**Key features:**

- Real-time streaming transcription with local ASR engines (X-ASR, SenseVoice)
- Live translation into 13 languages via Hy-MT2 or lightweight OPUS-MT
- AI meeting summaries generated locally with GGUF LLMs (Qwen / Gemma) or your own API
- Searchable history with export to TXT, SRT, and Markdown
- Built-in model manager — download or import models without touching the terminal
- Runs entirely on CPU; no GPU required
- UI in English, 中文, 한국어, 日本語

**Privacy first:** your data never leaves your device. No accounts, no subscriptions, no paywalls.

Open source under AGPL-3.0.

## Topics

Productivity, Open Source, AI, Privacy, Windows

## Maker Comment（发布者首评）

```
Hi Product Hunt! I'm the maker of VoxMinutes. I built it because I was tired of meeting tools that wanted my audio in their cloud or charged per minute. Happy to answer any questions about local ASR, Tauri, or the roadmap. macOS and Linux are planned next.
```

## Gallery Text Suggestions

1. Hero screenshot: VoxMinutes transcribing live with translation visible
2. Feature card: "System audio + microphone, recorded together"
3. Feature card: "13 languages, translated live, offline"
4. Feature card: "AI summaries generated on your device"
5. Feature card: "No accounts, no subscriptions, no cloud"

## First Comment Replies

**Q: Is it really free?**
> Yes, 100% free and open source under AGPL-3.0. No hidden tiers.

**Q: Does it work on Mac?**
> Not yet — Windows 10/11 only for now. macOS and Linux are on the roadmap.

**Q: How accurate is the transcription?**
> Very good for clear English/Chinese with SenseVoice or X-ASR. Noisy environments and heavy accents are still challenging, as with any local model.
