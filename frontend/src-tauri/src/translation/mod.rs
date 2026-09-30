// translation/mod.rs
//
// Local translation module (OPUS-MT, zh ⇄ en).
// Provides:
// - lazy-loaded per-direction engines (`get_engine`)
// - the `translate_text` command for the translation page
// - realtime hooks: queue_translation + process_pending_translations, which
//   emit `translate-update` events consumed by the frontend transcript view.

pub mod commands;
pub mod custom_api;
pub mod engine;
pub mod llm;
pub mod remote;

use serde::Serialize;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, LazyLock, Mutex};
use tauri::{AppHandle, Emitter, Runtime};

use engine::OpusMtEngine;

pub const MODEL_DIR_ZH_EN: &str = "opus-mt-zh-en";
pub const MODEL_DIR_EN_ZH: &str = "opus-mt-en-zh";

/// Realtime inline translation master switch (default off).
pub static TRANSLATION_ENABLED: AtomicBool = AtomicBool::new(false);

/// Realtime translation target language: 13 种语言代码之一（default "en"，
/// 对应默认 home "zh" 的默认目标）。允许等于 HOME_LANG（此时源语言==目标语言
/// 的段落直接跳过/返回原文）。
pub(crate) static TARGET_LANG: LazyLock<Mutex<String>> =
    LazyLock::new(|| Mutex::new("en".to_string()));

/// Home 语言（用户的母语/主要工作语言），取值限 en/zh/ko/ja（default "zh"）。
/// 仅用于推导默认目标语言（home != "en" → "en"，home == "en" → "zh"），
/// 不参与实时方向解析。
pub(crate) static HOME_LANG: LazyLock<Mutex<String>> =
    LazyLock::new(|| Mutex::new("zh".to_string()));

/// 翻译引擎选择："opus"（OPUS-MT ort 引擎，默认）| "hymt2"（Hy-MT2 LLM 引擎）。
pub(crate) static TRANSLATION_ENGINE: LazyLock<Mutex<String>> =
    LazyLock::new(|| Mutex::new("opus".to_string()));

/// 当前翻译引擎 id（"opus" | "hymt2" | "remote" | "custom-api"）。
/// 返回用户显式选择的引擎（远程模型选择已下沉到各使用处，总开关不再强制覆盖）；
/// 选择了 "remote" 但远程总开关已关闭时，回落本地默认引擎 "opus"；
/// 选择了 "custom-api" 但 summary.api_config 未配置可用端点（openai/anthropic
/// 协议且 endpoint 非空）时，同样回落 "opus"。
pub fn current_engine() -> String {
    let engine = TRANSLATION_ENGINE
        .lock()
        .map(|e| e.clone())
        .unwrap_or_else(|_| "opus".to_string());
    if engine == "remote" && !crate::audio::transcription::remote_enabled() {
        return "opus".to_string();
    }
    if engine == "custom-api" && crate::summary::config::custom_api_config().is_none() {
        return "opus".to_string();
    }
    engine
}

/// 当前目标语言设置（13 种语言代码之一）。
pub fn target_lang() -> String {
    TARGET_LANG
        .lock()
        .map(|t| t.clone())
        .unwrap_or_else(|_| "en".to_string())
}

/// 当前 home 语言（en/zh/ko/ja）。
pub fn home_lang() -> String {
    HOME_LANG
        .lock()
        .map(|h| h.clone())
        .unwrap_or_else(|_| "zh".to_string())
}

/// home 语言对应的默认目标语言：home != "en" → "en"，home == "en" → "zh"。
pub fn default_target_for_home(home: &str) -> String {
    if home == "en" {
        "zh".to_string()
    } else {
        "en".to_string()
    }
}

// ── Engines (lazy-loaded) ─────────────────────────────────────────────────────

static ZH_EN_ENGINE: LazyLock<Mutex<Option<Arc<OpusMtEngine>>>> =
    LazyLock::new(|| Mutex::new(None));
static EN_ZH_ENGINE: LazyLock<Mutex<Option<Arc<OpusMtEngine>>>> =
    LazyLock::new(|| Mutex::new(None));

fn model_dir(name: &str) -> PathBuf {
    crate::sherpa_onnx_engine::commands::resolved_models_dir().join(name)
}

pub fn get_engine(direction: &str) -> Result<Arc<OpusMtEngine>, String> {
    let (slot, dir_name) = match direction {
        "zh-en" => (&ZH_EN_ENGINE, MODEL_DIR_ZH_EN),
        "en-zh" => (&EN_ZH_ENGINE, MODEL_DIR_EN_ZH),
        other => return Err(format!("不支持的翻译方向: {}", other)),
    };

    let mut guard = slot.lock().map_err(|e| e.to_string())?;
    if let Some(engine) = guard.as_ref() {
        return Ok(engine.clone());
    }

    let dir = model_dir(dir_name);
    crate::llama_sidecar::emit_model_loading(dir_name, "start", None, None);
    let start = std::time::Instant::now();
    let engine = match OpusMtEngine::load(&dir) {
        Ok(engine) => engine,
        Err(e) => {
            let msg = e.to_string();
            crate::llama_sidecar::emit_model_loading(dir_name, "error", None, Some(msg.clone()));
            return Err(msg);
        }
    };
    crate::llama_sidecar::emit_model_loading(
        dir_name,
        "done",
        Some(start.elapsed().as_millis() as u64),
        None,
    );
    let engine = Arc::new(engine);
    *guard = Some(engine.clone());
    log::info!(
        "Translation engine ready: {} ({})",
        direction,
        dir.display()
    );
    Ok(engine)
}

