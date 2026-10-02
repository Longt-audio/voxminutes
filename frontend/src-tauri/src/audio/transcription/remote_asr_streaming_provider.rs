// audio/transcription/remote_asr_streaming_provider.rs
//
// 远程「流式」ASR provider：客户端通过 WebSocket 连网关 /v1/audio/realtime-asr，
// 持续喂 PCM16 音频，逐段收 partial / final 文本（真·实时，非 VAD 分段上传）。
// 网关负责桥接到上游（百炼 realtime WebSocket / MiMo SSE）。
//
// 文本处理统一走 flow.rs 流式管线（停顿分段 + 翻译窗口 + 草稿翻译）；
// 本文件只负责传输与重连，不再做任何句界/分段决策。

use super::provider::{TranscriptResult, TranscriptionError, TranscriptionProvider};
use crate::audio::AudioChunk;
use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use log::{info, warn};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use tauri::{AppHandle, Emitter, Runtime};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

static REMOTE_STREAM_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// 音频测试模式：测试会话免计费（网关 free=1，仍记用量流水、消耗显示 0）。
/// audio_test.rs 在测试开始前置位、停止/退出时复位；录音路径不受影响。
static ASR_FREE_BILLING: AtomicBool = AtomicBool::new(false);

pub fn set_asr_free_billing(v: bool) {
    ASR_FREE_BILLING.store(v, Ordering::Relaxed);
}

pub fn reset_remote_stream_sequence() {
    REMOTE_STREAM_SEQUENCE.store(0, Ordering::SeqCst);
}

/// 热切换对齐用：读/写当前序列号（与其它流式计数器统一到较大值，避免换引擎后撞 sequence_id）。
pub fn current_remote_stream_sequence() -> u64 {
    REMOTE_STREAM_SEQUENCE.load(Ordering::SeqCst)
}

pub fn set_remote_stream_sequence(v: u64) {
    REMOTE_STREAM_SEQUENCE.store(v, Ordering::SeqCst);
}

// ── 断线重连 / 读看门狗参数（2026-09-24）────────────────────────────────────
//
// 背景（2026-09-23 生产事故）：跨境下行链路 stall 18~30s，字幕冻结到用户停止录音。
// 原实现的两个问题：
//   ① 「连续 3 次重连失败就放弃整场」——18~30s 的抖动里，重连很可能连试 3 次都失败
//      （每次 connect 还可能挂在半死链路上），于是把「冻结后自愈」变成「整场不再出字」；
//   ② connect / close 都没有超时 —— 半死链路里重连本身就能挂到 OS TCP 超时（几十秒）。
/// 单次连接建立的上限（预检 `probe_streaming_channel` 早就用了 10s，运行时路径一直漏了）
const CONNECT_TIMEOUT_SECS: u64 = 10;
/// 关闭连接的上限（半死链路上 close 也可能挂在发送缓冲上）
const CLOSE_TIMEOUT_SECS: u64 = 3;
/// 读看门狗：超过这么久没有任何下行帧 → 判连接假死。网关每 5s 发一次应用层心跳
/// + 每 10s 一个 WS 原生 ping，15s ≈ 连丢 3 次心跳。
const IDLE_TIMEOUT_SECS: u64 = 15;
/// 退避档位（毫秒），超界后封顶 10s
const BACKOFFS_MS: [u64; 5] = [1_000, 2_000, 4_000, 8_000, 10_000];
/// 录音起步就连不上时的尝试次数（之后上报错误）：配置错时不能无限重连刷屏
const STARTUP_ATTEMPTS: u32 = 2;
/// 录音已停止（收尾阶段）后的尝试次数上限：用户已经停止录音，等不到字幕了
const TAIL_ATTEMPTS: u32 = 3;
/// 断线期间给用户可见提示的间隔（秒），避免每次退避都弹一次
const OUTAGE_WARN_EVERY_SECS: u64 = 30;

/// 断线后是否继续重试（纯函数，便于单测）：
///   · 录音还在继续 + 曾经连上过 → 一直重试（跨境链路抖动是分钟级的，放弃=整场没字幕）
///   · 录音还在继续 + 从未连上过 → 只试 STARTUP_ATTEMPTS 次（配置错了别刷屏）
///   · 录音已停止 → 只试 TAIL_ATTEMPTS 次把尾巴送出去
fn should_keep_retrying(channel_closed: bool, ever_connected: bool, failed: u32) -> bool {
    if channel_closed {
        failed < TAIL_ATTEMPTS
    } else if ever_connected {
        true
    } else {
        failed < STARTUP_ATTEMPTS
    }
}

/// 第 N 次重试的退避时长（纯函数，便于单测）：超界封顶。
fn backoff_ms(attempt: u32) -> u64 {
    BACKOFFS_MS[(attempt as usize).min(BACKOFFS_MS.len() - 1)]
}

/// 读看门狗判定（纯函数，便于单测）：true = 判假死、主动断开走重连。
/// 两个触发条件：太久没收到任何帧（含网关心跳），或网关心跳连续报上游假死。
fn watchdog_trip(idle: std::time::Duration) -> bool {
    idle.as_secs() >= IDLE_TIMEOUT_SECS
}

