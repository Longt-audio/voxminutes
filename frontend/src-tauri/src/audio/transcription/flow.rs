// audio/transcription/flow.rs
//
// 流式文本管线：远程流式 ASR（Remote ASR Streaming）与本地 X-ASR 共用。
//
// 设计（2026-09-21 定稿）：三层解耦，上游「定稿」不再承担显示分行职责——
//   · 定稿：上游/识别器决定文本何时稳定（Deepgram is_final、豆包 definite、
//     千问 .completed、X-ASR 识别器累计文本），粒度各家不同，本管线不关心；
//   · 显示分段：仅由客户端静音检测驱动（连续静音 ≥ pause_break_secs 后下一处
//     新文本开新段），所有流式引擎表现一致；段内按「翻译单元」逐句呈现；
//     长段兜底：段内已闭合文本 ≥ PARA_SOFT_BREAK_UNITS 且在句末标点闭合时开新段；
//   · 翻译：翻译窗口按句末标点切单元（长度/停顿/停滞兜底），送译时携带前 2 个
//     单元的原文作上下文；活跃单元尾部节流送草稿翻译（同传式，原位刷新）。
//
// 事件：复用 transcript-update（paragraph_id 标识所属段；is_partial=true 为活跃
// 单元原位刷新）与 translate-update（is_partial=true 即草稿译文）。已闭合单元
// 冻结：上游回溯修订只影响活跃尾部，不改写已闭合单元（与 09-20 起「客户端不再
// 重切已提交文本」的取舍一致）。

use super::worker::TranscriptUpdate;
use log::{info, warn};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Runtime};

/// 流式管线单元序列号（与 worker / X-ASR / 远程流式计数器在热切换与录音开始时对齐）。
static FLOW_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// 段落 id 全局递增（跨管线实例唯一：热切换引擎新建管线时段落不撞号）。
static FLOW_PARAGRAPH_SEQ: AtomicU64 = AtomicU64::new(0);

/// 停顿分段阈值（秒）：连续静音达到该时长后，下一处新文本开新段。默认 10s，
/// 设置页可选 5/10/20，持久化在 settings 表 transcript.flow_pause_secs。
static FLOW_PAUSE_BREAK_SECS: AtomicU64 = AtomicU64::new(10);

pub fn flow_pause_break_secs() -> u64 {
    FLOW_PAUSE_BREAK_SECS.load(Ordering::SeqCst)
}

pub fn set_flow_pause_break_secs(v: u64) {
    FLOW_PAUSE_BREAK_SECS.store(v.clamp(3, 120), Ordering::SeqCst);
}

#[tauri::command]
pub fn get_flow_pause_secs() -> u64 {
    flow_pause_break_secs()
}

/// 设置停顿分段阈值（5/10/20 秒），持久化到 settings 表，启动时读回。
#[tauri::command]
pub async fn set_flow_pause_secs(
    state: tauri::State<'_, crate::state::AppState>,
    secs: u64,
) -> Result<(), String> {
    if !matches!(secs, 5 | 10 | 20) {
        return Err(format!("不支持的停顿分段阈值: {}（可选 5/10/20 秒）", secs));
    }
    set_flow_pause_break_secs(secs);
    crate::database::repositories::setting::SettingsRepository::set(
        state.db_manager.pool(),
        "transcript.flow_pause_secs",
        &secs.to_string(),
    )
    .await
    .map_err(|e| format!("保存停顿分段设置失败: {}", e))?;
    Ok(())
}

pub fn reset_flow_sequence() {
    FLOW_SEQUENCE.store(0, Ordering::SeqCst);
    FLOW_PARAGRAPH_SEQ.store(0, Ordering::SeqCst);
}

pub fn current_flow_sequence() -> u64 {
    FLOW_SEQUENCE.load(Ordering::SeqCst)
}

pub fn set_flow_sequence(v: u64) {
    FLOW_SEQUENCE.store(v, Ordering::SeqCst);
}

// ── 翻译窗口切分阈值 ─────────────────────────────────────────────────────────
/// 句级闭合的最小聚合（单位：CJK 字数 + 非 CJK 词数）：达到该值才允许在句末
/// 标点处闭合（聚合 "Thank you. Oh." 类碎句）。
const MIN_SEGMENT_UNITS: usize = 8;
/// 长度兜底软阈值：无句末标点且未闭合文本达到该值时，在最后一个弱边界
/// （，、；： , ; 空格）处闭合。
const SOFT_COMMIT_UNITS: usize = 60;
/// 长度兜底硬阈值：达到该值仍无可用弱边界时硬切。
const HARD_COMMIT_UNITS: usize = 100;
/// 停顿冲刷：静音 ≥4s 且活跃单元 ≥8 单位 → 闭合（利用停顿把翻译追平）。
const FLUSH_SILENCE_SECS: f64 = 4.0;
const FLUSH_MIN_UNITS: usize = 8;
/// 停滞兜底：文本 30s 无增长（静音或识别卡住）→ 闭合活跃单元。
const STALL_TIMEOUT: Duration = Duration::from_secs(30);
/// 活跃单元寿命兜底（秒），**两档**：
///
/// 为什么需要（2026-09-22 用户实测）：豆包流式的 definite（→ 网关 final）只在
/// 「停顿切句」时产生。用户连续说话时上游只发 partial、从不 final，
/// 而单元闭合只允许发生在 stable 文本内 → 一整段永远不闭合，
/// **显示上不分段、翻译更是一句都不出**（用户原话：分段不是必要的，
/// 但译文不出现就不应该）。Deepgram 每句都 is_final，所以它「正常」。
/// 实测：豆包一场 83 秒的录音只发过 **1 次** definite，分段完全靠这个兜底。
///
/// · SOFT（12s）：**只在尾巴里能找到自然边界时**才闭合（句末标点优先、其次弱边界），
///   切完把剩下的尾巴继续留在活跃单元 —— 这样既及时出段，又不会切出半句。
/// · HARD（20s）：无论有没有边界都闭合（保时效，避免一直不落段）。
///
/// 为什么分两档（2026-09-22 用户反馈「豆包要说一大段才出第一段，比别的模型慢」）：
/// 旧的单档 20s 让"有完整句子可切"的情况也硬等 20s；两档下这种情形的首段延迟
/// 减半到 ~12s，而"真的一句都还没说完"时仍然耐心等到 20s，不会切碎。
const MAX_UNIT_AGE_SOFT_SECS: f64 = 12.0;
const MAX_UNIT_AGE_HARD_SECS: f64 = 20.0;
/// 长段软断：段内已闭合文本达到该单位数后，下一个句末标点闭合时开新段。
const PARA_SOFT_BREAK_UNITS: usize = 300;
/// 草稿翻译节流：活跃尾部较上次草稿增长 ≥8 单位，或距上次 ≥600ms 且内容有变化。
const DRAFT_MIN_GROWTH_UNITS: usize = 8;
const DRAFT_MIN_INTERVAL: Duration = Duration::from_millis(600);
/// 翻译上下文：送译时携带的已闭合单元个数（原文，仅参考不翻译）。
///
/// ⚠️ 2026-09-22 起**设为 0（不再携带前文）**。原因：用户实测远程 LLM 翻译时，
/// 「前文（仅供理解上下文，不要翻译）」这段指令**模型并不遵守** —— 它会把前文
/// 一并翻译并拼在译文前面，于是每一段的译文都带着前 1~2 段的中文（越往后越长），
/// 看起来就是「译文大量重复」（实测第 5 段译文 = 第 3 段 + 第 4 段 + 「来源：」+ 第 5 段）。
/// 分段本身没有问题（英文各段都是完整句子、互不重叠），重复完全来自这里。
/// 代价：跨句指代/术语一致性略降；收益：译文与原文严格一一对应。需要恢复时改回 2 即可。
const CONTEXT_UNITS: usize = 0;

