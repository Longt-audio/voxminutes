use super::provider::TranscriptionProvider;
use super::remote_asr_provider::RemoteAsrProvider;
use super::x_asr_provider::XAsrProvider;
use super::worker::TranscriptUpdate;
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, Runtime};

pub enum TranscriptionEngine {
    Provider(Arc<dyn TranscriptionProvider>),
}

impl TranscriptionEngine {
    pub async fn is_model_loaded(&self) -> bool {
        match self {
            Self::Provider(provider) => provider.is_model_loaded().await,
        }
    }

    pub async fn get_current_model(&self) -> Option<String> {
        match self {
            Self::Provider(provider) => provider.get_current_model().await,
        }
    }

    pub fn provider_name(&self) -> &str {
        match self {
            Self::Provider(provider) => provider.provider_name(),
        }
    }

    /// Set chunk context for the next transcription call.
    /// Used by streaming providers to emit partial results with correct metadata.
    pub fn set_chunk_context(
        &self,
        sequence_id: u64,
        chunk_start_time: f64,
        audio_start_time: f64,
        audio_end_time: f64,
        duration: f64,
    ) {
        match self {
            Self::Provider(provider) => {
                provider.set_chunk_context(sequence_id, chunk_start_time, audio_start_time, audio_end_time, duration)
            }
        }
    }
}

static REMOTE_ASR_ENDPOINT: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());
static REMOTE_ASR_MODEL: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());

pub fn set_remote_asr_config(endpoint: &str, model_name: &str) {
    if let Ok(mut e) = REMOTE_ASR_ENDPOINT.lock() {
        *e = endpoint.to_string();
    }
    if let Ok(mut m) = REMOTE_ASR_MODEL.lock() {
        *m = model_name.to_string();
    }
    // Persist to disk so the config survives app restarts
    let config = RemoteAsrPersistedConfig {
        endpoint: endpoint.to_string(),
        model: model_name.to_string(),
    };
    if let Err(e) = save_remote_asr_config_to_disk(&config) {
        warn!("Failed to persist remote ASR config: {}", e);
    }
}

pub fn get_remote_asr_endpoint() -> String {
    REMOTE_ASR_ENDPOINT.lock().map(|e| e.clone()).unwrap_or_default()
}

pub fn get_remote_asr_model() -> String {
    REMOTE_ASR_MODEL.lock().map(|m| m.clone()).unwrap_or_default()
}

pub fn is_remote_asr_configured() -> bool {
    let endpoint = get_remote_asr_endpoint();
    !endpoint.is_empty()
}

// ── Persistence ──────────────────────────────────────────────────────
// Saves remote ASR endpoint+model to a JSON file in the OS config directory,
// and restores it on the next app launch.

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RemoteAsrPersistedConfig {
    endpoint: String,
    model: String,
}

fn get_remote_asr_config_path() -> Option<PathBuf> {
    let mut path = dirs::config_dir()?;
    path.push("voxminutes");
    path.push("remote_asr_config.json");
    Some(path)
}

fn save_remote_asr_config_to_disk(config: &RemoteAsrPersistedConfig) -> Result<(), String> {
    let path = get_remote_asr_config_path()
        .ok_or_else(|| "Could not determine config directory".to_string())?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }
    let json = serde_json::to_string_pretty(config)
        .map_err(|e| format!("Failed to serialize remote ASR config: {}", e))?;
    // Use sync write — called from Tauri command handler which may be on a sync thread.
    std::fs::write(&path, json)
        .map_err(|e| format!("Failed to write remote ASR config: {}", e))?;
    info!("Persisted remote ASR config to {}", path.display());
    Ok(())
}