// ⚠️ 曾经还有一条「网关心跳连续报 up_ok=false → 断开重建上游会话」的规则，2026-09-24 晚删除：
//    网关的 up_ok=false 来自「有音频能量但上游长时间无帧」，而国歌/音乐/掌声这类**响但不是语音**
//    的片段上游本来就不会产帧 —— 那条规则会把正常会话判死（实测奏国歌期间每 ~35s 重连一次，
//    之后再也没出过字）。现在 up_ok=false 只记日志；真要重建上游会话，交给 socket 级硬故障
//    （网关关连接 → 本客户端的重连逻辑）。

/// http(s)://host[:port] → ws(s)://host[:port]（纯函数，便于单测）。
/// https 必须映射成 wss —— 客户端 tokio-tungstenite 因此必须编入 TLS feature，
/// 否则连 wss 会报 "TLS support not compiled in"（2026-09-22 事故）。
fn ws_base_from(base: &str) -> String {
    if let Some(rest) = base.strip_prefix("https://") {
        format!("wss://{}", rest)
    } else if let Some(rest) = base.strip_prefix("http://") {
        format!("ws://{}", rest)
    } else {
        base.to_string()
    }
}

/// 流式 ASR 的 WS URL（纯函数，便于单测）。
/// language 为空 = 不传该参数，交给上游按发音自动检测（百炼不接受字面 'auto'，见 HANDOVER 坑 #34）。
/// free = 音频测试免计费（网关 free=1）。
fn build_streaming_ws_url(base: &str, model: &str, language: &str, free: bool) -> String {
    format!(
        "{}/audio/realtime-asr?model={}{}{}",
        ws_base_from(base),
        urlencoding(model),
        if language.is_empty() {
            String::new()
        } else {
            format!("&language={}", urlencoding(language))
        },
        if free { "&free=1" } else { "" }
    )
}

/// 建带 Authorization 的握手请求并连接。`connect()`（真实转写）与
/// `probe_streaming_channel()`（预检）共用这一条路径。
/// `session` 由调用方给：真实转写传 `task_session::current_session()`（归到当前任务），
/// 预检传 `task_session::probe_session()`（一次性 id——预检不属于任何任务，
/// 用 current_session() 会把预检计入上一次录音，见 task_session.rs 顶部注释）。
async fn open_streaming_ws(
    base: &str,
    model: &str,
    license: &str,
    language: &str,
    free: bool,
    session: &str,
) -> Result<WebSocketStream<MaybeTlsStream<TcpStream>>, String> {
    let url = build_streaming_ws_url(base, model, language, free);
    let req = url
        .parse::<tokio_tungstenite::tungstenite::http::Uri>()
        .map_err(|e| format!("无效的 WS 地址: {}", e))?;
    // 自定义 header 带 Authorization（connect_async 的 request 支持）
    let mut request =
        tokio_tungstenite::tungstenite::client::IntoClientRequest::into_client_request(req)
            .map_err(|e| e.to_string())?;
    let auth_value: tokio_tungstenite::tungstenite::http::HeaderValue =
        format!("Bearer {}", license).parse().map_err(
            |e: tokio_tungstenite::tungstenite::http::header::InvalidHeaderValue| e.to_string(),
        )?;
    request.headers_mut().insert("Authorization", auth_value);
    // 任务会话（2026-09-22）：一次流式录音 = 网关侧一个任务。
    // 与本次录音期间逐块翻译带的 id 相同，网关才能把「识别 + 翻译」算进同一次录音。
    // 见 src/task_session.rs。头值只含 [A-Za-z0-9._-]，parse 不会失败；失败也不阻断连接。
    if let Ok(session) = session.parse() {
        request.headers_mut().insert("x-vox-session", session);
    }

    let (ws, _resp) = connect_async(request)
        .await
        .map_err(|e| format!("WebSocket 连接失败: {}", e))?;
    Ok(ws)
}

pub struct RemoteAsrStreamingProvider {
    endpoint: String,
    model_name: String,
    language: String,
}

impl RemoteAsrStreamingProvider {
    pub fn new(endpoint: String, model_name: String, language: String) -> Self {
        Self {
            endpoint: endpoint.trim_end_matches('/').to_string(),
            model_name,
            language,
        }
    }

