// Retranscription module - allows re-processing stored audio with Sherpa-ONNX (SenseVoice)

use super::common::create_transcript_segments;
use super::constants::AUDIO_EXTENSIONS;
use crate::audio::audio_processing::audio_to_mono;
use crate::audio::decoder::{decode_audio_file, probe_audio_duration};
use crate::audio::vad::get_speech_chunks_with_config;
use crate::state::AppState;
use anyhow::{anyhow, Result};
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tauri::{AppHandle, Emitter, Manager, Runtime};

/// Global flag to track if retranscription is in progress
static RETRANSCRIPTION_IN_PROGRESS: AtomicBool = AtomicBool::new(false);

/// Global flag to signal cancellation
static RETRANSCRIPTION_CANCELLED: AtomicBool = AtomicBool::new(false);

/// 取消哨兵：所有「用户点了停止识别」的错误文本统一用它，
/// `start_retranscription` 据此发 `retranscription-cancelled` 事件（而不是 error），
/// 前端显示「已停止识别」而不是红色失败提示。
pub const CANCELLED_SENTINEL: &str = "Retranscription cancelled";

fn cancelled_error() -> anyhow::Error {
    anyhow!(CANCELLED_SENTINEL)
}

fn is_cancelled_error(e: &anyhow::Error) -> bool {
    e.to_string().contains(CANCELLED_SENTINEL)
}

