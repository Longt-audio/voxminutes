# VoxMinutes 发布日工具包（Day-0 Launch Pack）

> 用途：发布日当天直接复制粘贴，无需再想文案。
> 配套文件：
> - 画廊图：本目录 `gallery/`（5 张 1600×900）
> - 视频：`docs/promotion-v2/videos/`（60s 横版 / 30s 精华版 / 竖屏版）
> - 仓库：https://github.com/Longt-audio/voxminutes
> - 下载：https://github.com/Longt-audio/voxminutes/releases

---

## 推荐发布日程

| 日期 | 时间（PDT） | 时间（GMT+8） | 平台 | 备注 |
|------|------------|---------------|------|------|
| 发布前 1 天 | - | 晚上 | 检查仓库、准备评论区回复模板 | 本工具包已含 |
| **发布日（建议周三）** | 07:30 | 22:30 | 提前登录所有平台 | 检查账号可用 |
| 发布日 | **08:00** | **23:00** | **Product Hunt** | 最重要的引爆点 |
| 发布日 | 08:30 | 23:30 | Hacker News (Show HN) | 间隔 30 分钟 |
| 发布日 | 09:00 | 次日 00:00 | Twitter/X thread | 附视频/GIF |
| 发布日 | 09:30 | 次日 00:30 | LinkedIn | 职场人群 |
| 发布日 | **21:00** | 次日 05:00 | V2EX「分享创造」 | 国内晚间高峰 |
| 发布日 | 21:30 | 次日 05:30 | 即刻 + 小红书 | 竖屏视频 |
| 次日 | 09:00 | 次日 17:00 | Reddit r/software | 海外次日补充 |
| 周六 | 全天 | 全天 | Reddit r/selfhosted | Self-Promotion Saturday |
| 发布日 +3 | 20:00 | 次日 04:00 | 知乎文章 + B 站视频 | 长内容沉淀 |

> 💡 今天是 2026-08-02（周日），Product Hunt 最佳发布日是周二~周四，**建议定在 8 月 5 日（周三）**。发布前 1 天晚上做好所有准备。

---

## 一、Product Hunt 发布内容

### 基本信息

| 字段 | 内容 |
|------|------|
| Product Name | VoxMinutes |
| Tagline（≤60 字符） | `Local-first meeting assistant; records system audio + mic` |
| Website | `https://github.com/Longt-audio/voxminutes` |
| Topics | Productivity, AI, Open Source, Privacy, Windows |
| Gallery（5 张） | 见下方画廊清单 |

### Description（正文，直接粘贴）

```
VoxMinutes is a completely free, open-source, local-first desktop app for Windows that turns your meetings into searchable text — without sending any audio or transcript to the cloud.

Most transcription tools either charge per minute, require cloud uploads, or can only record your microphone. VoxMinutes records BOTH system playback and your microphone at the same time, so it works for Zoom, Teams, Google Meet, webinars, online lectures, and interviews.

Key features:
- Real-time streaming transcription with local ASR engines (X-ASR, SenseVoice)
- Live translation into 13 languages (Chinese, English, Japanese, Korean, Cantonese & more)
- AI meeting summaries generated locally with GGUF LLMs (Qwen / Gemma) or your own API
- Searchable history with export to TXT, SRT, and Markdown
- Built-in model manager — download or import models without touching the terminal
- Runs entirely on CPU; no GPU required
- UI in English, 中文, 한국어, 日本語

Privacy first: your data never leaves your device. No accounts, no subscriptions, no paywalls.

Open source under AGPL-3.0. Built with Tauri 2, Rust, sherpa-onnx & llama.cpp.
```

### Maker Comment（发布者首条评论，直接粘贴）

```
Hi Product Hunt! I'm the maker of VoxMinutes.

I built it because I was tired of meeting tools that either charged per minute, wanted my audio in their cloud, or could only record one side of an online meeting.

The killer feature: VoxMinutes records system audio AND your microphone at the same time — so the other side's voice and yours both become searchable text, live, with zero data leaving your device.

Happy to answer any questions about local ASR, Tauri, or the roadmap. macOS and Linux are planned next!
```

### 画廊清单（5 张，按顺序上传）

| 顺序 | 文件 | 作用 |
|------|------|------|
| 1 | `gallery/hero.png` | 主图：App 界面 + 品牌口号 |
| 2 | `gallery/card-dual-channel.png` | 卖点：系统声音+麦克风双录 |
| 3 | `gallery/card-transcribe.png` | 卖点：实时转写 |
| 4 | `gallery/card-translate.png` | 卖点：13 语言实时翻译 |
| 5 | `gallery/card-summary.png` | 卖点：本地 AI 会议纪要 |

