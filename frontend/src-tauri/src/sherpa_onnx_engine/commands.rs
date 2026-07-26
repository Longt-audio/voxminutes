use crate::sherpa_onnx_engine::{SherpaOnnxEngine, XAsrOnlineEngine};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::Mutex;
use tauri::{command, AppHandle, Manager, Runtime};

pub static SHERPA_ONNX_ENGINE: Mutex<Option<Arc<SherpaOnnxEngine>>> = Mutex::new(None);
pub static XASR_ONLINE_ENGINE: Mutex<Option<Arc<XAsrOnlineEngine>>> = Mutex::new(None);
static MODELS_DIR: Mutex<Option<PathBuf>> = Mutex::new(None);
/// Name of the model the current engine slot was loaded for (same-model
/// short-circuit in `sherpa_onnx_load_model`).
static LOADED_MODEL_NAME: Mutex<Option<String>> = Mutex::new(None);
/// Language preference in effect when the SenseVoice engine was loaded
/// (language is baked into the recognizer at load time, so a preference
/// change forces a reload).
static LOADED_LANGUAGE: Mutex<Option<String>> = Mutex::new(None);

/// Try to find project-local models directory (for development).
/// From CWD (typically frontend/), walk up to project root and look for models/.
fn find_local_models_dir() -> Option<PathBuf> {
    let cwd = std::env::current_dir().ok()?;
    // Walk up from CWD to find a directory containing "models/" and "frontend/"
    let mut current = cwd.clone();
    for _ in 0..5 {
        let models = current.join("models");
        if models.exists() && current.join("frontend").exists() {
            // Found project root with models/ directory
            return Some(models.canonicalize().unwrap_or(models));
        }
        current = current.parent()?.to_path_buf();
    }
    // Fallback: just check ../models from CWD
    let parent_models = cwd.parent()?.join("models");
    if parent_models.exists() {
        return Some(parent_models.canonicalize().unwrap_or(parent_models));
    }
    None
}

/// Scan a base directory for subdirs that contain valid model files.
/// Returns the first valid model subdirectory path.
fn find_model_subdir(base_dir: &Path, prefer_name: &str) -> Option<PathBuf> {
    // First check the standard/preferred name
    let preferred = base_dir.join(prefer_name);
    if SherpaOnnxEngine::validate_model_dir(&preferred) {
        return Some(preferred);
    }
    // Then scan all subdirectories for a valid model
    if let Ok(entries) = std::fs::read_dir(base_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() && SherpaOnnxEngine::validate_model_dir(&path) {
                return Some(path);
            }
        }
    }
    None
}

pub fn set_models_directory<R: Runtime>(app: &AppHandle<R>) {
    // Prefer project-local models/ for development, then bundled resources, then app data dir
    let models_dir = if let Some(local) = find_local_models_dir() {
        log::info!("Using local models directory: {}", local.display());
        local
    } else if let Ok(resource_dir) = app.path().resource_dir() {
        let bundled = resource_dir.join("models");
        if bundled.exists() {
            log::info!("Using bundled models directory: {}", bundled.display());
            bundled
        } else {
            let app_data_dir = app.path().app_data_dir().expect("app_data_dir");
            let dir = app_data_dir.join("models");
            std::fs::create_dir_all(&dir).ok();
            log::info!("Using app data models directory: {}", dir.display());
            dir
        }
    } else {
        let app_data_dir = app.path().app_data_dir().expect("app_data_dir");
        let dir = app_data_dir.join("models");
        std::fs::create_dir_all(&dir).ok();
        log::info!("Using app data models directory: {}", dir.display());
        dir
    };
    std::fs::create_dir_all(&models_dir).ok();
    // Stage to an ASCII-safe path if the resolved dir contains non-ASCII chars
    // (sherpa-onnx cannot open non-ASCII model paths on Windows).
    let models_dir = crate::bundle_paths::stage_models_dir_for_native(&models_dir);
    *MODELS_DIR.lock().unwrap() = Some(models_dir);
}

