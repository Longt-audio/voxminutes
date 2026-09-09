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
/// 远程 ASR 模型的 mode（来自网关模型目录：streaming / batch）。空串表示未设置，需回退到模型名后缀判断。
static REMOTE_ASR_MODE: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());
static REMOTE_TRANSLATE_MODEL: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());
static REMOTE_TTS_MODEL: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());
static REMOTE_LICENSE: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());
static REMOTE_ENABLED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// 设置远程服务总开关（开启 + 已配置 endpoint 才真正生效）。
pub fn set_remote_enabled(v: bool) {
    REMOTE_ENABLED.store(v, std::sync::atomic::Ordering::SeqCst);
}

/// 远程服务是否可用（开关开启 && 已配置服务器地址）。
/// ASR / 翻译端据此切换到远程路径。
pub fn remote_enabled() -> bool {
    REMOTE_ENABLED.load(std::sync::atomic::Ordering::SeqCst) && is_remote_asr_configured()
}

/// 远程开关的原始状态（不含「是否已配置 endpoint」判断，供设置页回显）。
pub fn remote_enabled_raw() -> bool {
    REMOTE_ENABLED.load(std::sync::atomic::Ordering::SeqCst)
}

pub fn set_remote_asr_config(endpoint: &str, model_name: &str) {
    if let Ok(mut e) = REMOTE_ASR_ENDPOINT.lock() {
        *e = endpoint.to_string();
    }
    if let Ok(mut m) = REMOTE_ASR_MODEL.lock() {
        *m = model_name.to_string();
    }
    let config = RemoteAsrPersistedConfig {
        endpoint: endpoint.to_string(),
        model: model_name.to_string(),
        asr_mode: get_remote_asr_mode(),
        translate_model: get_remote_translate_model(),
        tts_model: get_remote_tts_model(),
    };
    if let Err(e) = save_remote_asr_config_to_disk(&config) {
        warn!("Failed to persist remote ASR config: {}", e);
    }
}

/// 设置远程服务（server_url + license + model）；license 存 OS keychain，不落明文 JSON。
pub fn set_remote_config(endpoint: &str, license: &str, model_name: &str) {
    if let Ok(mut e) = REMOTE_ASR_ENDPOINT.lock() {
        *e = endpoint.to_string();
    }
    if let Ok(mut l) = REMOTE_LICENSE.lock() {
        *l = license.to_string();
    }
    if let Ok(mut m) = REMOTE_ASR_MODEL.lock() {
        *m = model_name.to_string();
    }
    save_license_to_keyring(license);
    let config = RemoteAsrPersistedConfig {
        endpoint: endpoint.to_string(),
        model: model_name.to_string(),
        asr_mode: get_remote_asr_mode(),
        translate_model: get_remote_translate_model(),
        tts_model: get_remote_tts_model(),
    };
    if let Err(e) = save_remote_asr_config_to_disk(&config) {
        warn!("Failed to persist remote config: {}", e);
    }
}

pub fn get_remote_asr_endpoint() -> String {
    REMOTE_ASR_ENDPOINT.lock().map(|e| e.clone()).unwrap_or_default()
}

pub fn get_remote_asr_model() -> String {
    REMOTE_ASR_MODEL.lock().map(|m| m.clone()).unwrap_or_default()
}

pub fn get_remote_asr_mode() -> String {
    REMOTE_ASR_MODE.lock().map(|m| m.clone()).unwrap_or_default()
}

/// 判断远程 ASR 模型是否为「流式」：模型 id 带 -realtime / -streaming 后缀。
/// 仅作为「未设置 mode 时」的兜底判断，与网关模型目录的 mode=streaming 约定一致。
pub fn is_remote_asr_streaming_model(model_name: &str) -> bool {
    model_name.ends_with("-realtime") || model_name.ends_with("-streaming")
}

