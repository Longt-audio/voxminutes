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

        let mut full_text = String::new();
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
                            let text = t.to_string();
                            match serde_json::from_str::<serde_json::Value>(&text) {
                                Ok(v) => {
                                    let kind = v.get("type").and_then(|k| k.as_str()).unwrap_or("");
                                    let t = v.get("text").and_then(|t| t.as_str()).unwrap_or("");
                                    match kind {
                                        "partial" => {
                                            if !t.is_empty() {
                                                full_text = t.to_string();
                                                self.emit_update(&app, t, true);
                                            }
                                        }
                                        "final" => {
                                            if !t.is_empty() {
                                                full_text = t.to_string();
                                                self.emit_update(&app, t, false);
                                            }
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
                            // 上游关闭：结束
                            channel_closed = true;
                        }
                        Some(Err(e)) => {
                            warn!("远程流式 ASR 读失败: {}", e);
                            channel_closed = true;
                        }
                    }
                }
            }

            // 输入已结束：发 end 后等待最后的 final
            if channel_closed && !end_sent {
                end_sent = true;
                let _ = ws.send(WsMessage::Text("{\"type\":\"end\"}".into())).await;
                final_wait = 100; // 最多等 ~10s 收尾
            } else if end_sent {
                if final_wait == 0 {
                    break;
                }
                final_wait -= 1;
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        }

        let _ = ws.close(None).await;
        info!("🌐 远程流式 ASR 会话结束，最终文本长度 {}", full_text.len());
    }

    fn emit_update<R: Runtime>(&self, app: &AppHandle<R>, text: &str, is_partial: bool) {
        let seq = REMOTE_STREAM_SEQUENCE.fetch_add(1, Ordering::SeqCst);
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
        if !is_partial && !text.trim().is_empty() {
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
