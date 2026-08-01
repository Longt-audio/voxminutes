# VoxMinutes 推广方案 V2

> 仓库：https://github.com/Longt-audio/voxminutes  
> 版本：v0.1.2（Windows 10/11 x64）  
> 当前状态：0 stars / 0 forks，7 月 26 日创建，非常早期

本方案在 `docs/promotion`（另一版 AI Agent 所作）基础上做了**全面升级**，保留其有效部分，补足缺失的板块、渠道和执行力。

---

## 与 V1 相比的核心提升

| 维度 | V1 的做法 | V2 的提升 |
|------|----------|-----------|
| 渠道覆盖 | V2EX、Reddit、小红书、即刻、少数派、掘金、知乎（泛泛提到） | 新增 Product Hunt、Hacker News、Discord/Reddit 细分社区、Bilibili、YouTube Shorts、知乎视频、Twitter/X、LinkedIn、Indie Hackers、SegmentFault、开源中国(OSChina)、CSDN、微博、微信公众号/视频号 |
| 视频物料 | 1 个 30 秒脚本 + 已有 16:9 和竖屏成片 | 重新剪辑 4 个成片：60 秒横版完整版 + 60 秒竖版完整版 + 30 秒横版精华版 + 30 秒竖版精华版，全部加入中英双语动态字幕、章节提示、CTA 卡片 |
| 文案策略 | 1 套中文 + 1 套英文帖文 | 按平台调性分别撰写：V2EX、Reddit、Product Hunt、HN、小红书、知乎、少数派、B站/YouTube 描述各一版 |
| SEO / 搜索 | 未涉及 | 给出 GitHub Topics、README 关键词、Release Notes、issue 标签、反向链接清单 |
| 转化漏斗 | 只列发布平台 | 明确「曝光 → 点击 → 下载 → 安装 → 留存」每一步的优化动作和指标 |
| 互动运营 | 提到 24h 内回复 | 给出首周/首月的评论回复模板、置顶评论、更新节奏、issue 转化 |
| 竞品借势 | 无 | 给出与 Otter.ai、Whisper Desktop、飞书妙记等产品的差异点表达 |
| 数据复盘 | 无 | 给出 GitHub Insights、Google Trends、短链追踪、平台后端数据的查看和迭代方法 |

---

## 文件清单

```
docs/promotion-v2/
├── README.md                 # 本文件：总览与提升点
├── promotion-strategy.md     # 整体定位、目标人群、核心信息、漏斗与 KPI
├── platforms.md              # 各平台调性、规则、发布策略
├── posts/                    # 分平台文案
│   ├── v2ex.md
│   ├── reddit-selfhosted.md
│   ├── reddit-software.md
│   ├── product-hunt.md
│   ├── hacker-news.md
│   ├── xiaohongshu.md
│   ├── zhihu.md
│   ├── bilibili.md
│   ├── youtube.md
│   └── short-form-caption.md
├── video-script.md           # 新视频分镜脚本（60s 横版完整版 + 60s 竖版完整版 + 30s 精华版）
├── subtitle-list.md          # 新视频字幕时间轴
├── seo-keywords.md           # GitHub / 知乎 / 搜索关键词建议
├── schedule.md               # 发布排期与执行清单
└── videos/                   # 生成/输出的视频文件
    ├── voxminutes-v2-horizontal.mp4       # 60s 16:9 完整版（B站/YouTube/Reddit）
    ├── voxminutes-v2-vertical.mp4         # 60s 9:16 完整版（小红书/抖音/视频号）
    ├── voxminutes-v2-horizontal-short.mp4 # 30s 16:9 精华版（Twitter/X/即刻/广告）
    └── voxminutes-v2-vertical-short.mp4   # 30s 9:16 精华版（小红书/抖音/Shorts/Reels）
```

---

## 立即执行的最小闭环（推荐）

1. **优化 GitHub 首页**（30 分钟）→ 提升转化率
2. **发布 Product Hunt**（首周最重要的海外引爆点）
3. **发布 Hacker News Show HN + Reddit r/selfhosted**（同天或隔天）
4. **国内同步：V2EX「分享创造」+ 即刻「AI 探索站」+ 小红书竖版视频**
5. **一周后：B 站/知乎视频长文 + 微信公众号深度文章**
6. **持续：回复评论、收集 issue、发 patch release、反哺内容**

---

## 关键数据现状

- GitHub Stars：**0**（截至 2026-08-01）
- Forks：**0**
- Open Issues：**0**
- Releases：v0.1.0 / v0.1.1 / v0.1.2
- 资产：Windows installer `VoxMinutes_0.1.2_x64-setup.exe`
- 目标（发布后 30 天）：Stars 100+，下载 500+，Issue 反馈 10+

---

## 给后续“自动发帖”的接口

本方案把所有平台的文案、话题标签、最佳发布时间、评论区置顶话术都写好了。下一步如要自动化，可把 `posts/` 里的文案作为模板，结合平台 API / RPA 工具批量发布，并替换变量：

- `{REPO}` → `Longt-audio/voxminutes`
- `{VERSION}` → 当前 release tag
- `{DOWNLOAD}` → GitHub Release 链接
- `{EMOJI}` / `{TAGS}` 可按平台微调

---

> **说明**：`docs/promotion/` 是另一版 AI 的输出，本目录 `docs/promotion-v2/` 完全独立，未覆盖其文件。