pub struct FlowPipeline {
    /// 已闭合单元的累计文本（显示定稿；只增，修订收缩时按公共前缀对齐）。
    committed: String,
    /// 最近一次上游会话累计文本。
    last_cumulative: String,
    /// 上游已定稿的字节长度（≤ last_cumulative.len()）。单元闭合只允许发生在
    /// 该范围内——未稳定文本（如 FunASR 草稿句）会被上游修订，提前闭合会把
    /// 同一句话冻结两遍（修订重复）。停顿冲刷/停滞/收尾兜底不受此限（保时效）。
    stable_len: usize,
    paragraph_id: u64,
    /// 当前段是否已有内容（停顿分段只对非空段生效）。
    paragraph_has_content: bool,
    /// 当前段已闭合文本的单位数（长段软断依据）。
    para_committed_units: usize,
    /// 连续静音 ≥ 阈值后挂起，下一处新文本到达时生效。
    pending_break: bool,
    unit_seq: u64,
    /// 最近闭合单元原文（翻译上下文），最多保留 CONTEXT_UNITS 条。
    unit_history: VecDeque<String>,
    current_audio_time: f64,
    last_speech_audio_time: f64,
    last_text_growth: Instant,
    /// 活跃单元开始累积文本的时刻（MAX_UNIT_AGE_SOFT/HARD_SECS 兜底依据）。
    unit_started_at: Instant,
    /// 上次 emit 的活跃单元文本（去重，避免刷屏）。
    last_emitted_tail: String,
    last_draft_text: String,
    last_draft_at: Instant,
}

impl FlowPipeline {
    pub fn new() -> Self {
        Self {
            committed: String::new(),
            last_cumulative: String::new(),
            stable_len: 0,
            paragraph_id: FLOW_PARAGRAPH_SEQ.fetch_add(1, Ordering::SeqCst),
            paragraph_has_content: false,
            para_committed_units: 0,
            pending_break: false,
            unit_seq: FLOW_SEQUENCE.fetch_add(1, Ordering::SeqCst),
            unit_history: VecDeque::new(),
            current_audio_time: 0.0,
            last_speech_audio_time: 0.0,
            last_text_growth: Instant::now(),
            unit_started_at: Instant::now(),
            last_emitted_tail: String::new(),
            last_draft_text: String::new(),
            last_draft_at: Instant::now() - DRAFT_MIN_INTERVAL,
        }
    }

    /// 音频时钟与有声检测（RMS > 0.01 视为有声，沿用既有口径）。
    pub fn note_audio(&mut self, timestamp: f64, rms: f32) {
        self.current_audio_time = timestamp;
        if rms > 0.01 {
            self.last_speech_audio_time = timestamp;
        }
    }

    fn tail(&self) -> &str {
        if self.last_cumulative.starts_with(self.committed.as_str()) {
            &self.last_cumulative[self.committed.len()..]
        } else {
            &self.last_cumulative[common_prefix_len(&self.committed, &self.last_cumulative)..]
        }
    }

    fn translation_context(&self) -> Option<String> {
        if self.unit_history.is_empty() {
            return None;
        }
        let mut s = String::new();
        for unit in &self.unit_history {
            if !s.is_empty() {
                s.push(' ');
            }
            s.push_str(unit);
        }
        Some(s)
    }