fn get_models_dir() -> PathBuf {
    MODELS_DIR
        .lock()
        .unwrap()
        .clone()
        .unwrap_or_else(|| PathBuf::from("models"))
}

/// Resolved models directory for other modules (e.g. in-app model downloads).
pub fn resolved_models_dir() -> PathBuf {
    get_models_dir()
}

/// Find the SenseVoice model subdirectory (auto-detect actual name, only SenseVoice-type dirs).
fn find_sense_voice_dir() -> Option<PathBuf> {
    let base = get_models_dir();
    // Prefer known directory names
    for name in &[
        "sherpa-onnx-sense-voice-zh-en-ja-ko-yue-2024-07-17",
        "sense-voice",
    ] {
        let dir = base.join(name);
        if SherpaOnnxEngine::validate_model_dir(&dir) {
            return Some(dir);
        }
    }
    // Fallback: scan for any SenseVoice-compatible directory
    if let Ok(entries) = std::fs::read_dir(&base) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() && SherpaOnnxEngine::validate_model_dir(&path) {
                return Some(path);
            }
        }
    }
    None
}

/// Find the new X-ASR model directory (sherpa-onnx-x-asr-480ms-streaming-zipformer-transducer-zh-en-punct-*).
fn find_x_asr_model_dir() -> Option<PathBuf> {
    let base = get_models_dir();
    // Prefer the punct version
    for prefix in &[
        "sherpa-onnx-x-asr-480ms-streaming-zipformer-transducer-zh-en-punct",
        "sherpa-onnx-x-asr-480ms-streaming-zipformer-transducer-zh-en",
    ] {
        if let Ok(entries) = std::fs::read_dir(&base) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if name_str.starts_with(prefix) && entry.path().is_dir() {
                    let dir = entry.path();
                    if XAsrOnlineEngine::validate_model_dir(&dir) {
                        return Some(dir);
                    }
                }
            }
        }
    }
    None
}

/// Validate that the new X-ASR model directory has all required files.
fn validate_x_asr_model_dir(dir: &Path) -> bool {
    XAsrOnlineEngine::validate_model_dir(dir)
}

#[command]
pub async fn sherpa_onnx_init() -> Result<(), String> {
    Ok(())
}

