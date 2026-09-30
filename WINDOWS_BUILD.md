# Windows 打包与代码签名（2026-09-30）

> 本文解决两件事：**怎么打出 Windows 安装包**、**签名到底能不能消除 SmartScreen 警告**。

---

## 一、为什么不能在 Mac 上直接打 Windows 包

Tauri **官方不支持交叉编译**：Windows 的 NSIS 安装包必须在 Windows 上生成。
而且本项目的 C/C++ 依赖（`ring`、`onnxruntime`、`sherpa-onnx`、`llama-helper` 的 llama.cpp）
都需要 MSVC 工具链，从 macOS 交叉编译会失败（实测 `cargo check --target x86_64-pc-windows-msvc`
卡在 `ring`：找不到 `assert.h`）。

**所以：安装包必须在 Windows 机器或 GitHub Actions（windows-latest）上构建。**

---

## 二、两条可行路径

### 路径 A：私有仓库 + GitHub Actions（推荐，约 30~45 分钟）

**为什么用私有仓库**：GitHub Actions 需要代码在 GitHub 上，但**不要求是公开仓库**。
推到私有仓库既满足 CI 运行条件，又不会把代码公开。

```bash
# 1) 在 GitHub 网页上新建一个 PRIVATE 仓库，例如 voxminutes-build（不要勾选 README/gitignore）

# 2) 把本地代码推上去（注意：是推到新仓库，不是 origin）
cd ~/vox/voxminutes
git remote add build git@github.com:<你的用户名>/voxminutes-build.git
git push build main

# 3) 打开 https://github.com/<你的用户名>/voxminutes-build/actions
#    选左侧「Build Windows (nsis)」→ 右侧「Run workflow」→ 选 main → Run

# 4) 约 30~45 分钟后，在该次运行的 Artifacts 里下载 voxminutes-windows.zip
#    解压得到 VoxMinutes_0.2.0_x64-setup.exe
```

**这个工作流不需要任何 Secret**（我们没启用自动更新，所以不需要签名密钥）。

**第一次跑大概率要迭代 1~2 次**，常见失败点：

| 报错 | 原因与处理 |
|---|---|
| `runtime DLL 缺失或为 0 字节` | sherpa-onnx / ort 的预编译库没下载成功（网络/代理）。重跑一次通常就好 |
| 下载 ffmpeg 失败 | gyan.dev 偶尔抽风，重跑；或改 BtbN 的包 |
| `llama-helper` 构建失败 | 缺 libclang/LLVM。CI 的 windows-latest 自带，本地才需要装 |
| `externalBin ... not found` | `binaries/` 没准备好。CI 里的「Stage sidecars」步骤负责，看它有没有报错 |

**删除方式**（用完就撤）：`git remote remove build`，然后在 GitHub 上删掉那个仓库。

---

### 路径 B：在你的 Windows 机器上本地构建

适合你本来就有一台 Windows 开发机、且以后要经常打包的情况。

**需要装的工具链**（一次性，约 1~2 小时）：

| 工具 | 说明 |
|---|---|
| **Visual Studio 2022 生成工具** | 勾选「使用 C++ 的桌面开发」（含 MSVC + Windows SDK） |
| **Rust** | https://rustup.rs → 默认 `stable-x86_64-pc-windows-msvc` |
| **LLVM** | llama-helper 需要 libclang。装完设 `LIBCLANG_PATH` |
| **Node.js 20+ 与 pnpm** | `npm i -g pnpm` |

**然后**：

```powershell
cd <仓库根>
pwsh -File frontend/scripts/prepare-windows-sidecars.ps1   # 准备 ffmpeg.exe + llama-helper.exe
cd frontend
pnpm install --ignore-workspace
pnpm tauri build
# 产物： <仓库根>\target\release\bundle\nsis\VoxMinutes_0.2.0_x64-setup.exe
```

