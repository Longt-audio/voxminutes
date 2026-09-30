// audio/recording_commands.rs
//
// Slim Tauri command layer for recording functionality.
// Delegates to transcription and recording modules for actual implementation.

use anyhow::Result;
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use tauri::{AppHandle, Emitter, Manager, Runtime};
use tokio::task::JoinHandle;

use super::{
    default_input_device,  // Get default microphone
    default_output_device, // Get default system audio
    parse_audio_device,
    DeviceEvent,
    DeviceMonitorType,
    RecordingDeviceType,
    RecordingManager,
};

// Import transcription modules
use super::transcription::{self, reset_speech_detected_flag};

// Re-export TranscriptUpdate for backward compatibility
pub use super::transcription::TranscriptUpdate;

// ============================================================================
// GLOBAL STATE
// ============================================================================

// Simple recording state tracking
static IS_RECORDING: AtomicBool = AtomicBool::new(false);

// Global recording manager and transcription task to keep them alive during recording
static RECORDING_MANAGER: Mutex<Option<RecordingManager>> = Mutex::new(None);
static TRANSCRIPTION_TASK: Mutex<Option<JoinHandle<()>>> = Mutex::new(None);

// Listener ID for proper cleanup - prevents microphone from staying active after recording stops
static TRANSCRIPT_LISTENER_ID: Mutex<Option<tauri::EventId>> = Mutex::new(None);

// Default-device follower task handle and stop signal.
// Spun up when the user is following system default devices during recording.
static DEFAULT_DEVICE_FOLLOWER_STOP: Mutex<Option<Arc<tokio::sync::Notify>>> = Mutex::new(None);
static DEFAULT_DEVICE_FOLLOWER_HANDLE: Mutex<Option<JoinHandle<()>>> = Mutex::new(None);

// ============================================================================
// PUBLIC TYPES
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct RecordingArgs {
    pub save_path: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct TranscriptionStatus {
    pub chunks_in_queue: usize,
    pub is_processing: bool,
    pub last_activity_ms: u64,
}

// ============================================================================
// DEFAULT-DEVICE FOLLOWER HELPERS
// ============================================================================

/// Spawn the background task that watches system default devices and rebuilds
/// streams when they change. The task locks the global `RECORDING_MANAGER` when
/// it needs to perform a rebuild.
fn spawn_default_device_follower<R: Runtime>(
    app: AppHandle<R>,
    state: Arc<crate::audio::recording_state::RecordingState>,
    follow_mic: bool,
    follow_system: bool,
) {
    let stop = Arc::new(tokio::sync::Notify::new());
    {
        let mut global_stop = DEFAULT_DEVICE_FOLLOWER_STOP.lock().unwrap();
        *global_stop = Some(stop.clone());
    }

    let handle = tokio::spawn(default_device_follower_loop(
        state,
        stop,
        follow_mic,
        follow_system,
        app,
    ));

    {
        let mut global_handle = DEFAULT_DEVICE_FOLLOWER_HANDLE.lock().unwrap();
        *global_handle = Some(handle);
    }
}

/// Signal the follower task to stop and wait briefly for it to finish.
async fn stop_default_device_follower() {
    {
        let mut stop = DEFAULT_DEVICE_FOLLOWER_STOP.lock().unwrap();
        if let Some(s) = stop.take() {
            s.notify_one();
        }
    }

    let handle = {
        let mut handle = DEFAULT_DEVICE_FOLLOWER_HANDLE.lock().unwrap();
        handle.take()
    };

    if let Some(h) = handle {
        let _ = tokio::time::timeout(tokio::time::Duration::from_secs(2), h).await;
    }
}

/// Background loop: poll default input/output devices and rebuild streams when
/// they change, while keeping the recording session alive.
async fn default_device_follower_loop<R: Runtime>(
    state: Arc<crate::audio::recording_state::RecordingState>,
    stop: Arc<tokio::sync::Notify>,
    follow_mic: bool,
    follow_system: bool,
    app: AppHandle<R>,
) {
    use std::time::{Duration, Instant};

    let mut mic_last: Option<String> = None;
    let mut sys_last: Option<String> = None;
    let mut mic_changed_at: Option<Instant> = None;
    let mut sys_changed_at: Option<Instant> = None;

    info!(
        "🎧 Default-device follower started (follow_mic={}, follow_system={})",
        follow_mic, follow_system
    );

    loop {
        tokio::select! {
            _ = stop.notified() => {
                info!("🎧 Default-device follower stopping");
                break;
            }
            _ = tokio::time::sleep(Duration::from_secs(1)) => {}
        }

        if !state.is_recording() || state.is_rebuilding_streams() {
            continue;
        }

        let paused = state.is_paused();
        let pending = state.is_pending_device_check();
        if paused && !pending {
            continue;
        }
        if pending {
            state.set_pending_device_check(false);
        }

        let (stored_mic, stored_sys) = state.get_current_default_names();
        let current_mic = if follow_mic {
            default_input_device().ok().map(|d| d.name)
        } else {
            stored_mic.clone()
        };
        let current_sys = if follow_system {
            default_output_device().ok().map(|d| d.name)
        } else {
            stored_sys.clone()
        };

        let mic_force = follow_mic && state.is_stream_failed(RecordingDeviceType::Microphone);
        let sys_force = follow_system && state.is_stream_failed(RecordingDeviceType::System);

        // Debounced change detection. A device name must be stable for 2 seconds
        // before we actually rebuild, preventing rapid toggles.
        let mut mic_stable_changed = false;
        let mut sys_stable_changed = false;
        if !paused {
            if stored_mic != current_mic {
                if mic_last == current_mic
                    && mic_changed_at
                        .map(|t| t.elapsed() >= Duration::from_secs(2))
                        .unwrap_or(false)
                {
                    mic_stable_changed = true;
                } else if mic_last != current_mic {
                    mic_changed_at = Some(Instant::now());
                }
            } else {
                mic_changed_at = None;
            }

            if stored_sys != current_sys {
                if sys_last == current_sys
                    && sys_changed_at
                        .map(|t| t.elapsed() >= Duration::from_secs(2))
                        .unwrap_or(false)
                {
                    sys_stable_changed = true;
                } else if sys_last != current_sys {
                    sys_changed_at = Some(Instant::now());
                }
            } else {
                sys_changed_at = None;
            }
        }
        mic_last = current_mic.clone();
        sys_last = current_sys.clone();

        let defaults_changed = stored_mic != current_mic || stored_sys != current_sys;
        let needs_rebuild = mic_force
            || sys_force
            || mic_stable_changed
            || sys_stable_changed
            || (pending && defaults_changed);
        if !needs_rebuild {
            continue;
        }

        // If the microphone disappeared and we are following it, stop streams and
        // wait. Do not fail the recording session.
        if follow_mic && current_mic.is_none() && !state.is_waiting_for_device() {
            warn!("🎤 Default microphone disappeared — entering waiting state");
            state.set_rebuilding_streams(true);
            let rebuild_result = rebuild_streams_locked().await;
            state.set_rebuilding_streams(false);
            if rebuild_result.is_ok() {
                let _ = app.emit(
                    "waiting-for-audio-device",
                    serde_json::json!({
                        "microphone": serde_json::Value::Null,
                        "system_audio": current_sys,
                    }),
                );
            }
            continue;
        }

        // If we were waiting and a microphone is back, rebuild immediately.
        if state.is_waiting_for_device() {
            if current_mic.is_some() {
                info!("🎤 Default microphone returned — rebuilding streams");
                state.set_waiting_for_device(false);
                state.set_rebuilding_streams(true);
                let _ = rebuild_streams_locked().await;
                state.set_rebuilding_streams(false);
                notify_device_changed_and_restart_monitor(&app, &state).await;
            }
            continue;
        }

        // Normal case: default device changed (stable) or CPAL reported an error.
        info!("🎧 Default device change detected (mic_force={}, sys_force={}, mic_changed={}, sys_changed={}, pending={}) — rebuilding",
              mic_force, sys_force, mic_stable_changed, sys_stable_changed, pending);
        state.set_rebuilding_streams(true);
        match rebuild_streams_locked().await {
            Ok(()) => {
                notify_device_changed_and_restart_monitor(&app, &state).await;
            }
            Err(e) => {
                error!("❌ Failed to rebuild streams with current defaults: {}", e);
            }
        }
        state.set_rebuilding_streams(false);
    }

    info!("🎧 Default-device follower stopped");
}

/// Emit `default-device-changed` and restart the level monitor with the devices
/// currently in use.
async fn notify_device_changed_and_restart_monitor<R: Runtime>(
    app: &AppHandle<R>,
    state: &crate::audio::recording_state::RecordingState,
) {
    let mic_name = state.get_microphone_device().map(|d| d.name.clone());
    let sys_name = state.get_system_device().map(|d| d.name.clone());

    let _ = app.emit(
        "default-device-changed",
        serde_json::json!({
            "microphone": mic_name,
            "system_audio": sys_name,
        }),
    );

    let mut monitoring_names: Vec<String> = Vec::new();
    if let Some(ref name) = mic_name {
        monitoring_names.push(name.clone());
    }
    if let Some(ref name) = sys_name {
        monitoring_names.push(name.clone());
    }
    if !monitoring_names.is_empty() {
        let _ = crate::audio::simple_level_monitor::stop_monitoring().await;
        let _ = crate::audio::simple_level_monitor::start_monitoring(app.clone(), monitoring_names)
            .await;
    }
}

/// Lock the global recording manager and ask it to rebuild streams with the
/// current default devices. Runs in a blocking task so the std Mutex can be held
/// across the async rebuild.
async fn rebuild_streams_locked() -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        tokio::runtime::Handle::current().block_on(async {
            let mut guard = RECORDING_MANAGER.lock().unwrap();
            if let Some(manager) = guard.as_mut() {
                manager.restart_streams_with_current_defaults().await
            } else {
                Err(anyhow::anyhow!("Recording manager not available"))
            }
        })
    })
    .await
    .map_err(|e| format!("Rebuild task panicked: {}", e))?
    .map_err(|e| e.to_string())
}