#[command]
pub async fn sherpa_onnx_get_models() -> Result<Vec<serde_json::Value>, String> {
    let mut models = Vec::new();

    // SenseVoice model (gated by feature flag)
    if crate::FEATURE_SENSEVOICE_ENABLED {
        let sv_status = if let Some(ref _dir) = find_sense_voice_dir() {
            if SHERPA_ONNX_ENGINE.lock().unwrap().is_some() {
                "Loaded"
            } else {
                "Available"
            }
        } else {
            "Missing"
        };
        models.push(serde_json::json!({
            "name": "sense-voice",
            "status": sv_status,
            "size_mb": 350,
            "languages": ["zh", "en", "ja", "ko", "yue"],
            "architecture": "AED (SenseVoice via Sherpa-ONNX Rust)",
            "description": "多语言识别，内置自动标点、情感识别、语种检测",
            "has_punctuation": true,
            "has_timestamps": true,
            "has_hotwords": false
        }));
    }

    // Remote Qwen3-ASR model (requires LAN server)
    // Only mark as "Available" if the remote endpoint has been configured;
    // otherwise show "NotConfigured" so the frontend can hide/disable the option.
    let remote_status = if crate::audio::transcription::is_remote_asr_configured() {
        "Available"
    } else {
        "NotConfigured"
    };
    models.push(serde_json::json!({
        "name": "qwen3-asr-remote",
        "status": remote_status,
        "size_mb": 0,
        "languages": [
            "zh", "en", "yue", "ar", "de", "fr", "es", "pt", "id",
            "it", "ko", "ru", "th", "vi", "ja", "tr", "hi", "ms",
            "nl", "sv", "da", "fi", "pl", "cs", "fil", "fa", "el",
            "hu", "mk", "ro"
        ],
        "architecture": "Qwen3-ASR 1.7B (Remote vLLM)",
        "description": "远程局域网 ASR，29种语言+方言，高精度大模型",
        "has_punctuation": true,
        "has_timestamps": true,
        "has_hotwords": true,
        "is_remote": true,
        "hidden": true
    }));

    // Remote Qwen3-ASR streaming model (SSE token streaming)
    models.push(serde_json::json!({
        "name": "qwen3-asr-remote-streaming",
        "status": remote_status,
        "size_mb": 0,
        "languages": [
            "zh", "en", "yue", "ar", "de", "fr", "es", "pt", "id",
            "it", "ko", "ru", "th", "vi", "ja", "tr", "hi", "ms",
            "nl", "sv", "da", "fi", "pl", "cs", "fil", "fa", "el",
            "hu", "mk", "ro"
        ],
        "architecture": "Qwen3-ASR 1.7B (Remote vLLM Streaming)",
        "description": "远程局域网 ASR 流式版，逐字输出，首字延迟更低",
        "has_punctuation": true,
        "has_timestamps": true,
        "has_hotwords": true,
        "is_remote": true
    }));

    // X-ASR model (Zipformer2 streaming, now Rust-native via OnlineRecognizer)
    let x_asr_dir = find_x_asr_model_dir();
    let x_asr_status = match &x_asr_dir {
        Some(_dir) => {
            if XASR_ONLINE_ENGINE.lock().unwrap().is_some() {
                "Loaded"
            } else {
                "Available"
            }
        }
        None => "Missing",
    };
    models.push(serde_json::json!({
        "name": "x-asr-480ms",
        "status": x_asr_status,
        "size_mb": 0,
        "languages": ["zh", "en"],
        "architecture": "X-ASR Zipformer2 480ms",
        "description": "X-ASR Zipformer2 流式识别，480ms chunk，低延迟（Rust 原生）",
        "has_punctuation": true,
        "has_timestamps": false,
        "has_hotwords": false,
        "is_remote": false
    }));

    Ok(models)
}

