// audio/audio_test.rs
//
// Audio-test mode for the realtime-transcription setup dialog.
// Plays example_audio.wav through the default output device while capturing
// microphone + system audio through the same pipeline used by meeting recording.
// The selected ASR model transcribes the mixed stream and results are streamed
// to the frontend via the `audio-test-transcript` event.

use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc, Mutex,
};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::Result;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, SampleRate};
use log::{error, info, warn};
use tauri::{AppHandle, Emitter, Listener, Manager, Runtime};
use tokio::task::JoinHandle;

use super::{
    simple_level_monitor,
    transcription::{self, start_transcription_task, TranscriptUpdate},
    RecordingManager,
};

// Global test session so only one audio test can run at a time.
static AUDIO_TEST_SESSION: Mutex<Option<AudioTestSession>> = Mutex::new(None);

struct WavPlaybackController {
    stop_flag: Arc<AtomicBool>,
    thread_handle: Option<thread::JoinHandle<()>>,
}

impl WavPlaybackController {
    fn stop(&self) {
        self.stop_flag.store(true, Ordering::Relaxed);
    }

    fn join(self) {
        if let Some(handle) = self.thread_handle {
            let _ = handle.join();
        }
    }
}

struct AudioTestSession {
    manager: RecordingManager,
    _transcription_task: JoinHandle<()>,
    wav_playback: WavPlaybackController,
    transcript_listener_id: tauri::EventId,
    samples: Arc<Vec<f32>>,
    source_rate: u32,
}

/// Start an audio-test session.
///
/// * `model_name` — ASR model to exercise (e.g. "x-asr-480ms", "sense-voice",
///   "qwen3-asr-remote-streaming").
///
/// On success, returns the duration of the test WAV in seconds.
#[tauri::command]
pub async fn start_audio_test<R: Runtime>(
    app: AppHandle<R>,
    model_name: String,
) -> Result<f32, String> {
    // Prevent concurrent sessions.
    {
        let guard = AUDIO_TEST_SESSION.lock().unwrap();
        if guard.is_some() {
            return Err("音频测试正在进行中".to_string());
        }
    }

    info!("🎧 Starting audio test with model: {}", model_name);

    // 1. Persist the selected model as the active transcript config so the
    //    transcription worker picks it up.
    let provider = if model_name.starts_with("x-asr-") {
        "x-asr"
    } else if model_name.starts_with("qwen3-asr-remote") {
        "remote-qwen3-asr"
    } else {
        "sherpaonnx"
    };

    {
        let state = app.state::<crate::state::AppState>();
        crate::api::api::api_save_transcript_config(
            app.clone(),
            state,
            provider.to_string(),
            model_name.clone(),
            None,
            None,
        )
        .await
        .map_err(|e| format!("保存 ASR 配置失败: {}", e))?;
    }

    // 2. Validate / load the model before opening the audio pipeline.
    if let Err(e) = transcription::validate_transcription_model_ready(&app).await {
        error!("Audio test model validation failed: {}", e);
        return Err(format!(
            "ASR 模型加载失败：{}。请返回上一步更换模型后再试。",
            e
        ));
    }

    // 3. Resolve the test WAV resource path.
    let resource_dir = app
        .path()
        .resource_dir()
        .map_err(|e| format!("无法定位应用资源目录: {}", e))?;
    let wav_path = resource_dir.join("example_audio.wav");
    info!("Audio test WAV path: {:?}", wav_path);

    if !wav_path.exists() {
        return Err(format!(
            "找不到测试音频文件: {}。请确认应用已正确打包。",
            wav_path.display()
        ));
    }

    let wav_bytes = std::fs::read(&wav_path)
        .map_err(|e| format!("读取测试音频失败: {}", e))?;
    let (samples, source_rate) =
        parse_wav(&wav_bytes).map_err(|e| format!("解析 WAV 失败: {}", e))?;
    let samples = Arc::new(samples);

    let duration_seconds = samples.len() as f32 / source_rate as f32;

    // 4. Build a standalone RecordingManager (do NOT use the global
    //    RECORDING_MANAGER used by meeting recording).
    let mut manager = RecordingManager::new();
    let bypass_vad = model_name.starts_with("x-asr-");

    let transcription_receiver = manager
        .start_recording_with_defaults_and_auto_save(false, bypass_vad)
        .await
        .map_err(|e| format!("启动音频采集失败: {}", e))?;

    // Audio test should capture both system audio and microphone speech.
    // The RecordingState defaults to mic-muted for meeting scenarios, so
    // explicitly unmute here.
    manager.unmute_microphone();
    info!("🎤 Audio test: microphone unmuted");

    // 5. Start the transcription worker.
    let task_handle = start_transcription_task(app.clone(), transcription_receiver);

    // 6. Forward transcript-update events as audio-test-transcript while the
    //    dialog is open. The frontend mirrors the meeting-transcript display
    //    logic (replace-by-sequence_id for partials, append for finals).
    let app_for_transcript = app.clone();
    let transcript_listener_id = app.listen("transcript-update", move |event: tauri::Event| {
        if let Ok(update) = serde_json::from_str::<TranscriptUpdate>(event.payload()) {
            let _ = app_for_transcript.emit(
                "audio-test-transcript",
                serde_json::json!({
                    "text": update.text,
                    "timestamp": update.timestamp,
                    "source": update.source,
                    "sequence_id": update.sequence_id,
                    "chunk_start_time": update.chunk_start_time,
                    "is_partial": update.is_partial,
                    "confidence": update.confidence,
                    "audio_start_time": update.audio_start_time,
                    "audio_end_time": update.audio_end_time,
                    "duration": update.duration,
                }),
            );
        }
    });

    // 7. Start real-time audio level monitoring for the waveform bar.
    let mic_name = manager.get_state().get_microphone_device().map(|d| d.name.clone());
    let sys_name = manager.get_state().get_system_device().map(|d| d.name.clone());
    let mut monitoring_names = Vec::new();
    if let Some(name) = mic_name {
        monitoring_names.push(name);
    }
    if let Some(name) = sys_name {
        monitoring_names.push(name);
    }
    if !monitoring_names.is_empty() {
        let _ = simple_level_monitor::start_monitoring(app.clone(), monitoring_names).await;
    }

    // 8. Give the ASR pipeline a moment to be ready before starting playback.
    //    This ensures the very beginning of the test WAV is captured by the
    //    system-audio loopback and transcribed, rather than being played before
    //    the ASR is listening.
info!("⏳ Waiting for ASR pipeline to stabilize before playback...");
tokio::time::sleep(Duration::from_millis(3000)).await;

    // 9. Start playback of the test WAV on the default output device.  The
    //    system-audio capture loopback will pick it up and mix it with the mic.
    let (wav_playback, playback_ready) = spawn_wav_playback(samples.to_vec(), source_rate);
    let ready_result = tokio::task::spawn_blocking(move || {
        playback_ready.recv_timeout(Duration::from_secs(5)).ok()
    })
    .await
    .unwrap_or(None);
    if ready_result.is_some() {
        info!("✅ WAV playback is live");
    } else {
        warn!("⚠️ WAV playback readiness timeout; continuing anyway");
    }

    // Notify the frontend so it can synchronize the progress bar with real playback.
    let _ = app.emit(
        "audio-test-playback-started",
        serde_json::json!({ "duration": duration_seconds }),
    );

    let session = AudioTestSession {
        manager,
        _transcription_task: task_handle,
        wav_playback,
        transcript_listener_id,
        samples: samples.clone(),
        source_rate,
    };

    {
        let mut guard = AUDIO_TEST_SESSION.lock().unwrap();
        *guard = Some(session);
    }

    info!("✅ Audio test started, WAV duration: {:.2}s", duration_seconds);
    Ok(duration_seconds)
}