// ============================================================================
// RECORDING COMMANDS
// ============================================================================

/// Start recording with default devices
pub async fn start_recording<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    start_recording_with_meeting_name(app, None).await
}

/// Start recording with default devices and optional meeting name
pub async fn start_recording_with_meeting_name<R: Runtime>(
    app: AppHandle<R>,
    meeting_name: Option<String>,
) -> Result<(), String> {
    info!(
        "Starting recording with default devices, meeting: {:?}",
        meeting_name
    );

    // Check if already recording
    let current_recording_state = IS_RECORDING.load(Ordering::SeqCst);
    info!("🔍 IS_RECORDING state check: {}", current_recording_state);
    if current_recording_state {
        return Err("Recording already in progress".to_string());
    }

    // Validate that transcription models are available before starting recording
    info!("🔍 Validating transcription model availability before starting recording...");
    if let Err(validation_error) = transcription::validate_transcription_model_ready(&app).await {
        error!("Model validation failed: {}", validation_error);

        // Emit error event for frontend - actionable: false to show toast instead of modal
        // (download progress is already shown in top-right toast)
        let _ = app.emit("transcription-error", serde_json::json!({
            "error": validation_error,
            "userMessage": "Recording cannot start: Transcription model is still downloading. Please wait for the download to complete.",
            "actionable": false
        }));

        return Err(validation_error);
    }
    info!("✅ Transcription model validation passed");

    // Async-first approach - no more blocking operations!
    info!("🚀 Starting async recording initialization");

    // Create new recording manager
    let mut manager = RecordingManager::new();

    // Load recording preferences to get auto_save AND device preferences AND save folder
    let (auto_save, preferred_mic_name, preferred_system_name, save_folder) =
        match super::recording_preferences::load_recording_preferences(&app).await {
            Ok(prefs) => {
                info!("📋 Loaded recording preferences: auto_save={}, preferred_mic={:?}, preferred_system={:?}, save_folder={:?}",
                      prefs.auto_save, prefs.preferred_mic_device, prefs.preferred_system_device, prefs.save_folder);
                let folder = if prefs.save_folder.as_os_str().is_empty() {
                    super::recording_preferences::get_default_recordings_folder()
                } else {
                    prefs.save_folder
                };
                (
                    prefs.auto_save,
                    prefs.preferred_mic_device,
                    prefs.preferred_system_device,
                    folder,
                )
            }
            Err(e) => {
                warn!(
                    "Failed to load recording preferences, using defaults: {}",
                    e
                );
                (
                    true,
                    None,
                    None,
                    super::recording_preferences::get_default_recordings_folder(),
                )
            }
        };

    // ============================================================================
    // MICROPHONE DEVICE RESOLUTION: Preference → Default → None (optional)
    // ============================================================================
    let microphone_device = match preferred_mic_name {
        Some(ref pref_name) => {
            info!("🎤 Attempting to use preferred microphone: '{}'", pref_name);
            match parse_audio_device(&pref_name) {
                Ok(device) => {
                    info!("✅ Using preferred microphone: '{}'", device.name);
                    Some(Arc::new(device))
                }
                Err(e) => {
                    warn!(
                        "⚠️ Preferred microphone '{}' not available: {}",
                        pref_name, e
                    );
                    warn!("   Falling back to system default microphone...");
                    match default_input_device() {
                        Ok(device) => {
                            info!("✅ Using default microphone: '{}'", device.name);
                            Some(Arc::new(device))
                        }
                        Err(default_err) => {
                            warn!("⚠️ No microphone available (preferred and default both failed): {}", default_err);
                            warn!("   Recording will continue with system audio only");
                            None // Microphone is optional
                        }
                    }
                }
            }
        }
        None => {
            info!("🎤 No microphone preference set, using system default");
            match default_input_device() {
                Ok(device) => {
                    info!("✅ Using default microphone: '{}'", device.name);
                    Some(Arc::new(device))
                }
                Err(e) => {
                    warn!("⚠️ No default microphone available: {}", e);
                    warn!("   Recording will continue with system audio only");
                    None // Microphone is optional
                }
            }
        }
    };

    // ============================================================================
    // SYSTEM AUDIO DEVICE RESOLUTION: Preference → Default → None (optional)
    // ============================================================================
    let system_device = match preferred_system_name {
        Some(ref pref_name) => {
            info!(
                "🔊 Attempting to use preferred system audio: '{}'",
                pref_name
            );
            match parse_audio_device(&pref_name) {
                Ok(device) => {
                    info!("✅ Using preferred system audio: '{}'", device.name);
                    Some(Arc::new(device))
                }
                Err(e) => {
                    warn!(
                        "⚠️ Preferred system audio '{}' not available: {}",
                        pref_name, e
                    );
                    warn!("   Falling back to system default...");
                    match default_output_device() {
                        Ok(device) => {
                            info!("✅ Using default system audio: '{}'", device.name);
                            Some(Arc::new(device))
                        }
                        Err(default_err) => {
                            warn!("⚠️ No system audio available (preferred and default both failed): {}", default_err);
                            warn!("   Recording will continue with microphone only");
                            None // System audio is optional
                        }
                    }
                }
            }
        }
        None => {
            info!("🔊 No system audio preference set, using system default");
            match default_output_device() {
                Ok(device) => {
                    info!("✅ Using default system audio: '{}'", device.name);
                    Some(Arc::new(device))
                }
                Err(e) => {
                    warn!("⚠️ No default system audio available: {}", e);
                    warn!("   Recording will continue with microphone only");
                    None // System audio is optional
                }
            }
        }
    };

    // At least one audio source (microphone or system audio) is required
    if microphone_device.is_none() && system_device.is_none() {
        error!("❌ No audio devices available (neither microphone nor system audio)");
        return Err(
            "No audio devices available: neither a microphone nor a system audio device was found"
                .to_string(),
        );
    }

    // Configure default-device following: only follow when the user has not
    // manually picked a preferred device.
    let follow_mic = preferred_mic_name.is_none();
    let follow_system = preferred_system_name.is_none();
    manager.set_follow_flags(follow_mic, follow_system);

    // Always ensure a meeting name is set so incremental saver initializes
    let effective_meeting_name = meeting_name.clone().unwrap_or_else(|| {
        // 与录音文件夹一致的 Rec_ 前缀（语言无关），例：Rec_2025-10-03_08-25-23
        let now = chrono::Local::now();
        format!("Rec_{}", now.format("%Y-%m-%d_%H-%M-%S"))
    });
    manager.set_meeting_name(Some(effective_meeting_name));

    // Set up error callback
    let app_for_error = app.clone();
    manager.set_error_callback(move |error| {
        let _ = app_for_error.emit("recording-error", error.user_message());
    });

    // Start recording with resolved devices (replaces start_recording_with_defaults_and_auto_save call)
    // 在此之前先克隆设备名，用于后续音频电平监控
    let mic_name_for_monitoring = microphone_device.as_ref().map(|d| d.name.clone());
    let sys_name_for_monitoring = system_device.as_ref().map(|d| d.name.clone());

    // Determine if X-ASR is selected (requires VAD bypass for continuous streaming)
    let bypass_vad =
        match crate::api::api::api_get_transcript_config(app.clone(), app.clone().state(), None)
            .await
        {
            // X-ASR 与「远程流式 ASR」都需要持续音频流 → 绕过 VAD 分段
            Ok(Some(config)) => {
                config.model.starts_with("x-asr-")
                    || ((config.model == "qwen3-asr-remote"
                        || config.model.starts_with("qwen3-asr-remote")
                        || config.provider == "remote-qwen3-asr")
                        && crate::audio::transcription::engine::is_remote_asr_streaming())
            }
            _ => false,
        };
    if bypass_vad {
        info!("🎙️ 流式模式（X-ASR 或远程流式）：VAD 将被绕过，持续喂音频");
    }

    let transcription_receiver = manager
        .start_recording(
            microphone_device,
            system_device,
            auto_save,
            follow_mic,
            follow_system,
            bypass_vad,
            save_folder,
        )
        .await
        .map_err(|e| format!("Failed to start recording: {}", e))?;

    // Keep a handle to the state for the default-device follower before moving
    // the manager into the global static.
    let state_for_follower = manager.get_state().clone();

    // Store the manager globally to keep it alive
    {
        let mut global_manager = RECORDING_MANAGER.lock().unwrap();
        *global_manager = Some(manager);
    }

    // Start watching system default devices if we are following them.
    if follow_mic || follow_system {
        spawn_default_device_follower(app.clone(), state_for_follower, follow_mic, follow_system);
    }

    // Set recording flag and reset speech detection flag
    info!("🔍 Setting IS_RECORDING to true and resetting SPEECH_DETECTED_EMITTED");
    IS_RECORDING.store(true, Ordering::SeqCst);
    reset_speech_detected_flag(); // Reset for new recording session

    // Start optimized parallel transcription task and store handle
    let task_handle = transcription::start_transcription_task(app.clone(), transcription_receiver);
    {
        let mut global_task = TRANSCRIPTION_TASK.lock().unwrap();
        *global_task = Some(task_handle);
    }

    // CRITICAL: Listen for transcript-update events and save to recording manager
    // This enables transcript history persistence for page reload sync
    // Store listener ID for cleanup during stop_recording to ensure microphone is released
    {
        use tauri::Listener;
        let listener_id = app.listen("transcript-update", move |event: tauri::Event| {
            // Parse the transcript update from the event payload
            if let Ok(update) = serde_json::from_str::<TranscriptUpdate>(event.payload()) {
                // Create structured transcript segment
                let segment = crate::audio::recording_saver::TranscriptSegment {
                    id: format!("seg_{}", update.sequence_id),
                    text: update.text.clone(),
                    audio_start_time: update.audio_start_time,
                    audio_end_time: update.audio_end_time,
                    duration: update.duration,
                    display_time: update.timestamp.clone(), // Use wall-clock timestamp for display
                    confidence: update.confidence,
                    sequence_id: update.sequence_id,
                    translation: String::new(), // 译文在最终写盘时回填
                };

                // Save to recording manager
                if let Ok(manager_guard) = RECORDING_MANAGER.lock() {
                    if let Some(manager) = manager_guard.as_ref() {
                        manager.add_transcript_segment(segment);
                    }
                }
            }
        });
        let mut global_listener = TRANSCRIPT_LISTENER_ID.lock().unwrap();
        *global_listener = Some(listener_id);
        info!("✅ Transcript-update event listener registered for history persistence");
    }

    // Emit success event
    app.emit(
        "recording-started",
        serde_json::json!({
            "message": "Recording started successfully with parallel processing",
            "devices": ["Default Microphone", "Default System Audio"],
            "workers": 3
        }),
    )
    .map_err(|e| e.to_string())?;

    // Update tray menu to reflect recording state
    crate::tray::update_tray_menu(&app);

    // 启动音频电平监控
    {
        let mut monitoring_names: Vec<String> = Vec::new();
        if let Some(ref name) = mic_name_for_monitoring {
            monitoring_names.push(name.clone());
        }
        if let Some(ref name) = sys_name_for_monitoring {
            monitoring_names.push(name.clone());
        }
        if !monitoring_names.is_empty() {
            let _ =
                crate::audio::simple_level_monitor::start_monitoring(app.clone(), monitoring_names)
                    .await;
        }
    }

    info!("✅ Recording started successfully with async-first approach");

    Ok(())
}

