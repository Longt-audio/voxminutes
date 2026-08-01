# VoxMinutes 推广策略总览

## 一、产品定位

**一句话定位：**
> VoxMinutes 是一款完全免费、本地优先的 Windows 会议助手——同时录制系统声音和麦克风，实时转写、翻译、AI 总结，数据永远不出设备。

**核心差异化（务必反复讲）：**

1. **双路录音**：系统播放 + 麦克风同时录（线上会议、网课、访谈的刚需）。
2. **完全本地**：音频/文本不上传，无账号、无订阅、无付费墙。
3. **免费开源**：AGPL-3.0，安装包仅约 50 MB，模型按需下载。
4. **国内可用**：GitHub / HuggingFace / ModelScope 多源回退，无需代理。
5. **多语言 UI**：中 / 英 / 日 / 韩，降低海外传播门槛。

---

## 二、目标人群画像

| 人群 | 痛点 | 常用平台 |
|------|------|----------|
| 远程办公 / 跨国团队 | 会议纪要耗时、隐私担忧 | Reddit r/selfhosted、Product Hunt、HN、LinkedIn |
| 学生 / 网课用户 | 英文课听不懂、复习需要字幕 | Bilibili、小红书、知乎 |
| 播客 / 访谈创作者 | 需要本地音频转写、字幕导出 | Reddit r/podcasting、YouTube |
| 程序员 / 开源爱好者 | 讨厌 SaaS、希望数据自有 | GitHub、HN、V2EX、Twitter/X |
| 国内职场人 | 飞书妙记/讯飞收费、数据不放心 | V2EX、即刻、知乎、小红书 |

---

## 三、核心信息屋（Message House）

### 主信息
> 你的会议，你的数据。

### 支撑点
- 双声道同步录音（系统 + 麦克风）
- 实时转写：X-ASR / SenseVoice
- 实时翻译：13 种语言
- AI 会议纪要：本地 GGUF 大模型
- 完全免费、开源、本地运行

### 信任状
- 技术栈：Tauri 2 + Rust + sherpa-onnx + llama.cpp
- 模型：Qwen / Gemma / SenseVoice / 腾讯混元
- 许可证：AGPL-3.0
- 多语言界面

### 号召语（CTA）
- 海外：GitHub: Longt-audio/voxminutes — free download for Windows
- 国内：GitHub 搜 VoxMinutes，Windows 直接装，完全免费

---

## 四、转化漏斗与优化

```
[曝光]  平台推荐/搜索/社交分享
   ↓
[点击]  GitHub 仓库页  ← 优化 README 首屏、标题、demo GIF
   ↓
[兴趣]  README 阅读  ← 前 3 屏必须讲清“是什么、为什么、怎么用”
   ↓
[下载]  Release 页  ← 大按钮、版本说明、安装提示
   ↓
[安装]  安装包体验  ← SmartScreen 提示、首启向导、模型下载
   ↓
[激活]  完成第一次录音转写  ← onboarding 关键路径
   ↓
[留存]  更新日志、issue 反馈、功能预告
```

### 每步优化动作

| 阶段 | 现状 | 优化动作 |
|------|------|----------|
| 曝光 | 0 star，无认知 | 多平台发布 + SEO + 视频 |
| 点击 | README 已有 demo GIF | 增加视频封面图、一句话卖点、 shields.io badge |
| 兴趣 | 功能描述完整 | 增加“vs 竞品”表格、用户评价区（先留空模板） |
| 下载 | Release 有 exe | Release Notes 增加中文说明、checksum、Windows 安装截图 |
| 安装 | 无代码签名 | README 顶部加 SmartScreen 提示、提供便携版 |
| 激活 | 首启向导已有 | 录一个 60 秒安装+首次使用教程 |
| 留存 | 无 newsletter | 建议后续加 GitHub Releases Watch + 微信公众号/Newsletter |

---

## 五、KPI 与里程碑

### 30 天目标

