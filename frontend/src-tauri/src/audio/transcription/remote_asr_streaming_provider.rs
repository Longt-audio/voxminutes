// audio/transcription/remote_asr_streaming_provider.rs
//
// 远程「流式」ASR provider：客户端通过 WebSocket 连网关 /v1/audio/realtime-asr，
// 持续喂 PCM16 音频，逐段收 partial / final 文本（真·实时，非 VAD 分段上传）。
// 网关负责桥接到上游（百炼 realtime WebSocket / MiMo SSE）。

use super::provider::{TranscriptResult, TranscriptionError, TranscriptionProvider};
use crate::audio::AudioChunk;
use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use log::{info, warn};
use std::sync::atomic::{AtomicU64, Ordering};
use tauri::{AppHandle, Emitter, Runtime};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

static REMOTE_STREAM_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// 提交一个完整句子的最小文本单元数（CJK 字 + 非 CJK 词）。
const MIN_SEGMENT_UNITS: usize = 8;
/// 长度兜底软阈值：无强句界且未提交文本达到该值时，在最后一个弱边界处提交。
const SOFT_COMMIT_UNITS: usize = 48;
/// 长度兜底硬阈值：达到该值仍无可用弱边界时硬切提交。
const HARD_COMMIT_UNITS: usize = 80;

