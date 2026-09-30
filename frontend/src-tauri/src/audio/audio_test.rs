// audio/audio_test.rs
//
// Audio-test mode for the realtime-transcription setup dialog.
// Plays example_audio.wav through the default output device while capturing
// microphone + system audio through the same pipeline used by meeting recording.
// The selected ASR model transcribes the mixed stream and results are streamed
// to the frontend via the `audio-test-transcript` event.

use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc, Mutex, OnceLock,
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

/// 启动阶段的取消信号：start_audio_test 在注册进 AUDIO_TEST_SESSION 之前
/// （模型校验/设备启动窗口）被 stop 时，stop 没有会话可拿，就置位当前取消标志；
/// start 在关键检查点读取它并自行清理退出。会话注册后则走会话内的 cancelled 标志。
static CURRENT_CANCEL: Mutex<Option<Arc<AtomicBool>>> = Mutex::new(None);

/// 进程级缓存：example_audio.wav 的 24k→48k sinc 重采样耗时 ~5.6s，
/// 每次测试都重做会让「点开始 → 出声」等待接近 9 秒（用户感知为「加载时间长」）。
/// 文件内容固定，首次测试时算一次，之后的测试直接复用。
static RESAMPLED_WAV_CACHE: OnceLock<Arc<Vec<f32>>> = OnceLock::new();

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
    /// 播放控制器：启动阶段（等待管线稳定）为 None，播放开始后挂上。
    wav_playback: Option<WavPlaybackController>,
    samples: Arc<Vec<f32>>,
    source_rate: u32,
    /// 会话被 stop 时置位；start 任务在等待/播放检查点读取并放弃继续。
    cancelled: Arc<AtomicBool>,
    /// 注销 transcript-update 监听的闭包（stop 与 Drop 二选一执行）。
    /// 会话被覆盖/丢弃而不注销会导致监听泄漏：同一个转写事件被转发两次，
    /// 前端识别文本重复（2026-09-20 日志排查出的「识别重复」根因之一）。
    unlisten: Option<Box<dyn FnOnce() + Send>>,
}

impl Drop for AudioTestSession {
    fn drop(&mut self) {
        if let Some(unlisten) = self.unlisten.take() {
            unlisten();
        }
    }
}