/// 录音中热切换流式 ASR 引擎（仅限流式↔流式：X-ASR ↔ 远程流式）。
/// 前端先持久化新选择（transcript config + 远程模型选择），再调本命令。
/// 原则：录音管线完全不动（采集/保存不受影响），只重启转写子系统：
/// 换新 sender（旧 sender 释放 → 旧任务收 None 优雅收尾），新引擎新通道新任务。
pub async fn switch_streaming_asr_model<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    if !IS_RECORDING.load(Ordering::SeqCst) {
        // 未在录音：选择已被前端持久化，下次录音生效，无需动作
        return Ok(());
    }

    // 1. 读当前（新）配置，确认目标是流式家族
    let config = crate::api::api::api_get_transcript_config(app.clone(), app.clone().state(), None)
        .await
        .map_err(|e| format!("读取转写配置失败: {}", e))?
        .ok_or_else(|| "未找到转写配置".to_string())?;
    let is_xasr = config.model.starts_with("x-asr-");
    let is_remote =
        config.model.starts_with("qwen3-asr-remote") || config.provider == "remote-qwen3-asr";
    let is_streaming = is_xasr || (is_remote && transcription::engine::is_remote_asr_streaming());
    if !is_streaming {
        return Err(
            "录音中仅支持切换流式模型（X-ASR / 远程流式）；该模型将在下次录音生效".to_string(),
        );
    }

    // 2. 先验证并建好新引擎——失败则现有链路完全不动（旧引擎继续转写）
    transcription::validate_transcription_model_ready(&app)
        .await
        .map_err(|e| format!("新模型不可用: {}", e))?;
    let engine = transcription::engine::get_or_init_transcription_engine(&app)
        .await
        .map_err(|e| format!("新引擎初始化失败: {}", e))?;
    let engine_name = engine.provider_name().to_string();
    if engine_name != "x-asr" && engine_name != "Remote ASR Streaming" {
        return Err(format!("引擎 {} 不支持录音中切换", engine_name));
    }

    // 3. 换 feed 的 sender：旧 sender 在此释放，旧转写任务收 None 后自行收尾
    let feed = {
        let guard = RECORDING_MANAGER.lock().unwrap();
        guard.as_ref().and_then(|m| m.get_transcription_feed())
    };
    let Some(feed) = feed else {
        return Err("当前录音没有可切换的转写通道".to_string());
    };
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    super::pipeline::TranscriptionFeed::swap_sender(&feed, tx);

    // 4. 重启转写任务：不重置翻译会话；序列号与旧引擎输出对齐
    let handle = transcription::start_transcription_task_with_engine(app.clone(), rx, engine);
    {
        let mut global_task = TRANSCRIPTION_TASK.lock().unwrap();
        *global_task = Some(handle);
    }

    info!("🔀 录音中切换流式 ASR 引擎完成: {}", engine_name);
    let _ = app.emit(
        "asr-engine-switched",
        serde_json::json!({ "engine": engine_name }),
    );
    Ok(())
}

