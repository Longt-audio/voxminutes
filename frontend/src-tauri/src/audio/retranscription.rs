// Retranscription module - allows re-processing stored audio with Sherpa-ONNX (SenseVoice)

use crate::audio::audio_processing::audio_to_mono;
use crate::audio::decoder::{decode_audio_file, probe_audio_duration};
use crate::audio::vad::get_speech_chunks_with_config;
use super::common::create_transcript_segments;
use super::constants::AUDIO_EXTENSIONS;
use crate::state::AppState;
use anyhow::{anyhow, Result};
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tauri::{AppHandle, Emitter, Manager, Runtime};

/// Global flag to track if retranscription is in progress
static RETRANSCRIPTION_IN_PROGRESS: AtomicBool = AtomicBool::new(false);

/// Global flag to signal cancellation
static RETRANSCRIPTION_CANCELLED: AtomicBool = AtomicBool::new(false);

struct RetranscriptionGuard;

impl RetranscriptionGuard {
    fn acquire() -> Result<Self, String> {
        if RETRANSCRIPTION_IN_PROGRESS
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return Err("Retranscription already in progress".to_string());
        }
        Ok(RetranscriptionGuard)
    }
}

impl Drop for RetranscriptionGuard {
    fn drop(&mut self) {
        RETRANSCRIPTION_IN_PROGRESS.store(false, Ordering::SeqCst);
    }
}

// Slightly shorter redemption time for offline retranscription so short
// leading speech (e.g. greetings, confirmations) is less likely to be
// swallowed by VAD, while still avoiding fragmenting normal sentences.
const VAD_REDEMPTION_TIME_MS: u32 = 1500;

/// Local SenseVoice path: more sensitive silero pass so quiet/short utterances
/// (e.g. greetings buried under applause) are not dropped from offline results.
/// Values tuned on real recordings via the vad_real_recording_threshold_sweep
/// test: the most sensitive (positive, negative) pair that still does not stick
/// in-speech through sustained applause, with a shorter redemption time.
const LOCAL_VAD_THRESHOLDS: (f32, f32) = (0.35, 0.25);
const LOCAL_VAD_REDEMPTION_TIME_MS: u32 = 800;

/// Default real-time factor estimates when no benchmark is available.
fn default_rtf(model: Option<&str>, provider: Option<&str>) -> f64 {
    let is_remote = provider == Some("remote")
        || model.map(|m| m.starts_with("qwen3-asr-remote")).unwrap_or(false);
    if is_remote {
        0.08
    } else if model.map(|m| m.starts_with("x-asr-")).unwrap_or(false) {
        0.03
    } else {
        // Local Sherpa-ONNX SenseVoice
        0.35
    }
}

/// Estimate fixed/overhead stage durations based on the total audio duration.
/// These coefficients are tuned from observed runs: decode + VAD together is
/// roughly 2-4% of the audio duration on typical meeting recordings.
fn estimate_stage_durations(duration_seconds: f64) -> (f64, f64, f64) {
    let decode = duration_seconds * 0.015 + 5.0;
    let vad = duration_seconds * 0.012 + 3.0;
    let save = 5.0;
    (decode, vad, save)
}

/// Tracks dynamic progress and ETA for offline retranscription.
#[derive(Clone)]
struct ProgressEstimator {
    inner: std::sync::Arc<std::sync::Mutex<ProgressEstimatorInner>>,
}

struct ProgressEstimatorInner {
    start_time: Instant,
    decode_est: f64,
    vad_est: f64,
    save_est: f64,
    rtf: f64,
    total_speech_sec: f64,
    chunk_process_times: Vec<f64>,
    processed_speech_sec: f64,
}

impl ProgressEstimator {
    fn new(start_time: Instant, duration_seconds: f64, total_speech_sec: f64, rtf: f64) -> Self {
        let (decode_est, vad_est, save_est) = estimate_stage_durations(duration_seconds);
        Self {
            inner: std::sync::Arc::new(std::sync::Mutex::new(ProgressEstimatorInner {
                start_time,
                decode_est,
                vad_est,
                save_est,
                rtf,
                total_speech_sec,
                chunk_process_times: Vec::new(),
                processed_speech_sec: 0.0,
            })),
        }
    }

    fn record_decode_done(&self, actual_sec: f64) {
        let mut inner = self.inner.lock().unwrap();
        inner.decode_est = actual_sec.max(0.1);
    }

    fn record_vad_done(&self, actual_sec: f64) {
        let mut inner = self.inner.lock().unwrap();
        inner.vad_est = actual_sec.max(0.1);
    }

    fn set_total_speech_sec(&self, total_speech_sec: f64) {
        let mut inner = self.inner.lock().unwrap();
        inner.total_speech_sec = total_speech_sec;
    }

    fn record_chunk(&self, speech_sec: f64, process_time: f64) {
        let mut inner = self.inner.lock().unwrap();
        inner.chunk_process_times.push(process_time / speech_sec.max(0.1));
        inner.processed_speech_sec += speech_sec;
    }