/// Start an audio-test session.
///
/// * `model_name` — ASR model to exercise (e.g. "x-asr-480ms", "sense-voice",
///   "qwen3-asr-remote-streaming").
/// * `mic_device_name` / `system_device_name` — 可选的设备名（测试弹窗里用户显式
///   选择的采集设备）；均为 None 时沿用系统默认（macOS 智能选择）。
///
/// On success, returns the duration of the test WAV in seconds.
#[tauri::command]
pub async fn start_audio_test<R: Runtime>(
    app: AppHandle<R>,
    model_name: String,
    mic_device_name: Option<String>,
    system_device_name: Option<String>,
    remote_asr_model: Option<String>,
) -> Result<f32, String> {
    // 先停掉任何残留会话再启动：前端「停止→启动」两步之间旧会话可能尚未清理完，
    // 直接返回「正在进行中」会让重新测试偶发失败。停旧开新保证只有一个采集管线。
    stop_internal().await;

    info!("🎧 Starting audio test with model: {}", model_name);

    // 远程模型竞态修复（2026-09-28，与 api_save_transcript_config 的 remote_asr_model 同源）：
    // 前端把当前真实远程选择一并传来，后端以此为权威 —— 用户刚在弹窗里切完模型就点
    // 「开始测试」时，选择器的异步持久化可能还没落盘，不覆盖就会拿旧模型跑测试。
    if model_name.starts_with("qwen3-asr-remote") {
        if let Some(explicit) = remote_asr_model.as_deref() {
            transcription::apply_explicit_remote_asr_model(explicit);
        }
    }

    // 音频测试免计费：测试期间的远程流式会话带 free=1（网关记 0 积分消耗）。
    // 所有提前返回路径都必须复位，否则下一次录音会被误判成免费。
    transcription::remote_asr_streaming_provider::set_asr_free_billing(true);

    // 取消信号：在会话注册进 AUDIO_TEST_SESSION 之前（校验/建流窗口）收到 stop 时，
    // stop 会把该标志置位，start 在检查点读到后自行清理退出。
    let cancel = Arc::new(AtomicBool::new(false));
    *CURRENT_CANCEL.lock().unwrap() = Some(cancel.clone());

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
            None,
        )
        .await
        .map_err(|e| format!("保存 ASR 配置失败: {}", e))?;
    }

    // 2. Validate / load the model before opening the audio pipeline.
    if let Err(e) = transcription::validate_transcription_model_ready(&app).await {
        error!("Audio test model validation failed: {}", e);
        transcription::remote_asr_streaming_provider::set_asr_free_billing(false);
        return Err(format!(
            "ASR 模型加载失败：{}。请返回上一步更换模型后再试。",
            e
        ));
    }

    // 3. Resolve the test WAV resource path.
    //    dev 模式下 app.path().resource_dir() 可能报 "unknown path"（未打包无资源目录），
    //    此时回退到源码目录（CARGO_MANIFEST_DIR = frontend/src-tauri）。
    let wav_path = app
        .path()
        .resource_dir()
        .ok()
        .map(|d| d.join("example_audio.wav"))
        .filter(|p| p.exists())
        .unwrap_or_else(|| {
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("example_audio.wav")
        });

    if !wav_path.exists() {
        transcription::remote_asr_streaming_provider::set_asr_free_billing(false);
        return Err(format!(
            "找不到测试音频文件: {}。请确认应用已正确打包。",
            wav_path.display()
        ));
    }

    // 预重采样到 48kHz（P0 修复 + 进程级缓存）：example_audio.wav 是 24kHz，
    // sinc 重采样耗时 ~5.6s，原先每次测试都在这里重做，且「缓存进会话」随会话
    // 结束即失效——每次测试都重新付这笔时间。改为进程级 OnceLock 缓存，首次算一次，
    // 之后的测试（含 X-ASR/远程切换重启）零等待。
    let samples: Arc<Vec<f32>> = if let Some(cached) = RESAMPLED_WAV_CACHE.get() {
        cached.clone()
    } else {
        let wav_bytes = std::fs::read(&wav_path).map_err(|e| format!("读取测试音频失败: {}", e))?;
        let (parsed, parsed_rate) =
            parse_wav(&wav_bytes).map_err(|e| format!("解析 WAV 失败: {}", e))?;
        let t_resample = Instant::now();
        let resampled = if parsed_rate != 48000 {
            let rate = parsed_rate;
            let res = tokio::task::spawn_blocking(move || {
                super::audio_processing::resample_audio(&parsed, rate, 48000)
            })
            .await
            .map_err(|e| format!("重采样任务失败: {}", e));
            match res {
                Ok(v) => v,
                Err(e) => {
                    transcription::remote_asr_streaming_provider::set_asr_free_billing(false);
                    return Err(e);
                }
            }
        } else {
            parsed
        };
        info!(
            "🔊 WAV pre-resampled {}→48000Hz ({} samples) in {:.1?}",
            parsed_rate,
            resampled.len(),
            t_resample.elapsed()
        );
        RESAMPLED_WAV_CACHE
            .get_or_init(|| Arc::new(resampled))
            .clone()
    };
    let source_rate = 48000u32;

    let duration_seconds = samples.len() as f32 / source_rate as f32;

    // 4. Build a standalone RecordingManager (do NOT use the global
    //    RECORDING_MANAGER used by meeting recording).
    let mut manager = RecordingManager::new();
    // X-ASR 与「远程流式 ASR」都需要持续音频流 → 绕过 VAD 分段
    let bypass_vad = model_name.starts_with("x-asr-")
        || (model_name.starts_with("qwen3-asr-remote")
            && crate::audio::transcription::engine::is_remote_asr_streaming());

    // 设备：用户在测试弹窗显式选择时走自定义设备路径；否则沿用系统默认（macOS 智能选择）
    let transcription_receiver = if mic_device_name.is_some() || system_device_name.is_some() {
        use super::devices::configuration::{AudioDevice, DeviceType};
        let mic_device = match &mic_device_name {
            Some(name) => Some(Arc::new(
                AudioDevice::from_name_with_hint(name, &DeviceType::Input)
                    .map_err(|e| format!("无效的麦克风设备 '{}': {}", name, e))?,
            )),
            None => super::devices::default_input_device().ok().map(Arc::new),
        };
        let sys_device = match &system_device_name {
            Some(name) => Some(Arc::new(
                AudioDevice::from_name_with_hint(name, &DeviceType::Output)
                    .map_err(|e| format!("无效的系统音频设备 '{}': {}", name, e))?,
            )),
            None => super::devices::default_output_device().ok().map(Arc::new),
        };
        info!(
            "🎧 Audio test with custom devices: mic={:?}, system={:?}",
            mic_device_name, system_device_name
        );
        manager
            .start_recording(
                mic_device,
                sys_device,
                false,
                mic_device_name.is_none(),
                system_device_name.is_none(),
                bypass_vad,
                // 音频测试不设会议名，saver 不会创建会议目录；此参数仅为满足签名
                super::recording_preferences::get_default_recordings_folder(),
            )
            .await
            .map_err(|e| {
                transcription::remote_asr_streaming_provider::set_asr_free_billing(false);
                format!("启动音频采集失败: {}", e)
            })?
    } else {
        manager
            .start_recording_with_defaults_and_auto_save(
                false,
                bypass_vad,
                super::recording_preferences::get_default_recordings_folder(),
            )
            .await
            .map_err(|e| {
                transcription::remote_asr_streaming_provider::set_asr_free_billing(false);
                format!("启动音频采集失败: {}", e)
            })?
    };

    // Audio test should capture both system audio and microphone speech.
    // The RecordingState defaults to mic-muted for meeting scenarios, so
    // explicitly unmute here.
    manager.unmute_microphone();
    info!("🎤 Audio test: microphone unmuted");

    // 取消检查点：启动（校验/建流）期间用户已点停止 → 自行清理后静默退出。
    if cancel.load(Ordering::Relaxed) {
        info!("🎧 Audio test cancelled during startup (before transcription task)");
        transcription::remote_asr_streaming_provider::set_asr_free_billing(false);
        let _ = manager.stop_streams_and_force_flush().await;
        manager.cleanup_without_save().await;
        return Ok(duration_seconds);
    }

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
                    "paragraph_id": update.paragraph_id,
                }),
            );
        }
    });
    let unlisten_app = app.clone();
    let unlisten: Box<dyn FnOnce() + Send> = Box::new(move || {
        unlisten_app.unlisten(transcript_listener_id);
    });

    // 7. Start real-time audio level monitoring for the waveform bar.
    let mic_name = manager
        .get_state()
        .get_microphone_device()
        .map(|d| d.name.clone());
    let sys_name = manager
        .get_state()
        .get_system_device()
        .map(|d| d.name.clone());
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

    // ⚠️ 关键：先注册会话再进入等待。此前注册推迟到播放开始之后，导致
    // 「启动窗口内点停止」找不到会话而被忽略——旧会话继续跑、继续播放测试音、
    // 继续按时长计费（2026-09-20 日志：切换模型后 mimo 会话仍播完整段测试音）。
    // 会话与启动取消信号共用同一个 Arc：注册后 stop 走会话内标志，
    // 注册前 stop 走 CURRENT_CANCEL——无论停在哪一步，检查点都能读到。
    {
        let mut guard = AUDIO_TEST_SESSION.lock().unwrap();
        *guard = Some(AudioTestSession {
            manager,
            _transcription_task: task_handle,
            wav_playback: None,
            samples: samples.clone(),
            source_rate,
            cancelled: cancel.clone(),
            unlisten: Some(unlisten),
        });
    }

    // 8. Give the ASR pipeline a moment to be ready before starting playback.
    //    This ensures the very beginning of the test WAV is captured by the
    //    system-audio loopback and transcribed, rather than being played before
    //    the ASR is listening. 等待可被停止打断（100ms 粒度）。
    info!("⏳ Waiting for ASR pipeline to stabilize before playback...");
    let mut remaining_ms = 2000u32;
    while remaining_ms > 0 {
        if cancel.load(Ordering::Relaxed) {
            // stop 已接管会话并完成清理（stop_internal 里会复位免计费标志）
            return Ok(duration_seconds);
        }
        let step = remaining_ms.min(100);
        tokio::time::sleep(Duration::from_millis(step as u64)).await;
        remaining_ms -= step;
    }

    // 9. Start playback of the test WAV on the default output device.  The
    //    system-audio capture loopback will pick it up and mix it with the mic.
    let (wav_playback, playback_ready) = spawn_wav_playback(samples.to_vec(), source_rate);
    let ready_result = tokio::task::spawn_blocking(move || {
        playback_ready.recv_timeout(Duration::from_secs(5)).ok()
    })
    .await
    .unwrap_or(None);
    if cancel.load(Ordering::Relaxed) {
        // 播放流刚建好但用户已停止：立即停掉播放线程，避免测试音继续外放
        transcription::remote_asr_streaming_provider::set_asr_free_billing(false);
        wav_playback.stop();
        wav_playback.join();
        return Ok(duration_seconds);
    }
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

    // 把播放控制器挂回会话。若挂接时发现会话已被 stop 取走（take），说明
    // 停止发生在「播放已出声、控制器尚未挂接」的瞬间——立即停掉播放。
    {
        let mut guard = AUDIO_TEST_SESSION.lock().unwrap();
        match guard.as_mut() {
            Some(session) => {
                session.wav_playback = Some(wav_playback);
            }
            None => {
                transcription::remote_asr_streaming_provider::set_asr_free_billing(false);
                wav_playback.stop();
                wav_playback.join();
                return Ok(duration_seconds);
            }
        }
    }

    info!(
        "✅ Audio test started, WAV duration: {:.2}s",
        duration_seconds
    );
    Ok(duration_seconds)
}