#[command]
pub async fn sherpa_onnx_load_model(model_name: String) -> Result<(), String> {
    // Remote model: no local loading needed
    if model_name == "qwen3-asr-remote" || model_name.starts_with("qwen3-asr-remote") {
        log::info!("Remote ASR model selected, no local loading required: {}", model_name);
        return Ok(());
    }

    // X-ASR model: load via Rust-native OnlineRecognizer
    if model_name.starts_with("x-asr-") {
        // Same-model short-circuit: skip the ~3s engine rebuild.
        if LOADED_MODEL_NAME.lock().unwrap().as_deref() == Some(model_name.as_str())
            && XASR_ONLINE_ENGINE.lock().unwrap().is_some()
        {
            log::info!("X-ASR model '{}' already loaded, skipping reload", model_name);
            return Ok(());
        }
        let dir = find_x_asr_model_dir()
            .ok_or_else(|| format!(
                "X-ASR model not found at {}. Download the model first.",
                get_models_dir().display()
            ))?;
        crate::llama_sidecar::emit_model_loading(&model_name, "start", None, None);
        let start = std::time::Instant::now();
        let eng = match XAsrOnlineEngine::create_x_asr(&dir, &model_name) {
            Ok(eng) => eng,
            Err(e) => {
                let msg = format!("Failed to load X-ASR model: {}", e);
                crate::llama_sidecar::emit_model_loading(&model_name, "error", None, Some(msg.clone()));
                return Err(msg);
            }
        };
        *XASR_ONLINE_ENGINE.lock().unwrap() = Some(Arc::new(eng));
        *LOADED_MODEL_NAME.lock().unwrap() = Some(model_name.clone());
        crate::llama_sidecar::emit_model_loading(
            &model_name,
            "done",
            Some(start.elapsed().as_millis() as u64),
            None,
        );
        log::info!("X-ASR model loaded via Rust-native OnlineRecognizer");
        return Ok(());
    }

    // SenseVoice (and generic offline) models: language preference is baked
    // into the recognizer at load time, so only short-circuit when both the
    // model and the language preference are unchanged.
    let current_lang = crate::get_language_preference_internal()
        .filter(|l| !l.is_empty())
        .unwrap_or_else(|| "auto".to_string());
    if LOADED_MODEL_NAME.lock().unwrap().as_deref() == Some(model_name.as_str())
        && LOADED_LANGUAGE.lock().unwrap().as_deref() == Some(current_lang.as_str())
        && SHERPA_ONNX_ENGINE.lock().unwrap().is_some()
    {
        log::info!(
            "ASR model '{}' already loaded (language: {}), skipping reload",
            model_name,
            current_lang
        );
        return Ok(());
    }

    let dir = if model_name == "sense-voice" || model_name.starts_with("sense") {
        if !crate::FEATURE_SENSEVOICE_ENABLED {
            return Err("SenseVoice model is currently disabled. Enable FEATURE_SENSEVOICE_ENABLED in lib.rs to use it.".to_string());
        }
        find_sense_voice_dir()
    } else {
        find_model_subdir(&get_models_dir(), &model_name)
    }
    .ok_or_else(|| {
        format!(
            "Model '{}' not found at {}. Download it first.",
            model_name,
            get_models_dir().display()
        )
    })?;

    crate::llama_sidecar::emit_model_loading(&model_name, "start", None, None);
    let start = std::time::Instant::now();
    let eng = match SherpaOnnxEngine::new(&dir, &model_name) {
        Ok(eng) => eng,
        Err(e) => {
            let msg = format!("Failed: {}", e);
            crate::llama_sidecar::emit_model_loading(&model_name, "error", None, Some(msg.clone()));
            return Err(msg);
        }
    };
    *SHERPA_ONNX_ENGINE.lock().unwrap() = Some(Arc::new(eng));
    *LOADED_MODEL_NAME.lock().unwrap() = Some(model_name.clone());
    *LOADED_LANGUAGE.lock().unwrap() = Some(current_lang);
    crate::llama_sidecar::emit_model_loading(
        &model_name,
        "done",
        Some(start.elapsed().as_millis() as u64),
        None,
    );
    Ok(())
}

#[command]
pub async fn sherpa_onnx_is_model_loaded() -> Result<bool, String> {
    Ok(SHERPA_ONNX_ENGINE.lock().unwrap().is_some())
}
#[command]
pub async fn sherpa_onnx_get_current_model() -> Result<Option<String>, String> {
    Ok(SHERPA_ONNX_ENGINE
        .lock()
        .unwrap()
        .as_ref()
        .map(|e| e.get_model_name().to_string()))
}
#[command]
pub async fn sherpa_onnx_get_models_directory() -> Result<String, String> {
    Ok(get_models_dir().to_string_lossy().to_string())
}

pub fn get_or_init_engine() -> Result<Arc<SherpaOnnxEngine>, String> {
    SHERPA_ONNX_ENGINE
        .lock()
        .unwrap()
        .as_ref()
        .cloned()
        .ok_or_else(|| "Model not loaded.".into())
}

pub fn get_or_init_xasr_engine() -> Result<Arc<XAsrOnlineEngine>, String> {
    XASR_ONLINE_ENGINE
        .lock()
        .unwrap()
        .as_ref()
        .cloned()
        .ok_or_else(|| "X-ASR model not loaded.".into())
}

pub fn is_xasr_engine_loaded() -> bool {
    XASR_ONLINE_ENGINE.lock().unwrap().is_some()
}