pub fn reset_remote_stream_sequence() {
    REMOTE_STREAM_SEQUENCE.store(0, Ordering::SeqCst);
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
    async fn connect(&self) -> Result<WebSocketStream<MaybeTlsStream<TcpStream>>, String> {
        let base = crate::audio::transcription::remote_api_base()
            .ok_or_else(|| "远程服务未配置（缺少服务器地址）".to_string())?;
        let license = crate::audio::transcription::get_remote_license();
        if license.is_empty() {
            return Err("远程服务未配置（缺少授权码）".to_string());
        }
        // http(s)://host[:port] → ws(s)://host[:port]
        let ws_base = if let Some(rest) = base.strip_prefix("https://") {
            format!("wss://{}", rest)
        } else if let Some(rest) = base.strip_prefix("http://") {
            format!("ws://{}", rest)
        } else {
            base.clone()
        };
        let url = format!(
            "{}/audio/realtime-asr?model={}&language={}",
            ws_base,
            urlencoding(&self.model_name),
            urlencoding(&self.language)
        );
        let req = url
            .parse::<tokio_tungstenite::tungstenite::http::Uri>()
            .map_err(|e| format!("无效的 WS 地址: {}", e))?;
        // 自定义 header 带 Authorization（connect_async 的 request 支持）
        let mut request = tokio_tungstenite::tungstenite::client::IntoClientRequest::into_client_request(req)
            .map_err(|e| e.to_string())?;
        let headers = request.headers_mut();
        let auth_value: tokio_tungstenite::tungstenite::http::HeaderValue =
            format!("Bearer {}", license)
                .parse()
                .map_err(|e: tokio_tungstenite::tungstenite::http::header::InvalidHeaderValue| e.to_string())?;
        headers.insert("Authorization", auth_value);

        let (ws, _resp) = connect_async(request).await.map_err(|e| format!("WebSocket 连接失败: {}", e))?;
        info!("🌐 远程流式 ASR WebSocket 已连接: {}", base);
        Ok(ws)
    }

    /// 持续喂音频 + 收 partial，直到音频流结束。返回完整文本。
    pub async fn run_streaming<R: Runtime>(
        &self,
        mut receiver: tokio::sync::mpsc::UnboundedReceiver<AudioChunk>,
        app: AppHandle<R>,
    ) {
        let mut ws = match self.connect().await {
            Ok(w) => w,
            Err(e) => {
                warn!("远程流式 ASR 连接失败: {}", e);
                let _ = app.emit(
                    "transcription-error",
                    serde_json::json!({ "error": e, "userMessage": "远程流式 ASR 连接失败", "actionable": false }),
                );
                return;
            }
        };

        // ── 流式文本状态机：把网关下发的「会话累计文本」切成一句一行，
        //    与本地 X-ASR 一致（partial 只显示当前未提交句尾，完整句子按句界提交为 final）。
        let mut committed_len: usize = 0;
        let mut current_seq: u64 = REMOTE_STREAM_SEQUENCE.fetch_add(1, Ordering::SeqCst);
        let mut last_cumulative = String::new();
        let mut channel_closed = false;
        let mut end_sent = false;
        let mut final_wait = 0u32;

        loop {
            tokio::select! {
                biased;
                chunk = receiver.recv(), if !channel_closed => {
                    match chunk {
                        Some(chunk) => {
                            let pcm = Self::to_pcm16_16k(&chunk.data, chunk.sample_rate);
                            if ws.send(WsMessage::Binary(pcm.into())).await.is_err() {
                                warn!("远程流式 ASR 发送音频失败，连接已断");
                                channel_closed = true;
                            }
                        }
                        None => {
                            channel_closed = true;
                        }
                    }
                }
                msg = ws.next() => {
                    match msg {
                        Some(Ok(WsMessage::Text(t))) => {
                            let raw = t.to_string();
                            match serde_json::from_str::<serde_json::Value>(&raw) {
                                Ok(v) => {
                                    let kind = v.get("type").and_then(|k| k.as_str()).unwrap_or("");
                                    let payload = v.get("text").and_then(|t| t.as_str()).unwrap_or("");
                                    match kind {
                                        "partial" => {
                                            last_cumulative.clear();
                                            last_cumulative.push_str(payload);
                                            self.handle_stream_text(&app, payload, false, &mut committed_len, &mut current_seq);
                                        }
                                        "final" => {
                                            last_cumulative.clear();
                                            last_cumulative.push_str(payload);
                                            self.handle_stream_text(&app, payload, true, &mut committed_len, &mut current_seq);
                                        }
                                        "error" => {
                                            let m = v.get("message").and_then(|m| m.as_str()).unwrap_or("");
                                            let _ = app.emit("transcription-warning", m.to_string());
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
                            // 上游已关闭：无需再等，直接结束
                            break;
                        }
                        Some(Err(e)) => {
                            warn!("远程流式 ASR 读失败: {}", e);
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
            }

            // 输入已结束：发 end 后等待最后的 final
            if channel_closed && !end_sent {
                end_sent = true;
                let _ = ws.send(WsMessage::Text("{\"type\":\"end\"}".into())).await;
                final_wait = 100; // 最多等 ~10s 收尾
            }
        }

        // 收尾兜底：若上游异常关闭漏发 final，把最后未提交的尾巴提交掉，避免丢字
        if committed_len < last_cumulative.len() {
            let rest = last_cumulative[committed_len..].trim().to_string();
            if !rest.is_empty() {
                self.emit(&app, &rest, current_seq, false);
            }
        }

        let _ = ws.close(None).await;
        info!("🌐 远程流式 ASR 会话结束，最终累计文本 {} 字符", last_cumulative.len());
    }

    /// 把网关下发的一段「累计文本」推进分句状态机：
    /// - partial：把累计文本的未提交尾巴（当前句）作为 partial 原位更新；
    /// - 检测到完整句子（句界标点 / 长度兜底）则提交为 final 段落（一句一行）；
    /// - final：把剩余尾巴全部提交。
    fn handle_stream_text<R: Runtime>(
        &self,
        app: &AppHandle<R>,
        cumulative: &str,
        is_final: bool,
        committed_len: &mut usize,
        current_seq: &mut u64,
    ) {
        if cumulative.len() < *committed_len {
            // 网关重开流或文本被截断：从头开始
            *committed_len = 0;
        }

        let tail = &cumulative[*committed_len..];

        // 1) partial：当前未提交句尾原位更新
        if !tail.is_empty() {
            self.emit(app, tail, *current_seq, true);
        }

        // 2) 提交一个完整句子（强句界优先，长度兜底）
        if let Some(boundary) = find_commit_boundary(tail, MIN_SEGMENT_UNITS)
            .or_else(|| find_length_boundary(tail, SOFT_COMMIT_UNITS, HARD_COMMIT_UNITS, MIN_SEGMENT_UNITS))
        {
            let sentence = tail[..boundary].trim().to_string();
            if sentence.chars().filter(|c| !c.is_whitespace()).count() >= 2 {
                self.emit(app, &sentence, *current_seq, false);
                *committed_len += boundary;
                *current_seq = REMOTE_STREAM_SEQUENCE.fetch_add(1, Ordering::SeqCst);
            }
        }

        // 3) final：剩余尾巴全部提交
        if is_final {
            let rest = cumulative[*committed_len..].trim().to_string();
            if !rest.is_empty() {
                self.emit(app, &rest, *current_seq, false);
                *committed_len = cumulative.len();
                *current_seq = REMOTE_STREAM_SEQUENCE.fetch_add(1, Ordering::SeqCst);
            }
        }
    }

    fn emit<R: Runtime>(&self, app: &AppHandle<R>, text: &str, seq: u64, is_partial: bool) {
        if text.trim().is_empty() {
            return;
        }
        let update = super::worker::TranscriptUpdate {
            text: text.to_string(),
            timestamp: format_timestamp(),
            source: "Audio".to_string(),
            sequence_id: seq,
            chunk_start_time: 0.0,
            is_partial,
            confidence: 0.9,
            audio_start_time: 0.0,
            audio_end_time: 0.0,
            duration: 0.0,
        };
        let _ = app.emit("transcript-update", &update);
        if !is_partial {
            crate::translation::queue_translation(app, text, seq);
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

/// 在文本中找到第一个「完整句界」之后的切分字节偏移（前缀至少 min_units 个单元）。
/// 句界标点：。！？.!?，其后须为空白/引号/CJK 字符（与本地 X-ASR 同款口径）。
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
                    || matches!(n, '"' | '」' | '）' | '】')
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

/// 长度兜底：无标点语流下，达到 soft 单元时在最后一个弱边界（，、；：,; 或空格）提交；
/// 达到 hard 单元仍无可用弱边界时硬切。返回切分字节偏移。
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