/// 远程 ASR 是否走流式：优先用前端传来的 mode（网关模型目录下发），
/// 未设置时回退到模型名后缀（兼容旧配置 / 旧调用路径）。
pub fn is_remote_asr_streaming() -> bool {
    let mode = get_remote_asr_mode();
    if !mode.is_empty() {
        return mode == "streaming";
    }
    is_remote_asr_streaming_model(&get_remote_asr_model())
}

pub fn get_remote_translate_model() -> String {
    REMOTE_TRANSLATE_MODEL.lock().map(|m| m.clone()).unwrap_or_default()
}

pub fn get_remote_tts_model() -> String {
    REMOTE_TTS_MODEL.lock().map(|m| m.clone()).unwrap_or_default()
}

pub fn get_remote_license() -> String {
    REMOTE_LICENSE.lock().map(|l| l.clone()).unwrap_or_default()
}

/// 只设置授权码（供自动注册后写入），并持久化到 keychain。
pub fn set_remote_license(license: &str) {
    if let Ok(mut l) = REMOTE_LICENSE.lock() {
        *l = license.to_string();
    }
    save_license_to_keyring(license);
}

/// 设置三种能力各自的远程模型（模型选择器），并持久化到磁盘。
/// `asr_mode` 由前端从网关 /v1/models 的 mode 字段透传，用于决定远程 ASR 走流式还是非流式。
pub fn set_remote_models(
    asr: Option<String>,
    asr_mode: Option<String>,
    translate: Option<String>,
    tts: Option<String>,
) -> Result<(), String> {
    if let Some(m) = asr {
        if let Ok(mut g) = REMOTE_ASR_MODEL.lock() {
            *g = m;
        }
    }
    if let Some(m) = asr_mode {
        if let Ok(mut g) = REMOTE_ASR_MODE.lock() {
            *g = m;
        }
    }
    if let Some(m) = translate {
        if let Ok(mut g) = REMOTE_TRANSLATE_MODEL.lock() {
            *g = m;
        }
    }
    if let Some(m) = tts {
        if let Ok(mut g) = REMOTE_TTS_MODEL.lock() {
            *g = m;
        }
    }
    let config = RemoteAsrPersistedConfig {
        endpoint: get_remote_asr_endpoint(),
        model: get_remote_asr_model(),
        asr_mode: get_remote_asr_mode(),
        translate_model: get_remote_translate_model(),
        tts_model: get_remote_tts_model(),
    };
    save_remote_asr_config_to_disk(&config)
}

pub fn is_remote_asr_configured() -> bool {
    let endpoint = get_remote_asr_endpoint();
    !endpoint.is_empty()
}

/// 网关 /v1 根地址（未配置时返回 None）。ASR / 翻译都拼在其后。
pub fn remote_api_base() -> Option<String> {
    let endpoint = get_remote_asr_endpoint();
    if endpoint.is_empty() {
        return None;
    }
    Some(format!("{}/v1", endpoint.trim_end_matches('/')))
}

/// 健康检查地址（网关 /health，无需鉴权）。
pub fn remote_health_url(endpoint: &str) -> String {
    format!("{}/health", endpoint.trim_end_matches('/'))
}

// ── Persistence ──────────────────────────────────────────────────────
// Saves remote ASR endpoint+model to a JSON file in the OS config directory,
// and restores it on the next app launch.

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RemoteAsrPersistedConfig {
    endpoint: String,
    model: String,
    #[serde(default)]
    asr_mode: String,
    #[serde(default)]
    translate_model: String,
    #[serde(default)]
    tts_model: String,
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

// ── License 存 OS keychain ────────────────────────────────────────────
// Windows 走凭据管理器，macOS 走钥匙串，Linux 走 Secret Service（libsecret）。
// 读取在启动时做一次并缓存到 REMOTE_LICENSE，热路径不再访问 keychain。

const KEYRING_SERVICE: &str = "voxminutes";
const KEYRING_USER: &str = "remote-license";

fn save_license_to_keyring(license: &str) {
    match keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER) {
        Ok(entry) => {
            if license.is_empty() {
                let _ = entry.delete_credential();
            } else if let Err(e) = entry.set_password(license) {
                warn!("Failed to save license to keychain: {}", e);
            }
        }
        Err(e) => warn!("Keychain unavailable, license not persisted: {}", e),
    }
}

