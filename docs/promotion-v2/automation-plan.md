# VoxMinutes 自动化发帖计划

> 目标：把发布工具包（launch-kit）里的文案，通过自动化方式发布到各平台。
> 原则：**能全自动的全自动，高风险平台半自动，发布前关键动作人工确认。**

---

## 一、各平台自动化可行性总览

| 平台 | 方案 | 需要你提供的 | 自动化程度 | 风险 |
|------|------|-------------|-----------|------|
| GitHub | API / git push | 无（已有凭证） | ✅ 全自动 | 无 |
| Reddit | 官方 API（OAuth） | 注册 Reddit API app（5 分钟），给 client_id/secret + 授权 token | ✅ 全自动 | 低 |
| Telegram / Discord | Webhook / Bot | 建频道拿 webhook URL | ✅ 全自动 | 无 |
| V2EX | 浏览器自动化（Playwright） | 扫码登录一次 | ⚠️ 半自动（发布前确认） | 中 |
| Hacker News | 登录 Cookie 提交 | 提供登录态 | ⚠️ 半自动 | 中 |
| Product Hunt | 浏览器自动化 + 上传画廊图 | 登录态（建议扫码） | ⚠️ 半自动（**发布动作人工点**） | 中 |
| 小红书 | 无 API，风控严 | 浏览器登录态 | 🚫 只做内容填充，人工发布 | **高（易封号）** |
| 知乎 | 无 API，风控严 | 浏览器登录态 | 🚫 只做内容填充，人工发布 | **高** |
| B 站 | 无 API，风控严 | 浏览器登录态 | 🚫 只做内容填充，人工发布 | **高** |
| 抖音 | 无 API，风控严 | - | 🚫 不自动化 | **高** |

---

## 二、分层执行路线

### 第 0 层：内容准备（✅ 已完成）
- 全部文案：`docs/promotion-v2/launch-kit/DAY0-LAUNCH-PACK.md` + `posts/`
- 画廊图：`launch-kit/gallery/`（5 张 1600×900）
- 视频：`docs/promotion-v2/videos/`（4 个）

### 第 1 层：全自动（无需人工，随时可搭）
1. **Reddit**
   - 你操作：reddit.com/prefs/apps → create app（script 类型）→ 拿到 client_id / client_secret
   - 授权：`curl -u client_id:secret -d 'grant_type=password&username=...&password=...' https://www.reddit.com/api/v1/access_token` 拿 token
   - 脚本：POST 到 r/selfhosted、r/software（注意各板块 Self-Promotion 规则，r/selfhosted 周六发）
2. **Telegram / Discord**（可选）
   - 有频道/webhook 后，脚本把发布摘要 + 链接推送

### 第 2 层：浏览器半自动（需登录态）
1. **V2EX**
   - Playwright 打开 https://www.v2ex.com/new，填标题/正文（内容来自 launch-kit），你扫码登录
   - 发布前弹出确认，你点发布（或允许自动）
2. **Product Hunt**
   - 打开发布页，自动填 Name/Tagline/Description、上传 5 张画廊图、填 Maker Comment
   - **发布动作由你手动点**（PH 重视真实性，自动发布有风险）
3. **Hacker News**
   - 登录后通过 Cookie 提交 Show HN

### 第 3 层：国内平台（只做内容填充）
- 小红书 / 知乎 / B站：自动生成内容 + 打开发布页填好草稿，**你手动点发布**
- 文案/标签/置顶评论模板已在 launch-kit

### 第 4 层：运营自动化（✅ 可立即搭建）
- **定时提醒**：发布日（8/5）提醒 + 当日执行清单（已创建）
- **数据日报**：每日抓取 GitHub stars / release 下载量 / 各平台链接点击（可选）
- **评论回复提示**：定期检查新评论并给出回复建议（可选）

---

## 三、执行时间表

| 时间 | 动作 | 依赖 |
|------|------|------|
| 今天起 | 搭第 1 层 Reddit 脚本（你提供 API 凭证后 30 分钟可用） | 你注册 Reddit API app |
| 8/5（周三）22:00 | 定时提醒自动触发：发布前准备清单 | 已自动 |
| 8/5 23:00 | Product Hunt 首发（半自动，人工点发布） | 登录态 |
| 8/5 23:30 | HN Show HN（半自动） | 登录态 |
| 8/5 21:00 | V2EX（半自动） | 登录态 |
| 周六 | Reddit r/selfhosted（全自动） | API 凭证 |

---

## 四、风险与对策

| 风险 | 对策 |
|------|------|
| 小红书/知乎/抖音封号 | 只填充内容、人工发布；频率控制在每平台每周 ≤2 条 |
| Product Hunt 判定营销 | 发布动作人工点、回复评论亲自来 |
| Reddit 被当 spam | 遵守各板块规则、先回答评论、账号有一定 karma 再发 |
| V2EX 限发 | 一个账号当天只发一帖，不刷屏 |

---

## 五、待你决定的事项

- [ ] 是否先从 **Reddit** 开始？（需要你注册 API app，约 5 分钟）
- [ ] 是否接受 **浏览器自动化**（V2EX / PH / HN 需要扫码登录一次）？
- [ ] 小红书 / 知乎 / B站 确认走"半自动人工发布"？
- [ ] 是否开启 **每日数据日报** 定时任务？