/// Start recording with specific devices
pub async fn start_recording_with_devices<R: Runtime>(
    app: AppHandle<R>,
    mic_device_name: Option<String>,
    system_device_name: Option<String>,
) -> Result<(), String> {
    start_recording_with_devices_and_meeting(app, mic_device_name, system_device_name, None).await
}

/// Start recording with specific devices and optional meeting name
pub async fn start_recording_with_devices_and_meeting<R: Runtime>(
    app: AppHandle<R>,
    mic_device_name: Option<String>,
    system_device_name: Option<String>,
    meeting_name: Option<String>,
) -> Result<(), String> {
    info!(
        "Starting recording with specific devices: mic={:?}, system={:?}, meeting={:?}",
        mic_device_name, system_device_name, meeting_name
    );

    // Check if already recording
    let current_recording_state = IS_RECORDING.load(Ordering::SeqCst);
    info!("🔍 IS_RECORDING state check: {}", current_recording_state);
    if current_recording_state {
        return Err("Recording already in progress".to_string());
    }

    // Validate that transcription models are available before starting recording
    info!("🔍 Validating transcription model availability before starting recording...");
    if let Err(validation_error) = transcription::validate_transcription_model_ready(&app).await {
        error!("Model validation failed: {}", validation_error);

        // Emit error event for frontend - actionable: false to show toast instead of modal
        // (download progress is already shown in top-right toast)
        let _ = app.emit("transcription-error", serde_json::json!({
            "error": validation_error,
            "userMessage": "Recording cannot start: Transcription model is still downloading. Please wait for the download to complete.",
            "actionable": false
        }));

        return Err(validation_error);
    }
    info!("✅ Transcription model validation passed");

    // Parse devices（显式选择：无 (input)/(output) 后缀时按用途推断类型，如 macOS 的 "Mac mini扬声器"）
    let mic_device = if let Some(ref name) = mic_device_name {
        Some(Arc::new(
            crate::audio::devices::configuration::AudioDevice::from_name_with_hint(
                name,
                &crate::audio::devices::configuration::DeviceType::Input,
            )
            .map_err(|e| format!("Invalid microphone device '{}': {}", name, e))?,
        ))
    } else {
        None
    };

    let system_device = if let Some(ref name) = system_device_name {
        Some(Arc::new(
            crate::audio::devices::configuration::AudioDevice::from_name_with_hint(
                name,
                &crate::audio::devices::configuration::DeviceType::Output,
            )
            .map_err(|e| format!("Invalid system device '{}': {}", name, e))?,
        ))
    } else {
        None
    };

    // Async-first approach for custom devices - no more blocking operations!
    info!("🚀 Starting async recording initialization with custom devices");

    // Create new recording manager
    let mut manager = RecordingManager::new();

    // When a device name is explicitly provided we treat it as a fixed choice;
    // otherwise we follow the system default for that device.
    let follow_mic = mic_device_name.is_none();
    let follow_system = system_device_name.is_none();
    manager.set_follow_flags(follow_mic, follow_system);

    // Load recording preferences to check auto_save setting and save folder
    let (auto_save, save_folder) =
        match super::recording_preferences::load_recording_preferences(&app).await {
            Ok(prefs) => {
                info!(
                    "📋 Loaded recording preferences: auto_save={}, save_folder={:?}",
                    prefs.auto_save, prefs.save_folder
                );
                let folder = if prefs.save_folder.as_os_str().is_empty() {
                    super::recording_preferences::get_default_recordings_folder()
                } else {
                    prefs.save_folder
                };
                (prefs.auto_save, folder)
            }
            Err(e) => {
                warn!(
                    "Failed to load recording preferences, defaulting to auto_save=true: {}",
                    e
                );
                (
                    true, // Default to saving if preferences can't be loaded
                    super::recording_preferences::get_default_recordings_folder(),
                )
            }
        };

    // Always ensure a meeting name is set so incremental saver initializes
    let effective_meeting_name = meeting_name.clone().unwrap_or_else(|| {
        // 与录音文件夹一致的 Rec_ 前缀（语言无关）
        let now = chrono::Local::now();
        format!("Rec_{}", now.format("%Y-%m-%d_%H-%M-%S"))
    });
    manager.set_meeting_name(Some(effective_meeting_name));

    // Set up error callback
    let app_for_error = app.clone();
    manager.set_error_callback(move |error| {
        let _ = app_for_error.emit("recording-error", error.user_message());
    });

    // Start recording with specified devices and auto_save setting
    // Determine if X-ASR is selected (requires VAD bypass for continuous streaming)
    let bypass_vad =
        match crate::api::api::api_get_transcript_config(app.clone(), app.clone().state(), None)
            .await
        {
            // X-ASR 与「远程流式 ASR」都需要持续音频流 → 绕过 VAD 分段
            Ok(Some(config)) => {
                config.model.starts_with("x-asr-")
                    || ((config.model == "qwen3-asr-remote"
                        || config.model.starts_with("qwen3-asr-remote")
                        || config.provider == "remote-qwen3-asr")
                        && crate::audio::transcription::engine::is_remote_asr_streaming())
            }
            _ => false,
        };
    if bypass_vad {
        info!("🎙️ 流式模式（X-ASR 或远程流式）：VAD 将被绕过，持续喂音频");
    }

    let transcription_receiver = manager
        .start_recording(
            mic_device,
            system_device,
            auto_save,
            follow_mic,
            follow_system,
            bypass_vad,
            save_folder,
        )
        .await
        .map_err(|e| format!("Failed to start recording: {}", e))?;

    // Keep a handle to the state for the default-device follower before moving
    // the manager into the global static.
    let state_for_follower = manager.get_state().clone();

    // Store the manager globally to keep it alive
    {
        let mut global_manager = RECORDING_MANAGER.lock().unwrap();
        *global_manager = Some(manager);
    }

    // Start watching system default devices if we are following them.
    if follow_mic || follow_system {
        spawn_default_device_follower(app.clone(), state_for_follower, follow_mic, follow_system);
    }

    // Set recording flag and reset speech detection flag
    info!("🔍 Setting IS_RECORDING to true and resetting SPEECH_DETECTED_EMITTED");
    IS_RECORDING.store(true, Ordering::SeqCst);
    reset_speech_detected_flag(); // Reset for new recording session

    // Start optimized parallel transcription task and store handle
    let task_handle = transcription::start_transcription_task(app.clone(), transcription_receiver);
    {
        let mut global_task = TRANSCRIPTION_TASK.lock().unwrap();
        *global_task = Some(task_handle);
    }

    // CRITICAL: Listen for transcript-update events and save to recording manager
    // This enables transcript history persistence for page reload sync
    // Store listener ID for cleanup during stop_recording to ensure microphone is released
    {
        use tauri::Listener;
        let listener_id = app.listen("transcript-update", move |event: tauri::Event| {
            // Parse the transcript update from the event payload
            if let Ok(update) = serde_json::from_str::<TranscriptUpdate>(event.payload()) {
                // Create structured transcript segment
                let segment = crate::audio::recording_saver::TranscriptSegment {
                    id: format!("seg_{}", update.sequence_id),
                    text: update.text.clone(),
                    audio_start_time: update.audio_start_time,
                    audio_end_time: update.audio_end_time,
                    duration: update.duration,
                    display_time: update.timestamp.clone(), // Use wall-clock timestamp for display
                    confidence: update.confidence,
                    sequence_id: update.sequence_id,
                    translation: String::new(), // 译文在最终写盘时回填
                };

                // Save to recording manager
                if let Ok(manager_guard) = RECORDING_MANAGER.lock() {
                    if let Some(manager) = manager_guard.as_ref() {
                        manager.add_transcript_segment(segment);
                    }
                }
            }
        });
        let mut global_listener = TRANSCRIPT_LISTENER_ID.lock().unwrap();
        *global_listener = Some(listener_id);
        info!("✅ Transcript-update event listener registered for history persistence");
    }

    // Emit success event — 先 clone 设备名，后续监测还需要用到
    let mic_name_for_emit = mic_device_name.clone();
    let sys_name_for_emit = system_device_name.clone();
    app.emit(
        "recording-started",
        serde_json::json!({
            "message": "Recording started with custom devices and parallel processing",
            "devices": [
                mic_name_for_emit.unwrap_or_else(|| "Default Microphone".to_string()),
                sys_name_for_emit.unwrap_or_else(|| "Default System Audio".to_string())
            ],
            "workers": 3
        }),
    )
    .map_err(|e| e.to_string())?;

    // Update tray menu to reflect recording state
    crate::tray::update_tray_menu(&app);

    // 启动音频电平监控
    let monitoring_device_names: Vec<String> = mic_device_name
        .iter()
        .chain(system_device_name.iter())
        .cloned()
        .collect();
    if !monitoring_device_names.is_empty() {
        let _ = crate::audio::simple_level_monitor::start_monitoring(
            app.clone(),
            monitoring_device_names,
        )
        .await;
    }

    info!("✅ Recording started with custom devices using async-first approach");

    Ok(())
}