/// Unload both OPUS-MT direction engines, freeing their memory (called when
/// switching to a different translation engine). 实际卸载过时向前端发一次
/// model-loading unloaded 事件（两个方向共用一条 "opus-mt" 提示）。
pub fn unload_opus_engines() {
    let mut any = false;
    for (slot, direction) in [(&ZH_EN_ENGINE, "zh-en"), (&EN_ZH_ENGINE, "en-zh")] {
        if let Ok(mut guard) = slot.lock() {
            if guard.take().is_some() {
                any = true;
                log::info!("OPUS-MT 引擎已卸载 ({})，内存已释放", direction);
            }
        }
    }
    if any {
        crate::llama_sidecar::emit_model_unloaded("opus-mt", "manual");
    }
}

/// Whether the given direction's model files are present on disk.
pub fn is_model_installed(direction: &str) -> bool {
    let dir_name = match direction {
        "zh-en" => MODEL_DIR_ZH_EN,
        "en-zh" => MODEL_DIR_EN_ZH,
        _ => return false,
    };
    let dir = model_dir(dir_name);
    engine::REQUIRED_FILES.iter().all(|f| dir.join(f).exists())
}

// ── Language detection (CJK share heuristic) ──────────────────────────────────

/// Rough zh/en detection: true when CJK characters make up > 30% of the text.
pub fn is_chinese_dominant(text: &str) -> bool {
    let total = text.chars().filter(|c| !c.is_whitespace()).count();
    if total == 0 {
        return false;
    }
    let cjk = text
        .chars()
        .filter(|c| {
            let u = *c as u32;
            (0x4E00..=0x9FFF).contains(&u) || (0x3400..=0x4DBF).contains(&u)
        })
        .count();
    cjk * 10 > total * 3
}

/// 粗略源语言检测：含 hangul（0xAC00-0xD7AF、0x1100-0x11FF）→ "ko"，
/// 含假名（0x3040-0x30FF）→ "ja"，汉字占比 > 30% → "zh"，否则 → "en"。
pub fn detect_source_lang(text: &str) -> &'static str {
    let mut has_hangul = false;
    let mut has_kana = false;
    for c in text.chars() {
        let u = c as u32;
        if (0xAC00..=0xD7AF).contains(&u) || (0x1100..=0x11FF).contains(&u) {
            has_hangul = true;
        } else if (0x3040..=0x30FF).contains(&u) {
            has_kana = true;
        }
        if has_hangul && has_kana {
            break;
        }
    }
    if has_hangul {
        "ko"
    } else if has_kana {
        "ja"
    } else if is_chinese_dominant(text) {
        "zh"
    } else {
        "en"
    }
}

// ── Realtime translation queue ────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct TranslateTask {
    text: String,
    sequence_id: u64,
    /// 前几句原文（流式转写的翻译上下文，仅 LLM 引擎使用；OPUS-MT 忽略）。
    context: Option<String>,
}

static TRANSLATE_QUEUE: LazyLock<Mutex<VecDeque<TranslateTask>>> =
    LazyLock::new(|| Mutex::new(VecDeque::new()));

/// 已入队翻译的段落 sequence_id 集合：开启翻译时据此补译未入队的已提交段落。
static TRANSLATE_SEEN: LazyLock<Mutex<HashSet<u64>>> = LazyLock::new(|| Mutex::new(HashSet::new()));

/// 段落最终译文表（sequence_id → 定稿译文）：停止录音时随段落持久化到
/// 历史库与 transcripts.json。只有定稿（is_partial=false）且非空的译文会进来。
static FINAL_TRANSLATIONS: LazyLock<Mutex<HashMap<u64, String>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// 取某段落的最终译文（持久化路径用）。
pub fn final_translation(sequence_id: u64) -> Option<String> {
    FINAL_TRANSLATIONS
        .lock()
        .ok()
        .and_then(|m| m.get(&sequence_id).cloned())
}

/// 停止录音后的补译清单（2026-09-27，B2）：返回本次录音段落中
/// 「本应翻译但定稿译文缺失」的 (sequence_id, text)。
/// 「本应翻译」的判定与 queue_translation 入队时完全一致（同一 resolve_direction
/// + 非空 trim），所以源==目标 / 目标语言不支持的段落不会被误报为缺失。
/// 为什么需要：定稿翻译失败后旧实现只打 WARN、不重试不回队，FINAL_TRANSLATIONS
/// 里永远没有这些 seq（2026-09-26 晚一次 20 分钟录音丢了 27 段定稿译文）。
pub fn missing_final_translations(segments: &[(u64, String)]) -> Vec<(u64, String)> {
    if !TRANSLATION_ENABLED.load(Ordering::SeqCst) {
        return Vec::new();
    }
    let target = target_lang();
    let finals: HashSet<u64> = FINAL_TRANSLATIONS
        .lock()
        .map(|m| m.keys().copied().collect())
        .unwrap_or_default();
    segments
        .iter()
        .filter(|(seq, text)| {
            let text = text.trim();
            !text.is_empty()
                && !finals.contains(seq)
                && resolve_direction(text, &target).is_some()
        })
        .map(|(seq, text)| (*seq, text.clone()))
        .collect()
}