/// Stop the running audio-test session and release all resources.
#[tauri::command]
pub async fn stop_audio_test<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    info!("🛑 Stopping audio test");

    let session_opt = {
        let mut guard = AUDIO_TEST_SESSION.lock().unwrap();
        guard.take()
    };

    if let Some(mut session) = session_opt {
        // Stop playback first so the user doesn't continue hearing the test tone.
        session.wav_playback.stop();

        // Stop level monitoring.
        let _ = simple_level_monitor::stop_monitoring().await;

        // Stop capture and flush remaining audio through the pipeline.
        if let Err(e) = session.manager.stop_streams_and_force_flush().await {
            warn!("Error stopping audio test streams: {}", e);
        }

        // Clean up recording state.
        session.manager.cleanup_without_save().await;

        // Remove transcript listener.
        {
            use tauri::Listener;
            app.unlisten(session.transcript_listener_id);
        }

        // Wait for playback thread to finish.
        session.wav_playback.join();

        info!("✅ Audio test stopped and cleaned up");
    } else {
        info!("No active audio test session to stop");
    }

    Ok(())
}

/// Replay the test WAV in the existing audio-test session without restarting
/// the ASR pipeline. The ASR is already listening, so the repeated audio is
/// captured by the system-audio loopback and transcribed.
#[tauri::command]
pub async fn replay_audio_test<R: Runtime>(app: AppHandle<R>) -> Result<f32, String> {
    info!("🔁 Replaying audio test WAV");

    let (samples, source_rate) = {
        let guard = AUDIO_TEST_SESSION.lock().unwrap();
        match guard.as_ref() {
            Some(session) => (session.samples.clone(), session.source_rate),
            None => return Err("没有正在进行的音频测试".to_string()),
        }
    };

    let duration_seconds = samples.len() as f32 / source_rate as f32;

    // Stop any currently running playback and swap in a fresh controller.
    {
        let mut guard = AUDIO_TEST_SESSION.lock().unwrap();
        if let Some(session) = guard.as_mut() {
            session.wav_playback.stop();
        }
    }

    let (wav_playback, playback_ready) = spawn_wav_playback(samples.to_vec(), source_rate);
    let ready_result = tokio::task::spawn_blocking(move || {
        playback_ready.recv_timeout(Duration::from_secs(5)).ok()
    })
    .await
    .unwrap_or(None);
    if ready_result.is_some() {
        info!("✅ WAV replay is live");
    } else {
        warn!("⚠️ WAV replay readiness timeout; continuing anyway");
    }

    {
        let mut guard = AUDIO_TEST_SESSION.lock().unwrap();
        if let Some(session) = guard.as_mut() {
            session.wav_playback = wav_playback;
        }
    }

    // Notify the frontend so it can synchronize the progress bar with the replay.
    let _ = app.emit(
        "audio-test-playback-started",
        serde_json::json!({ "duration": duration_seconds }),
    );

    Ok(duration_seconds)
}