/// Stop recording with optimized graceful shutdown ensuring NO transcript chunks are lost
pub async fn stop_recording<R: Runtime>(
    app: AppHandle<R>,
    _args: RecordingArgs,
) -> Result<(), String> {
    info!(
        "🛑 Starting optimized recording shutdown - ensuring ALL transcript chunks are preserved"
    );

    // Check if recording is active
    if !IS_RECORDING.load(Ordering::SeqCst) {
        info!("Recording was not active");
        return Ok(());
    }

    // Emit shutdown progress to frontend
    let _ = app.emit(
        "recording-shutdown-progress",
        serde_json::json!({
            "stage": "stopping_audio",
            "message": "Stopping audio capture...",
            "progress": 20
        }),
    );

    // Stop the default-device follower first so it doesn't try to rebuild streams
    // while we are tearing everything down.
    stop_default_device_follower().await;

    // Step 1: Stop audio capture immediately (no more new chunks) with proper error handling
    let manager_for_cleanup = {
        let mut global_manager = RECORDING_MANAGER.lock().unwrap();
        global_manager.take()
    };

    let stop_result = if let Some(mut manager) = manager_for_cleanup {
        // Use FORCE FLUSH to immediately process all accumulated audio - eliminates 30s delay!
        info!("🚀 Using FORCE FLUSH to eliminate pipeline accumulation delays");
        let result = manager.stop_streams_and_force_flush().await;
        // Store manager back for later cleanup
        let manager_for_cleanup = Some(manager);
        (result, manager_for_cleanup)
    } else {
        warn!("No recording manager found to stop");
        (Ok(()), None)
    };

    let (stop_result, manager_for_cleanup) = stop_result;

    match stop_result {
        Ok(_) => {
            info!("✅ Audio streams stopped successfully - no more chunks will be created");
        }
        Err(e) => {
            error!("❌ Failed to stop audio streams: {}", e);
            return Err(format!("Failed to stop audio streams: {}", e));
        }
    }

    // Step 1.5: 转写事件监听**不能在这里摘掉**（2026-09-22 修复）。
    //
    // 旧实现先 `app.unlisten(TRANSCRIPT_LISTENER_ID)` 再等转写任务收尾，于是收尾阶段
    // 由流式管线 `finish()` 闭合的最后几个单元**没人落盘**：实测 14:09:10.347 摘监听、
    // 14:09:11.359 会话收尾（提交了 seq=32..35 四个单元并各自完成译文），
    // 而 transcripts.json / DB 里只有到 seq=31 —— 最后 ~15 秒的原文永久丢失，
    // 用户看到的现象正是「有译文但没有对应原文 / 末尾少一段」。
    // 现在把摘监听挪到「等转写任务跑完」之后（见下面 Step 2.5）。

    // Step 2: Signal transcription workers to finish processing ALL queued chunks
    let _ = app.emit(
        "recording-shutdown-progress",
        serde_json::json!({
            "stage": "processing_transcripts",
            "message": "Processing remaining transcript chunks...",
            "progress": 40
        }),
    );

    // Wait for transcription task with enhanced progress monitoring (NO TIMEOUT - we must process all chunks)
    let transcription_task = {
        let mut global_task = TRANSCRIPTION_TASK.lock().unwrap();
        global_task.take()
    };

    if let Some(task_handle) = transcription_task {
        // 2026-09-30：这里以前声称 "no timeout"，实际下面写的是 600s —— 
        // 真机实测停止录音卡 24.3s / 19.0s（见 f1 日志 18:27、18:33 两次），
        // 用户感知就是「点了停止半天没反应」。改成有上限的等待并诚实记录丢弃量。
        info!("⏳ 等待转写管线收尾（上限 20s；超时则带着已完成的部分收尾）");

        // Enhanced progress monitoring during shutdown
        let progress_app = app.clone();
        let progress_task = tokio::spawn(async move {
            let last_update = std::time::Instant::now();

            loop {
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

                // Emit periodic progress updates during shutdown
                let elapsed = last_update.elapsed().as_secs();
                let _ = progress_app.emit(
                    "recording-shutdown-progress",
                    serde_json::json!({
                        "stage": "processing_transcripts",
                        "message": format!("Processing transcripts... ({}s elapsed)", elapsed),
                        "progress": 40,
                        "detailed": true,
                        "elapsed_seconds": elapsed
                    }),
                );
            }
        });

        // 等待上限 20s（原为 600s）。取 20s 的依据：真机观测到的最慢一次收尾是 24.3s，
        // 其中绝大多数时间花在等上游返回最后几个 chunk；20s 能覆盖正常情况，
        // 又不会让用户面对「停止后干等几分钟」。超时不会崩，只是尾部可能少几段。
        let drain_started = std::time::Instant::now();
        match tokio::time::timeout(
            tokio::time::Duration::from_secs(20),
            task_handle,
        )
        .await
        {
            Ok(Ok(())) => {
                info!(
                    "✅ 转写管线收尾完成（耗时 {:.1}s，无数据丢失）",
                    drain_started.elapsed().as_secs_f64()
                );
            }
            Ok(Err(e)) => {
                warn!("⚠️ Transcription task completed with error: {:?}", e);
                // Continue anyway - the worker may have processed most chunks
            }
            Err(_) => {
                warn!(
                    "⏱️ 转写管线收尾超过 20s 仍未完成，带着已完成的部分收尾（尾部可能少几段文字）。\
                     若频繁出现，通常是上游流式识别迟迟不给 final —— 可查日志里的『首字延迟/心跳』。"
                );
            }
        }

        // Stop progress monitoring
        progress_task.abort();
    } else {
        info!("ℹ️ No transcription task found to wait for");
    }

    // Step 2.5: 转写任务已结束 → 现在才摘 transcript-update 监听（见 Step 1.5 的说明：
    // 收尾阶段 flow.finish() 还会提交最后几个单元，必须让监听活到那一刻）。
    {
        use tauri::Listener;
        if let Some(listener_id) = TRANSCRIPT_LISTENER_ID.lock().unwrap().take() {
            app.unlisten(listener_id);
            info!("✅ Transcript-update listener removed（收尾单元已全部落盘）");
        }
    }

    // Step 2.6: 排空翻译队列（2026-09-24）。尾部几个单元的定稿译文是在转写收尾
    // （Step 2 的 flow.finish()）时才入队的，翻译 worker 异步消费——不等它跑完，
    // 下面的写库 / 写 transcripts.json / emit recording-stopped 都拿不到尾部译文，
    // 历史记录里末尾几段会永久缺译文（前端收到 recording-stopped 时 store 里也没有）。
    if crate::translation::TRANSLATION_ENABLED.load(Ordering::SeqCst) {
        if // 20s → 10s：真机实测这一步单独吃掉 15.8s（18:33 那次停止录音）。
            // 尾部译文晚到一点可以接受，让用户干等十几秒不行。
            crate::translation::drain_pending_translations(std::time::Duration::from_secs(10))
            .await
        {
            info!("✅ 翻译队列已排空，最终译文随段落持久化");
        } else {
            warn!("⏱️ 翻译队列排空等待超时（20s），尾部段落的译文可能缺失");
        }

        // Step 2.7: 补译缺失的定稿译文（2026-09-27，B2；2026-09-28 改为多轮收敛）。
        // drain 只保证「队列里的」任务跑完；**失败的**定稿翻译（如网关 429、清洗后为空）
        // 不在队列里也没人管——2026-09-26 晚一次 20 分钟录音因此永久丢失 27 段译文。
        // 这里把「本应翻译但 FINAL_TRANSLATIONS 里没有」的段落重新入队，循环补译：
        // 每轮 drain 后重新清点，收敛（无缺失或无进展）或达到轮数上限（3 轮）才停。
        // 只在远程引擎下做：429/撞并发是网关特有的失败形态；本地/自定义引擎的失败
        // 语义不同（本地失败重发多半再失败），不动它们的行为。
        // 上限后仍缺的段落打 WARN 列出 seq —— 停止流程不能无限等。
        if crate::translation::current_engine() == "remote" {
            const MAX_RETRANSLATE_ROUNDS: usize = 3;
            let segment_pairs: Vec<(u64, String)> = manager_for_cleanup
                .as_ref()
                .map(|m| {
                    m.get_transcript_segments()
                        .into_iter()
                        .map(|s| (s.sequence_id, s.text))
                        .collect()
                })
                .unwrap_or_default();
            let mut prev_missing_count = usize::MAX;
            for round in 1..=MAX_RETRANSLATE_ROUNDS {
                let missing = crate::translation::missing_final_translations(&segment_pairs);
                if missing.is_empty() {
                    if round > 1 {
                        info!("✅ 补译完成，全部段落已有定稿译文");
                    }
                    break;
                }
                // 无进展收敛：上一轮补译后缺失数没减少（全是重试后仍失败/仍空的段落），
                // 再补同样的轮次也不会有变化，提前停。
                if missing.len() >= prev_missing_count {
                    let seqs: Vec<u64> = missing.iter().map(|(s, _)| *s).collect();
                    warn!(
                        "⚠️ 补译第 {} 轮后无进展，仍缺定稿译文的段落 seq={:?}（这些段落的历史记录将没有译文）",
                        round - 1,
                        seqs
                    );
                    break;
                }
                info!(
                    "🔄 停止录音：{} 个段落缺失定稿译文，重新入队补译（第 {}/{} 轮）",
                    missing.len(),
                    round,
                    MAX_RETRANSLATE_ROUNDS
                );
                for (seq, text) in &missing {
                    crate::translation::queue_translation(&app, text, *seq);
                }
                if !crate::translation::drain_pending_translations(
                    std::time::Duration::from_secs(20),
                )
                .await
                {
                    warn!("⏱️ 补译第 {} 轮排空等待超时（20s）", round);
                }
                prev_missing_count = missing.len();
                if round == MAX_RETRANSLATE_ROUNDS {
                    let still_missing =
                        crate::translation::missing_final_translations(&segment_pairs);
                    if !still_missing.is_empty() {
                        let seqs: Vec<u64> = still_missing.iter().map(|(s, _)| *s).collect();
                        warn!(
                            "⚠️ 补译已达轮数上限（{} 轮），仍缺定稿译文的段落 seq={:?}（这些段落的历史记录将没有译文）",
                            MAX_RETRANSLATE_ROUNDS,
                            seqs
                        );
                    } else {
                        info!("✅ 补译完成，全部段落已有定稿译文");
                    }
                }
            }
        }
    }

    // Step 3: Now safely unload Whisper model after ALL chunks are processed
    let _ = app.emit(
        "recording-shutdown-progress",
        serde_json::json!({
            "stage": "unloading_model",
            "message": "Unloading speech recognition model...",
            "progress": 70
        }),
    );

    info!("🧠 All transcript chunks processed. Now safely unloading transcription model...");

    // Determine which provider was used and unload the appropriate model (with timeout)
    let config = match tokio::time::timeout(
        tokio::time::Duration::from_secs(30), // 30 seconds max for DB operation
        crate::api::api::api_get_transcript_config(app.clone(), app.clone().state(), None),
    )
    .await
    {
        Ok(Ok(Some(config))) => Some(config.provider),
        Ok(Ok(None)) => None,
        Ok(Err(e)) => {
            warn!("⚠️ Failed to get transcript config: {:?}", e);
            None
        }
        Err(_) => {
            warn!("⏱️ Transcript config timeout (30s), continuing shutdown");
            None
        }
    };

    // Sherpa-ONNX engine stays loaded in memory for performance.
    // No explicit unload needed - it's lightweight and reused across recordings.
    info!("✅ Sherpa-ONNX engine stays loaded for next recording");

    // Step 3.5: Analytics module removed — skip tracking

    // Step 4: Finalize recording state and cleanup resources safely
    let _ = app.emit(
        "recording-shutdown-progress",
        serde_json::json!({
            "stage": "finalizing",
            "message": "Finalizing recording and cleaning up resources...",
            "progress": 90
        }),
    );

    // Perform final cleanup with the manager if available
    let (meeting_folder, meeting_name, raw_segments, recording_seconds) =
        if let Some(mut manager) = manager_for_cleanup {
            info!("🧹 Performing final cleanup and saving recording data");

            // Extract meeting info BEFORE async operations
            let meeting_folder = manager.get_meeting_folder();
            let meeting_name = manager.get_meeting_name();
            // 段落与时长也要在保存**之前**取：保存流程可能会重置内部状态
            let segments_snapshot = manager.get_transcript_segments();
            let duration_snapshot = manager.get_active_recording_duration().unwrap_or(0.0);

            match tokio::time::timeout(
                tokio::time::Duration::from_secs(300), // 5 minutes max for file I/O
                manager.save_recording_only(&app),
            )
            .await
            {
                Ok(Ok(_)) => {
                    info!("✅ Recording data saved successfully during cleanup");
                }
                Ok(Err(e)) => {
                    warn!(
                        "⚠️ Error during recording cleanup (transcripts preserved): {}",
                        e
                    );
                    // Don't fail shutdown - transcripts are already preserved
                }
                Err(_) => {
                    warn!("⏱️ File I/O timeout (5 minutes) reached during save, continuing shutdown");
                    // Don't fail shutdown - transcripts are already preserved
                }
            }

            // 原始段落（未经前端「按段落合并」）+ 时长：稍后写一份到历史库兜底
            (meeting_folder, meeting_name, segments_snapshot, duration_snapshot)
        } else {
            info!("ℹ️ No recording manager available for cleanup");
            (None, None, Vec::new(), 0.0)
        };

    // Set recording flag to false
    info!("🔍 Setting IS_RECORDING to false");
    IS_RECORDING.store(false, Ordering::SeqCst);

    // 停止音频电平监控
    let _ = crate::audio::simple_level_monitor::stop_monitoring().await;

    // Step 4.5: Prepare metadata for frontend (NO database save)
    // NOTE: We do NOT save to database here. The frontend will save after all transcripts are displayed.
    // This ensures the user sees all transcripts streaming in before the database save happens.
    let (folder_path_str, meeting_name_str) = match (&meeting_folder, &meeting_name) {
        (Some(path), Some(name)) => (Some(path.to_string_lossy().to_string()), Some(name.clone())),
        _ => (None, None),
    };

    info!("📤 Preparing recording metadata for frontend save");
    info!("   folder_path: {:?}", folder_path_str);
    info!("   meeting_name: {:?}", meeting_name_str);

    // 历史记录兜底写库（2026-09-24）：
    // 原来这里**完全不写库**，只靠前端收到 recording-stopped 之后保存 →
    // 「不在录音页/主窗口不在跑时从托盘停止录音」会没有任何历史记录
    // （audio.mp4 与 transcripts.json 都在磁盘上，库里却没有行，用户在历史里找不到）。
    // 现在 Rust 先写一份**原始段落**（幂等，见 replace_segments），前端随后用
    // 「按段落合并」的版本整体替换同一行 —— 两个写入方都不再是唯一依赖。
    if let (Some(folder), Some(name)) = (folder_path_str.as_deref(), meeting_name_str.as_deref()) {
        let last_segment_secs = raw_segments
            .iter()
            .map(|s| s.audio_end_time)
            .fold(0.0_f64, f64::max);
        let secs = if recording_seconds > 0.0 {
            recording_seconds
        } else {
            last_segment_secs
        };
        let duration_ms = if secs > 0.0 {
            Some((secs * 1000.0).round() as i64)
        } else {
            None
        };
        persist_recording_to_history_db(&app, folder, name, duration_ms, &raw_segments).await;
    }

    // Step 5: Complete shutdown
    let _ = app.emit(
        "recording-shutdown-progress",
        serde_json::json!({
            "stage": "complete",
            "message": "Recording stopped successfully",
            "progress": 100
        }),
    );

    // Emit final stop event with folder_path and meeting_name for frontend to save
    app.emit(
        "recording-stopped",
        serde_json::json!({
            "message": "Recording stopped - frontend will save after all transcripts received",
            "folder_path": folder_path_str,
            "meeting_name": meeting_name_str
        }),
    )
    .map_err(|e| e.to_string())?;

    // Update tray menu to reflect stopped state
    crate::tray::update_tray_menu(&app);

    info!("🎉 Recording stopped successfully with ZERO transcript chunks lost");
    Ok(())
}