/// 等待翻译队列排空：每 200ms 轮询，直到「队列空 且 worker 未在跑」或超时。
/// 队列空但 RUNNING=true 的组合正好覆盖「正在处理最后一个任务」的在飞情况。
/// 返回是否真正排空（false = 超时放弃，尾部译文可能缺失）。
pub async fn drain_pending_translations(max_wait: std::time::Duration) -> bool {
    let deadline = std::time::Instant::now() + max_wait;
    loop {
        let queue_empty = TRANSLATE_QUEUE
            .lock()
            .map(|q| q.is_empty())
            .unwrap_or(true);
        if queue_empty && !FINAL_WORKER_RUNNING.load(Ordering::SeqCst) {
            return true;
        }
        if std::time::Instant::now() >= deadline {
            return false;
        }
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }
}

/// 新录音开始（sequence 重置）：清空待译队列、已见集合与最终译文表，并作废在飞草稿。
pub fn reset_translation_session() {
    if let Ok(mut q) = TRANSLATE_QUEUE.lock() {
        q.clear();
    }
    if let Ok(mut seen) = TRANSLATE_SEEN.lock() {
        seen.clear();
    }
    if let Ok(mut m) = FINAL_TRANSLATIONS.lock() {
        m.clear();
    }
    invalidate_translation_drafts();
}

/// 该 sequence_id 是否已入队过翻译（用于补译去重）。
pub fn translation_seen(sequence_id: u64) -> bool {
    TRANSLATE_SEEN
        .lock()
        .map(|s| s.contains(&sequence_id))
        .unwrap_or(false)
}

#[derive(Debug, Clone, Serialize)]
pub struct TranslateUpdate {
    pub sequence_id: u64,
    pub original_text: String,
    pub translated_text: String,
    pub source_lang: String,
    pub target_lang: String,
    pub is_partial: bool,
}

/// 根据目标语言设置与当前引擎解析翻译方向。
/// hymt2 路径规则：源语言==目标语言 → 返回 None（跳过，无需翻译）；
/// 否则译成目标语言（home 语言不参与方向解析）。
/// 返回 (direction, source_lang, effective_target)；目标语言不被当前引擎
/// 支持时也返回 None（跳过）。
fn resolve_direction(text: &str, target: &str) -> Option<(String, String, String)> {
    if matches!(current_engine().as_str(), "hymt2" | "remote" | "custom-api") {
        // LLM 引擎（Hy-MT2 / 远程网关 / 自定义 API）：13 种语言互译，源语言按文本特征检测
        let source_lang = detect_source_lang(text);
        // 兼容存量 "auto"：按 home 的默认目标解析
        let effective_target: String = if target == "auto" {
            default_target_for_home(&home_lang())
        } else {
            target.to_string()
        };
        if source_lang == effective_target
            || !llm::SUPPORTED_TARGET_LANGS.contains(&effective_target.as_str())
        {
            return None;
        }
        Some((
            format!("{}-{}", source_lang, effective_target),
            source_lang.to_string(),
            effective_target,
        ))
    } else {
        // OPUS-MT 引擎：仅 zh ⇄ en
        if !matches!(target, "auto" | "zh" | "en") {
            log::warn!("OPUS-MT 引擎不支持目标语言 {}，跳过翻译", target);
            return None;
        }
        let source_is_zh = is_chinese_dominant(text);
        let (direction, source_lang) = if source_is_zh {
            ("zh-en", "zh")
        } else {
            ("en-zh", "en")
        };
        let effective_target: &str = if target == "auto" {
            if source_is_zh {
                "en"
            } else {
                "zh"
            }
        } else {
            target
        };
        if source_lang == effective_target {
            None
        } else {
            Some((
                direction.to_string(),
                source_lang.to_string(),
                effective_target.to_string(),
            ))
        }
    }
}

/// Queue a committed transcript segment for translation (no-op when disabled).
/// 方向按 resolve_direction 的 home 规则解析；解析为 None 时跳过。
/// 入队后自动触发单例 worker（不阻塞调用方）。
pub fn queue_translation<R: Runtime>(app: &AppHandle<R>, text: &str, sequence_id: u64) {
    queue_translation_with_context(app, text, sequence_id, None);
}

/// 带上下文入队：context 为前几句原文（流式转写管线提供），随任务进入 prompt。
pub fn queue_translation_with_context<R: Runtime>(
    app: &AppHandle<R>,
    text: &str,
    sequence_id: u64,
    context: Option<String>,
) {
    if !TRANSLATION_ENABLED.load(Ordering::SeqCst) {
        return;
    }
    let text = text.trim().to_string();
    if text.is_empty() {
        return;
    }
    let target = TARGET_LANG
        .lock()
        .map(|t| t.clone())
        .unwrap_or_else(|_| "en".to_string());
    if resolve_direction(&text, &target).is_none() {
        return;
    }
    if let Ok(mut q) = TRANSLATE_QUEUE.lock() {
        q.push_back(TranslateTask {
            text,
            sequence_id,
            context,
        });
    }
    if let Ok(mut seen) = TRANSLATE_SEEN.lock() {
        seen.insert(sequence_id);
    }
    kick_translation_worker(app);
}