/// Spawn a CPAL playback thread that plays the decoded mono WAV samples through
/// the default output device.  The returned controller can stop playback early.
fn spawn_wav_playback(
    samples: Vec<f32>,
    source_rate: u32,
) -> (WavPlaybackController, std::sync::mpsc::Receiver<()>) {
    let stop_flag = Arc::new(AtomicBool::new(false));
    let stop_flag_thread = stop_flag.clone();
    let (ready_tx, ready_rx) = std::sync::mpsc::channel::<()>();

    let handle = thread::spawn(move || {
        let t0 = Instant::now();
        let host = match cpal::default_host() {
            h => h,
        };
        let device = match host.default_output_device() {
            Some(d) => d,
            None => {
                error!("No default output device for WAV playback");
                return;
            }
        };
        info!("🔊 WAV playback: default output device resolved in {:.1?}", t0.elapsed());

        let target_rate = 48000u32;
        let output_samples = if source_rate != target_rate {
            super::audio_processing::resample_audio(&samples, source_rate, target_rate)
        } else {
            samples
        };

        let t1 = Instant::now();
        let supported = match device.supported_output_configs() {
            Ok(c) => c,
            Err(e) => {
                error!("Failed to enumerate output configs: {}", e);
                return;
            }
        };
        info!("🔊 WAV playback: enumerated configs in {:.1?}", t1.elapsed());

        let t2 = Instant::now();
        let config_range = match supported
            .filter(|c| c.sample_format() == SampleFormat::F32)
            .find_map(|c| c.try_with_sample_rate(SampleRate(target_rate)))
        {
            Some(c) => c,
            None => {
                error!("Default output device does not support 48kHz f32 playback");
                return;
            }
        };
        info!("🔊 WAV playback: selected config in {:.1?}", t2.elapsed());

        let channels = config_range.channels();
        let stream_rate = config_range.sample_rate().0;

        // Try a low-latency configuration first to avoid several seconds of
        // audible delay between the UI progress bar and actual playback.
        let mut stream_config = config_range.config();
        if let cpal::SupportedBufferSize::Range { min, .. } = config_range.buffer_size() {
            // Clamp to a sensible low-latency target (~10ms at 48kHz).
            let target = (*min).max(480).min(2048);
            stream_config.buffer_size = cpal::BufferSize::Fixed(target);
        }

        let samples_arc = Arc::new(output_samples);
        let idx = Arc::new(AtomicUsize::new(0));

        // Build with low-latency config; fall back to default if rejected.
        let err_callback = |err| error!("WAV playback stream error: {}", err);

        let build_stream = |config: &cpal::StreamConfig| {
            let samples_c = samples_arc.clone();
            let idx_c = idx.clone();
            let stop_flag_stream = stop_flag_thread.clone();
            device.build_output_stream(
                config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    if stop_flag_stream.load(Ordering::Relaxed) {
                        for s in data.iter_mut() {
                            *s = 0.0;
                        }
                        return;
                    }

                    let mut i = idx_c.load(Ordering::Relaxed);
                    for frame in data.chunks_mut(channels as usize) {
                        let sample = if i < samples_c.len() {
                            samples_c[i]
                        } else {
                            0.0
                        };
                        for ch in frame.iter_mut() {
                            *ch = sample;
                        }
                        i += 1;
                    }
                    idx_c.store(i, Ordering::Relaxed);
                },
                err_callback,
                None,
            )
        };

        let t3 = Instant::now();
        let stream = match build_stream(&stream_config) {
            Ok(s) => {
                info!("✅ WAV playback stream built with low-latency buffer size {:?} in {:.1?}", stream_config.buffer_size, t3.elapsed());
                s
            }
            Err(e) => {
                warn!("Low-latency playback config failed ({}), falling back to default buffer size", e);
                match build_stream(&config_range.config()) {
                    Ok(s) => s,
                    Err(e) => {
                        error!("Failed to build WAV playback stream: {}", e);
                        return;
                    }
                }
            }
        };

        let t4 = Instant::now();
        if let Err(e) = stream.play() {
            error!("Failed to start WAV playback: {}", e);
            return;
        }
        info!("🔊 WAV playback stream.play() took {:.1?}", t4.elapsed());
        let _ = ready_tx.send(());

        let duration = Duration::from_secs_f32(samples_arc.len() as f32 / stream_rate as f32);
        let start = Instant::now();
        while start.elapsed() < duration {
            if stop_flag_thread.load(Ordering::Relaxed) {
                break;
            }
            thread::sleep(Duration::from_millis(50));
        }

        // Small grace period then drop the stream.
        thread::sleep(Duration::from_millis(100));
    });

    (
        WavPlaybackController {
            stop_flag,
            thread_handle: Some(handle),
        },
        ready_rx,
    )
}