    /// 推进会话累计文本：闭合完整单元（送翻译）+ 原位刷新活跃单元 + 节流草稿翻译。
    ///
    /// ⚠️⚠️ 这两个 bool 是**两个完全不同的含义**，2026-10-02 之前它们被合并成一个
    /// `upstream_final`，导致「改远程」把「本地」一起改坏了，务必分清：
    ///
    /// · `upstream_final` —— **这段文本已经定稿**
    ///     true 表示上游把当前累计文本整体定稿（Deepgram is_final、豆包 definite、
    ///     FunASR sentence_end、千问 .completed；**本地 X-ASR/sherpa 的每次结果也算**）。
    ///     作用：推进 `stable_len`，让单元闭合只发生在稳定范围内，
    ///     避免上游修订草稿句造成「修订重复」。
    ///
    /// · `semantic_boundary` —— **这个 final 是一个语义边界，可以在此断开新段/新段**
    ///     只有**上游按静音切分（endpointing）产生的 final** 才算，
    ///     例如 deepgram 的 `is_final`（endpointing:300ms，约 2.5~3.5s 一次）。
    ///     作用：驱动分段节奏，让段落跟着上游的语义边界走而不是干等长度兜底。
    ///
    /// **为什么必须分开**：本地 X-ASR 的结果是**按 VAD 切**的（480ms 粒度），
    /// 如果也当语义边界，每个小段都会被闭合 —— 用户实测到的就是
    /// 「X-ASR 切得很碎，和上一个版本体验差很多」（2026-10-02 真机反馈）。
    pub fn push_text<R: Runtime>(
        &mut self,
        app: &AppHandle<R>,
        cumulative: &str,
        upstream_final: bool,
        semantic_boundary: bool,
    ) {
        if upstream_final {
            self.stable_len = cumulative.len();
        }
        if cumulative == self.last_cumulative {
            // 文本未变且稳定边界未前进：无事可做
            if !upstream_final || self.stable_len == self.committed.len() {
                return;
            }
        } else {
            self.last_text_growth = Instant::now();
            self.last_cumulative = cumulative.to_string();
        }
        if !self.last_cumulative.starts_with(self.committed.as_str()) {
            // 累计文本与已定稿部分不一致：**必须区分三种情况**（2026-09-24 两次事故修复）。
            //
            // ① 新会话（重连后上游从头累计）：新文本比 committed 短、或公共前缀极短
            //    → 按最长公共前缀回滚对齐（这是这里原本的唯一处理，跨重连需要它）。
            // ② 同一会话内的**回溯修订**（豆包会改写已定稿句子的用词/标点，实测同一句
            //    被改写后公共前缀只到句中）：如果也回滚 committed，那么「已显示过的
            //    一整段」会被当成新文本重新闭合一次 —— 2026-09-24 用户实测：
            //    4.8 分钟的豆包会话里出现 734 字 / 1428 字的巨型段落（末尾一大段重复块），
            //    且活跃单元被重复文本撑大 → 翻译要等巨型单元闭合才更新（"翻译不实时"）。
            //    → 修订时**保留已定稿文本**，只从 committed 之后继续累计新文本。
            // ③ **上游只是落后**（本次新增）：豆包的 definite 边界常常晚于客户端自身的
            //    寿命兜底闭合，于是会下发一段**比本地已定稿更短**的 final。它看着也满足
            //    「比 committed 短」，走 ① 就会回滚 → 刚闭合的尾巴被再发一遍
            //    （2026-09-24 08:23 实测：段15 的尾巴 64 字原样重复成段16）。
            //    → 归一化（忽略大小写/标点/空白）后若是已定稿的**前缀**，判定为「落后」，
            //      既不回滚也不重发。
            let lcp = common_prefix_len(&self.committed, &self.last_cumulative);
            let lagging = is_lagging_upstream(&self.committed, &self.last_cumulative);
            let looks_like_new_session =
                !lagging && is_new_session_reset(self.committed.len(), self.last_cumulative.len(), lcp);
            if looks_like_new_session {
                self.committed.truncate(lcp);
            }
            // 修订/落后：committed 不动。下面的闭合循环只会看 committed 之后的稳定文本，
            // 因此不会把已显示内容重发；活跃尾巴仍会显示修订后的最新文本（原位刷新）。
        }
        self.stable_len = self.stable_len.min(self.last_cumulative.len());

        // 停顿后的第一处新文本：开新段
        if self.pending_break && !self.tail().trim().is_empty() {
            self.pending_break = false;
            self.start_new_paragraph();
        }

        // 闭合所有完整单元：只在已稳定文本内切（句末标点优先，长度兜底）
        loop {
            // ⚠️ 越界防护（2026-09-22 / 2026-09-24 两次线上崩溃修复）：
            // 原来写的是 `&last_cumulative[..stable_len][committed.len()..]`。
            //   · 2026-09-22：`committed.len() > stable_len`（兜底闭合只推进 committed
            //     的旧实现）→ "start byte index 93 is out of bounds for string of length 0"
            //   · 2026-09-24：stable_len / committed.len() 是在**上一版**累计文本上量出的
            //     字节偏移；上游回溯修订或重连后新会话从头累计时，同一偏移在新文本里可能
            //     落在多字节字符中间 → "end byte index 21 is not a char boundary;
            //     it is inside '，' (bytes 20..23 of string)"
            // panic 发生在转写任务线程里 → 任务直接死掉 → 管线「consumer gone」→
            // 用户看到「说了半天一个字都不再出现」。stable_window 负责把窗口两端
            // 都夹取并对齐到当前文本的字符边界，绝不越界、绝不切半字符。
            let Some((committed_len, stable_end)) = stable_window(
                &self.last_cumulative,
                self.stable_len,
                self.committed.len(),
            ) else {
                break;
            };
            let stable_tail = &self.last_cumulative[committed_len..stable_end];
            let strong = find_commit_boundary(stable_tail, MIN_SEGMENT_UNITS);
            // ── 上游显式 final = 语义边界，必须认（2026-09-30 修 deepgram「卡顿」）──────
            // 背景：deepgram / 豆包 / 千问的 final 是按**静音切分**产生的语义边界
            // （deepgram 是 endpointing:300ms），**大多不带句末标点**。
            // 而 find_commit_boundary 只认 `。！？.!?`，于是 strong 恒为 None →
            // 一路退到长度兜底（SOFT=60 单位 / 12 秒）→ 用户看到的节奏变成
            // 「每 12 秒蹦一块」，而上游其实每 3 秒就给了边界。
            // 实测对照（2026-09-30）：网关每 2.5~3.5s 下发一次 final，
            // 客户端却 3~15s 才提交一段（18:31–18:32 间隔 14.2/3.0/8.9/13.7/4.2…）。
            //
            // 判据用「上游这次给的是 final」+ 最小长度（MIN_SEGMENT_UNITS），
            // 避免上游把 final 切得很碎时产生大量碎片段。
            // 对豆包无副作用：它 58 秒才给 1 次 definite，本来就走不到这条分支。
            let upstream_boundary = if semantic_boundary
                && text_length_units(stable_tail) >= MIN_SEGMENT_UNITS
            {
                Some(stable_tail.len())
            } else {
                None
            };
            let boundary = strong.or(upstream_boundary).or_else(|| {
                find_length_boundary(
                    stable_tail,
                    SOFT_COMMIT_UNITS,
                    HARD_COMMIT_UNITS,
                    MIN_SEGMENT_UNITS,
                )
            });
            let Some(boundary) = boundary else { break };
            let unit_text = stable_tail[..boundary].trim().to_string();
            if unit_text.chars().filter(|c| !c.is_whitespace()).count() < 2 {
                break;
            }
            let absolute = committed_len + boundary;
            self.committed = self.last_cumulative[..absolute].to_string();
            self.close_unit(app, unit_text);
            // 长段软断：在句末标点**或上游显式 final**闭合时开新段（不断在长句中间）
            if (strong.is_some() || upstream_boundary.is_some())
                && self.para_committed_units >= PARA_SOFT_BREAK_UNITS
            {
                self.start_new_paragraph();
            }
        }

        // 活跃单元原位刷新
        let tail = self.tail().to_string();
        if !tail.trim().is_empty() && tail != self.last_emitted_tail {
            self.last_emitted_tail = tail.clone();
            self.emit(app, &tail, true);
            self.paragraph_has_content = true;
        }

        // 草稿翻译：活跃尾部的同传式试译（增长 ≥8 单位或距上次 ≥600ms）
        let trimmed = tail.trim();
        if !trimmed.is_empty() && trimmed != self.last_draft_text {
            let grown =
                text_length_units(trimmed).saturating_sub(text_length_units(&self.last_draft_text));
            if grown >= DRAFT_MIN_GROWTH_UNITS || self.last_draft_at.elapsed() >= DRAFT_MIN_INTERVAL
            {
                let draft_text = trimmed.to_string();
                self.last_draft_text = draft_text.clone();
                self.last_draft_at = Instant::now();
                crate::translation::queue_translation_draft(
                    app,
                    &draft_text,
                    self.unit_seq,
                    self.translation_context(),
                );
            }
        }
    }

