// Quick audio transcription for live-translate feature
// Short audio files (<20s) are decoded and transcribed directly without VAD.
// Supports both local Sherpa-ONNX and remote Qwen3-ASR models.

use anyhow::{anyhow, Result};
use log::{info, warn};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;
use tauri::{command, AppHandle, Manager, Runtime};

use crate::audio::decoder::decode_audio_file;
use crate::audio::transcription::provider::TranscriptionProvider;
use crate::audio::transcription::remote_asr_provider::RemoteAsrProvider;
use crate::audio::transcription::x_asr_provider::XAsrProvider;

/// Transcribe a short audio file directly (no VAD, optimized for <20s clips).
#[command]
pub async fn quick_transcribe<R: Runtime>(
    app: AppHandle<R>,
    audio_path: String,
    language: Option<String>,
    model: Option<String>,
) -> Result<String, String> {
    let path = Path::new(&audio_path);

    // Resolve path strictly under app data directory to prevent path traversal
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    let resolved_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        app_data.join(path)
    };

    // Canonicalize and verify the path is inside app_data_dir
    let canonical = resolved_path
        .canonicalize()
        .map_err(|_| format!("Audio file not found or inaccessible: {}", audio_path))?;
    let canonical_app_data = app_data
        .canonicalize()
        .unwrap_or_else(|_| app_data.clone());
    if !canonical.starts_with(&canonical_app_data) {
        return Err(format!("Audio file path is outside allowed directory: {}", audio_path));
    }

    if !canonical.exists() {
        return Err(format!("Audio file not found: {}", audio_path));
    }

    info!("Quick transcribe: {}", resolved_path.display());

    // Step 1: Decode audio file (blocking IO → spawn_blocking)
    let path_for_decode = resolved_path.clone();
    let decoded = tokio::task::spawn_blocking(move || decode_audio_file(&path_for_decode))
        .await
        .map_err(|e| format!("Decode task panicked: {}", e))?
        .map_err(|e| format!("Failed to decode audio: {}", e))?;

    info!(
        "Decoded: {:.2}s, {}Hz, {} channels",
        decoded.duration_seconds, decoded.sample_rate, decoded.channels
    );

    if decoded.duration_seconds > 30.0 {
        warn!(
            "Audio duration {:.1}s exceeds recommended 20s limit",
            decoded.duration_seconds
        );
    }

    // Step 2: Convert to 16kHz mono f32 (Whisper/Sherpa-ONNX format)
    let samples = tokio::task::spawn_blocking(move || decoded.to_whisper_format())
        .await
        .map_err(|e| format!("Format conversion panicked: {}", e))?;

    if samples.is_empty() {
        return Err("No audio samples after preprocessing".to_string());
    }

    info!("Preprocessed: {} samples at 16kHz", samples.len());

    // Step 3: Select and initialize transcription provider based on model
    let model_name = model.as_deref().unwrap_or("sense-voice");
    let provider = get_or_init_provider(model_name)
        .await
        .map_err(|e| e.to_string())?;

    // Step 4: Transcribe
    if model_name.starts_with("x-asr-") {
        if let Some(xasr) = provider.as_any().downcast_ref::<XAsrProvider>() {
            let text = xasr.transcribe_file(&resolved_path).await?;
            info!("X-ASR quick transcribe result: '{}'", text);
            return Ok(text);
        }
        return Err("X-ASR provider downcast failed".to_string());
    }

    match provider.transcribe(samples, language).await {
        Ok(result) => {
            let text = result.text.trim().to_string();
            info!("Quick transcribe result: '{}'", text);
            Ok(text)
        }
        Err(e) => Err(format!("Transcription failed: {}", e)),
    }
}

/// Get or initialize the appropriate transcription provider (local or remote).
async fn get_or_init_provider(
    model_name: &str,
) -> Result<Arc<dyn TranscriptionProvider>> {
    let is_xasr = model_name.starts_with("x-asr-");
    let is_remote = model_name == "qwen3-asr-remote"
        || model_name.starts_with("qwen3-asr-remote");

    if is_xasr {
        info!("Using X-ASR for quick transcribe: {}", model_name);
        if !crate::sherpa_onnx_engine::commands::is_xasr_engine_loaded() {
            crate::sherpa_onnx_engine::commands::sherpa_onnx_load_model(model_name.to_string())
                .await
                .map_err(|e| anyhow!("Failed to load X-ASR model: {}", e))?;
        }
        let engine = crate::sherpa_onnx_engine::commands::get_or_init_xasr_engine()
            .map_err(|e| anyhow!("X-ASR engine not ready: {}", e))?;
        Ok(Arc::new(XAsrProvider::new_with_engine(model_name.to_string(), engine)))
    } else if is_remote {
        get_or_init_remote_asr().await
    } else {
        get_or_init_sherpa_onnx(model_name).await
    }
}

/// Get or initialize the remote Qwen3-ASR transcription provider.
async fn get_or_init_remote_asr() -> Result<Arc<dyn TranscriptionProvider>> {
    let endpoint = crate::audio::transcription::get_remote_asr_endpoint();
    let model_name = crate::audio::transcription::get_remote_asr_model();

    if endpoint.is_empty() {
        return Err(anyhow!(
            "Remote ASR endpoint not configured. Please set the remote ASR URL in Settings."
        ));
    }

    info!("Using remote Qwen3-ASR for quick transcribe: {} (model: {})", endpoint, model_name);

    let provider = RemoteAsrProvider::create_with_model_detection(&endpoint, &model_name, false)
        .await
        .map_err(|e| anyhow!("Failed to initialize remote ASR: {}", e))?;

    info!("Remote Qwen3-ASR health check passed");
    Ok(Arc::new(provider))
}