/// 停止录音时把这条录音写进历史库（页面无关的兜底，2026-09-24）。
///
/// 为什么需要：历史库的行原来只由**前端**在收到 `recording-stopped` 之后创建
/// （`api_save_transcript`）。只要那一刻前端不在跑（用户在别的页面 / webview 被节流 /
/// 从托盘停止），这条录音就**完全不会出现在历史里** —— 文件都在，只是库里没有行。
///
/// 语义：
/// * 按 `folder_path` 复用已有行（前端随后写合并版本时也走同一行，不会出现两条）；
/// * 段落用 `replace_segments` **整体替换**（幂等，重复调用不会撞主键）；
/// * 任何失败只记日志：绝不因为写库失败影响停止录音。
async fn persist_recording_to_history_db<R: Runtime>(
    app: &AppHandle<R>,
    folder_path: &str,
    title: &str,
    duration_ms: Option<i64>,
    segments: &[crate::audio::recording_saver::TranscriptSegment],
) {
    use crate::database::models::{DateTimeUtc, TranscriptSegment as DbSegment};
    use crate::database::repositories::recording::RecordingsRepository;
    use crate::database::repositories::transcript_segment::TranscriptSegmentsRepository;

    let state = app.state::<crate::state::AppState>();
    let pool = state.db_manager.pool();

    let recording_id = match RecordingsRepository::get_by_folder_path(pool, folder_path).await {
        Ok(Some(rec)) => rec.id,
        Ok(None) => {
            match RecordingsRepository::create_recording(
                pool,
                title,
                duration_ms,
                None,
                Some("realtime"),
                None,
                Some(folder_path),
            )
            .await
            {
                Ok(id) => id,
                Err(e) => {
                    log::error!("停止录音写历史库失败（create_recording）: {}", e);
                    return;
                }
            }
        }
        Err(e) => {
            log::error!("停止录音写历史库失败（按 folder 查历史行）: {}", e);
            return;
        }
    };

    let db_segments: Vec<DbSegment> = segments
        .iter()
        .map(|s| DbSegment {
            // 与前端 api_save_transcript 的 id 口径一致（seg_<sequence_id>），
            // 这样前端随后整体替换时不会留下两份
            id: if s.id.is_empty() {
                format!("seg_{}", s.sequence_id)
            } else {
                s.id.clone()
            },
            recording_id: recording_id.clone(),
            text: s.text.clone(),
            start_ms: (s.audio_start_time * 1000.0).round() as i64,
            end_ms: if s.audio_end_time > 0.0 {
                Some((s.audio_end_time * 1000.0).round() as i64)
            } else {
                None
            },
            speaker: None,
            source: Some("realtime".to_string()),
            // 此刻翻译队列已排空（见 stop_recording Step 2.6），取最终译文
            translation: crate::translation::final_translation(s.sequence_id).unwrap_or_default(),
            created_at: DateTimeUtc(chrono::Utc::now()),
        })
        .collect();

    match TranscriptSegmentsRepository::replace_segments(pool, &recording_id, &db_segments).await {
        Ok(()) => log::info!(
            "💾 停止录音：历史记录已写库 recording_id={} segments={}（前端稍后会用合并版本覆盖）",
            recording_id,
            db_segments.len()
        ),
        Err(e) => log::error!("停止录音写历史库失败（replace_segments）: {}", e),
    }
}