    fn progress_pct(&self, stage: &str, stage_fraction: f64) -> u32 {
        let inner = self.inner.lock().unwrap();
        let (d, v, t, s) = {
            let total = inner.decode_est + inner.vad_est + inner.total_speech_sec * inner.rtf + inner.save_est;
            if total <= 0.0 {
                (0.05, 0.05, 0.85, 0.05)
            } else {
                let d = inner.decode_est / total;
                let v = inner.vad_est / total;
                let s = inner.save_est / total;
                let t = 1.0 - d - v - s;
                (d, v, t, s)
            }
        };
        let frac = match stage {
            "decoding" => stage_fraction.clamp(0.0, 1.0) * d,
            "vad" => d + stage_fraction.clamp(0.0, 1.0) * v,
            "transcribing" => {
                let tx_frac = if inner.total_speech_sec > 0.0 {
                    inner.processed_speech_sec / inner.total_speech_sec
                } else {
                    0.0
                };
                d + v + tx_frac.clamp(0.0, 1.0) * t
            }
            "saving" => d + v + t + stage_fraction.clamp(0.0, 1.0) * s,
            "complete" => 1.0,
            _ => 0.0,
        };
        (frac * 100.0).clamp(0.0, 100.0) as u32
    }

    fn estimated_remaining(&self, stage: &str, stage_fraction: f64) -> f64 {
        let inner = self.inner.lock().unwrap();
        match stage {
            "decoding" => {
                (1.0 - stage_fraction.clamp(0.0, 1.0)) * inner.decode_est
                    + inner.vad_est
                    + inner.total_speech_sec * inner.rtf
                    + inner.save_est
            }
            "vad" => {
                (1.0 - stage_fraction.clamp(0.0, 1.0)) * inner.vad_est
                    + inner.total_speech_sec * inner.rtf
                    + inner.save_est
            }
            "transcribing" => {
                let remaining_speech = (inner.total_speech_sec - inner.processed_speech_sec).max(0.0);
                let eff_rtf = if inner.chunk_process_times.is_empty() {
                    inner.rtf
                } else {
                    let window = inner.chunk_process_times.len().min(5);
                    let sum: f64 = inner.chunk_process_times.iter().rev().take(window).sum();
                    sum / window as f64
                };
                remaining_speech * eff_rtf + inner.save_est
            }
            "saving" => (1.0 - stage_fraction.clamp(0.0, 1.0)) * inner.save_est,
            "complete" => 0.0,
            _ => 0.0,
        }
    }