| 指标 | 当前 | 目标 | 说明 |
|------|------|------|------|
| GitHub Stars | 0 | 100+ | 主要质量信号 |
| Forks | 0 | 15+ | 开发者兴趣 |
| Release 下载 | - | 500+ | 实际使用 |
| Issues / Discussions | 0 | 10+ | 社区反馈 |
| Product Hunt Upvotes | - | 100+ | 海外首发 |
| V2EX 点赞/回复 | - | 100+ / 30+ | 国内首发 |
| 小红书/即刻曝光 | - | 1万+ | 品牌认知 |

### 90 天目标

- Stars 500+
- 稳定 issue 反馈流
- 至少 1 次awesome-list收录
- 至少 1 篇媒体报道或 newsletter 收录

---

## 六、内容资产矩阵

把一次发布拆解成多种形式，最大化复用：

| 原始素材 | 可复用内容 |
|----------|-----------|
| 60–90s 横版演示视频 | Bilibili、YouTube、Twitter/X、Reddit 帖内嵌入、V2EX 附件 |
| 30–45s 竖版演示视频 | 小红书、抖音、快手、Instagram Reels、YouTube Shorts、TikTok |
| demo GIF | GitHub README、知乎/掘金文章、Twitter/X |
| 分镜脚本 | 推文/文章提纲、演讲 PPT |
| 安装教程 | 独立图文/视频、FAQ |
| 竞品对比 | Reddit/HN 讨论、知乎回答 |

---

## 七、竞品差异表达

**不要直接攻击竞品，只讲“为什么选 VoxMinutes”。**

| 竞品 / 方案 | 它们的局限 | VoxMinutes 的优势 |
|-------------|------------|-------------------|
| Otter.ai / 讯飞听见 | 按分钟计费、音频上传云端 | 完全免费、本地运行、数据不出设备 |
| Whisper Desktop / Buzz | 通常只录麦克风 | 同时录系统声音 + 麦克风 |
| 飞书妙记 / 腾讯会议云录制 | 需要账号、会议内容可被平台访问 | 无账号、纯本地、跨会议软件通用 |
| 浏览器插件 | 受浏览器音频权限限制、不稳定 | 系统级录音、Tauri 原生性能 |
| 自建 Whisper | 需要命令行、配置复杂 | 内置模型管理、图形界面、一键下载 |

---

## 八、风险与应对

| 风险 | 应对 |
|------|------|
| Windows SmartScreen 拦截 | README 顶部明确提示“无代码签名，点击更多信息→仍要运行” |
| 模型下载慢/失败 | 强调多源回退 + 手动导入教程；准备百度网盘/夸克备用链 |
| 用户反馈 ASR 准确率 | 先承认早期版本，建议换 SenseVoice / 检查音频质量；记录为 issue |
| macOS/Linux 用户流失 | 在 README 显著位置写“支持计划中”，收集需求；引导 star 关注 |
| 被质疑盈利模式 | 明确当前免费，未来可能提供云端/团队版，但核心功能永远免费 |

---

## 九、长期运营节奏

**第一周：引爆**
- Day 0：Product Hunt + HN + Reddit r/selfhosted
- Day 1：V2EX + 即刻 + 小红书
- Day 2–3：回复所有评论、置顶下载链接
- Day 4：发一条 Twitter/X thread 汇总反馈
- Day 5–7：Bilibili / 知乎长文

**第二周：沉淀**
- 汇总用户反馈，发 v0.1.3 patch
- 给首批 issue 贡献者写感谢
- 在 Reddit / Discord 做 Q&A

**第三周：扩展**
- 投稿 awesome-selfhosted、awesome-tauri-apps 等清单
- 联系中文科技媒体 / newsletter
- 准备 YouTube 完整教程

**第四周：复盘**
- 看 GitHub Insights、各平台数据
- 迭代文案和发布策略
- 规划 v0.2.0 宣传点（TTS / 字幕悬浮窗 / 划词翻译）

---

## 十、给自动化的输入

本策略拆出的可自动化动作：

1. 发布定时推文（Twitter/X、即刻、微博）
2. 跨平台内容同步（同一视频+不同文案）
3. GitHub Release 后自动发帖通知
4. 评论回复模板库
5. 周报数据抓取（stars、downloads、mentions）

下一步如需自动化，可基于本目录 `posts/` 和 `schedule.md` 做模板引擎。