/// Load the remote ASR config from disk and populate the in-memory statics.
/// Called once during app startup. Remote ASR is a reserved (stub) interface
/// in the open-source MVP: no default endpoint is provided — if the user has
/// not configured one, remote ASR simply stays unconfigured.
pub fn load_remote_asr_config_from_disk() {
    let path = match get_remote_asr_config_path() {
        Some(p) => p,
        None => return,
    };
    if path.exists() {
        match std::fs::read_to_string(&path) {
            Ok(content) => {
                match serde_json::from_str::<RemoteAsrPersistedConfig>(&content) {
                    Ok(config) => {
                        if let Ok(mut e) = REMOTE_ASR_ENDPOINT.lock() {
                            *e = config.endpoint.clone();
                        }
                        if let Ok(mut m) = REMOTE_ASR_MODEL.lock() {
                            *m = config.model.clone();
                        }
                        info!(
                            "Loaded remote ASR config from {}: {} (model: {})",
                            path.display(),
                            config.endpoint,
                            config.model
                        );
                    }
                    Err(e) => {
                        warn!("Failed to parse remote ASR config from {}: {}", path.display(), e);
                    }
                }
            }
            Err(e) => {
                warn!("Failed to read remote ASR config from {}: {}", path.display(), e);
            }
        }
    }
}

pub async fn validate_transcription_model_ready<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let config = match crate::api::api::api_get_transcript_config(
        app.clone(),
        app.clone().state(),
        None,
    )
    .await
    {
        Ok(Some(config)) => {
            info!(
                "📝 Found transcript config - provider: {}, model: {}",
                config.provider, config.model
            );
            config
        }
        Ok(None) | Err(_) => {
            info!("📝 No transcript config found, defaulting to sherpaonnx");
            crate::api::api::TranscriptConfig {
                provider: "sherpaonnx".to_string(),
                model: "sense-voice".to_string(),
                api_key: None,
            }
        }
    };

    let is_xasr = config.model.starts_with("x-asr-");
    let is_remote = config.model == "qwen3-asr-remote"
        || config.model.starts_with("qwen3-asr-remote")
        || config.provider == "remote-qwen3-asr";

    if is_xasr {
        info!("🔍 Validating X-ASR model: {}", config.model);
        if !crate::sherpa_onnx_engine::commands::is_xasr_engine_loaded() {
            crate::sherpa_onnx_engine::commands::sherpa_onnx_load_model(config.model.clone())
                .await
                .map_err(|e| format!("X-ASR model loading failed: {}", e))?;
        }
        info!("✅ X-ASR ready (Rust-native)");
        return Ok(());
    }

    if is_remote {
        let endpoint = get_remote_asr_endpoint();
        if endpoint.is_empty() {
            return Err("Remote ASR endpoint not configured. Please set the remote ASR URL in Settings.".to_string());
        }
        let is_streaming = config.model.contains("streaming");
        if is_streaming {
            info!("🔍 Validating remote Qwen3-ASR (streaming) at: {}", endpoint);
        } else {
            info!("🔍 Validating remote Qwen3-ASR at: {}", endpoint);
        }
        let provider = if is_streaming {
            // No-op callback for validation (no partial emissions needed)
            let noop_emitter: Arc<dyn Fn(&str, bool) + Send + Sync> = Arc::new(|_, _| {});
            let chunk_context: Arc<std::sync::Mutex<Option<super::remote_asr_provider::ChunkContext>>> =
                Arc::new(std::sync::Mutex::new(None));
            RemoteAsrProvider::new_streaming(endpoint.clone(), get_remote_asr_model(), noop_emitter, chunk_context)
        } else {
            RemoteAsrProvider::new(endpoint.clone(), get_remote_asr_model())
        };
        if !provider.check_health().await {
            return Err(format!("Cannot connect to remote ASR at {}. Please check the server is running.", endpoint));
        }
        info!("✅ Remote Qwen3-ASR ready");
        return Ok(());
    }

    if config.provider != "sherpaonnx" {
        return Err(format!(
            "Provider '{}' is not supported. Use 'sherpaonnx', 'x-asr', or 'remote-qwen3-asr'.",
            config.provider
        ));
    }

    info!("🔍 Validating Sherpa-ONNX model...");
    if let Err(e) = crate::sherpa_onnx_engine::commands::sherpa_onnx_init().await {
        return Err(format!("Failed to init Sherpa-ONNX: {}", e));
    }
    if !crate::sherpa_onnx_engine::commands::sherpa_onnx_is_model_loaded()
        .await
        .unwrap_or(false)
    {
        info!("🔧 Auto-loading Sherpa-ONNX model: {}", config.model);
        crate::sherpa_onnx_engine::commands::sherpa_onnx_load_model(config.model.clone())
            .await
            .map_err(|e| format!("Failed to load Sherpa-ONNX model '{}': {}", config.model, e))?;
    }
    info!("✅ Sherpa-ONNX model ready");
    Ok(())
}