// ── 草稿翻译（同传式）：活跃单元尾部的试译，latest-wins，不占定稿 FIFO 队列 ──

#[derive(Debug, Clone)]
struct DraftTask {
    text: String,
    sequence_id: u64,
    context: Option<String>,
    generation: u64,
}

static DRAFT_LATEST: LazyLock<Mutex<Option<DraftTask>>> = LazyLock::new(|| Mutex::new(None));
static DRAFT_GENERATION: AtomicU64 = AtomicU64::new(0);
static DRAFT_WORKER_RUNNING: AtomicBool = AtomicBool::new(false);

/// 活跃单元尾部的草稿翻译：新草稿取代旧草稿（只翻最新尾巴），结果以
/// translate-update(is_partial=true) 原位刷新。
///
/// 2026-09-22 起**所有引擎都做草稿**（此前 opus 被跳过，理由是「逐句够快」）。
/// 为什么改：草稿是「译文不出现」的正解 —— 它让译文跟着**当前活跃单元**实时刷新，
/// 而不必等客户端把半句冻结成段（那正是「分段碎/句子中间断开/译文对不上」的来源）。
/// opus 是本地小模型，草稿成本只是 CPU；远程/LLM 引擎本来就有草稿，节流见
/// DRAFT_MIN_GROWTH_UNITS / DRAFT_MIN_INTERVAL（≥8 单位或 ≥600ms）。
pub fn queue_translation_draft<R: Runtime>(
    app: &AppHandle<R>,
    text: &str,
    sequence_id: u64,
    context: Option<String>,
) {
    if !TRANSLATION_ENABLED.load(Ordering::SeqCst) {
        return;
    }
    let text = text.trim().to_string();
    if text.is_empty() {
        return;
    }
    let target = TARGET_LANG
        .lock()
        .map(|t| t.clone())
        .unwrap_or_else(|_| "en".to_string());
    if resolve_direction(&text, &target).is_none() {
        return;
    }
    let generation = DRAFT_GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
    if let Ok(mut d) = DRAFT_LATEST.lock() {
        *d = Some(DraftTask {
            text,
            sequence_id,
            context,
            generation,
        });
    }
    kick_draft_worker(app);
}

/// 作废全部在飞/待译草稿（单元闭合、录音重置时调用）：此后旧代草稿的结果
/// 一律丢弃，避免覆盖随后到达的定稿译文。
pub fn invalidate_translation_drafts() {
    DRAFT_GENERATION.fetch_add(1, Ordering::SeqCst);
    if let Ok(mut d) = DRAFT_LATEST.lock() {
        *d = None;
    }
}

fn kick_draft_worker<R: Runtime>(app: &AppHandle<R>) {
    if DRAFT_WORKER_RUNNING.swap(true, Ordering::SeqCst) {
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            let task = DRAFT_LATEST.lock().ok().and_then(|mut g| g.take());
            match task {
                Some(t) => process_draft(app.clone(), t).await,
                None => {
                    DRAFT_WORKER_RUNNING.store(false, Ordering::SeqCst);
                    // 释放标记前再查一次，避免与新入队任务竞态
                    let again = DRAFT_LATEST.lock().map(|g| g.is_some()).unwrap_or(false);
                    if !again {
                        return;
                    }
                    // 有新任务但 kick 可能已看到 RUNNING=true 而跳过：自己重新占位
                    if DRAFT_WORKER_RUNNING.swap(true, Ordering::SeqCst) {
                        return; // 另一个 worker 已启动，让位
                    }
                }
            }
        }
    });
}