/// Check if recording is active
pub async fn is_recording() -> bool {
    IS_RECORDING.load(Ordering::SeqCst)
}

/// Get recording statistics
pub async fn get_transcription_status() -> TranscriptionStatus {
    TranscriptionStatus {
        chunks_in_queue: 0,
        is_processing: IS_RECORDING.load(Ordering::SeqCst),
        last_activity_ms: 0,
    }
}

/// Pause the current recording
#[tauri::command]
pub async fn pause_recording<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    info!("Pausing recording");

    // Check if currently recording
    if !IS_RECORDING.load(Ordering::SeqCst) {
        return Err("No recording is currently active".to_string());
    }

    // Access the recording manager and pause it
    let manager_guard = RECORDING_MANAGER.lock().unwrap();
    if let Some(manager) = manager_guard.as_ref() {
        manager.pause_recording().map_err(|e| e.to_string())?;

        // Emit pause event to frontend
        app.emit(
            "recording-paused",
            serde_json::json!({
                "message": "Recording paused"
            }),
        )
        .map_err(|e| e.to_string())?;

        // Update tray menu to reflect paused state
        crate::tray::update_tray_menu(&app);

        info!("Recording paused successfully");
        Ok(())
    } else {
        Err("No recording manager found".to_string())
    }
}

/// Resume the current recording
#[tauri::command]
pub async fn resume_recording<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    info!("Resuming recording");

    // Check if currently recording
    if !IS_RECORDING.load(Ordering::SeqCst) {
        return Err("No recording is currently active".to_string());
    }

    // Access the recording manager and resume it
    let manager_guard = RECORDING_MANAGER.lock().unwrap();
    if let Some(manager) = manager_guard.as_ref() {
        manager.resume_recording().map_err(|e| e.to_string())?;

        // If the default device changed while paused, rebuild now.
        manager.check_default_devices_now();

        // Emit resume event to frontend
        app.emit(
            "recording-resumed",
            serde_json::json!({
                "message": "Recording resumed"
            }),
        )
        .map_err(|e| e.to_string())?;

        // Update tray menu to reflect resumed state
        crate::tray::update_tray_menu(&app);

        info!("Recording resumed successfully");
        Ok(())
    } else {
        Err("No recording manager found".to_string())
    }
}

/// Check if recording is currently paused
#[tauri::command]
pub async fn is_recording_paused() -> bool {
    let manager_guard = RECORDING_MANAGER.lock().unwrap();
    if let Some(manager) = manager_guard.as_ref() {
        manager.is_paused()
    } else {
        false
    }
}

/// Get detailed recording state
#[tauri::command]
pub async fn get_recording_state() -> serde_json::Value {
    let is_recording = IS_RECORDING.load(Ordering::SeqCst);
    let manager_guard = RECORDING_MANAGER.lock().unwrap();

    if let Some(manager) = manager_guard.as_ref() {
        serde_json::json!({
            "is_recording": is_recording,
            "is_paused": manager.is_paused(),
            "is_active": manager.is_active(),
            "is_waiting_for_device": manager.is_waiting_for_device(),
            "recording_duration": manager.get_recording_duration(),
            "active_duration": manager.get_active_recording_duration(),
            "total_pause_duration": manager.get_total_pause_duration(),
            "current_pause_duration": manager.get_current_pause_duration()
        })
    } else {
        serde_json::json!({
            "is_recording": is_recording,
            "is_paused": false,
            "is_active": false,
            "recording_duration": null,
            "active_duration": null,
            "total_pause_duration": 0.0,
            "current_pause_duration": null
        })
    }
}