/// 停止当前音频测试会话并释放全部资源（start 与 stop 命令共用）。
/// 会话不存在但启动还在早期阶段（尚未注册会话）时，置位启动取消信号，
/// 由 start 任务在检查点自行清理——保证「启动中点停止」真正生效。
async fn stop_internal() {
    let session_opt = {
        let mut guard = AUDIO_TEST_SESSION.lock().unwrap();
        guard.take()
    };

    // 测试会话结束：复位免计费标志（无论是否有会话）
    transcription::remote_asr_streaming_provider::set_asr_free_billing(false);

    let Some(mut session) = session_opt else {
        // 启动早期阶段（校验模型/建流中，会话尚未注册）：置位取消信号，
        // 由 start 任务在检查点读到后自行清理退出。
        if let Some(cancel) = CURRENT_CANCEL.lock().unwrap().as_ref() {
            cancel.store(true, Ordering::Relaxed);
            info!("🎧 Audio test cancelled while starting");
        }
        return;
    };

    session.cancelled.store(true, Ordering::Relaxed);
    let playback = session.wav_playback.take();

    // Stop playback first so the user doesn't continue hearing the test tone.
    if let Some(p) = &playback {
        p.stop();
    }

    // Stop level monitoring.
    let _ = simple_level_monitor::stop_monitoring().await;

    // Stop capture and flush remaining audio through the pipeline.
    if let Err(e) = session.manager.stop_streams_and_force_flush().await {
        warn!("Error stopping audio test streams: {}", e);
    }

    // Clean up recording state.
    session.manager.cleanup_without_save().await;

    // Remove transcript listener（会话 Drop 兜底双保险：此处显式注销）。
    if let Some(unlisten) = session.unlisten.take() {
        unlisten();
    }

    // Wait for playback thread to finish.
    if let Some(p) = playback {
        p.join();
    }

    info!("✅ Audio test stopped and cleaned up");
}

/// Stop the running audio-test session and release all resources.
#[tauri::command]
pub async fn stop_audio_test<R: Runtime>(_app: AppHandle<R>) -> Result<(), String> {
    info!("🛑 Stopping audio test");
    stop_internal().await;
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
            if let Some(p) = &session.wav_playback {
                p.stop();
            }
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
            session.wav_playback = Some(wav_playback);
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
        info!(
            "🔊 WAV playback: default output device resolved in {:.1?}",
            t0.elapsed()
        );

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
        info!(
            "🔊 WAV playback: enumerated configs in {:.1?}",
            t1.elapsed()
        );

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
                info!(
                    "✅ WAV playback stream built with low-latency buffer size {:?} in {:.1?}",
                    stream_config.buffer_size,
                    t3.elapsed()
                );
                s
            }
            Err(e) => {
                warn!(
                    "Low-latency playback config failed ({}), falling back to default buffer size",
                    e
                );
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