    pub fn provider_name(&self) -> &'static str {
        "Remote ASR Streaming"
    }

    /// 把 f32 采样重采样到 16k 并编码为 PCM16 字节。
    fn to_pcm16_16k(data: &[f32], sample_rate: u32) -> Vec<u8> {
        let samples = if sample_rate != 16000 {
            crate::audio::audio_processing::resample_audio(data, sample_rate, 16000)
        } else {
            data.to_vec()
        };
        let mut out = Vec::with_capacity(samples.len() * 2);
        for &s in &samples {
            let v = (s.clamp(-1.0, 1.0) * 32767.0) as i16;
            out.extend_from_slice(&v.to_le_bytes());
        }
        out
    }

    /// 连网关 WebSocket，返回连接；endpoint 形如 http://host:port 或 https://。
    /// 与 `probe_streaming_channel()` 共用 `open_streaming_ws`，避免两条路径漂移。
    async fn connect(&self) -> Result<WebSocketStream<MaybeTlsStream<TcpStream>>, String> {
        let base = crate::audio::transcription::remote_api_base()
            .ok_or_else(|| "远程服务未配置（缺少服务器地址）".to_string())?;
        let license = crate::audio::transcription::get_remote_license();
        if license.is_empty() {
            return Err("远程服务未配置（缺少授权码）".to_string());
        }
        // ⚠️ 必须有超时：半死链路上 connect_async 会一直挂在 OS TCP 超时（几十秒），
        // 看门狗此时并不在跑（读循环还没进去），用户看到的就是「一直在重连」。
        let ws = tokio::time::timeout(
            std::time::Duration::from_secs(CONNECT_TIMEOUT_SECS),
            open_streaming_ws(
                &base,
                &self.model_name,
                &license,
                &self.language,
                ASR_FREE_BILLING.load(Ordering::Relaxed),
                &crate::task_session::current_session(),
            ),
        )
        .await
        .map_err(|_| format!("连接超时（{}s 未建立）", CONNECT_TIMEOUT_SECS))??;
        info!("🌐 远程流式 ASR WebSocket 已连接: {}", base);
        Ok(ws)
    }

    /// 关闭 WS：带上限。半死链路上 close().await 会挂在发送缓冲上，把重连卡住。
    async fn close_ws(ws: &mut WebSocketStream<MaybeTlsStream<TcpStream>>) {
        let _ = tokio::time::timeout(
            std::time::Duration::from_secs(CLOSE_TIMEOUT_SECS),
            ws.close(None),
        )
        .await;
    }

    /// 断线期间给用户可见提示：首次立即提示，之后每 OUTAGE_WARN_EVERY_SECS 一次（不刷屏）。
    /// 提示只带 code，文案交给前端 i18n（中/英/日/韩）。
    fn warn_outage<R: Runtime>(        app: &AppHandle<R>,
        down_since: &mut Option<std::time::Instant>,
        last_warn: &mut Option<std::time::Instant>,
        err: &str,
    ) {
        let now = std::time::Instant::now();
        let started = *down_since.get_or_insert(now);
        let due = last_warn.map_or(true, |t| {
            now.duration_since(t).as_secs() >= OUTAGE_WARN_EVERY_SECS
        });
        if !due {
            return;
        }
        *last_warn = Some(now);
        let secs = now.duration_since(started).as_secs();
        warn!("远程流式 ASR 断线 {}s，仍在自动重连（录音未中断）：{}", secs, err);
        // 首次用 recAsrReconnecting，之后用 recAsrReconnectingLong（前端 i18n）
        let code = if secs == 0 { "reconnecting" } else { "reconnecting_long" };
        let _ = app.emit(
            "transcription-warning",
            serde_json::json!({ "code": code, "message": "" }).to_string(),
        );
    }

    /// 上报「识别链路是否还在跑」（2026-09-29）。
    ///
    /// 为什么需要：识别因**积分不足**（或鉴权/配置类错误）终止后，界面此前毫无持久痕迹——
    /// 底部常驻胶囊仍显示「实时转写中…」，用户以为还在识别，直到发现整场没有字幕。
    /// 事件只描述状态，不含文案（文案跟界面语言走，前端按 reason 取 i18n）。
    /// reason: ""（运行中）| "credits" | "config" | "unavailable" | "ended"
    fn emit_status<R: Runtime>(app: &AppHandle<R>, running: bool, reason: &str) {
        let _ = app.emit(
            "transcription-status",
            serde_json::json!({ "running": running, "reason": reason }),
        );
    }

    /// 放弃重连：上报可见错误（录音不受影响，用户可切模型/重试）。
    fn emit_giveup<R: Runtime>(
        app: &AppHandle<R>,
        failed: u32,
        ever_connected: bool,
        channel_closed: bool,
    ) {
        warn!(
            "远程流式 ASR 放弃重连（已失败 {} 次，ever_connected={} channel_closed={}）",
            failed, ever_connected, channel_closed
        );
        let _ = app.emit(
            "transcription-error",
            serde_json::json!({
                "error": "remote_asr_reconnect_failed",
                "userMessage": "远程识别连接中断且重连失败，转写已停止（录音仍在继续）。请检查网络或切换模型。",
                "actionable": true
            }),
        );
    }

    /// 持续喂音频 + 收 partial，直到音频流结束。返回完整文本。
    /// 远端会话在录音中途被关闭（上游抖动/网关重启/跨境链路假死）时自动重连：
    /// 退避期间继续接收音频并缓冲（上限约 15 秒），重连成功后先补发缓冲再继续实时喂。
    /// 终结性错误（积分不足 / 配置类错误）不重连；其余情况**录音还在继续就一直重试**
    /// （退避封顶 10s）——旧实现「连续 3 次失败就放弃整场」，会把一次十几秒的链路
    /// 抖动变成整场没有字幕（2026-09-23 跨境 stall 事故的复盘结论）。
    pub async fn run_streaming<R: Runtime>(
        &self,
        mut receiver: tokio::sync::mpsc::UnboundedReceiver<AudioChunk>,
        app: AppHandle<R>,
    ) {
        // 流式文本管线（停顿分段 + 翻译窗口 + 草稿翻译）。跨重连保持：
        // 网关新会话的累计文本变短时，管线按最长公共前缀自动对齐。
        let mut flow = super::flow::FlowPipeline::new();
        // ⚠️ 只为**豆包流式**开启「标点错位纠正」（2026-10-02 真机实测）。
        //
        // 豆包的 `definite` 分句把标点划到**下一句开头**，按它的边界直接闭合会让
        // 段落长成这样（用户实测）：
        //   00:18 …现场的气氛怎么样      ← 段尾没「？」
        //   00:45 ？好的主持人…           ← 段首是「？」
        // 而 qwen / deepgram 的 final 本身是完整句子（标点在段尾），**不需要**这个处理，
        // 且用户明确反馈「其他 asr 状态还可以」——所以严格按模型名收窄，不动其他上游。
        let cur_model = crate::audio::transcription::get_remote_asr_model();
        flow.set_upstream_final_punct_shifted(cur_model.contains("doubao"));
        if cur_model.contains("doubao") {
            log::info!("豆包流式：已启用「句末标点优先」切分（修正标点错位）");
        }
        let mut channel_closed = false;
        // 退避期缓冲上限：16kHz f32 每 chunk ≈0.6s，25 chunk ≈ 15 秒
        const MAX_PENDING_CHUNKS: usize = 25;
        // 本轮断线已失败次数（同时用作退避档位；健康会话后归零）
        let mut failed_attempts: u32 = 0;
        // 本次录音是否成功连上过：区分「配置错了一直连不上」（有限重试后报错）
        // 与「用着用着断了」（一直重试到录音结束）
        let mut ever_connected = false;
        // 断线起点 / 上次断线提示时间（提示节流用）
        let mut down_since: Option<std::time::Instant> = None;
        let mut last_outage_warn: Option<std::time::Instant> = None;
        let mut pending: std::collections::VecDeque<AudioChunk> = std::collections::VecDeque::new();
        // 识别链路终止原因（发给界面做常驻状态提示）：
        // "credits" 积分不足 | "config" 鉴权/配置/参数类终结错误
        // | "unavailable" 重连耗尽放弃 | "ended" 正常结束（录音停止）
        let mut stop_reason: &'static str = "ended";

        'session: loop {
            let mut ws = match self.connect().await {
                Ok(w) => {
                    ever_connected = true;
                    w
                }
                Err(e) => {
                    warn!("远程流式 ASR 连接失败: {}", e);
                    let failed_next = failed_attempts + 1;
                    if !should_keep_retrying(channel_closed, ever_connected, failed_next) {
                        Self::emit_giveup(&app, failed_next, ever_connected, channel_closed);
                        break 'session;
                    }
                    failed_attempts = failed_next;
                    Self::warn_outage(&app, &mut down_since, &mut last_outage_warn, &e);
                    let backoff = backoff_ms(failed_attempts - 1);
                    if !Self::drain_during_backoff(
                        &mut receiver,
                        &mut pending,
                        backoff,
                        MAX_PENDING_CHUNKS,
                    )
                    .await
                    {
                        channel_closed = true;
                    }
                    continue 'session;
                }
            };

            // 重连成功：先补发退避期缓冲的音频
            let mut flush_failed = false;
            while let Some(chunk) = pending.pop_front() {
                let pcm = Self::to_pcm16_16k(&chunk.data, chunk.sample_rate);
                if ws.send(WsMessage::Binary(pcm.into())).await.is_err() {
                    flush_failed = true;
                    break;
                }
            }
            if flush_failed {
                // 刚连上就发不出去：跟连接失败同等对待（否则会在
                // connect→flush 失败之间无退避地打转），同样受重试策略约束
                warn!("远程流式 ASR 补发缓冲音频失败，连接已断");
                let failed_next = failed_attempts + 1;
                if !should_keep_retrying(channel_closed, ever_connected, failed_next) {
                    Self::emit_giveup(&app, failed_next, ever_connected, channel_closed);
                    break 'session;
                }
                failed_attempts = failed_next;
                Self::warn_outage(&app, &mut down_since, &mut last_outage_warn, "补发缓冲音频失败");
                let backoff = backoff_ms(failed_attempts - 1);
                Self::close_ws(&mut ws).await;
                if !Self::drain_during_backoff(
                    &mut receiver,
                    &mut pending,
                    backoff,
                    MAX_PENDING_CHUNKS,
                )
                .await
                {
                    channel_closed = true;
                }
                continue 'session;
            }
            // 会话真的起来了：清空断线状态（退避档位 + 提示节流）
            failed_attempts = 0;
            down_since = None;
            last_outage_warn = None;
            // 识别链路回到运行态（重连成功也算恢复）：界面据此把「识别已停止」收回。
            Self::emit_status(&app, true, "");

            let mut end_sent = false;
            let mut final_wait = 0u32;
            let session_start = std::time::Instant::now();
            let mut got_text = false;
            // ── 诊断埋点（2026-09-30 加）──────────────────────────────────────
            // 这条链路此前**收到文本时一行日志都不打**：用户报「卡顿」「没有译文」，
            // 拿着 4 份日志也完全无法定位（音频测试那两次连一条文本记录都没有）。
            // 现在记录 首字延迟 / 每条 partial·final 的到达时刻 / 已发音频块数，
            // 并每 10s 打一条心跳 —— 「在发音频但收不到文本」从此一眼可见。
            let mut chunks_sent: u64 = 0;
            let mut texts_received: u64 = 0;
            let mut first_text_at: Option<std::time::Instant> = None;
            let mut last_text_at: Option<std::time::Instant> = None;
            let mut last_heartbeat = std::time::Instant::now();
            // 读看门狗（2026-09-23 首版，2026-09-24 接入应用层心跳）：
            // 跨境链路曾出现「能发不能收」的下行假死 —— 网关照常收到音频、照常下发文本，
            // 客户端 30+ 秒收不到任何帧，字幕一路冻结到停止录音才爆发式出现。
            // 现在网关每 5s 发一帧应用层心跳（JSON {type:hb,up_ok,up_idle_ms}）+ 每 10s
            // 一个 WS 原生 ping，所以安静时段也必然有帧；>15s 无任何帧 = 连丢 3 次心跳，
            // 判假死 → 主动断开走外层重连。
            let mut last_msg_at = std::time::Instant::now();
            // 本会话是否因远端主动关闭/读错误而结束（且输入未结束）→ 需要重连
            let mut ws_died = false;
            let mut fatal_billing = false;
            // 百炼 realtime（-realtime 后缀）：上游参数类错误（网关下发 error 消息）后
            // 网关会主动关闭连接；同参数重连必然再失败，标记会话终结不再重试。
            // mimo 桥接（-streaming 后缀）的段级 error 不关闭会话，不在此列。
            let mut fatal_param = false;
            // 配置类终结性错误（授权码无效 / 模型不可用…）：重连必然再失败。
            // 没有这一条，新策略（录音期间一直重试）会变成无限重连风暴。
            let mut fatal_config = false;

            loop {
                tokio::select! {
                    biased;
                    chunk = receiver.recv(), if !channel_closed => {
                        match chunk {
                            Some(chunk) => {
                                // 音频时钟 + RMS 有声检测（停顿分段/冲刷用）
                                if !chunk.data.is_empty() {
                                    let rms = (chunk.data.iter().map(|x| x * x).sum::<f32>()
                                        / chunk.data.len() as f32).sqrt();
                                    flow.note_audio(chunk.timestamp, rms);
                                }
                                let pcm = Self::to_pcm16_16k(&chunk.data, chunk.sample_rate);
                                let pcm_len = pcm.len();
                                if ws.send(WsMessage::Binary(pcm.into())).await.is_err() {
                                    warn!("远程流式 ASR 发送音频失败，连接已断");
                                    channel_closed = true;
                                } else {
                                    chunks_sent += 1;
                                    if chunks_sent == 1 {
                                        info!("🎙️ 远程流式 ASR 开始上行音频（首块 {} 字节）", pcm_len);
                                    }
                                    // 心跳挂在发送侧：音频块持续到达，所以无需往 select 里加定时分支
                                    // （不动 select 结构 = 不去碰那条已经调好的重连/看门狗逻辑）
                                    if last_heartbeat.elapsed() >= std::time::Duration::from_secs(10) {
                                        last_heartbeat = std::time::Instant::now();
                                        info!(
                                            "💓 远程流式 ASR 心跳：已发音频 {} 块 / 已收文本 {} 条 / 距上次文本 {}",
                                            chunks_sent,
                                            texts_received,
                                            match last_text_at {
                                                Some(t) => format!("{:.1}s", t.elapsed().as_secs_f32()),
                                                None => "从未收到".to_string(),
                                            }
                                        );
                                    }
                                }
                            }
                            None => {
                                channel_closed = true;
                            }
                        }
                    }
                    msg = ws.next() => {
                        // 任何帧（文本/二进制/ping/pong/close）都算存活证据，重置看门狗
                        if matches!(msg, Some(Ok(_))) {
                            last_msg_at = std::time::Instant::now();
                        }
                        match msg {
                            Some(Ok(WsMessage::Text(t))) => {
                                let raw = t.to_string();
                                match serde_json::from_str::<serde_json::Value>(&raw) {
                                    Ok(v) => {
                                        let kind = v.get("type").and_then(|k| k.as_str()).unwrap_or("");
                                        let payload = v.get("text").and_then(|t| t.as_str()).unwrap_or("");
                                        match kind {
                                            "partial" | "final" => {
                                                got_text = true;
                                                texts_received += 1;
                                                let now = std::time::Instant::now();
                                                let text_len = payload.chars().count();
                                                if first_text_at.is_none() {
                                                    first_text_at = Some(now);
                                                    info!(
                                                        "🎯 远程流式 ASR 首字延迟 {:.2}s（WS 连上→首条文本，{} 字）",
                                                        session_start.elapsed().as_secs_f32(),
                                                        text_len
                                                    );
                                                }
                                                last_text_at = Some(now);
                                                // 只记长度不记正文（日志可能被用户上传，避免带出会议内容）
                                                info!(
                                                    "📝 远程流式 ASR 收到 {}（t+{:.2}s，{} 字，本会话第 {} 条）",
                                                    kind,
                                                    session_start.elapsed().as_secs_f32(),
                                                    text_len,
                                                    texts_received
                                                );
                                                // final = 上游定稿边界；管线只在稳定文本内闭合单元
                                                // 远程流式：final 既是「定稿」也是「语义边界」——上游的 final 是按
                                                // 静音 endpointing 切出来的，本来就该作为分段依据
                                                flow.push_text(&app, payload, kind == "final", kind == "final");
                                            }
                                            "error" => {
                                                let m = v.get("message").and_then(|m| m.as_str()).unwrap_or("");
                                                warn!("远程流式 ASR 网关错误: {}", m);
                                                let _ = app.emit("transcription-warning", m.to_string());
                                                // 积分不足是终结性错误：网关随后会关闭连接，不做无意义重连
                                                if m.contains("积分不足") {
                                                    fatal_billing = true;
                                                }
                                                // 配置类终结性错误（授权码/模型/鉴权）：重连必然再失败。
                                                // 必须判出来，否则「录音期间一直重试」会变成无限重连风暴。
                                                const FATAL_MARKERS: [&str; 6] = [
                                                    "授权码无效",
                                                    "账号已停用",
                                                    "未知 ASR 模型",
                                                    "已下架",
                                                    "暂不可用",
                                                    "不是流式",
                                                ];
                                                if FATAL_MARKERS.iter().any(|k| m.contains(k)) {
                                                    fatal_config = true;
                                                }
                                                // 百炼 realtime 的上游错误（如 language='auto' 被拒）：
                                                // 重连带同样的参数必然再失败，直接终结会话并保留已识别文本。
                                                if self.model_name.ends_with("-realtime") {
                                                    fatal_param = true;
                                                }
                                            }
                                            "warning" => {
                                                // 网关主动提示（如 code=upstream_stalled：网关↔上游
                                                // 那段假死，网关会先提示再断开）。转成客户端可见 toast：
                                                // 带 code 的 JSON 由前端 useTranscriptionErrorToasts 解析。
                                                let code = v.get("code").and_then(|c| c.as_str()).unwrap_or("");
                                                let msg = v.get("message").and_then(|m| m.as_str()).unwrap_or("");
                                                warn!("远程流式 ASR 网关提示 code={} message={}", code, msg);
                                                let _ = app.emit(
                                                    "transcription-warning",
                                                    serde_json::json!({ "code": code, "message": msg }).to_string(),
                                                );
                                            }
                                            "hb" => {
                                                // 应用层心跳：任何帧都已重置 last_msg_at；这里只看上游活性
                                                let up_ok = v.get("up_ok").and_then(|b| b.as_bool()).unwrap_or(true);
                                                if !up_ok {
                                                    // 只记日志（原因见 watchdog_trip 上方注释）：上游长时间没帧
                                                    // 也可能是音乐/掌声，不适合据此断会话
                                                    let idle = v.get("up_idle_ms").and_then(|n| n.as_i64()).unwrap_or(-1);
                                                    warn!("网关心跳报告上游长时间无帧（up_idle={}ms）—— 仅记录", idle);
                                                }
                                            }
                                            _ => {}
                                        }
                                    }
                                    Err(_) => {}
                                }
                            }
                            Some(Ok(WsMessage::Binary(_))) => {}
                            Some(Ok(WsMessage::Ping(p))) => {
                                // 自动回 Pong
                                let _ = ws.send(WsMessage::Pong(p)).await;
                            }
                            Some(Ok(WsMessage::Pong(_))) | Some(Ok(WsMessage::Frame(_))) => {}
                            Some(Ok(WsMessage::Close(_))) | None => {
                                // 远端关闭：若输入未结束则视为异常中断（外层决定是否重连）
                                ws_died = !channel_closed;
                                break;
                            }
                            Some(Err(e)) => {
                                warn!("远程流式 ASR 读失败: {}", e);
                                ws_died = !channel_closed;
                                break;
                            }
                        }
                    }
                    // 收尾阶段倒计时：即使上游不发送/不关闭，也能在 ~10s 内退出，避免永久卡死
                    _ = tokio::time::sleep(std::time::Duration::from_millis(100)), if end_sent => {
                        if final_wait == 0 {
                            break;
                        }
                        final_wait -= 1;
                    }
                    // 管线周期检查：停顿冲刷翻译、停顿分段、停滞兜底（录音进行中每 500ms）
                    _ = tokio::time::sleep(std::time::Duration::from_millis(500)), if !channel_closed && !end_sent => {
                        flow.tick(&app);
                        // 读看门狗：>15s 无任何下行帧（网关 5s 应用层心跳 + 10s WS ping 都缺席，
                        // = 连丢 3 次心跳）或网关心跳连续报上游假死 → 判假死，断开走重连，
                        // 而不是无限期冻结
                        let idle = last_msg_at.elapsed();
                        if watchdog_trip(idle) {
                            warn!(
                                "远程流式 ASR 超过 {:?} 未收到任何下行帧（含网关心跳），判定连接假死，主动断开重连",
                                idle
                            );
                            ws_died = true;
                            break;
                        }
                    }
                }

                // 输入已结束：发 end 后等待最后的 final
                if channel_closed && !end_sent {
                    end_sent = true;
                    let _ = ws.send(WsMessage::Text("{\"type\":\"end\"}".into())).await;
                    final_wait = 100; // 最多等 ~10s 收尾
                }
            }

            // 健康会话（出过字或存活超过 10s）→ 退避档位归零：新一轮断线从小退避重来
            if got_text || session_start.elapsed().as_secs() >= 10 {
                failed_attempts = 0;
            }

            // 正常结束（录音停止、输入流关闭）或终结性错误：退出大循环
            if !ws_died || fatal_billing || fatal_param || fatal_config {
                if ws_died && fatal_billing {
                    warn!("远程流式 ASR 因积分不足终止，不重连");
                }
                if ws_died && fatal_param {
                    warn!("远程流式 ASR 因上游参数错误终止，不重连（重连必然再失败）");
                }
                if ws_died && fatal_config {
                    warn!("远程流式 ASR 因配置/鉴权类错误终止，不重连（重连必然再失败）");
                }
                if ws_died && fatal_billing {
                    stop_reason = "credits";
                } else if ws_died && (fatal_config || fatal_param) {
                    stop_reason = "config";
                }
                Self::close_ws(&mut ws).await;
                break 'session;
            }

            // 异常中断：退避后重连。录音还在继续 → 一直重试（退避封顶 10s）；
            // 录音已停止 → 只再试有限次把尾巴送出去，然后放弃。
            let failed_next = failed_attempts + 1;
            if !should_keep_retrying(channel_closed, ever_connected, failed_next) {
                Self::emit_giveup(&app, failed_next, ever_connected, channel_closed);
                stop_reason = "unavailable";
                Self::close_ws(&mut ws).await;
                break 'session;
            }
            failed_attempts = failed_next;
            let backoff = backoff_ms(failed_attempts - 1);
            warn!(
                "远程流式 ASR 会话中断，{}ms 后重连（本轮第 {} 次，录音未中断）…",
                backoff, failed_attempts
            );
            Self::warn_outage(&app, &mut down_since, &mut last_outage_warn, "会话中断");
            Self::close_ws(&mut ws).await;
            if !Self::drain_during_backoff(&mut receiver, &mut pending, backoff, MAX_PENDING_CHUNKS)
                .await
            {
                channel_closed = true;
            }
        }

        // 收尾：闭合活跃单元尾巴，避免丢字
        flow.finish(&app);
        info!("🌐 远程流式 ASR 会话结束");
        // 常驻状态：识别链路终止（"ended" 表示只是录音正常停止，界面无需额外提示）。
        Self::emit_status(&app, false, stop_reason);
    }

    /// 退避期内继续接收音频 chunk 并缓冲（掉线不丢音）。返回 false 表示输入流已结束。
    async fn drain_during_backoff(
        receiver: &mut tokio::sync::mpsc::UnboundedReceiver<AudioChunk>,
        pending: &mut std::collections::VecDeque<AudioChunk>,
        backoff_ms: u64,
        max_chunks: usize,
    ) -> bool {
        let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_millis(backoff_ms);
        loop {
            match tokio::time::timeout_at(deadline, receiver.recv()).await {
                Ok(Some(chunk)) => {
                    pending.push_back(chunk);
                    while pending.len() > max_chunks {
                        pending.pop_front();
                    }
                }
                Ok(None) => return false,
                Err(_) => return true,
            }
        }
    }
}