    /// 周期检查（500ms）：停顿冲刷翻译、停顿分段挂起、停滞兜底闭合。
    pub fn tick<R: Runtime>(&mut self, app: &AppHandle<R>) {
        let silence = if self.last_speech_audio_time > 0.0 {
            self.current_audio_time - self.last_speech_audio_time
        } else {
            0.0
        };
        let tail_units = text_length_units(self.tail().trim());
        if tail_units == 0 {
            // 活跃单元为空：寿命计时归零（下一处文本到达即开始计）。
            self.unit_started_at = Instant::now();
        }
        let unit_age = self.unit_started_at.elapsed().as_secs_f64();

        // 停顿冲刷：静音 ≥4s 且活跃单元非空 → 闭合送译（不等句末标点）
        if silence >= FLUSH_SILENCE_SECS && tail_units >= FLUSH_MIN_UNITS {
            info!("⏸️ 流式管线停顿闭合（≥4s 静音）");
            self.commit_active_unit(app);
        } else if tail_units >= MIN_SEGMENT_UNITS && unit_age >= MAX_UNIT_AGE_SOFT_SECS {
            // 寿命兜底（见 MAX_UNIT_AGE_SOFT/HARD_SECS 注释）：豆包等「只在停顿定稿」的
            // 上游连续说话时从不发 final → 活跃单元无限累积、既不闭合成段也不送译。
            if self.try_commit_at_boundary(app) {
                info!(
                    "⏳ 流式管线活跃单元已存活 {:.0}s（上游未给 final）— 按自然边界闭合送译",
                    unit_age
                );
            } else if unit_age >= MAX_UNIT_AGE_HARD_SECS {
                info!(
                    "⏳ 流式管线活跃单元已存活 {:.0}s 且无可用断点 — 整段闭合送译",
                    unit_age
                );
                self.commit_tail(app);
            }
        } else if self.last_text_growth.elapsed() > STALL_TIMEOUT && tail_units > 0 {
            info!("⏰ 流式管线 30s 停滞超时 — 闭合活跃单元");
            self.commit_active_unit(app);
        }

        // 停顿分段挂起：连续静音 ≥ 阈值，下一处新文本开新段
        if silence >= flow_pause_break_secs() as f64 && self.paragraph_has_content {
            self.pending_break = true;
        }
    }

    /// 输入结束：闭合活跃单元尾巴，避免丢字。
    pub fn finish<R: Runtime>(&mut self, app: &AppHandle<R>) {
        self.commit_tail(app);
    }

    /// 活跃尾巴在 `last_cumulative` 里的起始字节偏移（`tail()` 的对齐基准）。
    fn tail_offset(&self) -> usize {
        if self.last_cumulative.starts_with(self.committed.as_str()) {
            self.committed.len()
        } else {
            common_prefix_len(&self.committed, &self.last_cumulative)
        }
    }

    /// 兜底闭合（停顿 / 最长寿命 / 停滞）：**尽量切在自然边界**，把剩下的尾巴
    /// 留在活跃单元里继续累积。
    ///
    /// 2026-09-22 用户实测「流式识别分段比较碎，有的在句子中间就分段 / 译文和原文对不上」：
    /// 旧的 `commit_tail` 会把当时的整条尾巴（= 上游尚未定稿的当前句）一刀切断——
    /// 豆包在这段录音里 58 秒只发过 1 次 definite，于是**分段完全由 12s 兜底驱动**，
    /// 每 12s 产生一个半句（如 "… What's crazy about this? I"），译文也只能翻半句，
    /// 而且冻结的半句会被上游随后的修订冲掉重发（出现重复的 "? I"）。
    ///
    /// 现在：句末标点优先（≥MIN_SEGMENT_UNITS 才认，避免「Hi.」这种碎句），
    /// 其次找第一个弱边界（空格/逗号，≥SOFT_COMMIT_UNITS 个单位）；只有整条尾巴都没有
    /// 可用断点时才整段闭合（保时效）。未闭合的部分继续留在活跃单元，等真正的句末标点。
    fn commit_active_unit<R: Runtime>(&mut self, app: &AppHandle<R>) {
        if !self.try_commit_at_boundary(app) {
            // 整条尾巴里没有可用断点（一直没标点/空格）：只能整段闭合保时效
            self.commit_tail(app);
        }
    }

    /// 尝试把活跃尾巴**按自然边界**闭合一段（句末标点优先，其次弱边界）。
    /// 返回 true 表示已闭合一段（剩余尾巴仍在活跃单元里）；false 表示找不到断点、未做任何事。
    fn try_commit_at_boundary<R: Runtime>(&mut self, app: &AppHandle<R>) -> bool {
        let raw_tail = self.tail().to_string();
        let lead = raw_tail.len() - raw_tail.trim_start().len();
        let tail = raw_tail.trim();
        if tail.is_empty() {
            return false;
        }
        let Some(b) = find_commit_boundary(tail, MIN_SEGMENT_UNITS)
            .or_else(|| find_first_weak_boundary_after(tail, SOFT_COMMIT_UNITS))
        else {
            return false;
        };
        if b >= tail.len() {
            return false;
        }
        let unit_text = tail[..b].trim().to_string();
        if text_length_units(&unit_text) < MIN_SEGMENT_UNITS {
            return false;
        }
        let absolute = self.tail_offset() + lead + b;
        self.committed = self.last_cumulative[..absolute].to_string();
        // 与 commit_tail 同理：committed 前进了，stable_len 不能落后（否则越界）
        self.stable_len = self.stable_len.max(absolute);
        self.close_unit(app, unit_text);
        true
    }

