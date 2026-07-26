// audio/transcription/remote_asr_provider.rs
//
// Remote ASR provider — STUB for the open-source MVP.
//
// Remote (cloud/third-party) ASR is a reserved interface for the future paid
// version. This build performs no network transcription: all transcribe()
// calls fail with a clear error and the provider reports itself as not loaded.
// The public API surface (constructors, health check, chunk context) is kept
// so call sites in `engine.rs` / `quick_transcribe.rs` / `retranscription.rs`
// keep compiling unchanged; endpoint/model configuration is still persisted
// by `engine.rs` so the feature can be enabled later without schema changes.

use async_trait::async_trait;
use std::sync::Arc;

use super::provider::{TranscriptionError, TranscriptionProvider, TranscriptResult};

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

    /// MVP stub: remote ASR is unavailable, so the provider never reports healthy.
    pub async fn check_health(&self) -> bool {
        false
    }

    /// MVP stub: no server to query; echo back the configured model name.
    pub async fn detect_model_name(&self) -> String {
        self.model_name.clone()
    }

    /// Kept for call-site compatibility; simply wraps `new()` without any
    /// network model detection.
    pub async fn create_with_model_detection(
        endpoint: &str,
        configured_model: &str,
        _is_streaming: bool,
    ) -> Result<Self, String> {
        Ok(Self::new(endpoint.to_string(), configured_model.to_string()))
    }
}

#[async_trait]
impl TranscriptionProvider for RemoteAsrProvider {
    async fn transcribe(
        &self,
        _audio: Vec<f32>,
        _language: Option<String>,
    ) -> Result<TranscriptResult, TranscriptionError> {
        Err(TranscriptionError::EngineFailed(format!(
            "Remote ASR is not available in the open-source MVP (endpoint: {}).",
            self.endpoint
        )))
    }

    async fn is_model_loaded(&self) -> bool {
        false
    }

    async fn get_current_model(&self) -> Option<String> {
        Some(self.model_name.clone())
    }

    fn provider_name(&self) -> &'static str {
        "Remote ASR (stub)"
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

/// Diagnostic helper for the settings page ("test connection"). Performs a
/// real HTTP probe of the *user-supplied* endpoint — no audio ever leaves the
/// machine through this path.
pub async fn check_remote_asr_health(endpoint: &str) -> bool {
    let client = reqwest::Client::new();
    let url = format!("{}/health", endpoint.trim_end_matches('/'));
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