fn urlencoding(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.' || b == b'~' {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{:02X}", b));
        }
    }
    out
}

// 让 provider 也能走统一 trait（虽非流式路径会用它，但保留兼容）
#[async_trait]
impl TranscriptionProvider for RemoteAsrStreamingProvider {
    async fn transcribe(
        &self,
        _audio: Vec<f32>,
        _language: Option<String>,
    ) -> Result<TranscriptResult, TranscriptionError> {
        Err(TranscriptionError::EngineFailed(
            "流式 ASR 应走 run_streaming，不走 transcribe".to_string(),
        ))
    }

    async fn is_model_loaded(&self) -> bool {
        crate::audio::transcription::is_remote_asr_configured()
    }

    async fn get_current_model(&self) -> Option<String> {
        Some(self.model_name.clone())
    }

    fn provider_name(&self) -> &'static str {
        "Remote ASR Streaming"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// 流式识别通道预检：按当前配置（地址/授权码/模型/语言）**真实握手一次**，随即关闭。
///
/// 为什么不复用 HTTP `/health`：`/health` 走 reqwest（自带 TLS），只能证明「服务器在线」，
/// 证明不了流式 WS 通道可用（TLS feature、鉴权、路由、上游模型）。2026-09-22 事故正是
/// 客户端 tokio-tungstenite 未编入 TLS：`/health` 全绿、模型校验打「✅ Remote ASR ready」，
/// 但一开录就 0 字符。修法就是让预检真的握一次 wss。
///
/// 费用与延迟：连上即关、不发送任何音频；网关按实收 PCM 字节计费 → 0 音频 0 扣费。
/// 成功结果缓存 5 分钟（2026-09-28 从 20s 拉长：20s 连「开完一场会再开下一场」都覆盖不到，
/// 每次录音起步都白付一次 ~3.5s 跨境握手；拉长后配合「启动/打开录音弹窗/切换模型」三处
/// 提前预热（lib.rs warm_remote_streaming），点击开始时基本稳命中缓存）。
/// cache key 含地址/模型/授权码/语言，改配置自动失效；
/// **失败不缓存** —— 用户改完配置（换地址/补授权码）重试必须立刻重新探测。
pub async fn probe_streaming_channel() -> Result<(), String> {
    use std::sync::Mutex;
    use std::time::{Duration, Instant};

    static OK_CACHE: Mutex<Option<(String, Instant)>> = Mutex::new(None);
    const CACHE_TTL: Duration = Duration::from_secs(300);
    const TIMEOUT: Duration = Duration::from_secs(10);

    let base = crate::audio::transcription::remote_api_base()
        .ok_or_else(|| "远程服务未配置（缺少服务器地址）".to_string())?;
    let model = crate::audio::transcription::get_remote_asr_model();
    if model.is_empty() {
        return Err("未选择远程流式识别模型".to_string());
    }
    let license = crate::audio::transcription::get_remote_license();
    if license.is_empty() {
        return Err("远程服务未配置（缺少授权码）".to_string());
    }
    // 与 engine.rs 建 provider 时同一套语言语义：'auto'/空 → 不传 language
    let pref = crate::get_language_preference_internal().unwrap_or_else(|| "auto".to_string());
    let language = if pref.trim().is_empty() || pref == "auto" {
        String::new()
    } else {
        pref
    };

    let cache_key = format!("{}|{}|{}|{}", base, model, license, language);
    if let Ok(guard) = OK_CACHE.lock() {
        if let Some((key, at)) = guard.as_ref() {
            if key == &cache_key && at.elapsed() < CACHE_TTL {
                return Ok(());
            }
        }
    }

    let mut ws = tokio::time::timeout(
        TIMEOUT,
        // 预检不属于任何任务：用一次性 probe id（不看不写 CURRENT），
        // 否则会把这次秒关的预检计入上一次录音的任务（见 task_session.rs 顶部注释）。
        open_streaming_ws(
            &base,
            &model,
            &license,
            &language,
            false,
            &crate::task_session::probe_session(),
        ),
    )
    .await
    .map_err(|_| format!("流式识别连接超时（{}s）", TIMEOUT.as_secs()))??;
    // 立即关闭：不发音频 → 不产生费用
    let _ = ws.close(None).await;

    if let Ok(mut guard) = OK_CACHE.lock() {
        *guard = Some((cache_key, Instant::now()));
    }
    info!("✅ 远程流式识别通道预检通过: {} model={}", base, model);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ws_scheme_follows_http_scheme() {
        // https → wss（生产路径；TLS feature 缺失时就在这里暴露）
        assert_eq!(
            ws_base_from("https://api.voxmin.top"),
            "wss://api.voxmin.top"
        );
        // 本地隧道/自建 http → ws 明文
        assert_eq!(ws_base_from("http://127.0.0.1:8788"), "ws://127.0.0.1:8788");
        // remote_api_base() 会带 /v1，必须原样保留
        assert_eq!(
            ws_base_from("https://api.voxmin.top/v1"),
            "wss://api.voxmin.top/v1"
        );
    }

    #[test]
    fn ws_url_includes_model_language_free() {
        assert_eq!(
            build_streaming_ws_url(
                "https://api.voxmin.top/v1",
                "deepgram-nova-3-streaming",
                "zh",
                true
            ),
            "wss://api.voxmin.top/v1/audio/realtime-asr?model=deepgram-nova-3-streaming&language=zh&free=1"
        );
        // language 为空（auto）→ 不拼 language 参数（百炼不接受字面 'auto'）
        assert_eq!(
            build_streaming_ws_url(
                "http://127.0.0.1:8788/v1",
                "qwen-audio-3.0-asr-flash-streaming",
                "",
                false
            ),
            "ws://127.0.0.1:8788/v1/audio/realtime-asr?model=qwen-audio-3.0-asr-flash-streaming"
        );
    }

    #[test]
    fn ws_url_encodes_model_and_language() {
        let url = build_streaming_ws_url("https://h/v1", "a/b c", "zh-Hans", false);
        assert!(url.contains("model=a%2Fb%20c"), "model 未编码: {}", url);
        assert!(
            url.contains("language=zh-Hans"),
            "language 被误编码: {}",
            url
        );
    }

    // ── 断线重连策略（2026-09-24）────────────────────────────────────────
    // 事故复盘：跨境 stall 18~30s，旧策略「连续 3 次失败放弃整场」会把
    // 「冻结后自愈」变成「整场没有字幕」，所以改成「录音期间一直重试」。

    #[test]
    fn retry_keeps_going_while_recording_continues() {
        // 曾经连上过 + 录音还在继续 → 无论失败多少次都继续（退避封顶 10s）
        for failed in 1..50 {
            assert!(
                should_keep_retrying(false, true, failed),
                "录音中第 {} 次失败后不应放弃",
                failed
            );
        }
    }

    #[test]
    fn retry_startup_is_bounded() {
        // 从未连上过（配置错/服务不可达）→ 只试 STARTUP_ATTEMPTS 次，避免无限重连刷屏
        assert!(should_keep_retrying(false, false, 1));
        assert!(!should_keep_retrying(false, false, STARTUP_ATTEMPTS));
        assert!(!should_keep_retrying(false, false, STARTUP_ATTEMPTS + 1));
    }

    #[test]
    fn retry_tail_is_bounded_after_recording_stops() {
        // 录音已停止：只再试 TAIL_ATTEMPTS 次把尾巴送出去，然后放弃
        assert!(should_keep_retrying(true, true, 1));
        assert!(should_keep_retrying(true, true, TAIL_ATTEMPTS - 1));
        assert!(!should_keep_retrying(true, true, TAIL_ATTEMPTS));
    }

    #[test]
    fn backoff_grows_then_caps() {
        assert_eq!(backoff_ms(0), 1_000);
        assert_eq!(backoff_ms(1), 2_000);
        assert_eq!(backoff_ms(2), 4_000);
        assert_eq!(backoff_ms(3), 8_000);
        assert_eq!(backoff_ms(4), 10_000);
        // 超出档位封顶，不会 panic（旧实现 BACKOFFS_MS[attempt] 越界会 panic）
        assert_eq!(backoff_ms(5), 10_000);
        assert_eq!(backoff_ms(99), 10_000);
    }

    #[test]
    fn watchdog_trips_on_idle_frames() {
        use std::time::Duration;
        // 边界：刚好到阈值即判（15s = 连丢 3 次 5s 心跳）
        assert!(!watchdog_trip(Duration::from_millis(14_900)));
        assert!(watchdog_trip(Duration::from_secs(IDLE_TIMEOUT_SECS)));
        // 注意：up_ok=false **不再**参与判定（音乐/掌声会让上游长时间无帧，据此断会话会误杀）
        // —— 回归见 2026-09-24 晚「奏国歌后再无字幕」事故。
    }
}