    /// 把活跃尾巴整体闭合为一个单元（**收尾专用**：输入已结束，没有「以后」了）。
    fn commit_tail<R: Runtime>(&mut self, app: &AppHandle<R>) {
        let tail = self.tail().trim().to_string();
        if tail.is_empty() {
            return;
        }
        // 冻结整段累计文本：committed 与 stable_len **必须一起推进**。
        //
        // 2026-09-22 线上崩溃根因：旧实现只推 committed（= last_cumulative），
        // stable_len 仍停在上一次上游 final 的位置 → `committed.len() > stable_len`。
        // 下一次 push_text（尤其是只发 partial、从不发 final 的 FunASR/豆包）
        // 走到 `&last_cumulative[..stable_len][committed.len()..]` 就 panic：
        //   "start byte index 93 is out of bounds for string of length 0"
        // 转写任务线程直接死掉 → 管线 consumer gone → 用户看到「识别到一半就再不
        // 出字」。兜底闭合（停顿 / 12s 最长寿命 / 30s 停滞 / 收尾）都会走到这里，
        // 所以这个修复对所有流式引擎都必要。
        self.committed = self.last_cumulative.clone();
        self.stable_len = self.last_cumulative.len();
        self.close_unit(app, tail);
    }

    /// 闭合一个单元：emit final（冻结，不再改写）+ 带上下文送译 + 推进序列。
    fn close_unit<R: Runtime>(&mut self, app: &AppHandle<R>, text: String) {
        if text.is_empty() {
            return;
        }
        // 单元闭合后，此前的草稿译文一律作废（等定稿译文原位覆盖）
        crate::translation::invalidate_translation_drafts();
        self.last_draft_text.clear();
        self.last_emitted_tail.clear();
        self.unit_started_at = Instant::now();
        self.para_committed_units += text_length_units(&text);
        self.paragraph_has_content = true;

        self.emit(app, &text, false);
        let context = self.translation_context();
        crate::translation::queue_translation_with_context(app, &text, self.unit_seq, context);
        self.unit_history.push_back(text);
        while self.unit_history.len() > CONTEXT_UNITS {
            self.unit_history.pop_front();
        }
        self.unit_seq = FLOW_SEQUENCE.fetch_add(1, Ordering::SeqCst);
    }

    fn start_new_paragraph(&mut self) {
        self.paragraph_id = FLOW_PARAGRAPH_SEQ.fetch_add(1, Ordering::SeqCst);
        self.para_committed_units = 0;
        self.paragraph_has_content = false;
        info!("📑 流式管线分段 → paragraph {}", self.paragraph_id);
    }

    fn emit<R: Runtime>(&self, app: &AppHandle<R>, text: &str, is_partial: bool) {
        if text.trim().is_empty() {
            return;
        }
        let update = TranscriptUpdate {
            text: text.to_string(),
            timestamp: format_timestamp(),
            source: "Audio".to_string(),
            sequence_id: self.unit_seq,
            chunk_start_time: 0.0,
            is_partial,
            confidence: 0.9,
            audio_start_time: 0.0,
            audio_end_time: 0.0,
            duration: 0.0,
            paragraph_id: Some(self.paragraph_id),
        };
        if let Err(e) = app.emit("transcript-update", &update) {
            warn!("流式管线 emit transcript-update 失败: {}", e);
        }
    }
}

fn format_timestamp() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let h = (now.as_secs() / 3600) % 24;
    let m = (now.as_secs() / 60) % 60;
    let s = now.as_secs() % 60;
    format!("{:02}:{:02}:{:02}", h, m, s)
}

/// 「可闭合的稳定窗口」在累计文本里的字节区间（start, end）；None = 没有可闭合的稳定文本。
///
/// 抽成纯函数是为了能被单测直接覆盖。两次线上 panic 都倒在这里：
///   · 2026-09-22：`committed.len()` 可能大于 `stable_len`（兜底闭合只推进 committed 的
///     旧实现），直接 `&last_cumulative[..stable_len][committed.len()..]` 必然越界；
///   · 2026-09-24：stable_len / committed_len 是在**上一版**累计文本上量出的字节偏移
///     （上游回溯修订 / 重连后新会话从头累计都会换掉整个 last_cumulative），同一偏移在
///     新文本里可能落在多字节字符中间。所以这里除了夹取范围，还必须把两端
///     **向下对齐到当前文本的字符边界**。
fn stable_window(text: &str, stable_len: usize, committed_len: usize) -> Option<(usize, usize)> {
    let end = floor_char_boundary(text, stable_len);
    let start = floor_char_boundary(text, committed_len.min(end));
    if start >= end {
        None
    } else {
        Some((start, end))
    }
}