    fn elapsed_secs(&self) -> f64 {
        let inner = self.inner.lock().unwrap();
        inner.start_time.elapsed().as_secs_f64()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetranscriptionProgress {
    pub meeting_id: String,
    pub stage: String,
    pub progress_percentage: u32,
    pub message: String,
    pub elapsed_seconds: Option<f64>,
    pub estimated_remaining_seconds: Option<f64>,
    pub chunks_total: Option<usize>,
    pub chunks_processed: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetranscriptionResult {
    pub meeting_id: String,
    pub segments_count: usize,
    pub duration_seconds: f64,
    pub language: Option<String>,
    pub elapsed_seconds: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetranscriptionError {
    pub meeting_id: String,
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetranscriptionPartial {
    pub meeting_id: String,
    pub chunk_index: usize,
    pub chunks_total: usize,
    pub text: String,
    pub start_ms: f64,
    pub end_ms: f64,
}

pub fn is_retranscription_in_progress() -> bool {
    RETRANSCRIPTION_IN_PROGRESS.load(Ordering::SeqCst)
}

pub fn cancel_retranscription() {
    RETRANSCRIPTION_CANCELLED.store(true, Ordering::SeqCst);
}

/// Start retranscription of a meeting's audio
pub async fn start_retranscription<R: Runtime>(
    app: AppHandle<R>,
    meeting_id: String,
    meeting_folder_path: String,
    language: Option<String>,
    model: Option<String>,
    provider: Option<String>,
    estimated_rtf: Option<f64>,
) -> Result<RetranscriptionResult> {
    let _guard = RetranscriptionGuard::acquire().map_err(|e| anyhow!(e))?;

    RETRANSCRIPTION_CANCELLED.store(false, Ordering::SeqCst);

    let result = run_retranscription(app.clone(), meeting_id.clone(), meeting_folder_path, language, model, provider, estimated_rtf).await;

    super::common::unload_engine_after_batch().await;

    match &result {
        Ok(res) => {
            let _ = app.emit(
                "retranscription-complete",
                serde_json::json!({
                    "meeting_id": res.meeting_id,
                    "segments_count": res.segments_count,
                    "duration_seconds": res.duration_seconds,
                    "language": res.language,
                    "elapsed_seconds": res.elapsed_seconds
                }),
            );
        }
        Err(e) => {
            let _ = app.emit(
                "retranscription-error",
                RetranscriptionError {
                    meeting_id: meeting_id.clone(),
                    error: e.to_string(),
                },
            );
        }
    }

    result
}

fn find_audio_file(folder: &Path) -> Result<PathBuf> {
    let candidates = [
        "audio.mp4", "audio.m4a", "audio.wav", "audio.mp3",
        "audio.flac", "audio.ogg", "recording.mp4",
        "audio.mkv", "audio.webm", "audio.wma",
    ];

    for name in candidates {
        let path = folder.join(name);
        if path.exists() {
            return Ok(path);
        }
    }

    if let Ok(entries) = std::fs::read_dir(folder) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension() {
                let ext = ext.to_string_lossy().to_lowercase();
                if AUDIO_EXTENSIONS.contains(&ext.as_str()) {
                    return Ok(path);
                }
            }
        }
    }

    Err(anyhow!("No audio file found in: {}", folder.display()))
}

async fn run_retranscription<R: Runtime>(
    app: AppHandle<R>,
    meeting_id: String,
    meeting_folder_path: String,
    language: Option<String>,
    model: Option<String>,
    provider: Option<String>,
    estimated_rtf: Option<f64>,
) -> Result<RetranscriptionResult> {
    let start_time = Instant::now();
    let folder_path = PathBuf::from(&meeting_folder_path);
    let audio_path = find_audio_file(&folder_path)?;

    info!(
        "Starting retranscription for meeting {}",
        meeting_id
    );

    // Early validation: if remote ASR is requested, check endpoint is configured BEFORE
    // spending time on audio decoding and VAD processing.
    let is_remote = provider.as_deref() == Some("remote")
        || model
            .as_deref()
            .map(|m| m.starts_with("qwen3-asr-remote"))
            .unwrap_or(false);
    let is_xasr = model.as_deref().map(|m| m.starts_with("x-asr-")).unwrap_or(false);
    if is_remote {
        let endpoint = crate::audio::transcription::get_remote_asr_endpoint();
        if endpoint.is_empty() {
            return Err(anyhow!(
                "Remote ASR endpoint not configured. Please set the remote ASR URL in Settings."
            ));
        }
        info!("Using remote Qwen3-ASR for retranscription: {}", endpoint);
    }

    let rtf = estimated_rtf.unwrap_or_else(|| default_rtf(model.as_deref(), provider.as_deref()));
    info!("Retranscription estimated RTF: {:.3}", rtf);

    // Probe audio duration from metadata before full decode so we can show a
    // realistic ETA from the very first progress event.
    let duration_seconds = probe_audio_duration(&audio_path).unwrap_or_else(|e| {
        warn!("Failed to probe audio duration: {}, using 0", e);
        0.0
    });
    info!("Probed audio duration: {:.2}s", duration_seconds);

    // Create estimator with the probed duration; speech estimate is 80% of total.
    let estimator = ProgressEstimator::new(start_time, duration_seconds, duration_seconds * 0.8, rtf);
    emit_progress(&app, &meeting_id, "decoding", 0.0, "解码音频文件...", &estimator, None, None);

    if RETRANSCRIPTION_CANCELLED.load(Ordering::SeqCst) {
        return Err(anyhow!("Retranscription cancelled"));
    }

    let path_for_decode = audio_path.clone();
    let decoded = tokio::task::spawn_blocking(move || decode_audio_file(&path_for_decode))
        .await
        .map_err(|e| anyhow!("Decode task panicked: {}", e))??;

    info!(
        "Decoded audio: {:.2}s, {}Hz, {} channels",
        decoded.duration_seconds, decoded.sample_rate, decoded.channels
    );

    if RETRANSCRIPTION_CANCELLED.load(Ordering::SeqCst) {
        return Err(anyhow!("Retranscription cancelled"));
    }

    // Convert to mono + normalize, but skip resampling — VAD will handle it internally
    let vad_sample_rate = decoded.sample_rate;
    let audio_samples = tokio::task::spawn_blocking(move || {
        let mono = if decoded.channels > 1 {
            audio_to_mono(&decoded.samples, decoded.channels)
        } else {
            decoded.samples
        };
        let max_abs = mono.iter().filter(|s| s.is_finite()).map(|s| s.abs()).fold(0.0f32, f32::max);
        let mut mono = if max_abs > 1.0 {
            let scale = 1.0 / max_abs;
            mono.into_iter().map(|s| s * scale).collect()
        } else {
            mono
        };
        for s in &mut mono {
            if !s.is_finite() { *s = 0.0; } else { *s = s.clamp(-1.0, 1.0); }
        }
        mono
    })
    .await
    .map_err(|e| anyhow!("Mono conversion panicked: {}", e))?;
    info!(
        "Preprocessed audio: {}Hz, {} samples ({:.1}s)",
        vad_sample_rate,
        audio_samples.len(),
        audio_samples.len() as f64 / vad_sample_rate as f64
    );

    // Record actual decode + preprocessing time and advance to VAD stage.
    let decode_actual_sec = start_time.elapsed().as_secs_f64();
    estimator.record_decode_done(decode_actual_sec);
    emit_progress(&app, &meeting_id, "decoding", 1.0, "预处理完成", &estimator, None, None);

    emit_progress(&app, &meeting_id, "vad", 0.0, "VAD: 过滤静音, 检测语音段落...", &estimator, None, None);

    if RETRANSCRIPTION_CANCELLED.load(Ordering::SeqCst) {
        return Err(anyhow!("Retranscription cancelled"));
    }

    let app_for_vad = app.clone();
    let meeting_id_for_vad = meeting_id.clone();
    let estimator_for_vad = estimator.clone();
    let vad_start = Instant::now();

    // Local SenseVoice uses a tuned, more sensitive VAD pass (see constants);
    // remote / X-ASR keep the previous defaults.
    let (vad_redemption_ms, vad_thresholds) = if is_remote || is_xasr {
        (VAD_REDEMPTION_TIME_MS, None)
    } else {
        (LOCAL_VAD_REDEMPTION_TIME_MS, Some(LOCAL_VAD_THRESHOLDS))
    };

    let speech_segments = tokio::task::spawn_blocking(move || {
        get_speech_chunks_with_config(
            &audio_samples,
            vad_sample_rate,
            vad_redemption_ms,
            vad_thresholds,
            |vad_progress, segments_found| {
                let stage_fraction = vad_progress as f64 / 100.0;
                emit_progress(
                    &app_for_vad,
                    &meeting_id_for_vad,
                    "vad",
                    stage_fraction,
                    &format!("Detecting speech segments... {}% ({} found)", vad_progress, segments_found),
                    &estimator_for_vad,
                    None,
                    None,
                );
                !RETRANSCRIPTION_CANCELLED.load(Ordering::SeqCst)
            },
        )
    })
    .await
    .map_err(|e| anyhow!("VAD task panicked: {}", e))?
    .map_err(|e| anyhow!("VAD processing failed: {}", e))?;

    let total_segments = speech_segments.len();
    let vad_actual_sec = vad_start.elapsed().as_secs_f64();
    let total_speech_sec: f64 = speech_segments
        .iter()
        .map(|s| (s.end_timestamp_ms - s.start_timestamp_ms) / 1000.0)
        .sum();
    estimator.record_vad_done(vad_actual_sec);
    estimator.set_total_speech_sec(total_speech_sec);
    info!(
        "VAD detected {} speech segments, total speech {:.1}s, VAD took {:.1}s",
        total_segments, total_speech_sec, vad_actual_sec
    );

    if total_segments == 0 {
        warn!("No speech detected in audio");
        return Err(anyhow!("No speech detected in audio file"));
    }

    const DEFAULT_CHUNK_DURATION_SECS: u32 = 600; // 10 min for remote/X-ASR
    const LOCAL_CHUNK_DURATION_SECS: u32 = 120;   // 2 min for local SenseVoice
    const VAD_GAP_SAMPLES: usize = 800;             // 50ms silence gap between merged segments
    const MIN_SEGMENT_SAMPLES: usize = 1600;        // skip VAD artifacts < 100ms

    // Local SenseVoice is further chunked inside SherpaOnnxProvider, but keeping
    // the top-level chunks smaller gives the UI progress updates more frequently
    // and keeps memory spikes low. Remote/X-ASR can take larger chunks.
    let chunk_duration_secs: u32 = if is_remote || is_xasr {
        DEFAULT_CHUNK_DURATION_SECS
    } else {
        LOCAL_CHUNK_DURATION_SECS
    };
    let chunk_samples: usize = chunk_duration_secs as usize * 16000;

    let mut chunks: Vec<crate::audio::vad::SpeechSegment> = Vec::new();

    if is_remote || is_xasr {
        // Merge consecutive VAD segments into large chunks so every model gets ample
        // context. Safety: if a single VAD segment exceeds the chunk limit (e.g. 2
        // hours of continuous speech with no pause), it is split at chunk_samples
        // boundaries to prevent unbounded memory use or REST request timeouts.
        let mut cur_samples: Vec<f32> = Vec::new();
        let mut cur_start_ms: f64 = 0.0;

        for segment in &speech_segments {
            if segment.samples.len() < MIN_SEGMENT_SAMPLES { continue; }

            // Extremely long VAD segment: split into chunk-sized sub-chunks,
            // preferring silence boundaries so we don't cut mid-sentence.
            if segment.samples.len() > chunk_samples {
                // Flush any pending partial chunk first
                if !cur_samples.is_empty() {
                    // Use the start of the current (over-long) segment as the end of the
                    // partial chunk, not the end of the entire audio.
                    let end_ms = segment.start_timestamp_ms;
                    chunks.push(crate::audio::vad::SpeechSegment {
                        samples: std::mem::take(&mut cur_samples),
                        start_timestamp_ms: cur_start_ms,
                        end_timestamp_ms: end_ms,
                        confidence: 0.9,
                    });
                }
                let ranges = crate::audio::chunking::split_at_silence(
                    &segment.samples,
                    16000,
                    chunk_duration_secs as f64,
                    1.0,  // ±1s search radius
                    0.2,  // 200ms silence window
                    0.5,  // 500ms minimum tail
                );
                let ms_per_sample = (segment.end_timestamp_ms - segment.start_timestamp_ms) / segment.samples.len() as f64;
                for (start, end) in ranges {
                    chunks.push(crate::audio::vad::SpeechSegment {
                        samples: segment.samples[start..end].to_vec(),
                        start_timestamp_ms: segment.start_timestamp_ms + start as f64 * ms_per_sample,
                        end_timestamp_ms: segment.start_timestamp_ms + end as f64 * ms_per_sample,
                        confidence: segment.confidence,
                    });
                }
                continue;
            }

            let needed = if cur_samples.is_empty() {
                segment.samples.len()
            } else {
                VAD_GAP_SAMPLES + segment.samples.len()
            };

            if !cur_samples.is_empty() && cur_samples.len() + needed > chunk_samples {
                chunks.push(crate::audio::vad::SpeechSegment {
                    samples: std::mem::take(&mut cur_samples),
                    start_timestamp_ms: cur_start_ms,
                    end_timestamp_ms: segment.start_timestamp_ms,
                    confidence: 0.9,
                });
                cur_start_ms = segment.start_timestamp_ms;
                cur_samples.extend_from_slice(&segment.samples);
            } else {
                if cur_samples.is_empty() {
                    cur_start_ms = segment.start_timestamp_ms;
                } else {
                    cur_samples.extend(std::iter::repeat(0.0f32).take(VAD_GAP_SAMPLES));
                }
                cur_samples.extend_from_slice(&segment.samples);
            }
        }
        // Push final chunk
        if !cur_samples.is_empty() {
            let end_ms = speech_segments.last().map(|s| s.end_timestamp_ms).unwrap_or(0.0);
            chunks.push(crate::audio::vad::SpeechSegment {
                samples: cur_samples,
                start_timestamp_ms: cur_start_ms,
                end_timestamp_ms: end_ms,
                confidence: 0.9,
            });
        }
    } else {
        // Local SenseVoice: transcribe each VAD speech segment individually so the
        // saved segments keep real speech boundaries and timestamps. Merging them
        // into large chunks used to produce one giant segment per chunk; the
        // provider already windows long audio internally, and per-segment chunks
        // also give finer progress updates.
        for segment in &speech_segments {
            if segment.samples.len() < MIN_SEGMENT_SAMPLES { continue; }

            if segment.samples.len() > chunk_samples {
                // Extremely long single segment: split at silence boundaries.
                let ranges = crate::audio::chunking::split_at_silence(
                    &segment.samples,
                    16000,
                    chunk_duration_secs as f64,
                    1.0,  // ±1s search radius
                    0.2,  // 200ms silence window
                    0.5,  // 500ms minimum tail
                );
                let ms_per_sample = (segment.end_timestamp_ms - segment.start_timestamp_ms) / segment.samples.len() as f64;
                for (start, end) in ranges {
                    chunks.push(crate::audio::vad::SpeechSegment {
                        samples: segment.samples[start..end].to_vec(),
                        start_timestamp_ms: segment.start_timestamp_ms + start as f64 * ms_per_sample,
                        end_timestamp_ms: segment.start_timestamp_ms + end as f64 * ms_per_sample,
                        confidence: segment.confidence,
                    });
                }
            } else {
                chunks.push(segment.clone());
            }
        }
    }

    if is_remote || is_xasr {
        info!("Merged {} VAD segments into {} large chunks ({}s speech max/chunk)",
            total_segments, chunks.len(), chunk_duration_secs);
    } else {
        info!("Prepared {} per-VAD-segment chunks from {} VAD segments for local SenseVoice",
            chunks.len(), total_segments);
    }

    if chunks.is_empty() {
        return Err(anyhow!("No speech detected after VAD"));
    }

    let chunks_count = chunks.len();
    let chunk_total_speech_sec: f64 = chunks.iter().map(|c| c.samples.len() as f64 / 16000.0).sum();
    estimator.set_total_speech_sec(chunk_total_speech_sec);
    info!("Chunk speech total: {:.1}s across {} chunks", chunk_total_speech_sec, chunks_count);

    let mut all_transcripts: Vec<(String, f64, f64)> = Vec::new();

    if is_xasr {
        use crate::audio::transcription::x_asr_provider::XAsrProvider;
        let xasr_model = model.clone().unwrap_or_else(|| "x-asr-480ms".to_string());
        // Ensure engine is loaded
        if !crate::sherpa_onnx_engine::commands::is_xasr_engine_loaded() {
            if let Err(e) = crate::sherpa_onnx_engine::commands::sherpa_onnx_load_model(xasr_model.clone()).await {
                return Err(anyhow!("Failed to load X-ASR model: {}", e));
            }
        }
        let engine = crate::sherpa_onnx_engine::commands::get_or_init_xasr_engine()
            .map_err(|e| anyhow!("X-ASR engine not ready: {}", e))?;
        let xasr_provider = XAsrProvider::new_with_engine(xasr_model, engine);

        emit_progress(&app, &meeting_id, "transcribing", 0.0,
            &format!("X-ASR: {} 个大段, 准备发送...", chunks_count),
            &estimator, Some(chunks_count), Some(0));

        let total_speech_sec: f64 = chunks.iter().map(|c| c.samples.len() as f64 / 16000.0).sum();
        estimator.set_total_speech_sec(total_speech_sec);

        for (i, chunk) in chunks.iter().enumerate() {
            if RETRANSCRIPTION_CANCELLED.load(Ordering::SeqCst) { return Err(anyhow!("Cancelled")); }

            let speech_sec = chunk.samples.len() as f64 / 16000.0;
            let chunk_start = Instant::now();

            // 500ms silence padding pre/post for streaming transducer model
            const XASR_PAD: usize = 8000;
            let mut padded = vec![0.0f32; XASR_PAD + chunk.samples.len() + XASR_PAD];
            padded[XASR_PAD..XASR_PAD + chunk.samples.len()].copy_from_slice(&chunk.samples);

            let pcm_i16: Vec<i16> = padded.iter().map(|&s| (s.clamp(-1.0, 1.0) * 32767.0) as i16).collect();
            let temp_wav = folder_path.join(format!("_xasr_temp_{}.wav", i));
            write_wav_file(&temp_wav, &pcm_i16, 16000, 1)?;

            match xasr_provider.transcribe_file(&temp_wav).await {
                Ok(text) => {
                    let _ = std::fs::remove_file(&temp_wav);
                    let trimmed = text.trim().to_string();
                    if !trimmed.is_empty() {
                        debug!("X-ASR chunk {}/{} ({:.0}s): text='{}'", i + 1, chunks_count, speech_sec,
                            if trimmed.len() > 100 { let mut e = 100; while !trimmed.is_char_boundary(e) { e -= 1; } &trimmed[..e] } else { &trimmed });
                        all_transcripts.push((trimmed.clone(), chunk.start_timestamp_ms, chunk.end_timestamp_ms));
                        emit_partial(&app, &meeting_id, i, chunks_count, &trimmed, chunk.start_timestamp_ms, chunk.end_timestamp_ms);
                    }
                }
                Err(e) => {
                    let _ = std::fs::remove_file(&temp_wav);
                    warn!("X-ASR chunk {}/{} failed: {}", i + 1, chunks_count, e);
                }
            }
            let chunk_time = chunk_start.elapsed().as_secs_f64();
            estimator.record_chunk(speech_sec, chunk_time);
            emit_progress(&app, &meeting_id, "transcribing", 0.0,
                &format!("X-ASR 转写中 {}/{} ({:.0}s 音频)...", i + 1, chunks_count, speech_sec),
                &estimator, Some(chunks_count), Some(i + 1));
        }
    } else {
        emit_progress(&app, &meeting_id, "transcribing", 0.0,
            &format!("加载识别引擎... ({} 个大段)", chunks_count),
            &estimator, Some(chunks_count), Some(0));

        let transcription_provider = get_or_init_sherpa_onnx(model, provider).await?;

        let total_speech_sec: f64 = chunks.iter().map(|c| c.samples.len() as f64 / 16000.0).sum();
        estimator.set_total_speech_sec(total_speech_sec);

        for (i, chunk) in chunks.iter().enumerate() {
            if RETRANSCRIPTION_CANCELLED.load(Ordering::SeqCst) { return Err(anyhow!("Cancelled")); }

            let speech_sec = chunk.samples.len() as f64 / 16000.0;
            let chunk_start = Instant::now();

            match transcription_provider.transcribe(chunk.samples.clone(), language.clone()).await {
                Ok(result) => {
                    let trimmed = result.text.trim().to_string();
                    if !trimmed.is_empty() {
                        debug!("Chunk {}/{} ({:.0}s): text='{}'", i + 1, chunks_count, speech_sec,
                            if trimmed.len() > 100 { let mut e = 100; while !trimmed.is_char_boundary(e) { e -= 1; } &trimmed[..e] } else { &trimmed });
                        all_transcripts.push((trimmed.clone(), chunk.start_timestamp_ms, chunk.end_timestamp_ms));
                        emit_partial(&app, &meeting_id, i, chunks_count, &trimmed, chunk.start_timestamp_ms, chunk.end_timestamp_ms);
                    }
                }
                Err(e) => {
                    warn!("Transcription failed on chunk {}: {}", i, e);
                }
            }
            let chunk_time = chunk_start.elapsed().as_secs_f64();
            estimator.record_chunk(speech_sec, chunk_time);
            emit_progress(&app, &meeting_id, "transcribing", 0.0,
                &format!("转写中 {}/{} ({:.0}s 音频)...", i + 1, chunks_count, speech_sec),
                &estimator, Some(chunks_count), Some(i + 1));
        }
    }

    let transcribed_count = all_transcripts.len();
    info!("Transcription complete: {} segments transcribed", transcribed_count);

    if RETRANSCRIPTION_CANCELLED.load(Ordering::SeqCst) {
        return Err(anyhow!("Retranscription cancelled"));
    }

    emit_progress(&app, &meeting_id, "saving", 0.0, "保存转录结果到数据库...", &estimator, None, None);

    let segments = create_transcript_segments(&all_transcripts);

    let app_state = app
        .try_state::<AppState>()
        .ok_or_else(|| anyhow!("App state not available"))?;

    let pool = app_state.db_manager.pool();
    let mut conn = pool.acquire().await.map_err(|e| anyhow!("DB error: {}", e))?;
    let mut tx = sqlx::Connection::begin(&mut *conn)
        .await
        .map_err(|e| anyhow!("Failed to start transaction: {}", e))?;

    let now = chrono::Utc::now();

    // Only delete previous offline_asr segments, keep realtime ones
    sqlx::query("DELETE FROM transcript_segments WHERE recording_id = ? AND source = 'offline_asr'")
        .bind(&meeting_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| anyhow!("Failed to delete existing offline segments: {}", e))?;

    for segment in &segments {
        sqlx::query(
            "INSERT INTO transcript_segments (id, recording_id, text, start_ms, end_ms, source, created_at)
             VALUES (?, ?, ?, ?, ?, 'offline_asr', ?)"
        )
        .bind(&segment.id)
        .bind(&meeting_id)
        .bind(&segment.text)
        .bind(segment.audio_start_time.map(|s| (s * 1000.0) as i64).unwrap_or(0))
        .bind(segment.audio_end_time.map(|e| (e * 1000.0) as i64))
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(|e| anyhow!("Failed to insert transcript segment: {}", e))?;
    }

    // Mark the recording as transcribed and persist the probed audio duration
    // (imported recordings stay 'pending' and duration-less without this).
    sqlx::query("UPDATE recordings SET status = 'completed', duration_ms = ?, updated_at = ? WHERE id = ?")
        .bind((duration_seconds * 1000.0) as i64)
        .bind(now)
        .bind(&meeting_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| anyhow!("Failed to update recording status/duration: {}", e))?;

    tx.commit().await
        .map_err(|e| anyhow!("Failed to commit transaction: {}", e))?;

    info!(
        "Saved {} offline ASR segments for recording {}",
        segments.len(),
        meeting_id
    );

    emit_progress(&app, &meeting_id, "saving", 0.5, "写入转录文件...", &estimator, None, None);

    if let Err(e) = write_offline_transcripts_json(&folder_path, &segments) {
        warn!("Failed to write transcripts_offline.json: {}", e);
    }

    let audio_filename = audio_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("audio.mp4")
        .to_string();

    if let Err(e) = write_retranscription_metadata(
        &folder_path,
        &meeting_id,
        duration_seconds,
        &audio_filename,
    ) {
        warn!("Failed to update metadata.json: {}", e);
    }

    let elapsed = start_time.elapsed();
    emit_progress(&app, &meeting_id, "complete", 1.0, &format!("识别完成，共耗时 {:.1}s", elapsed.as_secs_f64()), &estimator, None, None);

    Ok(RetranscriptionResult {
        meeting_id,
        segments_count: segments.len(),
        duration_seconds,
        language,
        elapsed_seconds: elapsed.as_secs_f64(),
    })
}

/// Write raw i16 PCM to a WAV file.
fn write_wav_file(path: &Path, samples: &[i16], sample_rate: u32, channels: u16) -> Result<()> {
    use std::io::Write;
    let mut file = std::fs::File::create(path)?;
    let data_size = (samples.len() * std::mem::size_of::<i16>()) as u32;
    let file_size = 44 + data_size;

    // WAV header
    file.write_all(b"RIFF")?;
    file.write_all(&(file_size - 8).to_le_bytes())?;
    file.write_all(b"WAVE")?;

    // fmt chunk
    file.write_all(b"fmt ")?;
    file.write_all(&16u32.to_le_bytes())?; // chunk size
    file.write_all(&1u16.to_le_bytes())?;  // PCM format
    file.write_all(&channels.to_le_bytes())?;
    file.write_all(&sample_rate.to_le_bytes())?;
    let byte_rate = sample_rate * channels as u32 * 2;
    file.write_all(&byte_rate.to_le_bytes())?;
    file.write_all(&(channels * 2).to_le_bytes())?; // block align
    file.write_all(&16u16.to_le_bytes())?; // bits per sample

    // data chunk
    file.write_all(b"data")?;
    file.write_all(&data_size.to_le_bytes())?;
    for &s in samples {
        file.write_all(&s.to_le_bytes())?;
    }

    Ok(())
}

fn emit_progress<R: Runtime>(
    app: &AppHandle<R>,
    meeting_id: &str,
    stage: &str,
    stage_fraction: f64,
    message: &str,
    estimator: &ProgressEstimator,
    chunks_total: Option<usize>,
    chunks_processed: Option<usize>,
) {
    let progress = estimator.progress_pct(stage, stage_fraction);
    let elapsed_secs = estimator.elapsed_secs();
    let estimated_remaining = if stage == "complete" {
        None
    } else {
        let remaining = estimator.estimated_remaining(stage, stage_fraction);
        if remaining > 0.0 {
            Some(remaining)
        } else {
            None
        }
    };

    let _ = app.emit(
        "retranscription-progress",
        RetranscriptionProgress {
            meeting_id: meeting_id.to_string(),
            stage: stage.to_string(),
            progress_percentage: progress,
            message: message.to_string(),
            elapsed_seconds: Some(elapsed_secs),
            estimated_remaining_seconds: estimated_remaining,
            chunks_total,
            chunks_processed,
        },
    );
}

fn emit_partial<R: Runtime>(
    app: &AppHandle<R>,
    meeting_id: &str,
    chunk_index: usize,
    chunks_total: usize,
    text: &str,
    start_ms: f64,
    end_ms: f64,
) {
    let _ = app.emit(
        "retranscription-partial",
        RetranscriptionPartial {
            meeting_id: meeting_id.to_string(),
            chunk_index,
            chunks_total,
            text: text.to_string(),
            start_ms,
            end_ms,
        },
    );
}

/// Get or initialize the transcription engine (local Sherpa-ONNX or remote Qwen3-ASR)
async fn get_or_init_sherpa_onnx(
    model: Option<String>,
    provider: Option<String>,
) -> Result<Arc<dyn crate::audio::transcription::provider::TranscriptionProvider>> {
    use crate::audio::transcription::remote_asr_provider::RemoteAsrProvider;

    // Determine if remote ASR should be used:
    // 1) provider is explicitly "remote", or
    // 2) model name starts with "qwen3-asr-remote" (defense in depth)
    let is_remote = provider.as_deref() == Some("remote")
        || model
            .as_deref()
            .map(|m| m.starts_with("qwen3-asr-remote"))
            .unwrap_or(false);

    if is_remote {
        let endpoint = crate::audio::transcription::get_remote_asr_endpoint();
        let model_name = crate::audio::transcription::get_remote_asr_model();
        if endpoint.is_empty() {
            return Err(anyhow!(
                "Remote ASR endpoint not configured. Please set the remote ASR URL in Settings."
            ));
        }
        info!(
            "Using remote Qwen3-ASR for retranscription: {} (model: {})",
            endpoint, model_name
        );
        let remote_provider = RemoteAsrProvider::create_with_model_detection(&endpoint, &model_name, false)
            .await
            .map_err(|e| anyhow!("Failed to initialize remote ASR for retranscription: {}", e))?;
        info!("Remote Qwen3-ASR health check passed");
        return Ok(Arc::new(remote_provider));
    }

    // Use specified local model, defaulting to "sense-voice"
    let model_name = model.unwrap_or_else(|| "sense-voice".to_string());

    if !crate::sherpa_onnx_engine::commands::sherpa_onnx_is_model_loaded()
        .await
        .unwrap_or(false)
    {
        info!("Auto-loading Sherpa-ONNX model for retranscription: {}", model_name);
        crate::sherpa_onnx_engine::commands::sherpa_onnx_load_model(model_name)
            .await
            .map_err(|e| anyhow!("Failed to load Sherpa-ONNX model: {}", e))?;
    }

    let engine = crate::sherpa_onnx_engine::commands::get_or_init_engine()
        .map_err(|e| anyhow!("Sherpa-ONNX engine not ready: {}", e))?;
    let sherpa_provider =
        crate::audio::transcription::sherpa_onnx_provider::SherpaOnnxProvider::new(engine);
    Ok(Arc::new(sherpa_provider))
}

fn write_retranscription_metadata(
    folder: &Path,
    meeting_id: &str,
    duration_seconds: f64,
    audio_filename: &str,
) -> Result<()> {
    let metadata_path = folder.join("metadata.json");
    let temp_path = folder.join(".metadata.json.tmp");
    let now = chrono::Utc::now().to_rfc3339();

    let json = if metadata_path.exists() {
        let existing = std::fs::read_to_string(&metadata_path)?;
        let mut value: serde_json::Value = serde_json::from_str(&existing)?;
        if let Some(obj) = value.as_object_mut() {
            obj.insert("retranscribed_at".to_string(), serde_json::json!(now));
            obj.insert("status".to_string(), serde_json::json!("completed"));
            obj.insert("transcript_file".to_string(), serde_json::json!("transcripts_offline.json"));
            obj.insert("source".to_string(), serde_json::json!("offline_asr"));
        }
        value
    } else {
        serde_json::json!({
            "version": "1.0",
            "meeting_id": meeting_id,
            "created_at": now,
            "completed_at": now,
            "retranscribed_at": now,
            "duration_seconds": duration_seconds,
            "audio_file": audio_filename,
            "transcript_file": "transcripts_offline.json",
            "status": "completed",
            "source": "retranscription"
        })
    };

    let json_string = serde_json::to_string_pretty(&json)?;
    std::fs::write(&temp_path, &json_string)?;
    std::fs::rename(&temp_path, &metadata_path)?;

    info!("Wrote metadata.json to {}", metadata_path.display());
    Ok(())
}

/// Write offline ASR transcripts to transcripts_offline.json (separate from realtime transcripts.json)
fn write_offline_transcripts_json(folder: &Path, segments: &[crate::api::TranscriptSegment]) -> Result<()> {
    let transcript_path = folder.join("transcripts_offline.json");
    let temp_path = folder.join(".transcripts_offline.json.tmp");

    let json = serde_json::json!({
        "version": "1.0",
        "source": "offline_asr",
        "last_updated": chrono::Utc::now().to_rfc3339(),
        "total_segments": segments.len(),
        "segments": segments.iter().enumerate().map(|(i, s)| {
            serde_json::json!({
                "id": s.id,
                "text": s.text,
                "timestamp": s.timestamp,
                "audio_start_time": s.audio_start_time,
                "audio_end_time": s.audio_end_time,
                "duration": s.duration,
                "sequence_id": i
            })
        }).collect::<Vec<_>>()
    });

    let json_string = serde_json::to_string_pretty(&json)?;
    std::fs::write(&temp_path, &json_string)?;
    std::fs::rename(&temp_path, &transcript_path)?;

    info!(
        "Wrote transcripts_offline.json with {} segments to {}",
        segments.len(),
        transcript_path.display()
    );
    Ok(())
}

// Tauri commands

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetranscriptionStarted {
    pub meeting_id: String,
    pub message: String,
}

#[tauri::command]
pub async fn start_retranscription_command<R: Runtime>(
    app: AppHandle<R>,
    meeting_id: String,
    meeting_folder_path: String,
    language: Option<String>,
    model: Option<String>,
    provider: Option<String>,
    estimated_rtf: Option<f64>,
) -> Result<RetranscriptionStarted, String> {
    if RETRANSCRIPTION_IN_PROGRESS.load(Ordering::SeqCst) {
        return Err("Retranscription already in progress".to_string());
    }

    let meeting_id_clone = meeting_id.clone();

    tauri::async_runtime::spawn(async move {
        let result = start_retranscription(
            app,
            meeting_id_clone,
            meeting_folder_path,
            language,
            model,
            provider,
            estimated_rtf,
        )
        .await;

        if let Err(e) = result {
            error!("Retranscription failed: {}", e);
        }
    });

    Ok(RetranscriptionStarted {
        meeting_id,
        message: "Retranscription started".to_string(),
    })
}

#[tauri::command]
pub async fn cancel_retranscription_command() -> Result<(), String> {
    if !is_retranscription_in_progress() {
        return Err("No retranscription in progress".to_string());
    }
    cancel_retranscription();
    Ok(())
}

#[tauri::command]
pub async fn is_retranscription_in_progress_command() -> bool {
    is_retranscription_in_progress()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_transcript_segments_empty() {
        let transcripts: Vec<(String, f64, f64)> = vec![];
        let segments = create_transcript_segments(&transcripts);
        assert!(segments.is_empty());
    }

    #[test]
    fn test_create_transcript_segments_single() {
        let transcripts = vec![
            ("Hello world".to_string(), 0.0, 1500.0),
        ];
        let segments = create_transcript_segments(&transcripts);

        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].text, "Hello world");
        assert_eq!(segments[0].audio_start_time, Some(0.0));
        assert_eq!(segments[0].audio_end_time, Some(1.5));
        assert_eq!(segments[0].duration, Some(1.5));
    }

    #[test]
    fn test_cancellation_flag() {
        RETRANSCRIPTION_CANCELLED.store(false, Ordering::SeqCst);
        RETRANSCRIPTION_IN_PROGRESS.store(false, Ordering::SeqCst);
        assert!(!is_retranscription_in_progress());
        cancel_retranscription();
        assert!(RETRANSCRIPTION_CANCELLED.load(Ordering::SeqCst));
        RETRANSCRIPTION_CANCELLED.store(false, Ordering::SeqCst);
    }
}
