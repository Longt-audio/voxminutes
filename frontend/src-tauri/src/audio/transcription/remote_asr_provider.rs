// audio/transcription/remote_asr_provider.rs
//
// Remote ASR provider —— 走开发者网关的 OpenAI 兼容端点：
//   POST {server_url}/v1/audio/transcriptions  (multipart: file + language)
// 客户端只带 Authorization: Bearer <license>，不接触上游 API key。
// 网关负责鉴权、路由到真实 ASR 上游、计量、扣积分。

use async_trait::async_trait;
use log::info;
use std::sync::Arc;

use super::provider::{TranscriptResult, TranscriptionError, TranscriptionProvider};

/// Context for the current chunk being transcribed (kept for API compatibility
/// with the streaming call sites in `engine.rs`).
pub struct ChunkContext {
    pub sequence_id: u64,
    pub chunk_start_time: f64,
    pub audio_start_time: f64,
    pub audio_end_time: f64,
    pub duration: f64,
}

pub struct RemoteAsrProvider {
    endpoint: String,
    model_name: String,
    chunk_context: Arc<std::sync::Mutex<Option<ChunkContext>>>,
}

/// Encode 16kHz mono f32 samples as a 16-bit PCM WAV (44-byte header).
fn encode_wav_16k(samples: &[f32]) -> Vec<u8> {
    let data_size = (samples.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + data_size as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_size).to_le_bytes());
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&1u16.to_le_bytes()); // mono
    out.extend_from_slice(&16000u32.to_le_bytes());
    out.extend_from_slice(&32000u32.to_le_bytes()); // byte rate
    out.extend_from_slice(&2u16.to_le_bytes()); // block align
    out.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_size.to_le_bytes());
    for &s in samples {
        let v = (s.clamp(-1.0, 1.0) * 32767.0) as i16;
        out.extend_from_slice(&v.to_le_bytes());
    }
    out
}

impl RemoteAsrProvider {
    pub fn new(endpoint: String, model_name: String) -> Self {
        Self {
            endpoint: endpoint.trim_end_matches('/').to_string(),
            model_name,
            chunk_context: Arc::new(std::sync::Mutex::new(None)),
        }
    }

    pub fn new_streaming(
        endpoint: String,
        model_name: String,
        _partial_emitter: Arc<dyn Fn(&str, bool) + Send + Sync>,
        chunk_context: Arc<std::sync::Mutex<Option<ChunkContext>>>,
    ) -> Self {
        let mut provider = Self::new(endpoint, model_name);
        provider.chunk_context = chunk_context;
        provider
    }

    /// 网关健康检查（/health 无需鉴权）。
    pub async fn check_health(&self) -> bool {
        check_remote_asr_health(&self.endpoint).await
    }

    pub async fn detect_model_name(&self) -> String {
        self.model_name.clone()
    }

    pub async fn create_with_model_detection(
        endpoint: &str,
        configured_model: &str,
        _is_streaming: bool,
    ) -> Result<Self, String> {
        Ok(Self::new(
            endpoint.to_string(),
            configured_model.to_string(),
        ))
    }
}