/// 取消轮询 future：以 100ms 粒度等待全局取消标记置位。
/// 与 `tokio::select!` 搭配使用，可以在**长耗时的远程请求返回之前**把它中断掉。
///
/// 2026-09-20 日志排查：远程离线识别（qwen 文件转写实测 60~110s）期间，
/// 旧实现只在整块音频转写**结束之后**才检查取消标记，于是
/// ①「停止识别」按钮点了完全没反应（UI 一直停在「识别中 x%」）；
/// ② 任务继续占着 RETRANSCRIPTION_IN_PROGRESS，用户换模型重新识别只会被
///    「Retranscription already in progress」挡掉 → 看起来「所有离线远程模型都失败」；
/// ③ 等它终于跑完，又被完成后的取消检查丢掉结果（白等 100 秒）。
async fn wait_cancelled() {
    loop {
        if RETRANSCRIPTION_CANCELLED.load(Ordering::SeqCst) {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
}

struct RetranscriptionGuard;

impl RetranscriptionGuard {
    fn acquire() -> Result<Self, String> {
        if RETRANSCRIPTION_IN_PROGRESS
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return Err("Retranscription already in progress".to_string());
        }
        Ok(RetranscriptionGuard)
    }
}

impl Drop for RetranscriptionGuard {
    fn drop(&mut self) {
        RETRANSCRIPTION_IN_PROGRESS.store(false, Ordering::SeqCst);
    }
}

// Slightly shorter redemption time for offline retranscription so short
// leading speech (e.g. greetings, confirmations) is less likely to be
// swallowed by VAD, while still avoiding fragmenting normal sentences.
const VAD_REDEMPTION_TIME_MS: u32 = 1500;

/// Local SenseVoice path: more sensitive silero pass so quiet/short utterances
/// (e.g. greetings buried under applause) are not dropped from offline results.
/// Values tuned on real recordings via the vad_real_recording_threshold_sweep
/// test: the most sensitive (positive, negative) pair that still does not stick
/// in-speech through sustained applause, with a shorter redemption time.
const LOCAL_VAD_THRESHOLDS: (f32, f32) = (0.35, 0.25);
const LOCAL_VAD_REDEMPTION_TIME_MS: u32 = 800;

/// 耗时模型：(固定开销秒, RTF)。真实耗时 ≈ `固定开销 + RTF × 语音秒数`。
///
/// 2026-09-22 修「进度条走到一半突然结束」：远程识别的**固定开销**（建连、提交、
/// 轮询、模型加载）常常比识别本身还大 —— 实测 43.5s 语音的 MiMo 一共用了 7.5s，
/// 其中约 5s 是固定开销。旧模型只有 RTF，把 expected 估成 `43.5×0.06 = 2.6s`，
/// 于是心跳在 2.6s 就把条子爬到 95%，后面 5s 一直"卡"在高位，看起来像「到一半
/// 突然完成」。同时它也让转写阶段在总进度里的占比失真。
struct DurationModel {
    /// 与语音长度无关的固定开销（秒）
    fixed_secs: f64,
    /// 每秒钟语音的处理时间
    rtf: f64,
}

impl DurationModel {
    fn expected_secs(&self, speech_sec: f64) -> f64 {
        (self.fixed_secs + self.rtf * speech_sec).max(1.0)
    }
}

/// 远程离线（批量）模型的耗时模型。数值来自 2026-09-22 线上多轮实测。
fn remote_offline_model(model_id: &str) -> DurationModel {
    if model_id.contains("filetrans") {
        // qwen 异步文件转写：提交 + 3s 轮询，364s 音频实测 37.6s
        DurationModel {
            fixed_secs: 15.0,
            rtf: 0.06,
        }
    } else if model_id.contains("doubao") {
        // 豆包录音文件识别：364s 音频实测 33.5s（带语种提示）
        DurationModel {
            fixed_secs: 3.0,
            rtf: 0.09,
        }
    } else if model_id.contains("deepgram") {
        // Deepgram 批量：直连无跨境腿，364s 实测 10.6s
        DurationModel {
            fixed_secs: 1.5,
            rtf: 0.025,
        }
    } else if model_id.contains("mimo") {
        // MiMo 批量：43.5s 实测 7.5s（固定开销占比高），194s 实测 13s
        DurationModel {
            fixed_secs: 5.0,
            rtf: 0.05,
        }
    } else {
        // 未知远程模型：保守（宁可进度走得慢，也不要「看着卡死」）
        DurationModel {
            fixed_secs: 5.0,
            rtf: 0.3,
        }
    }
}

/// Default duration model when no benchmark is available.
fn default_duration_model(model: Option<&str>, provider: Option<&str>) -> DurationModel {
    let is_remote = provider == Some("remote")
        || model
            .map(|m| m.starts_with("qwen3-asr-remote"))
            .unwrap_or(false);
    if is_remote {
        // 前端传的是占位名 qwen3-asr-remote，按后端「离线远程模型」的真实选择取模型
        let offline = crate::audio::transcription::get_remote_asr_offline_model();
        return remote_offline_model(&offline);
    }
    if model.map(|m| m.starts_with("x-asr-")).unwrap_or(false) {
        DurationModel {
            fixed_secs: 0.3,
            rtf: 0.03,
        }
    } else {
        // Local Sherpa-ONNX SenseVoice
        DurationModel {
            fixed_secs: 0.5,
            rtf: 0.35,
        }
    }
}

/// Estimate fixed/overhead stage durations based on the total audio duration.
/// These coefficients are tuned from observed runs: decode + VAD together is
/// roughly 2-4% of the audio duration on typical meeting recordings.
fn estimate_stage_durations(duration_seconds: f64) -> (f64, f64, f64) {
    // 实测：解码 ≈0.006×时长 + 0.5s（364s→2.7s、53s→0.8s）；
    //       VAD   ≈0.005×时长 + 0.15s（364s→2.1s、53s→0.4s）。
    // save 从 5.0s 降到 0.3s —— **这是「进度条到一半突然结束」的主因**：保存阶段
    // 实际只要几十毫秒，却按 5s 预约了整条进度的一半以上（53s 音频那次 save 占比 57%），
    // 于是转写一结束条子立刻从 ~43% 跳到 100%。现在只留 0.3s 余量。
    let decode = duration_seconds * 0.006 + 0.5;
    let vad = duration_seconds * 0.005 + 0.15;
    let save = 0.3;
    (decode, vad, save)
}

/// Tracks dynamic progress and ETA for offline retranscription.
#[derive(Clone)]
struct ProgressEstimator {
    inner: std::sync::Arc<std::sync::Mutex<ProgressEstimatorInner>>,
}

struct ProgressEstimatorInner {
    start_time: Instant,
    decode_est: f64,
    vad_est: f64,
    save_est: f64,
    rtf: f64,
    /// 与语音长度无关的固定开销（秒），见 DurationModel
    fixed_overhead_secs: f64,
    total_speech_sec: f64,
    chunk_process_times: Vec<f64>,
    processed_speech_sec: f64,
}

impl ProgressEstimator {
    fn new(
        start_time: Instant,
        duration_seconds: f64,
        total_speech_sec: f64,
        rtf: f64,
        fixed_overhead_secs: f64,
    ) -> Self {
        let (decode_est, vad_est, save_est) = estimate_stage_durations(duration_seconds);
        Self {
            inner: std::sync::Arc::new(std::sync::Mutex::new(ProgressEstimatorInner {
                start_time,
                decode_est,
                vad_est,
                save_est,
                rtf,
                fixed_overhead_secs,
                total_speech_sec,
                chunk_process_times: Vec::new(),
                processed_speech_sec: 0.0,
            })),
        }
    }

    fn record_decode_done(&self, actual_sec: f64) {
        let mut inner = self.inner.lock().unwrap();
        inner.decode_est = actual_sec.max(0.1);
    }

    fn record_vad_done(&self, actual_sec: f64) {
        let mut inner = self.inner.lock().unwrap();
        inner.vad_est = actual_sec.max(0.1);
    }

    fn set_total_speech_sec(&self, total_speech_sec: f64) {
        let mut inner = self.inner.lock().unwrap();
        inner.total_speech_sec = total_speech_sec;
    }

    fn record_chunk(&self, speech_sec: f64, process_time: f64) {
        let mut inner = self.inner.lock().unwrap();
        inner
            .chunk_process_times
            .push(process_time / speech_sec.max(0.1));
        inner.processed_speech_sec += speech_sec;
    }

    fn progress_pct(&self, stage: &str, stage_fraction: f64) -> u32 {
        let inner = self.inner.lock().unwrap();
        let (d, v, t, s) = {
            let transcribe_est = inner.fixed_overhead_secs + inner.total_speech_sec * inner.rtf;
            let total = inner.decode_est + inner.vad_est + transcribe_est + inner.save_est;
            if total <= 0.0 {
                (0.05, 0.05, 0.85, 0.05)
            } else {
                let d = inner.decode_est / total;
                let v = inner.vad_est / total;
                let s = inner.save_est / total;
                let t = 1.0 - d - v - s;
                (d, v, t, s)
            }
        };
        let frac = match stage {
            "decoding" => stage_fraction.clamp(0.0, 1.0) * d,
            "vad" => d + stage_fraction.clamp(0.0, 1.0) * v,
            "transcribing" => {
                // 已完成的 chunk 占比 + 当前 chunk 的「进行中」估计（由心跳传入 0~1）。
                // 远程批量模型会把整段语音合成 1 个 chunk，processed_speech_sec 在
                // 调用返回前恒为 0 —— 只用已完成占比的话百分比会一直冻结在一个值上
                // （2026-09-20 用户实测「卡死在 8%」）。心跳按已用时间给一个双曲爬升的
                // 估计值填进来，保证条永远在动；chunk 真正完成后 processed 会接管，
                // 且 (done + in_flight) 恒不大于 1，进度不会回退。
                let done = if inner.total_speech_sec > 0.0 {
                    (inner.processed_speech_sec / inner.total_speech_sec).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                let in_flight = stage_fraction.clamp(0.0, 1.0) * (1.0 - done);
                d + v + (done + in_flight).clamp(0.0, 1.0) * t
            }
            "saving" => d + v + t + stage_fraction.clamp(0.0, 1.0) * s,
            "complete" => 1.0,
            _ => 0.0,
        };
        (frac * 100.0).clamp(0.0, 100.0) as u32
    }

    fn estimated_remaining(&self, stage: &str, stage_fraction: f64) -> f64 {
        let inner = self.inner.lock().unwrap();
        match stage {
            "decoding" => {
                (1.0 - stage_fraction.clamp(0.0, 1.0)) * inner.decode_est
                    + inner.vad_est
                    + inner.total_speech_sec * inner.rtf
                    + inner.save_est
            }
            "vad" => {
                (1.0 - stage_fraction.clamp(0.0, 1.0)) * inner.vad_est
                    + inner.total_speech_sec * inner.rtf
                    + inner.save_est
            }
            "transcribing" => {
                let remaining_speech =
                    (inner.total_speech_sec - inner.processed_speech_sec).max(0.0);
                let eff_rtf = if inner.chunk_process_times.is_empty() {
                    inner.rtf
                } else {
                    let window = inner.chunk_process_times.len().min(5);
                    let sum: f64 = inner.chunk_process_times.iter().rev().take(window).sum();
                    sum / window as f64
                };
                let fixed_left = if inner.processed_speech_sec <= 0.0 {
                    inner.fixed_overhead_secs
                } else {
                    0.0
                };
                remaining_speech * eff_rtf + fixed_left + inner.save_est
            }
            "saving" => (1.0 - stage_fraction.clamp(0.0, 1.0)) * inner.save_est,
            "complete" => 0.0,
            _ => 0.0,
        }
    }

    fn elapsed_secs(&self) -> f64 {
        let inner = self.inner.lock().unwrap();
        inner.start_time.elapsed().as_secs_f64()
    }

    /// 该段语音的预计耗时（固定开销 + RTF×语音秒数）——心跳的爬升基准。
    fn expected_secs_for(&self, speech_sec: f64) -> f64 {
        let inner = self.inner.lock().unwrap();
        (inner.fixed_overhead_secs + inner.rtf * speech_sec).max(1.0)
    }
}

/// 转写单个 chunk 期间的心跳：每 400ms 按「已用时间」发一次进度。
///
/// 为什么需要：远程批量模型（豆包录音文件识别 / qwen 异步文件转写 / mimo 批量）
/// 会把整段语音合并成 **1 个 chunk**，`processed_speech_sec` 在整个远程调用返回前
/// 恒为 0 → 页面百分比只在 VAD 结束时算过一次就再也不动（2026-09-20 用户实测：
/// 「卡死在百分之八」，而实际后台正在正常等待云端结果，mimo 甚至只差 1.1 秒就出结果）。
///
/// 进度用双曲爬升 `elapsed / (elapsed + expected)`：无论估计准不准，条永远在动、
/// 也永远不会提前到 100%（expected = 该 chunk 语音时长 × 模型 RTF 提示值）。
/// 消息里带上「已等待 Ns」，用户能直接看到还在推进。
struct ProgressHeartbeat {
    stop: Arc<AtomicBool>,
}

impl ProgressHeartbeat {
    fn start<R: Runtime>(
        app: AppHandle<R>,
        meeting_id: String,
        estimator: ProgressEstimator,
        chunk_index: usize,
        chunks_count: usize,
        speech_sec: f64,
        label: &'static str,
    ) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let stop_for_task = stop.clone();
        tauri::async_runtime::spawn(async move {
            let started = Instant::now();
            let expected = estimator.expected_secs_for(speech_sec);
            let mut tick = tokio::time::interval(std::time::Duration::from_millis(400));
            tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            loop {
                tick.tick().await;
                if stop_for_task.load(Ordering::SeqCst) {
                    break;
                }
                let elapsed = started.elapsed().as_secs_f64();
                // 进度曲线：到「预计耗时」时逼近 0.95，之后缓慢爬向 0.99。
                //
                // 为什么不用双曲 `elapsed/(elapsed+expected)`（2026-09-22 用户实测
                // 「进度条才走到一半就突然识别完成」）：该曲线在 elapsed == expected
                // 时只有 0.5，而真实耗时通常≈expected → 条子停在半路，chunk 一返回
                // 就由 50% 跳到 100%。改成线性逼近后，完成时刻的条子已在 ~95%，
                // 收尾只跳几个百分点。
                let x = elapsed / expected;
                let frac = if x <= 1.0 {
                    x * 0.95
                } else {
                    0.95 + 0.04 * (1.0 - (-(x - 1.0)).exp())
                };
                let message = format!(
                    "{}（第 {}/{} 段 · 已等待 {:.0}s）",
                    label,
                    chunk_index + 1,
                    chunks_count,
                    elapsed
                );
                emit_progress(
                    &app,
                    &meeting_id,
                    "transcribing",
                    frac,
                    &message,
                    &estimator,
                    Some(chunks_count),
                    Some(chunk_index),
                );
            }
        });
        Self { stop }
    }
}

impl Drop for ProgressHeartbeat {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetranscriptionProgress {
    pub meeting_id: String,
    pub stage: String,
    pub progress_percentage: u32,
    pub message: String,
    pub elapsed_seconds: Option<f64>,
    pub estimated_remaining_seconds: Option<f64>,
    pub chunks_total: Option<usize>,
    pub chunks_processed: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetranscriptionResult {
    pub meeting_id: String,
    pub segments_count: usize,
    pub duration_seconds: f64,
    pub language: Option<String>,
    pub elapsed_seconds: f64,
    /// 用户可见告警（上游内容风控部分拦截 / 档位降级 / 部分分片失败…）
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetranscriptionError {
    pub meeting_id: String,
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetranscriptionPartial {
    pub meeting_id: String,
    pub chunk_index: usize,
    pub chunks_total: usize,
    pub text: String,
    pub start_ms: f64,
    pub end_ms: f64,
}

pub fn is_retranscription_in_progress() -> bool {
    RETRANSCRIPTION_IN_PROGRESS.load(Ordering::SeqCst)
}

pub fn cancel_retranscription() {
    RETRANSCRIPTION_CANCELLED.store(true, Ordering::SeqCst);
}

/// Start retranscription of a meeting's audio
pub async fn start_retranscription<R: Runtime>(
    app: AppHandle<R>,
    meeting_id: String,
    meeting_folder_path: String,
    language: Option<String>,
    model: Option<String>,
    provider: Option<String>,
    estimated_rtf: Option<f64>,
) -> Result<RetranscriptionResult> {
    let _guard = RetranscriptionGuard::acquire().map_err(|e| anyhow!(e))?;

    // 任务会话（2026-09-22）：一次离线重识别 = 网关侧一个积分任务。
    // 本次重识别的所有 chunk 识别（以及随后对结果的逐句翻译）都带同一个
    // `x-vox-session` 头，用户中心显示为「一次离线识别：语音识别 X 积分 / 翻译 Y 积分」。
    // 见 src/task_session.rs。
    info!(
        "📊 离线重识别任务会话: {}",
        crate::task_session::start_retranscribe_session()
    );

    RETRANSCRIPTION_CANCELLED.store(false, Ordering::SeqCst);

    let result = run_retranscription(
        app.clone(),
        meeting_id.clone(),
        meeting_folder_path,
        language,
        model,
        provider,
        estimated_rtf,
    )
    .await;

    super::common::unload_engine_after_batch().await;

    match &result {
        Ok(res) => {
            let _ = app.emit(
                "retranscription-complete",
                serde_json::json!({
                    "meeting_id": res.meeting_id,
                    "segments_count": res.segments_count,
                    "duration_seconds": res.duration_seconds,
                    "language": res.language,
                    "elapsed_seconds": res.elapsed_seconds,
                    "warnings": res.warnings
                }),
            );
        }
        Err(e) => {
            // 用户主动取消不是故障：发独立事件，前端提示「已停止识别」而不是红色失败
            if is_cancelled_error(e) {
                info!("Retranscription cancelled by user: {}", meeting_id);
                let _ = app.emit(
                    "retranscription-cancelled",
                    serde_json::json!({ "meeting_id": meeting_id }),
                );
            } else {
                let _ = app.emit(
                    "retranscription-error",
                    RetranscriptionError {
                        meeting_id: meeting_id.clone(),
                        error: e.to_string(),
                    },
                );
            }
        }
    }

    result
}

pub(crate) fn find_audio_file(folder: &Path) -> Result<PathBuf> {
    let candidates = [
        "audio.mp4",
        "audio.m4a",
        "audio.wav",
        "audio.mp3",
        "audio.flac",
        "audio.ogg",
        "recording.mp4",
        "audio.mkv",
        "audio.webm",
        "audio.wma",
    ];

    for name in candidates {
        let path = folder.join(name);
        if path.exists() {
            return Ok(path);
        }
    }

    if let Ok(entries) = std::fs::read_dir(folder) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension() {
                let ext = ext.to_string_lossy().to_lowercase();
                if AUDIO_EXTENSIONS.contains(&ext.as_str()) {
                    return Ok(path);
                }
            }
        }
    }

    Err(anyhow!("No audio file found in: {}", folder.display()))
}

async fn run_retranscription<R: Runtime>(
    app: AppHandle<R>,
    meeting_id: String,
    meeting_folder_path: String,
    language: Option<String>,
    model: Option<String>,
    provider: Option<String>,
    estimated_rtf: Option<f64>,
) -> Result<RetranscriptionResult> {
    let start_time = Instant::now();
    // 语言语义（2026-09-22 随「按模型过滤的识别语言选择」明确）：
    //   · 前端**明确传了值** → 一律以它为准：具体语言码照用，'auto' 表示「明确不要语言提示」，
    //     **不再回落全局偏好**——否则「先录了中文（全局偏好=zh），再去离线重识别一段英文」
    //     会把 zh 当作英文录音的语言提示，比不传还糟。
    //   · 完全没传（旧客户端 / 导入等路径）→ 沿用用户的语言偏好（中文/English/自动），
    //     因为云端离线模型「已知语种」比「自动检测」明显更快也更稳（豆包实测
    //     说话人分离+指定语种 6.2~6.5s，而开 enable_auto_lang 的档位经常卡住）。
    let language = {
        let provided = language.as_deref().map(str::trim).filter(|l| !l.is_empty());
        let resolved = match provided {
            Some(l) => {
                if l == "auto" {
                    None
                } else {
                    Some(l.to_string())
                }
            }
            None => crate::get_language_preference_internal()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty() && l != "auto"),
        };
        if resolved.is_some() {
            info!("Retranscription language hint: {:?}", resolved);
        }
        resolved
    };
    // 记录本次离线识别实际使用的模型（metadata.json / 历史页展示用）——在 model 被移动前算好
    let model_used_for_metadata = model
        .as_deref()
        .map(|m| m.trim().to_string())
        .filter(|m| !m.is_empty() && !m.starts_with("qwen3-asr-remote"))
        .unwrap_or_else(crate::audio::transcription::get_remote_asr_offline_model);
    let folder_path = PathBuf::from(&meeting_folder_path);
    let audio_path = find_audio_file(&folder_path)?;

    info!("Starting retranscription for meeting {}", meeting_id);

    // Early validation: if remote ASR is requested, check endpoint is configured BEFORE
    // spending time on audio decoding and VAD processing.
    let is_remote = provider.as_deref() == Some("remote")
        || model
            .as_deref()
            .map(|m| m.starts_with("qwen3-asr-remote"))
            .unwrap_or(false);
    let is_xasr = model
        .as_deref()
        .map(|m| m.starts_with("x-asr-"))
        .unwrap_or(false);
    if is_remote {
        let endpoint = crate::audio::transcription::effective_remote_endpoint();
        if endpoint.is_empty() {
            // 内置默认地址兜底下实际不可达，保留作防御
            return Err(anyhow!(
                "Remote ASR endpoint not configured. Please set the remote ASR URL in Settings."
            ));
        }
        info!("Using remote ASR for retranscription: {}", endpoint);
    }

    let dur_model = default_duration_model(model.as_deref(), provider.as_deref());
    // 前端可传 estimated_rtf 覆盖斜率（历史行为），固定开销仍用模型内置值
    let rtf = estimated_rtf.unwrap_or(dur_model.rtf);
    info!(
        "Retranscription duration model: fixed={:.1}s rtf={:.3}",
        dur_model.fixed_secs, rtf
    );

    // Probe audio duration from metadata before full decode so we can show a
    // realistic ETA from the very first progress event.
    let duration_seconds = probe_audio_duration(&audio_path).unwrap_or_else(|e| {
        warn!("Failed to probe audio duration: {}, using 0", e);
        0.0
    });
    info!("Probed audio duration: {:.2}s", duration_seconds);

    // Create estimator with the probed duration; speech estimate is 80% of total.
    let estimator = ProgressEstimator::new(
        start_time,
        duration_seconds,
        duration_seconds * 0.8,
        rtf,
        dur_model.fixed_secs,
    );
    emit_progress(
        &app,
        &meeting_id,
        "decoding",
        0.0,
        "解码音频文件...",
        &estimator,
        None,
        None,
    );

    if RETRANSCRIPTION_CANCELLED.load(Ordering::SeqCst) {
        return Err(cancelled_error());
    }

    let path_for_decode = audio_path.clone();
    let decoded = tokio::task::spawn_blocking(move || decode_audio_file(&path_for_decode))
        .await
        .map_err(|e| anyhow!("Decode task panicked: {}", e))??;

    info!(
        "Decoded audio: {:.2}s, {}Hz, {} channels",
        decoded.duration_seconds, decoded.sample_rate, decoded.channels
    );

    if RETRANSCRIPTION_CANCELLED.load(Ordering::SeqCst) {
        return Err(cancelled_error());
    }

    // Convert to mono + normalize, but skip resampling — VAD will handle it internally
    let vad_sample_rate = decoded.sample_rate;
    let audio_samples = tokio::task::spawn_blocking(move || {
        let mono = if decoded.channels > 1 {
            audio_to_mono(&decoded.samples, decoded.channels)
        } else {
            decoded.samples
        };
        let max_abs = mono
            .iter()
            .filter(|s| s.is_finite())
            .map(|s| s.abs())
            .fold(0.0f32, f32::max);
        let mut mono = if max_abs > 1.0 {
            let scale = 1.0 / max_abs;
            mono.into_iter().map(|s| s * scale).collect()
        } else {
            mono
        };
        for s in &mut mono {
            if !s.is_finite() {
                *s = 0.0;
            } else {
                *s = s.clamp(-1.0, 1.0);
            }
        }
        mono
    })
    .await
    .map_err(|e| anyhow!("Mono conversion panicked: {}", e))?;
    info!(
        "Preprocessed audio: {}Hz, {} samples ({:.1}s)",
        vad_sample_rate,
        audio_samples.len(),
        audio_samples.len() as f64 / vad_sample_rate as f64
    );

    // Record actual decode + preprocessing time and advance to VAD stage.
    let decode_actual_sec = start_time.elapsed().as_secs_f64();
    estimator.record_decode_done(decode_actual_sec);
    emit_progress(
        &app,
        &meeting_id,
        "decoding",
        1.0,
        "预处理完成",
        &estimator,
        None,
        None,
    );

    emit_progress(
        &app,
        &meeting_id,
        "vad",
        0.0,
        "VAD: 过滤静音, 检测语音段落...",
        &estimator,
        None,
        None,
    );

    if RETRANSCRIPTION_CANCELLED.load(Ordering::SeqCst) {
        return Err(cancelled_error());
    }

    let app_for_vad = app.clone();
    let meeting_id_for_vad = meeting_id.clone();
    let estimator_for_vad = estimator.clone();
    let vad_start = Instant::now();

    // Local SenseVoice uses a tuned, more sensitive VAD pass (see constants);
    // remote / X-ASR keep the previous defaults.
    let (vad_redemption_ms, vad_thresholds) = if is_remote || is_xasr {
        (VAD_REDEMPTION_TIME_MS, None)
    } else {
        (LOCAL_VAD_REDEMPTION_TIME_MS, Some(LOCAL_VAD_THRESHOLDS))
    };

    let speech_segments = tokio::task::spawn_blocking(move || {
        get_speech_chunks_with_config(
            &audio_samples,
            vad_sample_rate,
            vad_redemption_ms,
            vad_thresholds,
            |vad_progress, segments_found| {
                let stage_fraction = vad_progress as f64 / 100.0;
                emit_progress(
                    &app_for_vad,
                    &meeting_id_for_vad,
                    "vad",
                    stage_fraction,
                    &format!(
                        "Detecting speech segments... {}% ({} found)",
                        vad_progress, segments_found
                    ),
                    &estimator_for_vad,
                    None,
                    None,
                );
                !RETRANSCRIPTION_CANCELLED.load(Ordering::SeqCst)
            },
        )
    })
    .await
    .map_err(|e| anyhow!("VAD task panicked: {}", e))?
    .map_err(|e| anyhow!("VAD processing failed: {}", e))?;

    let total_segments = speech_segments.len();
    let vad_actual_sec = vad_start.elapsed().as_secs_f64();
    let total_speech_sec: f64 = speech_segments
        .iter()
        .map(|s| (s.end_timestamp_ms - s.start_timestamp_ms) / 1000.0)
        .sum();
    estimator.record_vad_done(vad_actual_sec);
    estimator.set_total_speech_sec(total_speech_sec);
    info!(
        "VAD detected {} speech segments, total speech {:.1}s, VAD took {:.1}s",
        total_segments, total_speech_sec, vad_actual_sec
    );

    if total_segments == 0 {
        warn!("No speech detected in audio");
        return Err(anyhow!("No speech detected in audio file"));
    }

    // 远程批量：3 分钟/段。两个理由：
    //  ① 内联 base64 上游（MiMo 的 input_audio.data）有 10MB 上限——16kHz 单声道 WAV
    //     3 分钟 = 5.76MB（base64 后 ≈7.7MB）安全；6 分钟 = 11.2MB（base64 15MB）必被拒，
    //     2026-09-22 实测 MiMo 正是报 `input_audio.data exceeds maximum size of 10MB`
    //     而「一个结果都没有」。
    //  ② 进度 = 已完成 chunk 占比，段越少「最后一跳」越大；3 分钟粒度让 6 分钟会议
    //     有 2~3 段，进度条与取消响应都真实得多。
    const REMOTE_CHUNK_DURATION_SECS: u32 = 180;
    // X-ASR 是本地模型、处理很快，保留大盘子减少段间开销。
    const XASR_CHUNK_DURATION_SECS: u32 = 600;
    const LOCAL_CHUNK_DURATION_SECS: u32 = 120; // 2 min for local SenseVoice
    const VAD_GAP_SAMPLES: usize = 800; // 50ms silence gap between merged segments
    const MIN_SEGMENT_SAMPLES: usize = 1600; // skip VAD artifacts < 100ms

    // Local SenseVoice is further chunked inside SherpaOnnxProvider, but keeping
    // the top-level chunks smaller gives the UI progress updates more frequently
    // and keeps memory spikes low. Remote/X-ASR can take larger chunks.
    let chunk_duration_secs: u32 = if is_remote {
        REMOTE_CHUNK_DURATION_SECS
    } else if is_xasr {
        XASR_CHUNK_DURATION_SECS
    } else {
        LOCAL_CHUNK_DURATION_SECS
    };
    let chunk_samples: usize = chunk_duration_secs as usize * 16000;

    let mut chunks: Vec<crate::audio::vad::SpeechSegment> = Vec::new();

    if is_remote || is_xasr {
        // Merge consecutive VAD segments into large chunks so every model gets ample
        // context. Safety: if a single VAD segment exceeds the chunk limit (e.g. 2
        // hours of continuous speech with no pause), it is split at chunk_samples
        // boundaries to prevent unbounded memory use or REST request timeouts.
        let mut cur_samples: Vec<f32> = Vec::new();
        let mut cur_start_ms: f64 = 0.0;

        for segment in &speech_segments {
            if segment.samples.len() < MIN_SEGMENT_SAMPLES {
                continue;
            }

            // Extremely long VAD segment: split into chunk-sized sub-chunks,
            // preferring silence boundaries so we don't cut mid-sentence.
            if segment.samples.len() > chunk_samples {
                // Flush any pending partial chunk first
                if !cur_samples.is_empty() {
                    // Use the start of the current (over-long) segment as the end of the
                    // partial chunk, not the end of the entire audio.
                    let end_ms = segment.start_timestamp_ms;
                    chunks.push(crate::audio::vad::SpeechSegment {
                        samples: std::mem::take(&mut cur_samples),
                        start_timestamp_ms: cur_start_ms,
                        end_timestamp_ms: end_ms,
                        confidence: 0.9,
                    });
                }
                let ranges = crate::audio::chunking::split_at_silence(
                    &segment.samples,
                    16000,
                    chunk_duration_secs as f64,
                    1.0, // ±1s search radius
                    0.2, // 200ms silence window
                    0.5, // 500ms minimum tail
                );
                let ms_per_sample = (segment.end_timestamp_ms - segment.start_timestamp_ms)
                    / segment.samples.len() as f64;
                for (start, end) in ranges {
                    chunks.push(crate::audio::vad::SpeechSegment {
                        samples: segment.samples[start..end].to_vec(),
                        start_timestamp_ms: segment.start_timestamp_ms
                            + start as f64 * ms_per_sample,
                        end_timestamp_ms: segment.start_timestamp_ms + end as f64 * ms_per_sample,
                        confidence: segment.confidence,
                    });
                }
                continue;
            }

            let needed = if cur_samples.is_empty() {
                segment.samples.len()
            } else {
                VAD_GAP_SAMPLES + segment.samples.len()
            };

            if !cur_samples.is_empty() && cur_samples.len() + needed > chunk_samples {
                chunks.push(crate::audio::vad::SpeechSegment {
                    samples: std::mem::take(&mut cur_samples),
                    start_timestamp_ms: cur_start_ms,
                    end_timestamp_ms: segment.start_timestamp_ms,
                    confidence: 0.9,
                });
                cur_start_ms = segment.start_timestamp_ms;
                cur_samples.extend_from_slice(&segment.samples);
            } else {
                if cur_samples.is_empty() {
                    cur_start_ms = segment.start_timestamp_ms;
                } else {
                    cur_samples.extend(std::iter::repeat(0.0f32).take(VAD_GAP_SAMPLES));
                }
                cur_samples.extend_from_slice(&segment.samples);
            }
        }
        // Push final chunk
        if !cur_samples.is_empty() {
            let end_ms = speech_segments
                .last()
                .map(|s| s.end_timestamp_ms)
                .unwrap_or(0.0);
            chunks.push(crate::audio::vad::SpeechSegment {
                samples: cur_samples,
                start_timestamp_ms: cur_start_ms,
                end_timestamp_ms: end_ms,
                confidence: 0.9,
            });
        }
    } else {
        // Local SenseVoice: transcribe each VAD speech segment individually so the
        // saved segments keep real speech boundaries and timestamps. Merging them
        // into large chunks used to produce one giant segment per chunk; the
        // provider already windows long audio internally, and per-segment chunks
        // also give finer progress updates.
        for segment in &speech_segments {
            if segment.samples.len() < MIN_SEGMENT_SAMPLES {
                continue;
            }

            if segment.samples.len() > chunk_samples {
                // Extremely long single segment: split at silence boundaries.
                let ranges = crate::audio::chunking::split_at_silence(
                    &segment.samples,
                    16000,
                    chunk_duration_secs as f64,
                    1.0, // ±1s search radius
                    0.2, // 200ms silence window
                    0.5, // 500ms minimum tail
                );
                let ms_per_sample = (segment.end_timestamp_ms - segment.start_timestamp_ms)
                    / segment.samples.len() as f64;
                for (start, end) in ranges {
                    chunks.push(crate::audio::vad::SpeechSegment {
                        samples: segment.samples[start..end].to_vec(),
                        start_timestamp_ms: segment.start_timestamp_ms
                            + start as f64 * ms_per_sample,
                        end_timestamp_ms: segment.start_timestamp_ms + end as f64 * ms_per_sample,
                        confidence: segment.confidence,
                    });
                }
            } else {
                chunks.push(segment.clone());
            }
        }
    }

    if is_remote || is_xasr {
        info!(
            "Merged {} VAD segments into {} large chunks ({}s speech max/chunk)",
            total_segments,
            chunks.len(),
            chunk_duration_secs
        );
    } else {
        info!(
            "Prepared {} per-VAD-segment chunks from {} VAD segments for local SenseVoice",
            chunks.len(),
            total_segments
        );
    }

    if chunks.is_empty() {
        return Err(anyhow!("No speech detected after VAD"));
    }

    let chunks_count = chunks.len();
    let chunk_total_speech_sec: f64 = chunks
        .iter()
        .map(|c| c.samples.len() as f64 / 16000.0)
        .sum();
    estimator.set_total_speech_sec(chunk_total_speech_sec);
    info!(
        "Chunk speech total: {:.1}s across {} chunks",
        chunk_total_speech_sec, chunks_count
    );

    let mut all_transcripts: Vec<super::common::TranscriptEntry> = Vec::new();
    // 逐段失败原因（2026-09-22 新增）。
    // 为什么需要：旧实现把「分片识别失败」只记一条 WARN 就继续，最后哪怕**所有分片
    // 都失败、0 段结果**，也会照常走保存流程：DELETE 掉上一次的离线结果、写入空
    // transcripts_offline.json、metadata 标记 completed，并发 retranscription-complete
    // → 用户看到「识别完成，共 0 段」，还丢掉了此前可用的离线结果。
    // 现在只要一条转写都没拿到就直接报错（保留旧结果 + 前端明确提示失败原因，
    // 例如 MiMo 的内容风控拒答）。
    let mut chunk_failures: Vec<String> = Vec::new();
    // 上游给出的**用户可见告警**（内容风控部分拦截 / 档位降级 / 语言提示被拒…）。
    // 汇总后随完成事件下发 + 写进 metadata.json，由前端明确告诉用户
    // 「结果不完整是上游内容策略，不是软件故障」。
    let mut user_warnings: Vec<String> = Vec::new();

    if is_xasr {
        use crate::audio::transcription::x_asr_provider::XAsrProvider;
        let xasr_model = model.clone().unwrap_or_else(|| "x-asr-480ms".to_string());
        // Ensure engine is loaded
        if !crate::sherpa_onnx_engine::commands::is_xasr_engine_loaded() {
            if let Err(e) =
                crate::sherpa_onnx_engine::commands::sherpa_onnx_load_model(xasr_model.clone())
                    .await
            {
                return Err(anyhow!("Failed to load X-ASR model: {}", e));
            }
        }
        let engine = crate::sherpa_onnx_engine::commands::get_or_init_xasr_engine()
            .map_err(|e| anyhow!("X-ASR engine not ready: {}", e))?;
        let xasr_provider = XAsrProvider::new_with_engine(xasr_model, engine);

        emit_progress(
            &app,
            &meeting_id,
            "transcribing",
            0.0,
            &format!("X-ASR: {} 个大段, 准备发送...", chunks_count),
            &estimator,
            Some(chunks_count),
            Some(0),
        );

        let total_speech_sec: f64 = chunks
            .iter()
            .map(|c| c.samples.len() as f64 / 16000.0)
            .sum();
        estimator.set_total_speech_sec(total_speech_sec);

        for (i, chunk) in chunks.iter().enumerate() {
            if RETRANSCRIPTION_CANCELLED.load(Ordering::SeqCst) {
                return Err(cancelled_error());
            }

            let speech_sec = chunk.samples.len() as f64 / 16000.0;
            let chunk_start = Instant::now();

            // 500ms silence padding pre/post for streaming transducer model
            const XASR_PAD: usize = 8000;
            let mut padded = vec![0.0f32; XASR_PAD + chunk.samples.len() + XASR_PAD];
            padded[XASR_PAD..XASR_PAD + chunk.samples.len()].copy_from_slice(&chunk.samples);

            let pcm_i16: Vec<i16> = padded
                .iter()
                .map(|&s| (s.clamp(-1.0, 1.0) * 32767.0) as i16)
                .collect();
            let temp_wav = folder_path.join(format!("_xasr_temp_{}.wav", i));
            write_wav_file(&temp_wav, &pcm_i16, 16000, 1)?;

            // 用户点「停止识别」时立刻中断本段（不等它跑完）
            let heartbeat = ProgressHeartbeat::start(
                app.clone(),
                meeting_id.clone(),
                estimator.clone(),
                i,
                chunks_count,
                speech_sec,
                "X-ASR 识别中",
            );
            let xasr_result = tokio::select! {
                r = xasr_provider.transcribe_file(&temp_wav) => r,
                _ = wait_cancelled() => {
                    info!("重识别已被用户取消（X-ASR 第 {}/{} 段）", i + 1, chunks_count);
                    let _ = std::fs::remove_file(&temp_wav);
                    return Err(cancelled_error());
                }
            };
            drop(heartbeat);
            match xasr_result {
                Ok(text) => {
                    let _ = std::fs::remove_file(&temp_wav);
                    let trimmed = text.trim().to_string();
                    if !trimmed.is_empty() {
                        debug!(
                            "X-ASR chunk {}/{} ({:.0}s): text='{}'",
                            i + 1,
                            chunks_count,
                            speech_sec,
                            if trimmed.len() > 100 {
                                let mut e = 100;
                                while !trimmed.is_char_boundary(e) {
                                    e -= 1;
                                }
                                &trimmed[..e]
                            } else {
                                &trimmed
                            }
                        );
                        all_transcripts.push(super::common::TranscriptEntry {
                            text: trimmed.clone(),
                            start_ms: chunk.start_timestamp_ms,
                            end_ms: chunk.end_timestamp_ms,
                            speaker: String::new(),
                        });
                        emit_partial(
                            &app,
                            &meeting_id,
                            i,
                            chunks_count,
                            &trimmed,
                            chunk.start_timestamp_ms,
                            chunk.end_timestamp_ms,
                        );
                    }
                }
                Err(e) => {
                    let _ = std::fs::remove_file(&temp_wav);
                    warn!("X-ASR chunk {}/{} failed: {}", i + 1, chunks_count, e);
                    chunk_failures.push(format!("第 {}/{} 段: {}", i + 1, chunks_count, e));
                }
            }
            let chunk_time = chunk_start.elapsed().as_secs_f64();
            estimator.record_chunk(speech_sec, chunk_time);
            emit_progress(
                &app,
                &meeting_id,
                "transcribing",
                0.0,
                &format!(
                    "X-ASR 转写中 {}/{} ({:.0}s 音频)...",
                    i + 1,
                    chunks_count,
                    speech_sec
                ),
                &estimator,
                Some(chunks_count),
                Some(i + 1),
            );
        }
    } else {
        emit_progress(
            &app,
            &meeting_id,
            "transcribing",
            0.0,
            &format!("加载识别引擎... ({} 个大段)", chunks_count),
            &estimator,
            Some(chunks_count),
            Some(0),
        );

        let transcription_provider = get_or_init_sherpa_onnx(model, provider).await?;

        let total_speech_sec: f64 = chunks
            .iter()
            .map(|c| c.samples.len() as f64 / 16000.0)
            .sum();
        estimator.set_total_speech_sec(total_speech_sec);

        for (i, chunk) in chunks.iter().enumerate() {
            if RETRANSCRIPTION_CANCELLED.load(Ordering::SeqCst) {
                return Err(cancelled_error());
            }

            let speech_sec = chunk.samples.len() as f64 / 16000.0;
            let chunk_start = Instant::now();

            // 用户点「停止识别」时立刻中断本段。远程批量识别（qwen 文件转写
            // 提交+轮询、豆包录音文件识别）单段可达 60~110s，必须能中途掐断。
            // 同时开心跳：远程调用期间 processed_speech_sec 恒为 0，没有心跳
            // 百分比就会一直冻结在 VAD 结束时的那个值（「卡死在 8%」）。
            let heartbeat = ProgressHeartbeat::start(
                app.clone(),
                meeting_id.clone(),
                estimator.clone(),
                i,
                chunks_count,
                speech_sec,
                if is_remote {
                    "云端识别中"
                } else {
                    "本地识别中"
                },
            );
            let transcribe_result = tokio::select! {
                r = transcription_provider.transcribe(chunk.samples.clone(), language.clone()) => r,
                _ = wait_cancelled() => {
                    info!("重识别已被用户取消（第 {}/{} 段，已等待 {:.1}s）",
                        i + 1, chunks_count, chunk_start.elapsed().as_secs_f64());
                    return Err(cancelled_error());
                }
                // 等待期心跳（2026-09-25）：远程批量单段可达 60~110s，提交后到响应之间
                // 此前没有任何日志（70s 真空），无法区分「卡在上传」还是「卡在上游轮询」。
                // reqwest 的 send() 无法拆分上传/等响应两段，这里一条覆盖全程。
                // 该分支永不返回，只负责打日志；不影响请求与取消语义。
                _ = async {
                    loop {
                        tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                        if is_remote {
                            info!(
                                "离线识别 chunk {}/{} 已提交，等待上游响应（已等待 {:.0}s）",
                                i + 1,
                                chunks_count,
                                chunk_start.elapsed().as_secs_f64()
                            );
                        }
                    }
                } => unreachable!("等待期心跳循环不会返回"),
            };
            drop(heartbeat);
            match transcribe_result {
                Ok(result) => {
                    for w in &result.warnings {
                        if !user_warnings.contains(w) {
                            user_warnings.push(w.clone());
                        }
                    }
                    let trimmed = result.text.trim().to_string();
                    if !trimmed.is_empty() {
                        debug!(
                            "Chunk {}/{} ({:.0}s): text='{}'",
                            i + 1,
                            chunks_count,
                            speech_sec,
                            if trimmed.len() > 100 {
                                let mut e = 100;
                                while !trimmed.is_char_boundary(e) {
                                    e -= 1;
                                }
                                &trimmed[..e]
                            } else {
                                &trimmed
                            }
                        );
                        // 分句级结果（豆包录音文件识别）：逐句落段，带时间戳与说话人
                        if !result.utterances.is_empty() {
                            let mut last_end = chunk.start_timestamp_ms;
                            for u in &result.utterances {
                                let start = chunk.start_timestamp_ms + u.start_ms as f64;
                                let end = chunk.start_timestamp_ms + u.end_ms as f64;
                                let text = u.text.trim().to_string();
                                if !text.is_empty() {
                                    all_transcripts.push(super::common::TranscriptEntry {
                                        text,
                                        start_ms: start.max(last_end),
                                        end_ms: end.max(start),
                                        speaker: u.speaker.clone(),
                                    });
                                    last_end = end.max(start);
                                }
                            }
                        } else {
                            all_transcripts.push(super::common::TranscriptEntry {
                                text: trimmed.clone(),
                                start_ms: chunk.start_timestamp_ms,
                                end_ms: chunk.end_timestamp_ms,
                                speaker: String::new(),
                            });
                        }
                        emit_partial(
                            &app,
                            &meeting_id,
                            i,
                            chunks_count,
                            &trimmed,
                            chunk.start_timestamp_ms,
                            chunk.end_timestamp_ms,
                        );
                    }
                }
                Err(e) => {
                    warn!("Transcription failed on chunk {}: {}", i, e);
                    chunk_failures.push(format!("第 {}/{} 段: {}", i + 1, chunks_count, e));
                }
            }
            let chunk_time = chunk_start.elapsed().as_secs_f64();
            estimator.record_chunk(speech_sec, chunk_time);
            emit_progress(
                &app,
                &meeting_id,
                "transcribing",
                0.0,
                &format!(
                    "转写中 {}/{} ({:.0}s 音频)...",
                    i + 1,
                    chunks_count,
                    speech_sec
                ),
                &estimator,
                Some(chunks_count),
                Some(i + 1),
            );
        }
    }

    let transcribed_count = all_transcripts.len();
    info!(
        "Transcription complete: {} segments transcribed",
        transcribed_count
    );

    // 部分分片失败：结果不完整但可用 → 作为用户可见告警（不报错、保留结果）
    if transcribed_count > 0 && !chunk_failures.is_empty() {
        user_warnings.push(format!(
            "有 {} 段音频识别失败，结果可能不完整：{}",
            chunk_failures.len(),
            chunk_failures[0]
        ));
    }

    // 一段结果都没拿到 → 明确报错，**不要**继续走保存（见 chunk_failures 注释）。
    if transcribed_count == 0 {
        let detail = chunk_failures
            .first()
            .cloned()
            .unwrap_or_else(|| "所有分片均未返回识别结果".to_string());
        warn!(
            "离线识别未取得任何结果（{} 个分片，{} 个失败）：{}",
            chunks_count,
            chunk_failures.len(),
            detail
        );
        return Err(anyhow!(
            "离线识别未取得任何结果（{} 段音频全部失败）：{}",
            chunks_count,
            detail
        ));
    }

    // 注意：这里**故意不再**因为取消标记而丢弃结果。
    // 取消现在由上面的 select! 在每段转写**进行中**中断（真正能中断长耗时远程请求），
    // 能走到这里说明全部音频已经识别成功。旧实现在这里丢弃结果，导致
    // 「用户点停止 → 请求跑完 → 8 段结果被扔掉」：白等 100 秒 + 空结果
    // （2026-09-20 日志 15:04:03「Transcription complete: 8 segments」紧接
    // 「Retranscription failed: Retranscription cancelled」）。已经做完的工作应当保存。
    if RETRANSCRIPTION_CANCELLED.load(Ordering::SeqCst) {
        info!("识别已完成，忽略迟到的取消请求（保存结果而不是丢弃）");
    }

    emit_progress(
        &app,
        &meeting_id,
        "saving",
        0.0,
        "保存转录结果到数据库...",
        &estimator,
        None,
        None,
    );

    let segments = create_transcript_segments(&all_transcripts);

    let app_state = app
        .try_state::<AppState>()
        .ok_or_else(|| anyhow!("App state not available"))?;

    let pool = app_state.db_manager.pool();
    let mut conn = pool
        .acquire()
        .await
        .map_err(|e| anyhow!("DB error: {}", e))?;
    let mut tx = sqlx::Connection::begin(&mut *conn)
        .await
        .map_err(|e| anyhow!("Failed to start transaction: {}", e))?;

    let now = chrono::Utc::now();

    // Only delete previous offline_asr segments, keep realtime ones
    sqlx::query(
        "DELETE FROM transcript_segments WHERE recording_id = ? AND source = 'offline_asr'",
    )
    .bind(&meeting_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| anyhow!("Failed to delete existing offline segments: {}", e))?;

    for segment in &segments {
        sqlx::query(
            "INSERT INTO transcript_segments (id, recording_id, text, start_ms, end_ms, speaker, source, created_at)
             VALUES (?, ?, ?, ?, ?, ?, 'offline_asr', ?)"
        )
        .bind(&segment.id)
        .bind(&meeting_id)
        .bind(&segment.text)
        .bind(segment.audio_start_time.map(|s| (s * 1000.0) as i64).unwrap_or(0))
        .bind(segment.audio_end_time.map(|e| (e * 1000.0) as i64))
        .bind(segment.speaker.as_deref())
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(|e| anyhow!("Failed to insert transcript segment: {}", e))?;
    }

    // Mark the recording as transcribed and persist the probed audio duration
    // (imported recordings stay 'pending' and duration-less without this).
    sqlx::query(
        "UPDATE recordings SET status = 'completed', duration_ms = ?, updated_at = ? WHERE id = ?",
    )
    .bind((duration_seconds * 1000.0) as i64)
    .bind(now)
    .bind(&meeting_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| anyhow!("Failed to update recording status/duration: {}", e))?;

    tx.commit()
        .await
        .map_err(|e| anyhow!("Failed to commit transaction: {}", e))?;

    info!(
        "Saved {} offline ASR segments for recording {}",
        segments.len(),
        meeting_id
    );

    emit_progress(
        &app,
        &meeting_id,
        "saving",
        0.5,
        "写入转录文件...",
        &estimator,
        None,
        None,
    );

    if let Err(e) = write_offline_transcripts_json(&folder_path, &segments) {
        warn!("Failed to write transcripts_offline.json: {}", e);
    }

    let audio_filename = audio_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("audio.mp4")
        .to_string();

    let elapsed_before_meta = start_time.elapsed().as_secs_f64();
    if let Err(e) = write_retranscription_metadata(
        &folder_path,
        &meeting_id,
        duration_seconds,
        elapsed_before_meta,
        &audio_filename,
        &model_used_for_metadata,
        &user_warnings,
    ) {
        warn!("Failed to update metadata.json: {}", e);
    }

    let elapsed = start_time.elapsed();
    info!(
        "离线识别完成: 音频 {:.1}s / {:.1} 分钟，识别耗时 {:.1}s（模型 {}）",
        duration_seconds,
        duration_seconds / 60.0,
        elapsed.as_secs_f64(),
        model_used_for_metadata
    );
    emit_progress(
        &app,
        &meeting_id,
        "complete",
        1.0,
        &format!("识别完成，共耗时 {:.1}s", elapsed.as_secs_f64()),
        &estimator,
        None,
        None,
    );

    Ok(RetranscriptionResult {
        meeting_id,
        segments_count: segments.len(),
        duration_seconds,
        language,
        elapsed_seconds: elapsed.as_secs_f64(),
        warnings: user_warnings,
    })
}

/// Write raw i16 PCM to a WAV file.
fn write_wav_file(path: &Path, samples: &[i16], sample_rate: u32, channels: u16) -> Result<()> {
    use std::io::Write;
    let mut file = std::fs::File::create(path)?;
    let data_size = (samples.len() * std::mem::size_of::<i16>()) as u32;
    let file_size = 44 + data_size;

    // WAV header
    file.write_all(b"RIFF")?;
    file.write_all(&(file_size - 8).to_le_bytes())?;
    file.write_all(b"WAVE")?;

    // fmt chunk
    file.write_all(b"fmt ")?;
    file.write_all(&16u32.to_le_bytes())?; // chunk size
    file.write_all(&1u16.to_le_bytes())?; // PCM format
    file.write_all(&channels.to_le_bytes())?;
    file.write_all(&sample_rate.to_le_bytes())?;
    let byte_rate = sample_rate * channels as u32 * 2;
    file.write_all(&byte_rate.to_le_bytes())?;
    file.write_all(&(channels * 2).to_le_bytes())?; // block align
    file.write_all(&16u16.to_le_bytes())?; // bits per sample

    // data chunk
    file.write_all(b"data")?;
    file.write_all(&data_size.to_le_bytes())?;
    for &s in samples {
        file.write_all(&s.to_le_bytes())?;
    }

    Ok(())
}

fn emit_progress<R: Runtime>(
    app: &AppHandle<R>,
    meeting_id: &str,
    stage: &str,
    stage_fraction: f64,
    message: &str,
    estimator: &ProgressEstimator,
    chunks_total: Option<usize>,
    chunks_processed: Option<usize>,
) {
    let progress = estimator.progress_pct(stage, stage_fraction);
    let elapsed_secs = estimator.elapsed_secs();
    let estimated_remaining = if stage == "complete" {
        None
    } else {
        let remaining = estimator.estimated_remaining(stage, stage_fraction);
        if remaining > 0.0 {
            Some(remaining)
        } else {
            None
        }
    };

    let _ = app.emit(
        "retranscription-progress",
        RetranscriptionProgress {
            meeting_id: meeting_id.to_string(),
            stage: stage.to_string(),
            progress_percentage: progress,
            message: message.to_string(),
            elapsed_seconds: Some(elapsed_secs),
            estimated_remaining_seconds: estimated_remaining,
            chunks_total,
            chunks_processed,
        },
    );
}

fn emit_partial<R: Runtime>(
    app: &AppHandle<R>,
    meeting_id: &str,
    chunk_index: usize,
    chunks_total: usize,
    text: &str,
    start_ms: f64,
    end_ms: f64,
) {
    let _ = app.emit(
        "retranscription-partial",
        RetranscriptionPartial {
            meeting_id: meeting_id.to_string(),
            chunk_index,
            chunks_total,
            text: text.to_string(),
            start_ms,
            end_ms,
        },
    );
}

/// Get or initialize the transcription engine (local Sherpa-ONNX or remote ASR)
async fn get_or_init_sherpa_onnx(
    model: Option<String>,
    provider: Option<String>,
) -> Result<Arc<dyn crate::audio::transcription::provider::TranscriptionProvider>> {
    use crate::audio::transcription::remote_asr_provider::RemoteAsrProvider;

    // Determine if remote ASR should be used:
    // 1) provider is explicitly "remote", or
    // 2) model name starts with "qwen3-asr-remote" (defense in depth)
    let is_remote = provider.as_deref() == Some("remote")
        || model
            .as_deref()
            .map(|m| m.starts_with("qwen3-asr-remote"))
            .unwrap_or(false);

    if is_remote {
        let endpoint = crate::audio::transcription::effective_remote_endpoint();
        // 离线重识别走独立的「离线远程模型」选择（非流式），与实时转录的流式选择分开。
        // 前端传的是占位名 qwen3-asr-remote（真实选择已持久化到 asr_offline）；
        // 若某路径传了真实模型 id，则直接采用（防御性支持）。
        let model_name = model
            .as_deref()
            .map(|m| m.trim())
            .filter(|m| !m.is_empty() && !m.starts_with("qwen3-asr-remote"))
            .map(|m| m.to_string())
            .unwrap_or_else(crate::audio::transcription::get_remote_asr_offline_model);
        if endpoint.is_empty() {
            // 内置默认地址兜底下实际不可达，保留作防御
            return Err(anyhow!(
                "Remote ASR endpoint not configured. Please set the remote ASR URL in Settings."
            ));
        }
        if model_name.is_empty() {
            return Err(anyhow!(
                "离线语音识别未选择远程模型：请先选择非流式的远程语音识别模型（流式模型不能用于离线重识别）"
            ));
        }
        // 2026-09-19 日志排查：离线重识别曾静默回退到实时流式模型（mimo-v2.5-asr-streaming），
        // 按流式价计费且整段音频产生重复文本。此处强制校验：流式模型一律拒绝。
        if crate::audio::transcription::is_remote_asr_streaming_model(&model_name) {
            return Err(anyhow!(
                "离线语音识别不能使用流式模型「{}」：请选择非流式模型",
                model_name
            ));
        }
        info!(
            "Using remote ASR for retranscription: {} (model: {})",
            endpoint, model_name
        );
        let remote_provider =
            RemoteAsrProvider::create_with_model_detection(&endpoint, &model_name, false)
                .await
                .map_err(|e| {
                    anyhow!("Failed to initialize remote ASR for retranscription: {}", e)
                })?;
        info!("Remote ASR health check passed");
        return Ok(Arc::new(remote_provider));
    }

    // Use specified local model, defaulting to "sense-voice"
    let model_name = model.unwrap_or_else(|| "sense-voice".to_string());

    if !crate::sherpa_onnx_engine::commands::sherpa_onnx_is_model_loaded()
        .await
        .unwrap_or(false)
    {
        info!(
            "Auto-loading Sherpa-ONNX model for retranscription: {}",
            model_name
        );
        crate::sherpa_onnx_engine::commands::sherpa_onnx_load_model(model_name)
            .await
            .map_err(|e| anyhow!("Failed to load Sherpa-ONNX model: {}", e))?;
    }

    let engine = crate::sherpa_onnx_engine::commands::get_or_init_engine()
        .map_err(|e| anyhow!("Sherpa-ONNX engine not ready: {}", e))?;
    let sherpa_provider =
        crate::audio::transcription::sherpa_onnx_provider::SherpaOnnxProvider::new(engine);
    Ok(Arc::new(sherpa_provider))
}

fn write_retranscription_metadata(
    folder: &Path,
    meeting_id: &str,
    duration_seconds: f64,
    elapsed_seconds: f64,
    audio_filename: &str,
    model_used: &str,
    warnings: &[String],
) -> Result<()> {
    let metadata_path = folder.join("metadata.json");
    let temp_path = folder.join(".metadata.json.tmp");
    let now = chrono::Utc::now().to_rfc3339();

    let json = if metadata_path.exists() {
        let existing = std::fs::read_to_string(&metadata_path)?;
        let mut value: serde_json::Value = serde_json::from_str(&existing)?;
        if let Some(obj) = value.as_object_mut() {
            obj.insert("retranscribed_at".to_string(), serde_json::json!(now));
            obj.insert("status".to_string(), serde_json::json!("completed"));
            obj.insert(
                "transcript_file".to_string(),
                serde_json::json!("transcripts_offline.json"),
            );
            obj.insert("source".to_string(), serde_json::json!("offline_asr"));
            // 音频时长与识别耗时（历史页展示「音频 X 分 Y 秒 · 识别用时 Z 秒」）。
            //
            // ⚠️ 2026-09-22 修复：旧实现在「metadata.json 已存在」分支里**从不更新
            // duration_seconds** —— 录音流程写下的 `duration_seconds: 0.0` 会一直留着，
            // 即使这里已经探测到真实时长（364.22s）。用户因此看不到音频长度。
            if duration_seconds > 0.0 {
                obj.insert(
                    "duration_seconds".to_string(),
                    serde_json::json!(duration_seconds),
                );
            }
            if elapsed_seconds > 0.0 {
                obj.insert(
                    "retranscribe_elapsed_seconds".to_string(),
                    serde_json::json!(elapsed_seconds),
                );
            }
            if !model_used.is_empty() {
                obj.insert(
                    "retranscribed_model".to_string(),
                    serde_json::json!(model_used),
                );
            }
            // ⚠️ 2026-09-25 修复：必须**无条件**写入，哪怕 `warnings` 是空数组。
            //
            // 旧实现是 `if !warnings.is_empty() { obj.insert(...) }`，于是：
            //   ① 用 MiMo 识别 → 命中内容审核 → 写下 "MiMo 上游内容审核拦截了 1/2 段音频…"；
            //   ② 换豆包重新识别 → 这次没有告警 → **不覆盖** → 旧告警永久留在 metadata.json；
            //   ③ 历史页每次打开都从 metadata.json 读出来显示 → 用户看到「换模型重识别后
            //      告警还在」，误以为这次也被拦截了（2026-09-25 用户实测反馈）。
            //
            // 「本次没有告警」是正常且最常见的成功情形，恰恰最需要把上一次的清掉。
            // 对比上面几个字段（duration_seconds / model_used）：它们的"空值"只在异常时出现，
            // 所以保留判断无妨；而 warnings 的空值是**正常态**，判断就是错的。
            obj.insert(
                "retranscription_warnings".to_string(),
                serde_json::json!(warnings),
            );
        }
        value
    } else {
        serde_json::json!({
            "version": "1.0",
            "meeting_id": meeting_id,
            "created_at": now,
            "completed_at": now,
            "retranscribed_at": now,
            "duration_seconds": duration_seconds,
            "retranscribe_elapsed_seconds": elapsed_seconds,
            "audio_file": audio_filename,
            "transcript_file": "transcripts_offline.json",
            "status": "completed",
            "source": "retranscription",
            "retranscribed_model": model_used,
            "retranscription_warnings": warnings
        })
    };

    let json_string = serde_json::to_string_pretty(&json)?;
    std::fs::write(&temp_path, &json_string)?;
    std::fs::rename(&temp_path, &metadata_path)?;

    info!("Wrote metadata.json to {}", metadata_path.display());
    Ok(())
}

/// Write offline ASR transcripts to transcripts_offline.json (separate from realtime transcripts.json)
fn write_offline_transcripts_json(
    folder: &Path,
    segments: &[crate::api::TranscriptSegment],
) -> Result<()> {
    let transcript_path = folder.join("transcripts_offline.json");
    let temp_path = folder.join(".transcripts_offline.json.tmp");

    let json = serde_json::json!({
        "version": "1.0",
        "source": "offline_asr",
        "last_updated": chrono::Utc::now().to_rfc3339(),
        "total_segments": segments.len(),
        "segments": segments.iter().enumerate().map(|(i, s)| {
            serde_json::json!({
                "id": s.id,
                "text": s.text,
                "timestamp": s.timestamp,
                "audio_start_time": s.audio_start_time,
                "audio_end_time": s.audio_end_time,
                "duration": s.duration,
                "speaker": s.speaker,
                "sequence_id": i
            })
        }).collect::<Vec<_>>()
    });

    let json_string = serde_json::to_string_pretty(&json)?;
    std::fs::write(&temp_path, &json_string)?;
    std::fs::rename(&temp_path, &transcript_path)?;

    info!(
        "Wrote transcripts_offline.json with {} segments to {}",
        segments.len(),
        transcript_path.display()
    );
    Ok(())
}

// Tauri commands

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetranscriptionStarted {
    pub meeting_id: String,
    pub message: String,
}

#[tauri::command]
pub async fn start_retranscription_command<R: Runtime>(
    app: AppHandle<R>,
    meeting_id: String,
    meeting_folder_path: String,
    language: Option<String>,
    model: Option<String>,
    provider: Option<String>,
    estimated_rtf: Option<f64>,
) -> Result<RetranscriptionStarted, String> {
    if RETRANSCRIPTION_IN_PROGRESS.load(Ordering::SeqCst) {
        return Err("Retranscription already in progress".to_string());
    }
    if crate::audio::merge::is_merge_in_progress() {
        return Err("A merge operation is in progress".to_string());
    }

    let meeting_id_clone = meeting_id.clone();

    tauri::async_runtime::spawn(async move {
        let result = start_retranscription(
            app,
            meeting_id_clone,
            meeting_folder_path,
            language,
            model,
            provider,
            estimated_rtf,
        )
        .await;

        if let Err(e) = result {
            error!("Retranscription failed: {}", e);
        }
    });

    Ok(RetranscriptionStarted {
        meeting_id,
        message: "Retranscription started".to_string(),
    })
}

#[tauri::command]
pub async fn cancel_retranscription_command() -> Result<(), String> {
    if !is_retranscription_in_progress() {
        return Err("No retranscription in progress".to_string());
    }
    cancel_retranscription();
    Ok(())
}

#[tauri::command]
pub async fn is_retranscription_in_progress_command() -> bool {
    is_retranscription_in_progress()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 2026-09-25 用户实测回归：**换模型重新识别后，上一次的识别告警仍然显示**。
    ///
    /// 真实场景：先用 MiMo → 命中内容审核 → 写下
    /// "MiMo 上游内容审核拦截了 1/2 段音频…"；再换豆包重识别（这次没有告警）→
    /// 旧实现因为 `if !warnings.is_empty()` **不覆盖**该字段 → 历史页每次从
    /// metadata.json 读出来照旧显示，用户以为这次也被拦截了。
    ///
    /// 这个测试钉住的是：**空告警也必须落盘**（把上一次的清掉）。
    #[test]
    fn empty_warnings_clear_previous_warnings_in_metadata() {
        let dir = tempfile::tempdir().expect("tempdir");
        let folder = dir.path();

        // ① 第一次识别：有告警（模拟 MiMo 内容审核拦截）
        let first = vec!["MiMo 上游内容审核拦截了 1/2 段音频，这部分内容未能识别".to_string()];
        write_retranscription_metadata(folder, "m1", 166.0, 12.0, "audio.wav", "MiMo-ASR", &first)
            .expect("first write");

        let read = |f: &Path| -> Vec<String> {
            let raw = std::fs::read_to_string(f.join("metadata.json")).expect("read metadata");
            let v: serde_json::Value = serde_json::from_str(&raw).expect("parse metadata");
            v.get("retranscription_warnings")
                .and_then(|w| w.as_array())
                .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
                .unwrap_or_default()
        };
        assert_eq!(read(folder), first, "第一次的告警应落盘");

        // ② 第二次重识别：**没有告警**（换豆包成功）→ 必须把上一次的清掉
        write_retranscription_metadata(folder, "m1", 166.0, 9.0, "audio.wav", "Doubao-ASR", &[])
            .expect("second write");
        assert!(
            read(folder).is_empty(),
            "重新识别无告警时，旧告警必须被清掉，实际 {:?}",
            read(folder)
        );

        // ③ 第三次又有告警 → 正常写回（确认不是"永远写空"）
        let third = vec!["部分分片失败".to_string()];
        write_retranscription_metadata(folder, "m1", 166.0, 11.0, "audio.wav", "MiMo-ASR", &third)
            .expect("third write");
        assert_eq!(read(folder), third, "新告警应正常落盘");

        // ④ 顺带确认：其它字段没有被这次改动带坏（模型名每次都要更新）
        let raw = std::fs::read_to_string(folder.join("metadata.json")).unwrap();
        let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(v.get("retranscribed_model").and_then(|m| m.as_str()), Some("MiMo-ASR"));
    }

    /// 2026-09-22 用户实测回归：「离线识别进度条到 50% 左右突然结束」。
    ///
    /// 真实数据（53s 音频 / 43.5s 语音 / MiMo）：解码 0.8s、VAD 0.4s、识别 7.5s、保存 ~0.05s。
    /// 旧模型把「保存」按固定 5s 预约 → 保存阶段独占 57% 进度、识别阶段只剩 30%，
    /// 于是识别一结束条子就从 ~43% 跳到 100%。现在保存只留 0.3s 余量 +
    /// 识别用「固定开销 5s + 0.05×语音」估 → 接近完成时条子应已在 90% 以上。
    #[test]
    fn progress_reflects_transcribe_time_not_save_reserve() {
        let est = ProgressEstimator::new(Instant::now(), 52.82, 52.82 * 0.8, 0.05, 5.0);
        est.record_decode_done(0.8);
        est.record_vad_done(0.4);
        est.set_total_speech_sec(43.5);

        // 识别进行中（心跳爬到 95%）：应已接近满格
        let during = est.progress_pct("transcribing", 0.95);
        assert!(during >= 85, "识别接近完成时进度应 ≥85%，实际 {}", during);

        // 分片真正返回后（processed = 全部语音）：不应低于识别结束前太多
        est.record_chunk(43.5, 7.5);
        let after_chunk = est.progress_pct("transcribing", 0.0);
        assert!(after_chunk >= 90, "分片完成后应 ≥90%，实际 {}", after_chunk);

        // 保存阶段只占很小一段，且进度不得回退
        let saving = est.progress_pct("saving", 0.0);
        assert!(
            saving >= after_chunk,
            "进度不得回退：{} → {}",
            after_chunk,
            saving
        );
        assert_eq!(est.progress_pct("complete", 1.0), 100);
    }

    /// 远程模型的耗时模型必须体现「固定开销」：43.5s 语音的 MiMo 实测 7.5s，
    /// 单靠 RTF 会估成 2.6s（心跳提前爬满、ETA 骗人）。
    #[test]
    fn remote_duration_model_includes_fixed_overhead() {
        let mimo = remote_offline_model("mimo-v2.5-asr");
        let e1 = mimo.expected_secs(43.5);
        assert!(
            (6.0..=9.0).contains(&e1),
            "MiMo 43.5s 语音应估 6~9s，实际 {:.1}s",
            e1
        );
        let e2 = mimo.expected_secs(194.0);
        assert!(
            (12.0..=18.0).contains(&e2),
            "MiMo 194s 语音应估 12~18s，实际 {:.1}s",
            e2
        );

        let dg = remote_offline_model("deepgram-nova-3");
        let e3 = dg.expected_secs(174.0);
        assert!(
            (4.0..=9.0).contains(&e3),
            "Deepgram 174s 应估 4~9s，实际 {:.1}s",
            e3
        );
    }

    #[test]
    fn test_create_transcript_segments_empty() {
        let transcripts: Vec<crate::audio::common::TranscriptEntry> = vec![];
        let segments = create_transcript_segments(&transcripts);
        assert!(segments.is_empty());
    }

    #[test]
    fn test_create_transcript_segments_single() {
        let transcripts = vec![crate::audio::common::TranscriptEntry {
            text: "Hello world".to_string(),
            start_ms: 0.0,
            end_ms: 1500.0,
            speaker: String::new(),
        }];
        let segments = create_transcript_segments(&transcripts);

        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].text, "Hello world");
        assert_eq!(segments[0].audio_start_time, Some(0.0));
        assert_eq!(segments[0].audio_end_time, Some(1.5));
        assert_eq!(segments[0].duration, Some(1.5));
    }

    #[test]
    fn test_cancellation_flag() {
        RETRANSCRIPTION_CANCELLED.store(false, Ordering::SeqCst);
        RETRANSCRIPTION_IN_PROGRESS.store(false, Ordering::SeqCst);
        assert!(!is_retranscription_in_progress());
        cancel_retranscription();
        assert!(RETRANSCRIPTION_CANCELLED.load(Ordering::SeqCst));
        RETRANSCRIPTION_CANCELLED.store(false, Ordering::SeqCst);
    }

    /// 「停止识别」必须能中断**在途**的长耗时请求：wait_cancelled + select! 是关键。
    /// 旧实现只在整块音频转写结束后才检查取消标记，远程文件转写要跑 60~110s，
    /// 期间点停止完全没反应（2026-09-20 用户实测）。
    #[tokio::test]
    async fn test_wait_cancelled_interrupts_inflight_work() {
        RETRANSCRIPTION_CANCELLED.store(false, Ordering::SeqCst);
        // 模拟一个 30s 的在途远程请求：取消后应远快于 30s 返回
        let started = Instant::now();
        let flag_setter = tokio::spawn(async {
            tokio::time::sleep(std::time::Duration::from_millis(150)).await;
            cancel_retranscription();
        });
        let outcome = tokio::select! {
            _ = tokio::time::sleep(std::time::Duration::from_secs(30)) => "completed",
            _ = wait_cancelled() => "cancelled",
        };
        let _ = flag_setter.await;
        assert_eq!(outcome, "cancelled");
        assert!(
            started.elapsed() < std::time::Duration::from_secs(2),
            "取消应在 200ms 内生效，实测 {:?}",
            started.elapsed()
        );
        RETRANSCRIPTION_CANCELLED.store(false, Ordering::SeqCst);
    }

    #[test]
    fn test_cancelled_error_is_recognized() {
        assert!(is_cancelled_error(&cancelled_error()));
        assert!(!is_cancelled_error(&anyhow!("boom")));
    }
}