> ⚠️ 仓库根是 cargo workspace，产物在 **`<仓库根>\target\`** 下，
> 不是 `frontend\src-tauri\target\`，也不是 `llama-helper\target\`。
>
> ⚠️ `pnpm install` 必须带 `--ignore-workspace`（仓库根有 pnpm-workspace.yaml）。

---

## 三、签名到底能不能消除 SmartScreen 警告？

**结论先说：不能立刻消除。而且 EV 证书已经不再有用了。**

以下是微软官方文档
（[Windows 应用开发者的 SmartScreen 声誉](https://learn.microsoft.com/zh-cn/windows/apps/package-and-deploy/smartscreen-reputation)）的原文要点：

### 3.1 首次下载时的实际表现

| 证书类型 | 首次下载时的 SmartScreen 行为 |
|---|---|
| Microsoft Store 分发 | ✅ **无警告**（微软重新签名） |
| 有效证书（**OV 或 EV**） | ⚠️ **仍然警告** —— 在信誉累积之前标记为「无法识别的应用」；区别是**会显示已验证的发布者名称** |
| **无签名** | ⚠️ 警告 ——「Windows 保护你的电脑」 |
| 自签名证书 | ⚠️ 与无签名完全相同 |

### 3.2 ⚠️ EV 证书不再绕过 SmartScreen

原文：

> 几年前，使用扩展验证（EV）代码签名证书对文件进行签名将导致默认的 SmartScreen 信誉良好，
> 但**此行为不再存在**。EV 证书对于企业采购可能很重要，但它们**不再影响 SmartScreen 行为**。
> 仅仅为了规避 SmartScreen 警告而为 EV 支付额外费用，**已经没有理由了**。

**所以：不要为了"消除警告"去买贵的 EV 证书。**

### 3.3 信誉是怎么积累的

> SmartScreen 信誉会自动建立起来……**可能需要数周时间以及来自广泛用户的数百次全新安装。**

### 3.4 签名真正的价值（值得为之付费的理由）

> 如果文件**未签名**，SmartScreen 信誉必须**为每个新版本从头开始积累，起始信誉为零**。
> 信誉**无法从以前的版本转移**，除非两者都使用相同的发布者标识进行签名。

也就是说：
- **不签名** → 你每发一个版本，都要从零重新攒信誉；
- **签名** → 证书信誉可以继承，老用户升级新版不会再被拦。

这才是签名的主要收益，不是"立刻没警告"。

### 3.5 三个具体问题的答案

**Q：这个测试版（未签名）会看到什么？**

| 场景 | 会看到什么 |
|---|---|
| 从网络下载该 exe 后双击 | **「Windows 已保护你的电脑」**（因为下载的文件带 MOTW 标记）。点「更多信息」→「仍要运行」 |
| 安装过程 | **不会**弹 UAC「未知发布者」—— 我们配的是 `installMode: currentUser`，安装器**不请求管理员权限**，所以没有提权弹窗 |
| 用 U 盘 / 局域网拷贝过去（无 MOTW） | 可能**什么都不弹** |
| Windows 11 开了「智能应用控制」 | ⚠️ 可能**直接阻止**未签名程序运行，且不给「仍要运行」的选项。测试机跑不起来时先查这里 |

**Q：申请 SignPath 通过后，就不会出现未知发布者了？**

- 发布者名称会从「未知发布者」变成 **"SignPath Foundation"**（证书是发给基金会的，不是你）；
- 但**首次下载仍然会弹 SmartScreen 警告**，直到信誉累积够（数周 + 数百次全新安装）；
- 好处是**证书信誉能继承**，之后每个新版本不用从零攒。

**Q：申请周期？赶得上 3 天后的发布吗？**

- ❌ **赶不上。** SignPath Foundation 是**人工审核**，官方未公布时限，条款里明确写着
  「我们保留拒绝的权利，且没有独立仲裁机制」。
- 条款里有一条对你很关键：**对可执行程序，他们要求"一定的可验证声誉"**
  （"we require a certain verifiable reputation"）。你现在：公开仓库 + AGPL + 官网 +
  ~400 次下载 —— 够格申请，但**不保证通过**。
- 现实预期：**几周到几个月，且首次被拒/被要求补材料很常见**。
- **建议：当作发布后的改进项，不要放进这次发布的计划里。**

---

## 四、SignPath 申请的硬性前提（缺一条就会被拒或要求补）

来源：[SignPath Foundation 条款](https://signpath.org/terms)、[申请入口](https://signpath.org/apply)

| # | 要求 | 你的现状 |
|---|---|---|
| 1 | OSI 认可的开源许可证，且无商业双授权 | ✅ AGPL-3.0 |
| 2 | 不含专有代码 | ✅ |
| 3 | 项目在活跃维护 | ✅ |
| 4 | **已经以"要被签名的形式"发布过** | ✅ 有 Windows Release |
| 5 | 功能在下载页/商店页有文档说明 | ✅ 官网 |
| 6 | **项目主页/下载页必须有「Code signing policy」段落**，含指定字样、团队角色、隐私政策链接 | ❌ **还没有，必须补** |
| 7 | **源码必须公开，且能对应到被签名的二进制** | ⚠️ **冲突**：`origin/main` 落后本地 52 个提交，Releases 里的包也不是从当前公开源码构建的 |
| 8 | 所有成员必须开启 **MFA**（GitHub + SignPath） | ⚠️ 请自查 |
| 9 | 每次发布需 **Approver 手动批准**签名；构建走 CI | ⚠️ 需配置 |
| 10 | 产物元数据约束：product name = 项目名，version 每次构建一致 | ⚠️ 需配置 |
| 11 | **安装时必须展示隐私政策**（因为软件会把音频传给外部系统） | ❌ **NSIS 目前不显示，需补** |

### 第 6 条要写的内容（可直接照抄）

在项目主页（GitHub README）与官网下载区加一段，标题必须是 **Code signing policy**：

```markdown
## Code signing policy