/// Get or initialize the Sherpa-ONNX transcription engine.
async fn get_or_init_sherpa_onnx(
    model_name: &str,
) -> Result<Arc<dyn TranscriptionProvider>> {
    let model_to_load = if model_name == "sense-voice" || model_name.starts_with("sense") {
        "sense-voice"
    } else {
        model_name
    };

    if !crate::sherpa_onnx_engine::commands::sherpa_onnx_is_model_loaded()
        .await
        .unwrap_or(false)
    {
        info!("Auto-loading Sherpa-ONNX model for quick transcribe: {}", model_to_load);
        crate::sherpa_onnx_engine::commands::sherpa_onnx_load_model(model_to_load.to_string())
            .await
            .map_err(|e| anyhow!("Failed to load Sherpa-ONNX model '{}': {}", model_to_load, e))?;
    }

    let engine = crate::sherpa_onnx_engine::commands::get_or_init_engine()
        .map_err(|e| anyhow!("Sherpa-ONNX engine not ready: {}", e))?;
    let sherpa_provider =
        crate::audio::transcription::sherpa_onnx_provider::SherpaOnnxProvider::new(engine);
    Ok(Arc::new(sherpa_provider))
}

/// Result of an ASR speed benchmark.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsrBenchmarkResult {
    pub rtf: f64,
    pub audio_duration_seconds: f64,
    pub processing_time_seconds: f64,
}

/// Benchmark ASR speed by transcribing the bundled example audio.
/// Returns the real-time factor (RTF = processing_time / audio_duration).
#[command]
pub async fn benchmark_asr<R: Runtime>(
    app: AppHandle<R>,
    model: Option<String>,
    provider: Option<String>,
) -> Result<AsrBenchmarkResult, String> {
    // Ensure the test audio exists in app_data_dir.
    let relative_path = prepare_auto_test_audio(app.clone()).await?;
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    let audio_path = app_data.join(&relative_path);

    // Decode the audio to obtain its duration (not timed).
    let audio_path_for_decode = audio_path.clone();
    let decoded = tokio::task::spawn_blocking(move || decode_audio_file(&audio_path_for_decode))
        .await
        .map_err(|e| format!("Decode task panicked: {}", e))?
        .map_err(|e| format!("Failed to decode audio: {}", e))?;

    let audio_duration_seconds = decoded.duration_seconds;
    info!(
        "ASR benchmark audio: {:.2}s, {}Hz, {} channels",
        audio_duration_seconds, decoded.sample_rate, decoded.channels
    );

    // Resolve the same provider that will be used for retranscription.
    let model_name = if provider.as_deref() == Some("remote") {
        "qwen3-asr-remote".to_string()
    } else {
        model.unwrap_or_else(|| "sense-voice".to_string())
    };

    let transcription_provider = get_or_init_provider(&model_name)
        .await
        .map_err(|e| e.to_string())?;

    // Time the actual transcription only (model loading is already done above).
    let start = Instant::now();
    if model_name.starts_with("x-asr-") {
        if let Some(xasr) = transcription_provider.as_any().downcast_ref::<XAsrProvider>() {
            xasr.transcribe_file(&audio_path).await.map_err(|e| e.to_string())?;
        } else {
            return Err("X-ASR provider downcast failed".to_string());
        }
    } else {
        let samples = tokio::task::spawn_blocking(move || decoded.to_whisper_format())
            .await
            .map_err(|e| format!("Format conversion panicked: {}", e))?;
        transcription_provider
            .transcribe(samples, None)
            .await
            .map_err(|e| e.to_string())?;
    }
    let processing_time_seconds = start.elapsed().as_secs_f64();

    let rtf = if audio_duration_seconds > 0.0 {
        processing_time_seconds / audio_duration_seconds
    } else {
        0.0
    };

    info!(
        "ASR benchmark result: model={}, rtf={:.3}, audio={:.2}s, processing={:.2}s",
        model_name, rtf, audio_duration_seconds, processing_time_seconds
    );

    Ok(AsrBenchmarkResult {
        rtf,
        audio_duration_seconds,
        processing_time_seconds,
    })
}

/// Copy the bundled example audio file into the app data directory so it can be
/// used for the AUTO ASR model speed test. Returns the relative file name.
#[command]
pub async fn prepare_auto_test_audio<R: Runtime>(app: AppHandle<R>) -> Result<String, String> {
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    let dest = app_data.join("example_audio.wav");

    // Try bundled resource directory first.
    let mut src = app
        .path()
        .resource_dir()
        .map_err(|e| format!("Failed to get resource dir: {}", e))?
        .join("example_audio.wav");

    if !src.exists() {
        // Development fallback: project root / current working directory.
        if let Ok(cwd) = std::env::current_dir() {
            let fallback = cwd.join("example_audio.wav");
            if fallback.exists() {
                src = fallback;
            }
        }
    }

    if !src.exists() {
        // Production fallback: directory containing the executable.
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                let fallback = exe_dir.join("example_audio.wav");
                if fallback.exists() {
                    src = fallback;
                }
            }
        }
    }

    if !src.exists() {
        return Err("example_audio.wav not found".to_string());
    }

    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create app data dir: {}", e))?;
    }

    std::fs::copy(&src, &dest)
        .map_err(|e| format!("Failed to copy example audio: {}", e))?;

    Ok("example_audio.wav".to_string())
}
