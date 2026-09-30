// audio/transcription/provider.rs
//
// Defines the unified TranscriptionProvider trait and common types for all
// transcription engines (Whisper, Parakeet, future providers).

use async_trait::async_trait;

// ============================================================================
// TRANSCRIPTION PROVIDER TRAIT & ERROR TYPES
// ============================================================================

/// Granular error types for transcription operations
#[derive(Debug, Clone)]
pub enum TranscriptionError {
    ModelNotLoaded,
    AudioTooShort { samples: usize, minimum: usize },
    EngineFailed(String),
    UnsupportedLanguage(String),
}

impl std::fmt::Display for TranscriptionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ModelNotLoaded => write!(f, "No transcription model is loaded"),
            Self::AudioTooShort { samples, minimum } => write!(
                f,
                "Audio too short: {} samples (minimum {})",
                samples, minimum
            ),
            Self::EngineFailed(msg) => write!(f, "Transcription engine failed: {}", msg),
            Self::UnsupportedLanguage(lang) => {
                write!(f, "Language '{}' is not supported by this provider", lang)
            }
        }
    }
}

impl std::error::Error for TranscriptionError {}

/// 分句（含说话人）：上游返回分句级结果时逐句给出（豆包录音文件识别等）。
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct TranscriptUtterance {
    pub text: String,
    /// 分句在本次上传音频内的起始/结束毫秒偏移
    pub start_ms: i64,
    pub end_ms: i64,
    /// 说话人匿名编号（"1"/"2"/…；无说话人分离能力时为空串）
    pub speaker: String,
}

/// Unified transcription result across all providers
#[derive(Debug, Clone)]
pub struct TranscriptResult {
    pub text: String,
    pub confidence: Option<f32>, // None if provider doesn't support confidence scores
    pub is_partial: bool,
    /// 分句级结果（含说话人）。空表示上游未返回分句信息。
    pub utterances: Vec<TranscriptUtterance>,
    /// 上游给出的**用户可见告警**（内容风控部分拦截、档位降级、语言提示被拒…）。
    /// 由网关随响应下发；离线识别会把它们汇总后展示给用户，避免把「结果不完整」
    /// 误当成软件故障（2026-09-22 用户要求）。
    pub warnings: Vec<String>,
}

/// Trait for transcription providers (Whisper, Parakeet, future providers)
#[async_trait]
pub trait TranscriptionProvider: Send + Sync {
    /// Transcribe audio samples to text
    ///
    /// # Arguments
    /// * `audio` - Audio samples (16kHz mono, f32 format)
    /// * `language` - Optional language hint (e.g., "en", "es", "fr")
    ///
    /// # Returns
    /// * `TranscriptResult` with text, optional confidence, and partial flag
    async fn transcribe(
        &self,
        audio: Vec<f32>,
        language: Option<String>,
    ) -> std::result::Result<TranscriptResult, TranscriptionError>;

    /// Check if a model is currently loaded
    async fn is_model_loaded(&self) -> bool;

    /// Get the name of the currently loaded model
    async fn get_current_model(&self) -> Option<String>;

    /// Get the provider name (for logging/debugging)
    fn provider_name(&self) -> &'static str;

    /// Set context for the next transcription call (used by streaming providers
    /// to emit partial results with correct metadata). Default no-op.
    fn set_chunk_context(
        &self,
        _sequence_id: u64,
        _chunk_start_time: f64,
        _audio_start_time: f64,
        _audio_end_time: f64,
        _duration: f64,
    ) {
    }

    /// Downcast helper for provider-specific operations (e.g. X-ASR streaming).
    /// Default panics — only override if your provider needs direct access.
    fn as_any(&self) -> &dyn std::any::Any {
        panic!("as_any not implemented for this provider")
    }
}