Free code signing provided by [SignPath.io](https://about.signpath.io), certificate by
[SignPath Foundation](https://signpath.org).

- Committers and reviewers: [Longteng](https://github.com/Longt-audio) (project maintainer)
- Approvers: [Longteng](https://github.com/Longt-audio)

This program will transfer audio and text data to networked systems **only when the user
explicitly selects a remote model or enables the remote service**. See our
[Privacy Policy](https://voxmin.top/privacy.html) for details, including the list of
upstream AI providers and data retention periods.
```

### 第 11 条怎么补（我可以帮你改）

Tauri 的 `bundle.licenseFile` 可以让 NSIS 安装器显示一个许可页。

> ⚠️ 注意：SignPath 的所有条件里，**第 7 条是真正的拦路虎** ——
> 他们整个模式的前提就是「签名的二进制能从公开源码可验证地构建出来」。
> **不 push 代码 = 拿不到 SignPath 签名**，这两件事无法同时成立，需要你决策。

---

## 五、如果确实想花钱签名：还有更便宜的选择

微软在官方文档里推荐的方案是 **[Azure Artifact Signing](https://learn.microsoft.com/zh-cn/azure/trusted-signing/)**
（原 Trusted Signing）：

| 项目 | 说明 |
|---|---|
| 价格 | **$9.99/月起**（约 ¥72/月），按签名次数分档 |
| 硬件令牌 | **不需要**（传统 OV 证书要买 U 盘令牌，很麻烦） |
| CI 集成 | 原生支持 GitHub Actions / Azure DevOps |
| 身份验证 | 需要（微软会验证你的身份；个人身份可验证） |
| SmartScreen | 与 OV 证书相同 —— **仍然要攒信誉**，但证书信誉可继承 |

对比传统 CA 的 OV 证书（¥1000~3000/年 + 硬件令牌 + 3~10 工作日审核），
Artifact Signing 更便宜、更快、更适合 CI。

> **但请记住第三节的结论**：无论走哪条路，**首发都会弹警告**。
> 所以签名不是首发的阻塞项 —— 先用未签名包发布，把「点更多信息 → 仍要运行」的说明写好，
> 等有真实用户量之后再决定要不要签名。