---

## 二、Hacker News（Show HN，直接粘贴）

**标题：**
```
Show HN: VoxMinutes – free, offline meeting assistant for Windows
```

**正文：**
```
I built VoxMinutes, a free, open-source, local-first meeting assistant for Windows. It records system audio and microphone together, then transcribes, translates, and summarizes entirely offline.

Why: existing tools either upload audio to the cloud, charge by the minute, or only capture one audio source. I wanted a self-hosted alternative for online meetings and interviews.

Stack:
- Desktop: Tauri 2 + Next.js + React
- System / audio: Rust
- ASR: sherpa-onnx (X-ASR, SenseVoice)
- Translation: OPUS-MT (ONNX) / Hy-MT2 (llama.cpp sidecar)
- Summaries: local GGUF models (Qwen / Gemma) or OpenAI-compatible API
- Database: SQLite

Highlights:
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

It's an early v0.1.x release. Windows 10/11 x64 only; macOS/Linux are planned. I'd love feedback on accuracy, UX, and the model-download flow.
```

> ⚠️ HN 注意：不要加任何营销性 emoji / 标题党，工程师风格即可。主动在评论里披露局限（无代码签名、macOS/Linux 未支持）。

---

## 三、Reddit r/selfhosted（周六发，直接粘贴）

**标题：**
```
VoxMinutes – free, open-source, fully local meeting assistant: records system audio + mic together, real-time transcription, translation & summaries (Windows)
```

**正文：**
```
Hi r/selfhosted!

I built VoxMinutes, a 100% free, local-first meeting assistant for Windows (Tauri 2 + Rust, AGPL-3.0).

Why: Most meeting transcription tools either charge per minute, require uploading your audio to the cloud, or can only capture your mic OR system sound — not both.

What it does:
- Dual-channel recording — system playback + microphone captured simultaneously and auto-mixed. Great for online meetings, lectures, interviews
- Real-time streaming transcription (X-ASR: streaming Chinese–English; SenseVoice: ZH/EN/JA/KO/Cantonese)
- Live translation — Hy-MT2 (Tencent Hunyuan) supports 13 target languages, plus lightweight OPUS-MT
- AI meeting minutes — generated offline by local GGUF LLMs (Qwen / Gemma), or via your own API
- History & export — SQLite-backed, searchable, export to TXT / SRT / Markdown
- Runs on CPU alone; 8 GB RAM recommended, no GPU needed

Privacy: your audio and transcripts never leave your device. No accounts, no subscriptions, no paywall.

First launch includes an onboarding wizard with in-app model downloads (multi-source, mirror fallback) or local import — no command line needed. UI in English / 中文 / 한국어 / 日本語.

GitHub: https://github.com/Longt-audio/voxminutes
Download: https://github.com/Longt-audio/voxminutes/releases

Early v0.1.x release; macOS/Linux planned. Feedback welcome!
(Installer isn't code-signed yet — SmartScreen may warn; click "More info → Run anyway".)
```

---

## 四、Twitter/X Thread（发 5 条，直接粘贴）

**推 1（附视频）**
```
Free + offline meeting assistant for Windows 🎙️
Records system audio AND mic together, transcribes in real time.

No cloud. No subscription. Open source.

https://github.com/Longt-audio/voxminutes
```

**推 2**
```
Why I built it: most meeting tools charge per minute, upload your audio, or can only record ONE side of an online call.

I wanted the other side's voice + mine both captured and transcribed — entirely on-device.
```

**推 3**
```
What VoxMinutes does:
- Records system audio + mic simultaneously
- Real-time transcription (X-ASR / SenseVoice)
- Live translation into 13 languages
- AI meeting minutes from local GGUF models
- Export to TXT / SRT / Markdown
```

**推 4**
```
Privacy is the whole point: your audio and transcripts never leave your device. No accounts, no paywalls, every feature free.

Built with Tauri 2 + Rust + sherpa-onnx + llama.cpp. AGPL-3.0.
```

**推 5**
```
Windows 10/11 installer: https://github.com/Longt-audio/voxminutes/releases
macOS & Linux are on the roadmap.

Star the repo if this is useful to you ⭐
```

> 建议发布后置顶推 1，把推 5 挂在评论区。