/// 执行一次草稿翻译：只发 is_partial=true 的增量，永不发定稿；生成代过期即
/// 停止 emit（远程 SSE 无法中断，静默丢弃后续增量）。
async fn process_draft<R: Runtime>(app: AppHandle<R>, task: DraftTask) {
    let target = TARGET_LANG
        .lock()
        .map(|t| t.clone())
        .unwrap_or_else(|_| "en".to_string());
    let Some((direction, source_lang, effective_target)) = resolve_direction(&task.text, &target)
    else {
        return;
    };
    let generation = task.generation;
    let is_stale = move || DRAFT_GENERATION.load(Ordering::SeqCst) != generation;

    let seq = task.sequence_id;
    let original = task.text.clone();
    let engine_kind = current_engine();
    let context = task.context.clone();

    let mut partial = String::new();
    let mut tokens_since_emit = 0usize;
    let mut last_emit = std::time::Instant::now();

    match engine_kind.as_str() {
        "remote" => {
            let app_stream = app.clone();
            let original = original.clone();
            let src_lang = source_lang.clone();
            let tgt_lang = effective_target.clone();
            let stale = is_stale.clone();
            let _ = remote::translate_remote(
                &task.text,
                &direction,
                true,
                context.as_deref(),
                Some(&mut |delta: &str| {
                    if stale() {
                        return;
                    }
                    partial.push_str(delta);
                    tokens_since_emit += 1;
                    if tokens_since_emit >= 8
                        || last_emit.elapsed() >= std::time::Duration::from_millis(250)
                    {
                        tokens_since_emit = 0;
                        last_emit = std::time::Instant::now();
                        let update = TranslateUpdate {
                            sequence_id: seq,
                            original_text: original.clone(),
                            translated_text: partial.clone(),
                            source_lang: src_lang.clone(),
                            target_lang: tgt_lang.clone(),
                            is_partial: true,
                        };
                        let _ = app_stream.emit("translate-update", &update);
                    }
                }),
                // 草稿不重试（max_retries=0）：它很快会被新一代草稿取代，重试是白花钱；
                // 但同样要经过 remote.rs 的网关并发信号量。
                0,
            )
            .await;
        }
        "custom-api" => {
            if let Some(config) = crate::summary::config::custom_api_config() {
                let app_stream = app.clone();
                let original = original.clone();
                let src_lang = source_lang.clone();
                let tgt_lang = effective_target.clone();
                let stale = is_stale.clone();
                let _ = custom_api::translate_custom_api(
                    &config,
                    &task.text,
                    &direction,
                    true,
                    context.as_deref(),
                    Some(&mut |delta: &str| {
                        if stale() {
                            return;
                        }
                        partial.push_str(delta);
                        tokens_since_emit += 1;
                        if tokens_since_emit >= 8
                            || last_emit.elapsed() >= std::time::Duration::from_millis(250)
                        {
                            tokens_since_emit = 0;
                            last_emit = std::time::Instant::now();
                            let update = TranslateUpdate {
                                sequence_id: seq,
                                original_text: original.clone(),
                                translated_text: partial.clone(),
                                source_lang: src_lang.clone(),
                                target_lang: tgt_lang.clone(),
                                is_partial: true,
                            };
                            let _ = app_stream.emit("translate-update", &update);
                        }
                    }),
                )
                .await;
            }
        }
        "hymt2" => {
            let app_stream = app.clone();
            let original = original.clone();
            let src_lang = source_lang.clone();
            let tgt_lang = effective_target.clone();
            let stale = is_stale.clone();
            let text_b = task.text.clone();
            let direction_b = direction.clone();
            let context_b = context.clone();
            let _ = tokio::task::spawn_blocking(move || {
                llm::translate(
                    &text_b,
                    &direction_b,
                    true,
                    context_b.as_deref(),
                    Some(&mut |delta: &str| {
                        if stale() {
                            return;
                        }
                        partial.push_str(delta);
                        tokens_since_emit += 1;
                        if tokens_since_emit >= 8
                            || last_emit.elapsed() >= std::time::Duration::from_millis(250)
                        {
                            tokens_since_emit = 0;
                            last_emit = std::time::Instant::now();
                            let update = TranslateUpdate {
                                sequence_id: seq,
                                original_text: original.clone(),
                                translated_text: partial.clone(),
                                source_lang: src_lang.clone(),
                                target_lang: tgt_lang.clone(),
                                is_partial: true,
                            };
                            let _ = app_stream.emit("translate-update", &update);
                        }
                    }),
                )
            })
            .await;
        }
        _ => {}
    }
}

/// 空译文告警的节流（60s 内只提示一次）：翻译是逐句调用的，一次故障会连续空很多条。
static LAST_TRANSLATION_WARNING_SECS: AtomicU64 = AtomicU64::new(0);

/// 发一条「翻译告警」给前端（60s 节流）。前端在 AppShell 全局监听并 toast。
fn emit_translation_warning_throttled<R: Runtime>(app: &AppHandle<R>, message: &str) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let last = LAST_TRANSLATION_WARNING_SECS.load(Ordering::Relaxed);
    if now.saturating_sub(last) < 60 {
        return;
    }
    LAST_TRANSLATION_WARNING_SECS.store(now, Ordering::Relaxed);
    let _ = app.emit(
        "translation-warning",
        serde_json::json!({ "message": message }),
    );
}

/// 最终译文单例 worker：队列串行消费，fire-and-forget，不阻塞调用方
/// （修复长段 beam 翻译串行阻塞 X-ASR 轮询循环的问题）。
static FINAL_WORKER_RUNNING: AtomicBool = AtomicBool::new(false);

/// 触发最终译文 worker（幂等）：已有 worker 在跑时直接返回。
pub fn kick_translation_worker<R: Runtime>(app: &AppHandle<R>) {
    if !TRANSLATION_ENABLED.load(Ordering::SeqCst) {
        return;
    }
    if FINAL_WORKER_RUNNING.swap(true, Ordering::SeqCst) {
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            process_pending_translations(app.clone()).await;
            // 释放标记前再查队列，避免与新入队任务竞态
            let empty = TRANSLATE_QUEUE.lock().map(|q| q.is_empty()).unwrap_or(true);
            if empty {
                FINAL_WORKER_RUNNING.store(false, Ordering::SeqCst);
                return;
            }
        }
    });
}