#[async_trait]
impl TranscriptionProvider for RemoteAsrProvider {
    async fn transcribe(
        &self,
        audio: Vec<f32>,
        language: Option<String>,
    ) -> Result<TranscriptResult, TranscriptionError> {
        let api_base = crate::audio::transcription::remote_api_base().ok_or_else(|| {
            TranscriptionError::EngineFailed("远程服务未配置（缺少服务器地址）".to_string())
        })?;
        let license = crate::audio::transcription::get_remote_license();
        if license.is_empty() {
            return Err(TranscriptionError::EngineFailed(
                "远程服务未配置（缺少授权码）".to_string(),
            ));
        }

        let wav = encode_wav_16k(&audio);
        let part = reqwest::multipart::Part::bytes(wav)
            .file_name("audio.wav")
            .mime_str("audio/wav")
            .map_err(|e| TranscriptionError::EngineFailed(e.to_string()))?;

        let mut form = reqwest::multipart::Form::new()
            .text("model", self.model_name.clone())
            .part("file", part);
        if let Some(lang) = language {
            if !lang.is_empty() && lang != "auto" {
                form = form.text("language", lang);
            }
        }

        let client = reqwest::Client::new();
        let resp = client
            .post(format!("{}/audio/transcriptions", api_base))
            .bearer_auth(&license)
            // 任务会话（2026-09-22）：一次离线重识别的全部 chunk 归入同一个任务，
            // 用户中心显示为「一次离线识别 · 语音识别」。见 src/task_session.rs。
            .header("x-vox-session", crate::task_session::current_session())
            // 请求 id（2026-09-22）：网关按 (用户, 请求 id) 去重，超时重发不再重复扣费。
            // 每次调用生成新 id（同一次重试必须复用同一个，见 task_session::new_request_id）。
            .header("x-vox-request-id", crate::task_session::new_request_id())
            .multipart(form)
            // 离线重识别的异步模型（豆包录音识别 / qwen filetrans）单任务可能
            // 需要 1~2 分钟（qwen filetrans 实测 63s 音频 ≈104s），而客户端会把
            // 最长 600s 的语音合成 1 个 chunk → qwen 约需 1000s。60s 超时会中途
            // 掐断（「卡在十几%不动」+ 结果丢失），10 分钟对长录音仍不够。
            // 放宽到 20 分钟，与网关侧轮询上限（qwen 20 分钟 / 豆包 5 分钟）对齐。
            .timeout(std::time::Duration::from_secs(1200))
            .send()
            .await
            .map_err(|e| TranscriptionError::EngineFailed(format!("远程 ASR 请求失败: {}", e)))?;

        let status = resp.status();
        if !status.is_success() {
            let detail = resp.text().await.unwrap_or_default();
            return Err(TranscriptionError::EngineFailed(format!(
                "远程 ASR 错误 (HTTP {}): {}",
                status, detail
            )));
        }

        let json: serde_json::Value = resp.json().await.map_err(|e| {
            TranscriptionError::EngineFailed(format!("远程 ASR 响应解析失败: {}", e))
        })?;

        let text = json
            .get("text")
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .to_string();

        // 分句级结果（含说话人）：豆包录音文件识别等返回 utterances 数组
        let utterances: Vec<super::provider::TranscriptUtterance> = json
            .get("utterances")
            .and_then(|u| u.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|u| {
                        let text = u.get("text")?.as_str()?.to_string();
                        if text.trim().is_empty() {
                            return None;
                        }
                        Some(super::provider::TranscriptUtterance {
                            text,
                            start_ms: u.get("start_ms").and_then(|v| v.as_i64()).unwrap_or(0),
                            end_ms: u.get("end_ms").and_then(|v| v.as_i64()).unwrap_or(0),
                            speaker: u
                                .get("speaker")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string(),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        // 上游用户可见告警（内容风控部分拦截 / 档位降级 / 语言提示被拒）。
        // 网关随响应下发；这里原样透传给重识别流程汇总展示 —— 让用户知道「结果不完整
        // 是上游内容策略，不是软件坏了」。
        let warnings: Vec<String> = json
            .get("warnings")
            .and_then(|w| w.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|w| w.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
        for w in &warnings {
            log::warn!("⚠️ 远程 ASR 上游告警: {}", w);
        }

        // 响应正文落日志（2026-09-22 补）：离线远程识别的响应**此前没有任何 INFO 日志**，
        // 排查「只识别出一句话 / 文本丢失」时完全看不到网关到底返回了什么（唯一打正文的是
        // retranscription 里一条 debug!，而日志级别钉在 Info）→ 只能靠猜。
        // 只记长度 + 首尾片段（不整段落盘）。text 明显短 / 含非转写文案时一眼可辨。
        {
            let head: String = text.chars().take(60).collect();
            let tail: String = {
                let n = text.chars().count();
                text.chars().skip(n.saturating_sub(40)).collect()
            };
            info!(
                "📝 远程 ASR 响应 (model={}): text={} 字 utterances={} 段 | 首: {} | 尾: {}",
                self.model_name,
                text.chars().count(),
                utterances.len(),
                head.replace('\n', " "),
                tail.replace('\n', " ")
            );
        }

        Ok(TranscriptResult {
            text,
            confidence: None,
            is_partial: false,
            utterances,
            warnings,
        })
    }

    async fn is_model_loaded(&self) -> bool {
        crate::audio::transcription::is_remote_asr_configured()
    }

    async fn get_current_model(&self) -> Option<String> {
        Some(self.model_name.clone())
    }

    fn provider_name(&self) -> &'static str {
        "Remote ASR"
    }

    fn set_chunk_context(
        &self,
        sequence_id: u64,
        chunk_start_time: f64,
        audio_start_time: f64,
        audio_end_time: f64,
        duration: f64,
    ) {
        if let Ok(mut ctx) = self.chunk_context.lock() {
            *ctx = Some(ChunkContext {
                sequence_id,
                chunk_start_time,
                audio_start_time,
                audio_end_time,
                duration,
            });
        }
    }
}

/// 健康检查：真实 HTTP 探测用户配置的网关 /health 端点，不上传任何音频。
pub async fn check_remote_asr_health(endpoint: &str) -> bool {
    let client = reqwest::Client::new();
    let url = crate::audio::transcription::remote_health_url(endpoint);
    match client
        .get(&url)
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
    {
        Ok(resp) => resp.status().is_success(),
        Err(_) => false,
    }
}
