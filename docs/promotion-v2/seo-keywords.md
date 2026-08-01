# SEO 与关键词优化建议

## 一、GitHub 仓库优化

### 1. Topics（仓库标签）✅ 待手动设置

> ⚠️ Topics 需要在 GitHub 网页上手动设置（无法通过本地文件修改）。

**操作步骤：**
1. 打开仓库主页 https://github.com/Longt-audio/voxminutes
2. 点击右上角 **About** 栏的齿轮图标 ⚙️（或「Edit repository details」）
3. 在 **Topics** 输入框逐个添加下方标签，回车确认
4. 点 **Save changes**

**完整推荐列表（GitHub 上限 20 个）：**

```
meeting-assistant
real-time-transcription
speech-to-text
local-first
offline
privacy
translation
ai-summaries
tauri
rust
opensource
windows
whisper-alternative
sensevoice
qwen
gguf
asr
transcription-software
meeting-notes
multi-language
```

> ✅ **已实际设置（2026-08-02，通过 GitHub API）：** 上述 20 个已全部生效。第一版用 `productivity` 占位，后应项目要求换成 `multi-language`（13 语言翻译是核心卖点），`productivity` 已移除。
> 同时已更新仓库 Description 为：`VoxMinutes - free, local-first meeting assistant for Windows. Records system audio & mic together; real-time transcription, translation (13 languages) & AI summaries. Data never leaves your device.`（替换了原来四语言堆叠的旧描述）。

### 2. README 标题优化 ✅ 已应用（2026-08-02）

原标题保留但结构已重写，核心关键词自然融入：

> VoxMinutes — Your local meeting assistant · 100% free · Records system audio & mic together · Real-time transcription, translation & summaries — all on your device

README 已应用的优化（见仓库根 `README.md` / `README.zh-CN.md`）：
- ✅ 顶部嵌入 60 秒演示视频（`<video>` 标签，GitHub 可播放），附 30s 精华版 + 竖屏版链接
- ✅ 新增「Quick Links」导航栏，降低跳出率
- ✅ 新增「Why VoxMinutes」痛点对比表（vs Otter.ai / 讯飞听见 / Whisper 类工具）
- ✅ 新增「Quick Start」三步快速上手 + SmartScreen 安全提示（消除安装顾虑）
- ✅ 新增 badge：PRs Welcome
- ✅ 新增「Contributing」社区的 Issues / Discussions / Star 引导
- ✅ 路线图增加社区需求驱动说明（引导 issue 投票）
- ✅ 全文自然包含关键词：free meeting transcription software / offline speech to text / record system audio and microphone / local AI meeting notes / Windows transcription tool

### 3. Release Notes 优化

每个 Release 标题和正文都应包含：
- 版本号
- 核心更新点
- “VoxMinutes” 品牌词
- 下载链接
- 中文/英文双语（国内搜索会抓取 Release 页）

示例：
```
VoxMinutes v0.1.3 — free local meeting assistant for Windows
- Improved ASR accuracy for noisy environments
- Added model import progress indicator
- Fixed SmartScreen-related installer issue

Download: ...
```

### 4. Issue / Discussion 标签

建议标签：
- bug
- feature
- documentation
- good first issue
- help wanted
- windows
- macOS (planned)
- Linux (planned)
- question

### 5. 增加反向链接

在以下位置添加仓库链接：
- 个人网站 / 博客
- Twitter/X bio
- Reddit profile
- 知乎/掘金/少数派个人主页
- 即刻个人介绍
- Product Hunt maker profile
- YouTube / Bilibili 视频描述

---

## 二、搜索关键词矩阵

### 中文关键词

| 关键词 | 搜索意图 | 适用平台 |
|--------|----------|----------|
| 免费会议转写软件 | 工具搜索 | 知乎、百度搜索 |
| 本地语音识别软件 | 隐私/技术 | 知乎、V2EX |
| 实时翻译软件 | 翻译需求 | 小红书、B站 |
| 会议纪要 AI 工具 | 效率需求 | 知乎、即刻 |
| 系统声音录制软件 | 录音需求 | B站、知乎 |
| 免费 Otter 替代 | 竞品替代 | 知乎、Reddit |
| 开源会议助手 | 开源用户 | V2EX、掘金 |
| Windows 实时字幕 | 辅助功能 | B站、知乎 |

### 英文关键词

| 关键词 | 搜索意图 | 适用平台 |
|--------|----------|----------|
| free meeting transcription software | 工具搜索 | Google, YouTube |
| offline speech to text windows | 离线/Windows | Google, Reddit |
| record system audio and mic | 录音需求 | Google, YouTube |
| open source meeting assistant | 开源 | GitHub, HN |
| local ai meeting notes | AI/本地 | Product Hunt, Twitter |
| Otter.ai free alternative | 竞品替代 | Reddit, Google |
| whisper desktop alternative | 竞品替代 | Reddit, GitHub |
| real time translation meeting | 翻译 | Google, YouTube |

---

## 三、内容 SEO

### 知乎 / 掘金 / 少数派文章

标题中必须包含 1–2 个核心关键词：
- “免费会议转写工具”
- “本地语音识别”
- “开源会议助手”

正文中自然出现 3–5 次品牌词“VoxMinutes”。

### YouTube / Bilibili

- 标题包含核心关键词
- 描述前 2 行放链接和卖点
- 标签包含英文/中文关键词
- 添加章节时间戳

### 小红书 / 抖音

- 文案包含关键词：免费、本地、会议、转写、翻译
- 话题标签即关键词
- 评论区互动提升搜索权重

---

## 四、可投稿的 awesome-list

增加反向链接和流量：

- awesome-selfhosted
- awesome-tauri-apps
- awesome-windows
- awesome-open-source
- awesome-productivity
- awesome-speech-recognition
- awesome-local-ai

投稿方式：提交 PR 或在 Discussion 中自荐。