pub async fn get_or_init_transcription_engine<R: Runtime>(
    app: &AppHandle<R>,
) -> Result<TranscriptionEngine, String> {
    let config = match crate::api::api::api_get_transcript_config(
        app.clone(),
        app.clone().state(),
        None,
    )
    .await
    {
        Ok(Some(config)) => {
            info!(
                "📝 Transcript config - provider: {}, model: {}",
                config.provider, config.model
            );
            config
        }
        Ok(None) | Err(_) => {
            info!("📝 No transcript config found, defaulting to sherpaonnx");
            crate::api::api::TranscriptConfig {
                provider: "sherpaonnx".to_string(),
                model: "sense-voice".to_string(),
                api_key: None,
            }
        }
    };

    let is_xasr = config.model.starts_with("x-asr-");
    let is_remote = config.model == "qwen3-asr-remote"
        || config.model.starts_with("qwen3-asr-remote")
        || config.provider == "remote-qwen3-asr";

    if is_xasr {
        info!("🦊 Initializing X-ASR streaming transcription engine (Rust-native): {}", config.model);
        // Load the X-ASR OnlineRecognizer engine if not already loaded
        if !crate::sherpa_onnx_engine::commands::is_xasr_engine_loaded() {
            crate::sherpa_onnx_engine::commands::sherpa_onnx_load_model(config.model.clone())
                .await
                .map_err(|e| format!("Failed to load X-ASR model: {}", e))?;
        }
        let engine = crate::sherpa_onnx_engine::commands::get_or_init_xasr_engine()
            .map_err(|e| format!("X-ASR engine not ready: {}", e))?;
        let provider = XAsrProvider::new_with_engine(config.model.clone(), engine);
        info!("✅ X-ASR provider ready (Rust-native OnlineRecognizer)");
        return Ok(TranscriptionEngine::Provider(Arc::new(provider)));
    }

    if is_remote {
        let endpoint = get_remote_asr_endpoint();
        let model_name = get_remote_asr_model();
        let is_streaming = config.model.contains("streaming");

        if is_streaming {
            info!("🦊 Initializing remote Qwen3-ASR STREAMING transcription engine at: {}", endpoint);
        } else {
            info!("🦊 Initializing remote Qwen3-ASR transcription engine at: {}", endpoint);
        }

        let provider = if is_streaming {
            let chunk_context: Arc<std::sync::Mutex<Option<super::remote_asr_provider::ChunkContext>>> =
                Arc::new(std::sync::Mutex::new(None));
            let emitter = build_sse_emitter(app.clone(), chunk_context.clone());
            RemoteAsrProvider::new_streaming(endpoint.clone(), model_name.clone(), emitter, chunk_context)
        } else {
            RemoteAsrProvider::new(endpoint.clone(), model_name.clone())
        };
        provider.check_health().await;
        let detected_model = provider.detect_model_name().await;

        let provider = if !detected_model.is_empty() && detected_model != model_name {
            info!("🔄 Using detected model: {}", detected_model);
            if is_streaming {
                let chunk_context: Arc<std::sync::Mutex<Option<super::remote_asr_provider::ChunkContext>>> =
                    Arc::new(std::sync::Mutex::new(None));
                let emitter = build_sse_emitter(app.clone(), chunk_context.clone());
                RemoteAsrProvider::new_streaming(endpoint, detected_model, emitter, chunk_context)
            } else {
                RemoteAsrProvider::new(endpoint, detected_model)
            }
        } else {
            provider
        };

        // The final provider instance (especially after model-name detection) must
        // have a cached successful health check, otherwise the worker will see
        // `is_model_loaded() == false` and skip every audio chunk.
        provider.check_health().await;

        info!("✅ Remote Qwen3-ASR provider ready");
        return Ok(TranscriptionEngine::Provider(Arc::new(provider)));
    }

    if config.provider != "sherpaonnx" {
        return Err(format!(
            "Provider '{}' is not supported. Use 'sherpaonnx', 'x-asr', or 'remote-qwen3-asr'.",
            config.provider
        ));
    }

    info!("🦊 Initializing Sherpa-ONNX native transcription engine");
    if !crate::sherpa_onnx_engine::commands::sherpa_onnx_is_model_loaded()
        .await
        .unwrap_or(false)
    {
        info!("🔧 Auto-loading Sherpa-ONNX model: {}", config.model);
        crate::sherpa_onnx_engine::commands::sherpa_onnx_load_model(config.model.clone())
            .await
            .map_err(|e| format!("Failed to load Sherpa-ONNX model: {}", e))?;
    }
    let engine = crate::sherpa_onnx_engine::commands::get_or_init_engine()
        .map_err(|e| format!("Sherpa-ONNX engine not ready: {}", e))?;
    let provider = crate::audio::transcription::sherpa_onnx_provider::SherpaOnnxProvider::new(engine);
    info!("✅ Sherpa-ONNX provider ready");
    Ok(TranscriptionEngine::Provider(Arc::new(provider)))
}