fn load_license_from_keyring() -> String {
    match keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER) {
        Ok(entry) => entry.get_password().unwrap_or_default(),
        Err(_) => String::new(),
    }
}

/// Load the remote config from disk and populate the in-memory statics.
/// Called once during app startup.
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
                        if let Ok(mut m) = REMOTE_ASR_MODE.lock() {
                            *m = config.asr_mode.clone();
                        }
                        if let Ok(mut m) = REMOTE_TRANSLATE_MODEL.lock() {
                            *m = config.translate_model.clone();
                        }
                        if let Ok(mut m) = REMOTE_TTS_MODEL.lock() {
                            *m = config.tts_model.clone();
                        }
                        info!(
                            "Loaded remote config from {}: {} (model: {})",
                            path.display(),
                            config.endpoint,
                            config.model
                        );
                    }
                    Err(e) => {
                        warn!("Failed to parse remote config from {}: {}", path.display(), e);
                    }
                }
            }
            Err(e) => {
                warn!("Failed to read remote config from {}: {}", path.display(), e);
            }
        }
    }

    // license 从 OS keychain 读回（与 JSON 解耦）
    let license = load_license_from_keyring();
    if !license.is_empty() {
        if let Ok(mut l) = REMOTE_LICENSE.lock() {
            *l = license;
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
        || config.provider == "remote-qwen3-asr"
        || remote_enabled();

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
        let model_name = get_remote_asr_model();
        let is_streaming = is_remote_asr_streaming();
        if is_streaming {
            info!("🔍 Validating remote STREAMING ASR at: {} model={}", endpoint, model_name);
        } else {
            info!("🔍 Validating remote ASR at: {} model={}", endpoint, model_name);
        }
        // 流式只校验网关健康（真正连接在 run_streaming 里建立）；非流式校验完整健康
        if is_streaming {
            if !super::remote_asr_provider::check_remote_asr_health(&endpoint).await {
                return Err(format!("Cannot connect to remote ASR at {}. Please check the server is running.", endpoint));
            }
        } else {
            let provider = RemoteAsrProvider::new(endpoint.clone(), model_name);
            if !provider.check_health().await {
                return Err(format!("Cannot connect to remote ASR at {}. Please check the server is running.", endpoint));
            }
        }
        info!("✅ Remote ASR ready");
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
        || config.provider == "remote-qwen3-asr"
        || remote_enabled();

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
        let is_streaming = is_remote_asr_streaming();

        if is_streaming {
            info!("🦊 Initializing remote STREAMING ASR engine at: {} model={}", endpoint, model_name);
            let provider = super::remote_asr_streaming_provider::RemoteAsrStreamingProvider::new(
                endpoint.clone(),
                model_name.clone(),
                crate::get_language_preference_internal().unwrap_or_else(|| "zh".to_string()),
            );
            // 流式 provider 在 run_streaming 里按需连 WS，这里只做一次健康探测
            let _ = provider.is_model_loaded().await;
            info!("✅ Remote streaming ASR provider ready");
            return Ok(TranscriptionEngine::Provider(Arc::new(provider)));
        }

        let provider = RemoteAsrProvider::new(endpoint.clone(), model_name.clone());
        provider.check_health().await;
        let detected_model = provider.detect_model_name().await;

        let provider = if !detected_model.is_empty() && detected_model != model_name {
            info!("🔄 Using detected model: {}", detected_model);
            RemoteAsrProvider::new(endpoint, detected_model)
        } else {
            provider
        };

        // The final provider instance (especially after model-name detection) must
        // have a cached successful health check, otherwise the worker will see
        // `is_model_loaded() == false` and skip every audio chunk.
        provider.check_health().await;

        info!("✅ Remote ASR provider ready");
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