/// Drain the queue, translate each task and emit `translate-update` events.
/// Blocking inference runs inside; call from async contexts (it spawns the
/// blocking work on the tokio blocking pool).
pub async fn process_pending_translations<R: Runtime>(app: AppHandle<R>) {
    if !TRANSLATION_ENABLED.load(Ordering::SeqCst) {
        if let Ok(mut q) = TRANSLATE_QUEUE.lock() {
            q.clear();
        }
        return;
    }

    let target = TARGET_LANG
        .lock()
        .map(|t| t.clone())
        .unwrap_or_else(|_| "en".to_string());

    loop {
        let task = match TRANSLATE_QUEUE.lock() {
            Ok(mut q) => q.pop_front(),
            Err(_) => None,
        };
        let Some(task) = task else { break };

        let Some((direction, source_lang, effective_target)) =
            resolve_direction(&task.text, &target)
        else {
            // 以前这里是静默 continue：用户「没有译文」时日志里完全看不出是「方向解析为
            // 同语种/不支持」还是「引擎失败」。补一行 INFO 让排查有据（2026-09-22）。
            log::info!(
                "翻译跳过 seq={}（目标 {} / 引擎 {} 判定无需翻译或目标语言不支持）: {}",
                task.sequence_id,
                target,
                current_engine(),
                task.text.chars().take(30).collect::<String>()
            );
            continue;
        };

        let text = task.text.clone();
        let seq = task.sequence_id;
        let engine_kind = current_engine();
        let is_remote = engine_kind == "remote";
        let is_custom_api = engine_kind == "custom-api";
        let is_hymt2 = engine_kind == "hymt2";
        let direction_for_task = direction.clone();
        let original_for_task = task.text.clone();
        let source_lang_task = source_lang.clone();
        let target_lang_task = effective_target.clone();
        let context_for_task = task.context.clone();

        let mut emptied_by_cleanup = false;
        let result: Result<String, String> = if is_remote {
            // 远程引擎：走网关 chat completions（SSE 流式，异步）
            let app_stream = app.clone();
            let original = original_for_task.clone();
            let src_lang = source_lang_task.clone();
            let tgt_lang = target_lang_task.clone();
            let mut partial = String::new();
            let mut tokens_since_emit = 0usize;
            let mut last_emit = std::time::Instant::now();
            remote::translate_remote(
                &text,
                &direction_for_task,
                true,
                context_for_task.as_deref(),
                Some(&mut |delta: &str| {
                    partial.push_str(delta);
                    tokens_since_emit += 1;
                    if tokens_since_emit >= 8
                        || last_emit.elapsed() >= std::time::Duration::from_millis(250)
                    {
                        tokens_since_emit = 0;
                        last_emit = std::time::Instant::now();
                        let update = TranslateUpdate {
                            sequence_id: seq,
                            original_text: original.clone(),
                            translated_text: partial.clone(),
                            source_lang: src_lang.clone(),
                            target_lang: tgt_lang.clone(),
                            is_partial: true,
                        };
                        if let Err(e) = app_stream.emit("translate-update", &update) {
                            log::warn!("translate-update (partial) emit failed: {}", e);
                        }
                    }
                }),
                // 定稿翻译对可重试错误（429 / 5xx / 网络错误）重试 3 次（1s/2s/4s 退避，
                // 复用同一 x-vox-request-id，见 remote.rs）。此前 429 直接进 WARN 不重试，
                // 2026-09-26 晚一次 20 分钟录音因此永久丢失 27 段的定稿译文。
                // 注意：若失败发生在流式中途，重试会让上面的 partial 累积从头发送，
                // 重试期间的 is_partial 增量可能短暂重复，定稿（is_partial=false）不受影响。
                3,
            )
            .await
            .map(|r| {
                emptied_by_cleanup = r.emptied_by_cleanup;
                r.text
            })
        } else if is_custom_api {
            // 自定义 API 引擎：走用户配置的 OpenAI 兼容 / Anthropic 端点（SSE 流式，异步）
            match crate::summary::config::custom_api_config() {
                Some(config) => {
                    let app_stream = app.clone();
                    let original = original_for_task.clone();
                    let src_lang = source_lang_task.clone();
                    let tgt_lang = target_lang_task.clone();
                    let mut partial = String::new();
                    let mut tokens_since_emit = 0usize;
                    let mut last_emit = std::time::Instant::now();
                    custom_api::translate_custom_api(
                        &config,
                        &text,
                        &direction_for_task,
                        true,
                        context_for_task.as_deref(),
                        Some(&mut |delta: &str| {
                            partial.push_str(delta);
                            tokens_since_emit += 1;
                            if tokens_since_emit >= 8
                                || last_emit.elapsed() >= std::time::Duration::from_millis(250)
                            {
                                tokens_since_emit = 0;
                                last_emit = std::time::Instant::now();
                                let update = TranslateUpdate {
                                    sequence_id: seq,
                                    original_text: original.clone(),
                                    translated_text: partial.clone(),
                                    source_lang: src_lang.clone(),
                                    target_lang: tgt_lang.clone(),
                                    is_partial: true,
                                };
                                if let Err(e) = app_stream.emit("translate-update", &update) {
                                    log::warn!("translate-update (partial) emit failed: {}", e);
                                }
                            }
                        }),
                    )
                    .await
                }
                None => {
                    Err("自定义 API 未配置，请先在会议总结的 API 设置中填写端点地址。".to_string())
                }
            }
        } else {
            let app_stream = app.clone();
            let original = original_for_task.clone();
            let src_lang = source_lang_task.clone();
            let tgt_lang = target_lang_task.clone();
            let text_for_blocking = text.clone();
            let direction_for_blocking = direction_for_task.clone();
            let context_for_blocking = context_for_task.clone();
            tokio::task::spawn_blocking(move || {
                if is_hymt2 {
                    // Hy-MT2 LLM 引擎：走 llama-helper sidecar，ASR 模式指令；
                    // 流式生成，节流 emit 部分译文（原始输出快照，未清洗）
                    let mut partial = String::new();
                    let mut tokens_since_emit = 0usize;
                    let mut last_emit = std::time::Instant::now();
                    llm::translate(
                        &text_for_blocking,
                        &direction_for_blocking,
                        true,
                        context_for_blocking.as_deref(),
                        Some(&mut |delta: &str| {
                            partial.push_str(delta);
                            tokens_since_emit += 1;
                            if tokens_since_emit >= 8
                                || last_emit.elapsed() >= std::time::Duration::from_millis(250)
                            {
                                tokens_since_emit = 0;
                                last_emit = std::time::Instant::now();
                                let update = TranslateUpdate {
                                    sequence_id: seq,
                                    original_text: original.clone(),
                                    translated_text: partial.clone(),
                                    source_lang: src_lang.clone(),
                                    target_lang: tgt_lang.clone(),
                                    is_partial: true,
                                };
                                if let Err(e) = app_stream.emit("translate-update", &update) {
                                    log::warn!("translate-update (partial) emit failed: {}", e);
                                }
                            }
                        }),
                    )
                } else {
                    get_engine(&direction_for_blocking).and_then(|engine| {
                        // 实时路径用贪心解码：句级输入质量已足够，速度优先（~0.3-1s/句）
                        let first = engine
                            .translate_greedy(&text_for_blocking)
                            .map_err(|e| e.to_string());
                        // 空译文重试一次（2026-09-24）：上游偶发退化响应（实测一次
                        // `qwen-flash` 只回 14 个 token、正文全空且没有 reasoning_tokens；
                        // 网关侧现在对空正文免单）。重试一次基本能恢复，避免用户看到空白译文。
                        if matches!(&first, Ok(t) if t.trim().is_empty()) {
                            log::warn!(
                                "翻译返回空结果，重试一次 seq={} 输入 {} 字",
                                seq,
                                text_for_blocking.chars().count()
                            );
                            let retry = engine
                                .translate_greedy(&text_for_blocking)
                                .map_err(|e| e.to_string());
                            return match retry {
                                Ok(t) if !t.trim().is_empty() => Ok(t),
                                _ => first,
                            };
                        }
                        first
                    })
                }
            })
            .await
            .unwrap_or_else(|e| Err(format!("翻译任务失败: {}", e)))
        };

        match result {
            Ok(translated) if !translated.trim().is_empty() => {
                // 成功也记一行 INFO：此前只有失败有日志，「到底翻没翻」在 App 日志里
                // 无从判断（2026-09-22 排查「实时转录没有译文」时的盲区）。
                let engine = current_engine();
                log::info!(
                    "✅ 翻译完成 seq={} {}→{} 引擎={} {} 字 → {} 字",
                    seq,
                    source_lang,
                    effective_target,
                    engine,
                    task.text.chars().count(),
                    translated.chars().count()
                );
                // 定稿译文落表：停止录音时随段落持久化（历史库 + transcripts.json）
                if let Ok(mut m) = FINAL_TRANSLATIONS.lock() {
                    m.insert(seq, translated.clone());
                }
                let update = TranslateUpdate {
                    sequence_id: seq,
                    original_text: task.text.clone(),
                    translated_text: translated,
                    source_lang,
                    target_lang: effective_target,
                    is_partial: false,
                };
                if let Err(e) = app.emit("translate-update", &update) {
                    log::warn!("translate-update emit failed: {}", e);
                }
            }
            Ok(translated) => {
                // 空译文按**失败**处理（2026-09-28：此前空结果照样打「✅ 翻译完成 0 字」，
                // seq=44 那次的日志看起来像是成功了，实际译文永久丢失）。
                // 不计入翻译完成、不落 FINAL_TRANSLATIONS（→ 停止录音时的补译循环会重新入队）、
                // 不发空定稿事件（前端虽有空串守卫，但既然按失败处理就不该发「成功形态」的事件；
                // 已有的非空草稿因此也不会被空结果覆盖）。
                // 两种「空」要分开说（2026-09-28 生产实测教训）：
                //   ① 上游有产出但全是回声/非目标语言，被清洗清空（qwen-flash 对句中
                //      截断的长句偶发英文续写）——那次调用是正常扣费的；
                //   ② 上游真的没产出（推理预算吃光/上游异常，网关侧免单）。
                // 告警文案必须带**实际**模型名——旧文案硬编码「如 MiMo」举例，
                // 用户看到时以为 MiMo 报错，但当时在用的其实是 qwen-flash。
                let _ = translated;
                let engine = current_engine();
                // 远程引擎报实际远程模型名；本地/自定义引擎报引擎名（否则会用无关的远程选择误导）
                let model = if engine == "remote" {
                    let m = crate::audio::transcription::get_remote_translate_model();
                    if m.is_empty() { "remote".to_string() } else { m }
                } else {
                    engine.clone()
                };
                let cause = if emptied_by_cleanup {
                    "上游返回的不是译文（原文回声/非目标语言输出，已被过滤）"
                } else {
                    "上游未产出内容（推理预算不足或上游异常）"
                };
                log::warn!(
                    "⚠️ 翻译返回空结果 seq={} 引擎={} 模型={}（输入 {} 字）——{}",
                    seq, engine, model, task.text.chars().count(), cause
                );
                emit_translation_warning_throttled(
                    &app,
                    &format!(
                        "翻译模型 {} 返回了空结果（输入 {} 字，{}）。若反复出现，请换用其它翻译模型。",
                        model,
                        task.text.chars().count(),
                        cause
                    ),
                );
            }
            Err(e) => {
                log::warn!("Translation failed for seq={}: {}", seq, e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_source_lang_hangul_is_ko() {
        assert_eq!(detect_source_lang("안녕하세요, hello"), "ko");
        // 韩文音节块之外的 hangul jamo 也算
        assert_eq!(detect_source_lang("한국어"), "ko");
    }

    #[test]
    fn detect_source_lang_kana_is_ja() {
        assert_eq!(detect_source_lang("これはテストです"), "ja");
        assert_eq!(detect_source_lang("カタカナ"), "ja");
    }

    #[test]
    fn detect_source_lang_hanzi_dominant_is_zh() {
        assert_eq!(detect_source_lang("今天天气真不错，我们出去走走"), "zh");
    }

    #[test]
    fn detect_source_lang_latin_is_en() {
        assert_eq!(detect_source_lang("Hello, this is a test."), "en");
        assert_eq!(detect_source_lang("Bonjour le monde"), "en");
        // 汉字占比不足 30% 时回落 en
        assert_eq!(detect_source_lang("abcdefgh 中"), "en");
    }

    // resolve_direction 测试需要改全局静态（引擎/home），用互斥锁串行化，
    // 避免用例间互相干扰。
    static RESOLVE_TEST_LOCK: Mutex<()> = Mutex::new(());

    fn with_engine_home<T>(engine: &str, home: &str, f: impl FnOnce() -> T) -> T {
        let _guard = RESOLVE_TEST_LOCK.lock().unwrap();
        let saved_engine = current_engine();
        let saved_home = home_lang();
        *TRANSLATION_ENGINE.lock().unwrap() = engine.to_string();
        *HOME_LANG.lock().unwrap() = home.to_string();
        let result = f();
        *TRANSLATION_ENGINE.lock().unwrap() = saved_engine;
        *HOME_LANG.lock().unwrap() = saved_home;
        result
    }

    #[test]
    fn resolve_direction_source_equals_target_skips() {
        // target=en，英文输入：源==目标 → None（跳过，无需翻译）
        let r = with_engine_home("hymt2", "zh", || {
            resolve_direction("Hello, this is a test.", "en")
        });
        assert_eq!(r, None);
        // target=zh，中文输入：同样跳过
        let r = with_engine_home("hymt2", "zh", || {
            resolve_direction("今天天气真不错，我们出去走走", "zh")
        });
        assert_eq!(r, None);
    }

    #[test]
    fn resolve_direction_translates_to_target() {
        // target=en，中文/日语输入 → 译成目标语言（home 不参与）
        let r = with_engine_home("hymt2", "zh", || {
            resolve_direction("今天天气真不错，我们出去走走", "en")
        });
        assert_eq!(
            r,
            Some(("zh-en".to_string(), "zh".to_string(), "en".to_string()))
        );
        let r = with_engine_home("hymt2", "zh", || {
            resolve_direction("これはテストです", "en")
        });
        assert_eq!(
            r,
            Some(("ja-en".to_string(), "ja".to_string(), "en".to_string()))
        );
        // home=en 时规则不变：target=zh，英文输入 → en-zh
        let r = with_engine_home("hymt2", "en", || {
            resolve_direction("Hello, this is a test.", "zh")
        });
        assert_eq!(
            r,
            Some(("en-zh".to_string(), "en".to_string(), "zh".to_string()))
        );
    }

    #[test]
    fn resolve_direction_legacy_auto_uses_default_target() {
        // 存量 "auto" 按 home 的默认目标解析（home=zh → 默认目标 en）
        let r = with_engine_home("hymt2", "zh", || {
            resolve_direction("Hello, this is a test.", "auto")
        });
        assert_eq!(r, None); // 源 en == 默认目标 en → 跳过
        let r = with_engine_home("hymt2", "zh", || {
            resolve_direction("今天天气真不错，我们出去走走", "auto")
        });
        assert_eq!(
            r,
            Some(("zh-en".to_string(), "zh".to_string(), "en".to_string()))
        );
        // home=en → 默认目标 zh；中文输入源==目标 → 跳过，英文输入 → en-zh
        let r = with_engine_home("hymt2", "en", || {
            resolve_direction("今天天气真不错，我们出去走走", "auto")
        });
        assert_eq!(r, None);
        let r = with_engine_home("hymt2", "en", || {
            resolve_direction("Hello, this is a test.", "auto")
        });
        assert_eq!(
            r,
            Some(("en-zh".to_string(), "en".to_string(), "zh".to_string()))
        );
    }

    #[test]
    fn default_target_for_home_rules() {
        assert_eq!(default_target_for_home("zh"), "en");
        assert_eq!(default_target_for_home("ja"), "en");
        assert_eq!(default_target_for_home("ko"), "en");
        assert_eq!(default_target_for_home("en"), "zh");
    }
}