/// Build the SSE partial-result emitter callback used by streaming remote ASR providers.
/// Centralized here to avoid duplicating the same ~35 lines in model-selection branches.
fn build_sse_emitter<R: Runtime>(
    app: AppHandle<R>,
    chunk_context: Arc<std::sync::Mutex<Option<super::remote_asr_provider::ChunkContext>>>,
) -> Arc<dyn Fn(&str, bool) + Send + Sync> {
    Arc::new(move |text: &str, is_partial: bool| {
        let ctx = match chunk_context.lock().ok() {
            Some(guard) => guard,
            None => {
                warn!("SSE partial callback: chunk_context lock failed");
                return;
            }
        };
        let ctx = match &*ctx {
            Some(c) => c,
            None => {
                warn!("SSE partial callback: chunk_context is None");
                return;
            }
        };
        let update = TranscriptUpdate {
            text: text.to_string(),
            timestamp: format_timestamp_simple(),
            source: "Audio".to_string(),
            sequence_id: ctx.sequence_id,
            chunk_start_time: ctx.chunk_start_time,
            is_partial,
            confidence: 0.9,
            audio_start_time: ctx.audio_start_time,
            audio_end_time: ctx.audio_end_time,
            duration: ctx.duration,
        };
        match app.emit("transcript-update", &update) {
            Ok(_) => {
                if is_partial {
                    debug!("🔵 Emitted partial transcript seq={} text_len={}", ctx.sequence_id, text.len());
                } else {
                    info!("🔵 Emitted final transcript seq={} text_len={}", ctx.sequence_id, text.len());
                }
            }
            Err(e) => warn!("SSE partial callback: emit failed: {}", e),
        }
    })
}

/// Simple timestamp formatter for use in callback closures
/// (cannot call worker::format_current_timestamp because it's in a different module)
fn format_timestamp_simple() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let hours = (now.as_secs() / 3600) % 24;
    let minutes = (now.as_secs() / 60) % 60;
    let seconds = now.as_secs() % 60;
    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
}