/// Parse a WAV file into mono f32 samples.
/// (Moved here from the removed `tts_output` module.)
fn parse_wav(wav_bytes: &[u8]) -> Result<(Vec<f32>, u32), String> {
    if wav_bytes.len() < 44 {
        return Err("WAV 数据太短".to_string());
    }
    if &wav_bytes[0..4] != b"RIFF" || &wav_bytes[8..12] != b"WAVE" {
        return Err("无效的 WAV 文件".to_string());
    }

    let mut sample_rate = 16000u32;
    let mut bits_per_sample = 16u16;
    let mut channels = 1u16;
    let mut data_offset = 0usize;
    let mut data_size = 0usize;

    let mut pos = 12usize;
    while pos + 8 <= wav_bytes.len() {
        let chunk_id = &wav_bytes[pos..pos + 4];
        let chunk_size = u32::from_le_bytes([
            wav_bytes[pos + 4],
            wav_bytes[pos + 5],
            wav_bytes[pos + 6],
            wav_bytes[pos + 7],
        ]) as usize;

        if chunk_id == b"fmt " && chunk_size >= 16 && pos + 24 <= wav_bytes.len() {
            channels = u16::from_le_bytes([wav_bytes[pos + 10], wav_bytes[pos + 11]]);
            sample_rate = u32::from_le_bytes([
                wav_bytes[pos + 12],
                wav_bytes[pos + 13],
                wav_bytes[pos + 14],
                wav_bytes[pos + 15],
            ]);
            bits_per_sample = u16::from_le_bytes([wav_bytes[pos + 22], wav_bytes[pos + 23]]);
        }

        if chunk_id == b"data" {
            data_offset = pos + 8;
            data_size = chunk_size;
            break;
        }

        pos += 8 + chunk_size + (chunk_size % 2);
    }

    if data_offset == 0 || data_offset + data_size > wav_bytes.len() {
        return Err("未找到 WAV data chunk".to_string());
    }

    let pcm = &wav_bytes[data_offset..data_offset + data_size];
    let samples: Vec<f32> = match bits_per_sample {
        16 => pcm
            .chunks_exact(2)
            .map(|b| i16::from_le_bytes([b[0], b[1]]) as f32 / 32768.0)
            .collect(),
        24 => pcm
            .chunks_exact(3)
            .map(|b| {
                let mut v = (b[0] as i32) | ((b[1] as i32) << 8) | ((b[2] as i32) << 16);
                if v & 0x800000 != 0 {
                    v |= !0xFFFFFF;
                }
                v as f32 / 8388608.0
            })
            .collect(),
        32 => pcm
            .chunks_exact(4)
            .map(|b| i32::from_le_bytes([b[0], b[1], b[2], b[3]]) as f32 / 2147483648.0)
            .collect(),
        _ => return Err(format!("不支持的采样位数: {}", bits_per_sample)),
    };

    if channels <= 1 {
        Ok((samples, sample_rate))
    } else {
        let mono: Vec<f32> = samples
            .chunks(channels as usize)
            .map(|frame| frame.iter().sum::<f32>() / channels as f32)
            .collect();
        Ok((mono, sample_rate))
    }
}