---

## 五、V2EX（国内晚间发，直接粘贴）

**标题：**
```
【开源免费】VoxMinutes：完全本地运行的会议助手 —— 系统声音+麦克风同步录制，实时转写/翻译/AI 总结，数据不出设备
```

**正文：**
```
大家好，分享一下我独立开发的一款桌面应用 VoxMinutes。

这是一个 100% 免费、完全本地运行的 Windows 会议助手，核心解决一个问题：开线上会议时，对方的声音（系统播放）和我的声音（麦克风）能同时录下来、实时出文字，并且所有数据都不离开我的电脑。

主要功能：
- 双路同步录音：系统音频 + 麦克风同时采集、自动混音
- 实时流式转写：X-ASR（中英低延迟）/ SenseVoice（中/英/日/韩/粤语）
- 实时翻译：Hy-MT2（腾讯混元）支持 13 种目标语言
- AI 会议纪要：本地 GGUF 大模型（Qwen / Gemma）离线生成
- 历史记录 & 导出：SQLite 存储，导出 TXT / SRT / Markdown
- 纯 CPU 运行，不需要显卡；多语言界面（中/英/日/韩）

隐私：音频和文字全程不离开电脑，无账号、无订阅、无付费墙，所有功能永久免费。

国内用户友好：首次启动有引导向导下载模型，下载源自动在 GitHub / HuggingFace 镜像 / ModelScope 间回退，无需代理。

GitHub：https://github.com/Longt-audio/voxminutes
下载：https://github.com/Longt-audio/voxminutes/releases

（安装包暂未代码签名，SmartScreen 可能提示，点「更多信息 → 仍要运行」即可）
```

**评论区置顶：**
```
下载：https://github.com/Longt-audio/voxminutes/releases
安装包约 50MB，不内置模型，首次启动按向导下载即可，国内自动回退可用镜像。
```

---

## 六、即刻 + 小红书（竖屏视频，直接粘贴）

**文案（通用）：**
```
开会还在手写笔记？这个免费工具帮你全搞定👇

✅ 系统声音+麦克风同时录，线上会议一字不漏
✅ 实时转写，说话即出文字
✅ 实时翻译 13 种语言，英文会也不怕
✅ 本地 AI 一键生成会议纪要，数据不上传
✅ 全功能免费，无订阅无广告

Windows 直接装，GitHub 搜 VoxMinutes 就能下载。
```

**标签：**
```
#效率工具 #会议纪要 #AI工具 #开源软件 #打工人必备 #免费软件 #远程办公 #翻译神器
```

**评论区置顶：**
```
GitHub：github.com/Longt-audio/voxminutes
安装包不内置模型，首次启动按向导下载即可，国内会自动连可用镜像。
```

---

## 七、高频问题回复模板（发布后 24h 内必用）

| 问题 | 回复 |
|------|------|
| Is it really free? / 免费吗？ | Yes, 100% free and open source (AGPL-3.0). No hidden tiers. |
| Mac 能用吗？ | Not yet — Windows 10/11 only. macOS and Linux are on the roadmap. Please star to follow progress! |
| 模型下载慢怎么办？ | 应用内支持多源回退；也可以复制链接用迅雷/IDM/aria2 下载后，在「设置 → 模型」导入。 |
| 识别准确率怎么样？ | 中英文用 X-ASR 或 SenseVoice，安静环境下准确率不错；嘈杂/口音重时会有误差，后续会加降噪和说话人分离。 |
| SmartScreen 警告？ | 安装包暂未代码签名，点「更多信息 → 仍要运行」。源码公开，安全。 |
| 有云端版本吗？ | 当前完全本地；未来可能会提供可选的云端高精度 ASR 和团队协作，但核心功能永远免费。 |

---

## 八、发布前检查清单

- [ ] GitHub 仓库：README 四语言、Topics 20 个、Description 已更新（✅ 已全部完成）
- [ ] Release v0.1.2 安装包可正常下载（✅ 已验证存在）
- [ ] 各平台账号已登录，测试可发帖
- [ ] 5 张画廊图 + 视频已下载到本地
- [ ] 把本文档标记为已读，回复模板复制到备忘录
- [ ] 发布当天每小时查看 Product Hunt 并回复评论

---

> 发布后记得：把真实用户反馈整理成 GitHub issue；有修复就发 v0.1.3 patch release，再在各平台更新一次「已修复 xxx」——这是保持热度的关键。