/// Get the meeting folder path for the current recording
/// Returns the path if a meeting name was set and folder structure initialized
#[tauri::command]
pub async fn get_meeting_folder_path() -> Result<Option<String>, String> {
    let manager_guard = RECORDING_MANAGER.lock().unwrap();
    if let Some(manager) = manager_guard.as_ref() {
        Ok(manager
            .get_meeting_folder()
            .map(|p| p.to_string_lossy().to_string()))
    } else {
        Ok(None)
    }
}

/// Get accumulated transcript segments from current recording session
/// Used for syncing frontend state after page reload during active recording
#[tauri::command]
pub async fn get_transcript_history(
) -> Result<Vec<crate::audio::recording_saver::TranscriptSegment>, String> {
    let manager_guard = RECORDING_MANAGER.lock().unwrap();

    if let Some(manager) = manager_guard.as_ref() {
        Ok(manager.get_transcript_segments())
    } else {
        Ok(Vec::new()) // No recording active, return empty
    }
}

/// Get meeting name from current recording session
/// Used for syncing frontend state after page reload during active recording
#[tauri::command]
pub async fn get_recording_meeting_name() -> Result<Option<String>, String> {
    let manager_guard = RECORDING_MANAGER.lock().unwrap();

    if let Some(manager) = manager_guard.as_ref() {
        Ok(manager.get_meeting_name())
    } else {
        Ok(None)
    }
}

/// 当前录音已存储段落的 (sequence_id, text) 列表（供开启翻译时补译）。
/// 无录音进行时返回空列表。
pub fn committed_segment_texts() -> Vec<(u64, String)> {
    let manager_guard = RECORDING_MANAGER.lock().unwrap();
    if let Some(manager) = manager_guard.as_ref() {
        manager
            .get_transcript_segments()
            .into_iter()
            .map(|s| (s.sequence_id, s.text))
            .collect()
    } else {
        Vec::new()
    }
}

// ============================================================================
// MICROPHONE MUTE COMMANDS
// ============================================================================

/// Set microphone mute state
#[tauri::command]
pub async fn set_mic_mute<R: Runtime>(app: AppHandle<R>, enabled: bool) -> Result<bool, String> {
    let manager_guard = RECORDING_MANAGER.lock().unwrap();
    if let Some(manager) = manager_guard.as_ref() {
        if enabled {
            manager.mute_microphone();
        } else {
            manager.unmute_microphone();
        }
        let new_state = manager.is_mic_muted();
        let _ = app.emit(
            "mic-mute-changed",
            serde_json::json!({ "muted": new_state }),
        );
        Ok(new_state)
    } else {
        Err("No active recording".to_string())
    }
}

/// Get current microphone mute state
#[tauri::command]
pub async fn get_mic_mute() -> Result<bool, String> {
    let manager_guard = RECORDING_MANAGER.lock().unwrap();
    if let Some(manager) = manager_guard.as_ref() {
        Ok(manager.is_mic_muted())
    } else {
        Err("No active recording".to_string())
    }
}

/// Toggle microphone mute state
#[tauri::command]
pub async fn toggle_mic_mute<R: Runtime>(app: AppHandle<R>) -> Result<bool, String> {
    let manager_guard = RECORDING_MANAGER.lock().unwrap();
    if let Some(manager) = manager_guard.as_ref() {
        let new_state = manager.toggle_mic_mute();
        let _ = app.emit(
            "mic-mute-changed",
            serde_json::json!({ "muted": new_state }),
        );
        Ok(new_state)
    } else {
        Err("No active recording".to_string())
    }
}

// ============================================================================
// DEVICE MONITORING COMMANDS (AirPods/Bluetooth disconnect/reconnect support)
// ============================================================================

/// Response structure for device events
#[derive(Debug, Serialize, Clone)]
#[serde(tag = "type")]
pub enum DeviceEventResponse {
    DeviceDisconnected {
        device_name: String,
        device_type: String,
    },
    DeviceReconnected {
        device_name: String,
        device_type: String,
    },
    DeviceListChanged,
}

impl From<DeviceEvent> for DeviceEventResponse {
    fn from(event: DeviceEvent) -> Self {
        match event {
            DeviceEvent::DeviceDisconnected {
                device_name,
                device_type,
            } => DeviceEventResponse::DeviceDisconnected {
                device_name,
                device_type: format!("{:?}", device_type),
            },
            DeviceEvent::DeviceReconnected {
                device_name,
                device_type,
            } => DeviceEventResponse::DeviceReconnected {
                device_name,
                device_type: format!("{:?}", device_type),
            },
            DeviceEvent::DeviceListChanged => DeviceEventResponse::DeviceListChanged,
        }
    }
}

/// Reconnection status information
#[derive(Debug, Serialize, Clone)]
pub struct ReconnectionStatus {
    pub is_reconnecting: bool,
    pub disconnected_device: Option<DisconnectedDeviceInfo>,
}

/// Information about a disconnected device
#[derive(Debug, Serialize, Clone)]
pub struct DisconnectedDeviceInfo {
    pub name: String,
    pub device_type: String,
}

/// Poll for audio device events (disconnect/reconnect)
/// Should be called periodically (every 1-2 seconds) by frontend during recording
#[tauri::command]
pub async fn poll_audio_device_events() -> Result<Option<DeviceEventResponse>, String> {
    let mut manager_guard = RECORDING_MANAGER.lock().unwrap();

    if let Some(manager) = manager_guard.as_mut() {
        if let Some(event) = manager.poll_device_events() {
            info!("📱 Device event polled: {:?}", event);
            Ok(Some(event.into()))
        } else {
            Ok(None)
        }
    } else {
        // Not recording, no events
        Ok(None)
    }
}

/// Get current reconnection status
/// Returns whether the system is attempting to reconnect and which device
#[tauri::command]
pub async fn get_reconnection_status() -> Result<ReconnectionStatus, String> {
    let manager_guard = RECORDING_MANAGER.lock().unwrap();

    if let Some(manager) = manager_guard.as_ref() {
        let state = manager.get_state();
        let disconnected_device = state
            .get_disconnected_device()
            .map(|(device, device_type)| DisconnectedDeviceInfo {
                name: device.name.clone(),
                device_type: format!("{:?}", device_type),
            });

        Ok(ReconnectionStatus {
            is_reconnecting: manager.is_reconnecting(),
            disconnected_device,
        })
    } else {
        // Not recording, no reconnection in progress
        Ok(ReconnectionStatus {
            is_reconnecting: false,
            disconnected_device: None,
        })
    }
}

/// Get information about the active audio output device
/// Used to warn users about Bluetooth playback issues
#[tauri::command]
pub async fn get_active_audio_output() -> Result<super::playback_monitor::AudioOutputInfo, String> {
    super::playback_monitor::get_active_audio_output()
        .await
        .map_err(|e| format!("Failed to get audio output info: {}", e))
}

/// Manually trigger device reconnection attempt
/// Useful for UI "Retry" button
#[tauri::command]
pub async fn attempt_device_reconnect(
    device_name: String,
    device_type: String,
) -> Result<bool, String> {
    // Parse device type first
    let monitor_type = match device_type.as_str() {
        "Microphone" => DeviceMonitorType::Microphone,
        "SystemAudio" => DeviceMonitorType::SystemAudio,
        _ => return Err(format!("Invalid device type: {}", device_type)),
    };

    // Check if recording is active
    {
        let manager_guard = RECORDING_MANAGER.lock().unwrap();
        if manager_guard.is_none() {
            return Err("Recording not active".to_string());
        }
    } // Release lock

    // Spawn blocking task to handle the async reconnection
    let result = tokio::task::spawn_blocking(move || {
        tokio::runtime::Handle::current().block_on(async {
            let mut manager_guard = RECORDING_MANAGER.lock().unwrap();
            if let Some(manager) = manager_guard.as_mut() {
                manager
                    .attempt_device_reconnect(&device_name, monitor_type)
                    .await
            } else {
                Err(anyhow::anyhow!("Recording not active"))
            }
        })
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?;

    match result {
        Ok(success) => {
            if success {
                info!("✅ Manual reconnection successful");
            } else {
                warn!("❌ Manual reconnection failed - device not available");
            }
            Ok(success)
        }
        Err(e) => {
            error!("Manual reconnection error: {}", e);
            Err(e.to_string())
        }
    }
}