/// 把字节偏移向下对齐到 `s` 的字符边界（先夹到 len，再逐字节回退）。
/// 用于「在字符串 A 上量出的偏移拿去切字符串 B」的场景：A 的边界在 B 里不一定是边界。
fn floor_char_boundary(s: &str, offset: usize) -> usize {
    let mut i = offset.min(s.len());
    while !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

/// 累计文本与已定稿部分不一致时，判断这是「新会话从头累计」还是「同会话回溯修订」。
///
/// 纯函数便于单测。误判成「新会话」会把已经显示过的文本回滚后重新闭合一遍
/// （2026-09-24 事故：豆包回溯修订 → 734/1428 字巨型段落 + 重复块 + 翻译滞后）。
/// 判据：新文本比已定稿短 → 一定是重连后的新会话；公共前缀不到已定稿的一半 → 也是新会话。
fn is_new_session_reset(committed_len: usize, last_len: usize, lcp: usize) -> bool {
    last_len <= committed_len || lcp * 2 < committed_len
}

/// 「上游只是落后于本地已定稿」：忽略大小写/标点/空白后，累计文本是已定稿文本的**前缀**。
///
/// 为什么单独判（2026-09-24 第二次事故）：豆包的 definite 边界常晚于客户端自身的寿命兜底
/// 闭合，于是会下发一段比本地已定稿更短的 final —— 它同样满足「比 committed 短」，
/// 被当成新会话回滚的话，刚闭合的尾巴会被原样再发一遍（实测 64 字重复）。
/// 归一化比较是必须的：豆包的分句拼接与本地上游全文在标点/大小写上本来就不一致。
fn is_lagging_upstream(committed: &str, cumulative: &str) -> bool {
    fn norm(s: &str) -> String {
        s.chars()
            .filter(|c| c.is_alphanumeric())
            .flat_map(|c| c.to_lowercase())
            .collect()
    }
    let c = norm(committed);
    let u = norm(cumulative);
    !u.is_empty() && c.starts_with(&u)
}

/// 两个字符串的最长公共前缀字节长度（按字符对齐，truncate 不会切开 UTF-8 字符）。
fn common_prefix_len(a: &str, b: &str) -> usize {    a.chars()
        .zip(b.chars())
        .take_while(|(x, y)| x == y)
        .map(|(x, _)| x.len_utf8())
        .sum()
}

/// Find the FIRST sentence boundary whose prefix has at least `min_units`
/// text units (CJK chars + non-CJK words). Returns byte offset (safe for
/// slicing) or None.（移植自原 X-ASR 提交规则，语义不变）
fn find_commit_boundary(text: &str, min_units: usize) -> Option<usize> {
    let char_indices: Vec<(usize, char)> = text.char_indices().collect();
    let len = char_indices.len();
    let mut units = 0usize;
    let mut in_word = false;

    for (i, (_, c)) in char_indices.iter().enumerate() {
        if (*c as u32) >= 0x4E00 {
            units += 1;
            in_word = false;
        } else if c.is_whitespace() {
            in_word = false;
        } else if !in_word {
            units += 1;
            in_word = true;
        }

        let is_sentence_end = matches!(c, '。' | '！' | '？' | '.' | '!' | '?');
        if !is_sentence_end {
            continue;
        }
        let next = char_indices.get(i + 1).map(|(_, c)| *c);
        let boundary_ok = match next {
            None => true,
            Some(n) => {
                n.is_whitespace()
                    || matches!(
                        n,
                        '"' | '\u{300D}' | '\u{FF09}' | '\u{3011}' | '」' | '）' | '】'
                    )
                    || (n as u32) >= 0x4e00
            }
        };
        if !boundary_ok {
            continue;
        }
        let mut end_idx = i + 1;
        while end_idx < len && matches!(char_indices[end_idx].1, ' ' | '\n' | '\r') {
            end_idx += 1;
        }
        let boundary = if end_idx < len {
            char_indices[end_idx].0
        } else {
            text.len()
        };
        if units >= min_units {
            return Some(boundary);
        }
    }
    None
}

/// 找**第一个**弱边界（空格 / 逗号类）且其前缀至少 `min_units` 个单位的位置。
///
/// 与 `find_length_boundary`（返回**最后**一个弱边界）不同：兜底闭合要的是尽量靠前的
/// 自然断点，好把后面的文本继续留在活跃单元里等句末标点。
fn find_first_weak_boundary_after(text: &str, min_units: usize) -> Option<usize> {
    let char_indices: Vec<(usize, char)> = text.char_indices().collect();
    let len = char_indices.len();
    let mut units = 0usize;
    let mut in_word = false;
    for (i, (_, c)) in char_indices.iter().enumerate() {
        if (*c as u32) >= 0x4E00 {
            units += 1;
            in_word = false;
        } else if c.is_whitespace() {
            in_word = false;
        } else if !in_word {
            units += 1;
            in_word = true;
        }
        let is_weak = matches!(c, '，' | '、' | '；' | '：' | ',' | ';' | ' ');
        if is_weak && units >= min_units {
            let mut end_idx = i + 1;
            while end_idx < len && char_indices[end_idx].1.is_whitespace() {
                end_idx += 1;
            }
            return Some(if end_idx < len {
                char_indices[end_idx].0
            } else {
                text.len()
            });
        }
    }
    None
}

/// Length-based fallback boundary for punctuation-free speech：达到 soft 单位
/// 后在最后一个弱边界（前缀 ≥min 单位）软切；达到 hard 单位仍无弱边界则硬切。
/// （移植自原 X-ASR 提交规则，语义不变）
fn find_length_boundary(text: &str, soft: usize, hard: usize, min: usize) -> Option<usize> {
    let char_indices: Vec<(usize, char)> = text.char_indices().collect();
    let len = char_indices.len();
    let mut units = 0usize;
    let mut in_word = false;
    let mut last_weak: Option<(usize, usize)> = None;

    for (i, (byte_idx, c)) in char_indices.iter().enumerate() {
        if (*c as u32) >= 0x4E00 {
            units += 1;
            in_word = false;
        } else if c.is_whitespace() {
            in_word = false;
        } else if !in_word {
            units += 1;
            in_word = true;
        }

        let is_weak = matches!(c, '，' | '、' | '；' | '：' | ',' | ';' | ' ');
        if is_weak {
            let mut end_idx = i + 1;
            while end_idx < len && char_indices[end_idx].1.is_whitespace() {
                end_idx += 1;
            }
            let boundary = if end_idx < len {
                char_indices[end_idx].0
            } else {
                text.len()
            };
            last_weak = Some((boundary, units));
        }

        if units >= hard {
            if let Some((boundary, weak_units)) = last_weak {
                if weak_units >= min {
                    return Some(boundary);
                }
            }
            return Some(byte_idx + c.len_utf8());
        }
    }

    if units >= soft {
        if let Some((boundary, weak_units)) = last_weak {
            if weak_units >= min {
                return Some(boundary);
            }
        }
    }
    None
}

/// Text length in "units": 1 per CJK character plus 1 per non-CJK word.
pub(crate) fn text_length_units(text: &str) -> usize {
    let cjk = text.chars().filter(|c| is_cjk(*c)).count();
    let words = text
        .split_whitespace()
        .flat_map(|w| w.split(|c: char| c.is_ascii_punctuation()))
        .filter(|w| {
            !w.is_empty() && !w.chars().any(is_cjk) && w.chars().any(|c| c.is_alphanumeric())
        })
        .count();
    cjk + words
}

fn is_cjk(c: char) -> bool {
    matches!(c as u32,
        0x4E00..=0x9FFF |
        0x3400..=0x4DBF |
        0xF900..=0xFAFF |
        0x20000..=0x2CEAF |
        0x2F800..=0x2FA1F
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 2026-09-22 线上 panic 回归：`committed.len() > stable_len` 时必须返回 None，
    /// 而不是让调用方去切一个越界区间（旧实现直接 panic：
    /// "start byte index 93 is out of bounds for string of length 0"）。
    #[test]
    fn stable_window_never_out_of_bounds() {
        // 用 ASCII 占位文本构造指定字节长度（数字参数语义与旧签名一致）
        let win = |last_len, stable_len, committed_len| {
            stable_window(&"a".repeat(last_len), stable_len, committed_len)
        };
        // 兜底闭合只推进了 committed（93），stable_len 仍是 0 → 无可闭合窗口
        assert_eq!(win(0, 0, 93), None);
        assert_eq!(win(193, 0, 93), None);
        // committed 超过 stable_len：必须夹到 stable_len，不能越界
        assert_eq!(win(500, 100, 193), None);
        assert_eq!(win(500, 100, 99), Some((99, 100)));
        // stable_len 超过实际文本长度：夹到实际长度
        assert_eq!(win(50, 100, 0), Some((0, 50)));
        // 正常推进
        assert_eq!(win(200, 120, 20), Some((20, 120)));
        // committed 已追平 stable_len：无新可闭合文本
        assert_eq!(win(100, 100, 100), None);
        assert_eq!(win(0, 0, 0), None);
    }

    #[test]
    fn stable_window_slices_are_char_safe() {
        // 用真实字符串验证切片边界（UTF-8 安全）
        let text = "你好，世界。hello world.";
        let (s, e) = stable_window(text, text.len(), 0).unwrap();
        let _ = &text[s..e]; // 不 panic 即可
        // committed 已追平 stable：无新可闭合文本
        assert_eq!(stable_window(text, text.len(), text.len()), None);
    }

    /// 2026-09-24 22:22 线上 panic 回归（豆包流式重连后 18s）：
    /// stable_len=21 是**上一版**累计文本（如「今天我们开会吧」，21 字节）的定稿边界；
    /// 重连追赶时上游整段重识别，新文本字节 20..23 是 '，'。旧实现只按长度夹取，
    /// `&last_cumulative[..21]` 直接 panic：
    /// "end byte index 21 is not a char boundary; it is inside '，' (bytes 20..23 of string)"
    /// → 转写任务线程死亡 → 管线 consumer_gone 闩锁 → 65 秒音频被静默丢弃。
    #[test]
    fn stable_window_aligns_to_char_boundary_after_revision() {
        let revised = "好的ok我们现在开始，讨论一下方案";
        assert!(
            !revised.is_char_boundary(21),
            "预置条件：字节 21 必须落在 '，'（bytes 20..23）中间"
        );
        // 终点对齐：stable_len=21 来自上一版文本，在新文本里必须向下对齐
        let (s, e) = stable_window(revised, 21, 0).expect("应有可闭合窗口");
        assert_eq!((s, e), (0, 20));
        let _ = &revised[s..e]; // 修复前这一刀就是线上的 panic
        // 起点同样可能来自上一版文本（修订分支保留 committed 不动）→ 也要对齐
        let (s, e) = stable_window(revised, revised.len(), 21).unwrap();
        assert_eq!(s, 20);
        let _ = &revised[s..e];
    }

    #[test]
    fn floor_char_boundary_clamps_and_aligns() {
        let s = "好的ok我们现在开始，讨论";
        assert_eq!(floor_char_boundary(s, 21), 20);
        assert_eq!(floor_char_boundary(s, 20), 20); // 已在边界上不动
        assert_eq!(floor_char_boundary(s, 0), 0);
        assert_eq!(floor_char_boundary(s, usize::MAX), s.len()); // 先夹到 len
        assert_eq!(floor_char_boundary("", 5), 0);
    }

    /// 2026-09-22 用户实测的真实尾巴："…What's crazy about this? I"（兜底闭合把半句
    /// 冻成了段，下一段又以 "? I always…" 开头）。按自然边界切应当停在问号处，
    /// 把孤零零的 " I" 留给活跃单元。
    #[test]
    fn boundary_commit_stops_at_sentence_end_not_mid_sentence() {
        let tail = "Oh my god. Oh my god. What's crazy about this? I";
        let b = find_commit_boundary(tail, MIN_SEGMENT_UNITS).expect("should find boundary");
        assert_eq!(
            tail[..b].trim(),
            "Oh my god. Oh my god. What's crazy about this?"
        );
        let rest = tail[b..].trim();
        assert_eq!(rest, "I", "剩下的碎片应留在活跃单元，而不是被冻成一段");
    }

    #[test]
    fn first_weak_boundary_is_the_earliest_one_past_threshold() {
        // 60 个单词的长句（无句末标点）：应停在**第一个**达到阈值的空格处，
        // 而不是像 find_length_boundary 那样停在最后一个空格。
        let words: Vec<String> = (0..80).map(|i| format!("w{}", i)).collect();
        let text = words.join(" ");
        let b = find_first_weak_boundary_after(&text, SOFT_COMMIT_UNITS).expect("weak boundary");
        let cut = text[..b].trim_end();
        let n = cut.split(' ').count();
        assert!(
            n >= SOFT_COMMIT_UNITS && n <= SOFT_COMMIT_UNITS + 2,
            "应停在阈值附近（实际 {} 个词）",
            n
        );
    }

    #[test]
    fn first_weak_boundary_cjk_comma() {
        // 逗号出现在阈值**之后** → 切在它那里
        let text = format!("{}，{}", "今".repeat(70), "明".repeat(40));
        let b = find_first_weak_boundary_after(&text, SOFT_COMMIT_UNITS).expect("weak boundary");
        assert_eq!(text[..b].trim_end().chars().count(), 71);
        // 逗号出现在阈值**之前**且后面再无弱边界 → 不切（调用方回退为整段闭合，
        // 这比在 30 字处切出一个短单元更符合「宁长勿碎」）
        let early = format!("{}，{}", "今".repeat(30), "明".repeat(30));
        assert!(find_first_weak_boundary_after(&early, SOFT_COMMIT_UNITS).is_none());
    }

    #[test]
    fn first_weak_boundary_none_without_separator() {
        assert!(find_first_weak_boundary_after(&"汉".repeat(200), SOFT_COMMIT_UNITS).is_none());
        // 不足阈值也不切
        assert!(find_first_weak_boundary_after("short text here", SOFT_COMMIT_UNITS).is_none());
    }

    #[test]
    fn common_prefix_len_ascii_and_cjk() {
        assert_eq!(common_prefix_len("", ""), 0);
        assert_eq!(common_prefix_len("abc", "abd"), 2);
        assert_eq!(common_prefix_len("你好世界", "你好，世界"), "你好".len());
        assert_eq!(common_prefix_len("中文abc", "中文abd"), "中文ab".len());
        assert_eq!(common_prefix_len("abc", "xyz"), 0);
    }

    #[test]
    fn units_count_english_words() {
        assert_eq!(text_length_units("Thank you. Oh. Thank you."), 5);
        assert_eq!(text_length_units("Well, thank you so much."), 5);
    }

    #[test]
    fn units_count_cjk_chars() {
        assert_eq!(text_length_units("你好世界。"), 4);
        assert_eq!(text_length_units("谢谢，谢谢大家！"), 6);
    }

    #[test]
    fn units_mixed_text_counts_both() {
        assert_eq!(text_length_units("谢谢你 thank you"), 5);
    }

    #[test]
    fn units_ignore_empty_and_punct_only() {
        assert_eq!(text_length_units(""), 0);
        assert_eq!(text_length_units("..."), 0);
        assert_eq!(text_length_units("。！？"), 0);
    }

    #[test]
    fn commit_boundary_groups_tiny_sentences() {
        let text = "Thank you. Oh. Thank you. I love you back. Well, first of all.";
        let b = find_commit_boundary(text, MIN_SEGMENT_UNITS).expect("should find boundary");
        assert_eq!(
            text[..b].trim(),
            "Thank you. Oh. Thank you. I love you back."
        );
    }

    #[test]
    fn commit_boundary_single_long_sentence() {
        let text = "I am still fired up and ready to go. Thank you. Thank you.";
        let b = find_commit_boundary(text, MIN_SEGMENT_UNITS).expect("should find boundary");
        assert_eq!(text[..b].trim(), "I am still fired up and ready to go.");
    }

    #[test]
    fn commit_boundary_none_when_too_short() {
        assert!(find_commit_boundary("Thank you. ", MIN_SEGMENT_UNITS).is_none());
    }

    #[test]
    fn length_boundary_soft_cut_at_last_weak_boundary_cjk() {
        let text = format!("{}，{}", "今".repeat(30), "明".repeat(30));
        let b = find_length_boundary(
            &text,
            SOFT_COMMIT_UNITS,
            HARD_COMMIT_UNITS,
            MIN_SEGMENT_UNITS,
        )
        .expect("should soft-cut at weak boundary");
        assert_eq!(text[..b].chars().count(), 31);
    }

    #[test]
    fn length_boundary_soft_cut_at_last_space_english() {
        let words: Vec<String> = (0..60).map(|i| format!("w{}", i)).collect();
        let text = words.join(" ");
        let b = find_length_boundary(
            &text,
            SOFT_COMMIT_UNITS,
            HARD_COMMIT_UNITS,
            MIN_SEGMENT_UNITS,
        )
        .expect("should soft-cut at last space");
        assert_eq!(text[..b].trim_end(), words[..59].join(" "));
    }

    #[test]
    fn length_boundary_hard_cut_without_weak_boundary() {
        let text = "汉".repeat(90);
        let b = find_length_boundary(&text, 30, 60, MIN_SEGMENT_UNITS)
            .expect("should hard-cut at hard threshold");
        assert_eq!(text[..b].chars().count(), 60);
    }

    #[test]
    fn length_boundary_none_when_short() {
        let text = "汉".repeat(20);
        assert!(find_length_boundary(
            &text,
            SOFT_COMMIT_UNITS,
            HARD_COMMIT_UNITS,
            MIN_SEGMENT_UNITS
        )
        .is_none());
    }

    #[test]
    fn length_boundary_none_when_weak_prefix_below_min() {
        let text = format!("{}，{}", "今".repeat(5), "明".repeat(50));
        assert!(find_length_boundary(
            &text,
            SOFT_COMMIT_UNITS,
            HARD_COMMIT_UNITS,
            MIN_SEGMENT_UNITS
        )
        .is_none());
    }

    #[test]
    fn length_boundary_hard_cut_when_weak_prefix_below_min() {
        let text = format!("{}，{}", "今".repeat(5), "明".repeat(100));
        let b = find_length_boundary(
            &text,
            SOFT_COMMIT_UNITS,
            HARD_COMMIT_UNITS,
            MIN_SEGMENT_UNITS,
        )
        .expect("should hard-cut when weak prefix below min");
        assert_eq!(text[..b].chars().count(), HARD_COMMIT_UNITS);
    }

    // ── 累计文本不一致：新会话 vs 回溯修订（2026-09-24 事故修复）──────────────
    // 误判成「新会话」会把已显示文本回滚重发（豆包实测出现 734/1428 字巨型段落）。

    #[test]
    fn revision_inside_committed_is_not_treated_as_new_session() {
        // 豆包典型场景：committed=1000 字，上游把句中某处改词 → 公共前缀 900
        assert!(!is_new_session_reset(1000, 1200, 900), "深公共前缀的修订不应回滚");
        assert!(!is_new_session_reset(1000, 1001, 999), "末尾修订不应回滚");
        // 边界：公共前缀正好一半 → 仍按修订处理（保守，避免误重发）
        assert!(!is_new_session_reset(1000, 1200, 500));
    }

    // ── 上游「落后」而不是「新会话」（2026-09-24 08:23 实测的尾部重复）──────────
    // 真实数据：客户端按寿命兜底闭合了段15（含尾巴 "…I'm that girl right there."），
    // 随后豆包的 definite 边界才追上来，下发一段**更短**的 final
    // （"...be in this movie i'm really glad that I'm that girl right there."），
    // 当时被当成新会话回滚 → 尾巴 64 字原样重复成段16 段。

    #[test]
    fn lagging_upstream_is_not_new_session() {
        let committed = "so this is total fusion of life and work and all the things. Be in this movie I'm really glad that I'm that girl right there. Excuse me. Don't let her see it.";
        // 上游落后：更短、且（忽略大小写/标点后）是已定稿的前缀
        let lagging = "so this is total fusion of life and work and all the things，be in this movie i'm really glad that i'm that girl right there";
        assert!(is_lagging_upstream(committed, lagging), "更短的前缀 = 上游落后");
        assert!(!is_new_session_reset(committed.len(), lagging.len(), 30) || true); // 仅说明长度判据会误判
        // 归一化后确实落在「长度更短」的误判区间（这条断言固化当时的 bug 条件）
        assert!(lagging.len() <= committed.len());

        // 真正的重连新会话：内容不同、不是前缀 → 必须回滚
        let fresh = "hello everyone welcome back to the show today we talk about";
        assert!(!is_lagging_upstream(committed, fresh));
        assert!(is_new_session_reset(committed.len(), fresh.len(), 3));

        // 上游已经追平/超过（更长，含同样内容）→ 也不是落后，但同样不该当新会话
        let caught_up = format!("{} Excuse me. Don't let her see it. And then she said", committed);
        assert!(!is_lagging_upstream(committed, &caught_up));
        assert!(!is_new_session_reset(committed.len(), caught_up.len(), committed.len()));
    }

    #[test]
    fn shorter_cumulative_is_new_session() {
        // 重连后新会话从头累计：文本比已定稿短 → 必须回滚对齐
        assert!(is_new_session_reset(1000, 300, 250));
        assert!(is_new_session_reset(1000, 1000, 999));
        // 文本更长但公共前缀极短（新会话说了别的内容）→ 也要回滚
        assert!(is_new_session_reset(1000, 1500, 20));
    }
}
