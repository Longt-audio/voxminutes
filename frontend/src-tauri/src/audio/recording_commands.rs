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
    parse_audio_device,
    default_input_device,   // Get default microphone
    default_output_device,  // Get default system audio
    RecordingManager,
    RecordingDeviceType,
    DeviceEvent,
    DeviceMonitorType
};

// Import transcription modules
use super::transcription::{
    self,
    reset_speech_detected_flag,
};

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
        state, stop, follow_mic, follow_system, app,
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

    info!("🎧 Default-device follower started (follow_mic={}, follow_system={})", follow_mic, follow_system);

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
                if mic_last == current_mic && mic_changed_at.map(|t| t.elapsed() >= Duration::from_secs(2)).unwrap_or(false) {
                    mic_stable_changed = true;
                } else if mic_last != current_mic {
                    mic_changed_at = Some(Instant::now());
                }
            } else {
                mic_changed_at = None;
            }

            if stored_sys != current_sys {
                if sys_last == current_sys && sys_changed_at.map(|t| t.elapsed() >= Duration::from_secs(2)).unwrap_or(false) {
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
        let needs_rebuild = mic_force || sys_force || mic_stable_changed || sys_stable_changed || (pending && defaults_changed);
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
                let _ = app.emit("waiting-for-audio-device", serde_json::json!({
                    "microphone": serde_json::Value::Null,
                    "system_audio": current_sys,
                }));
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

    let _ = app.emit("default-device-changed", serde_json::json!({
        "microphone": mic_name,
        "system_audio": sys_name,
    }));

    let mut monitoring_names: Vec<String> = Vec::new();
    if let Some(ref name) = mic_name {
        monitoring_names.push(name.clone());
    }
    if let Some(ref name) = sys_name {
        monitoring_names.push(name.clone());
    }
    if !monitoring_names.is_empty() {
        let _ = crate::audio::simple_level_monitor::stop_monitoring().await;
        let _ = crate::audio::simple_level_monitor::start_monitoring(app.clone(), monitoring_names).await;
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

    // Load recording preferences to get auto_save AND device preferences
    let (auto_save, preferred_mic_name, preferred_system_name) =
        match super::recording_preferences::load_recording_preferences(&app).await {
            Ok(prefs) => {
                info!("📋 Loaded recording preferences: auto_save={}, preferred_mic={:?}, preferred_system={:?}",
                      prefs.auto_save, prefs.preferred_mic_device, prefs.preferred_system_device);
                (prefs.auto_save, prefs.preferred_mic_device, prefs.preferred_system_device)
            }
            Err(e) => {
                warn!("Failed to load recording preferences, using defaults: {}", e);
                (true, None, None)
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
                    warn!("⚠️ Preferred microphone '{}' not available: {}", pref_name, e);
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
            info!("🔊 Attempting to use preferred system audio: '{}'", pref_name);
            match parse_audio_device(&pref_name) {
                Ok(device) => {
                    info!("✅ Using preferred system audio: '{}'", device.name);
                    Some(Arc::new(device))
                }
                Err(e) => {
                    warn!("⚠️ Preferred system audio '{}' not available: {}", pref_name, e);
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
        // Example: Meeting 2025-10-03_08-25-23
        let now = chrono::Local::now();
        format!(
            "Meeting {}",
            now.format("%Y-%m-%d_%H-%M-%S")
        )
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
    let bypass_vad = match crate::api::api::api_get_transcript_config(
        app.clone(), app.clone().state(), None
    ).await {
        Ok(Some(config)) => config.model.starts_with("x-asr-"),
        _ => false,
    };
    if bypass_vad {
        info!("🎙️ X-ASR mode: VAD will be bypassed for continuous streaming");
    }

    let transcription_receiver = manager
        .start_recording(microphone_device, system_device, auto_save, follow_mic, follow_system, bypass_vad)
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
        spawn_default_device_follower(
            app.clone(),
            state_for_follower,
            follow_mic,
            follow_system,
        );
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
    app.emit("recording-started", serde_json::json!({
        "message": "Recording started successfully with parallel processing",
        "devices": ["Default Microphone", "Default System Audio"],
        "workers": 3
    })).map_err(|e| e.to_string())?;

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
            let _ = crate::audio::simple_level_monitor::start_monitoring(
                app.clone(),
                monitoring_names,
            )
            .await;
        }
    }

    info!("✅ Recording started successfully with async-first approach");

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

    // Parse devices
    let mic_device = if let Some(ref name) = mic_device_name {
        Some(Arc::new(parse_audio_device(name).map_err(|e| {
            format!("Invalid microphone device '{}': {}", name, e)
        })?))
    } else {
        None
    };

    let system_device = if let Some(ref name) = system_device_name {
        Some(Arc::new(parse_audio_device(name).map_err(|e| {
            format!("Invalid system device '{}': {}", name, e)
        })?))
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

    // Load recording preferences to check auto_save setting
    let auto_save = match super::recording_preferences::load_recording_preferences(&app).await {
        Ok(prefs) => {
            info!("📋 Loaded recording preferences: auto_save={}", prefs.auto_save);
            prefs.auto_save
        }
        Err(e) => {
            warn!("Failed to load recording preferences, defaulting to auto_save=true: {}", e);
            true // Default to saving if preferences can't be loaded
        }
    };

    // Always ensure a meeting name is set so incremental saver initializes
    let effective_meeting_name = meeting_name.clone().unwrap_or_else(|| {
        let now = chrono::Local::now();
        format!(
            "Meeting {}",
            now.format("%Y-%m-%d_%H-%M-%S")
        )
    });
    manager.set_meeting_name(Some(effective_meeting_name));

    // Set up error callback
    let app_for_error = app.clone();
    manager.set_error_callback(move |error| {
        let _ = app_for_error.emit("recording-error", error.user_message());
    });

    // Start recording with specified devices and auto_save setting
    // Determine if X-ASR is selected (requires VAD bypass for continuous streaming)
    let bypass_vad = match crate::api::api::api_get_transcript_config(
        app.clone(), app.clone().state(), None
    ).await {
        Ok(Some(config)) => config.model.starts_with("x-asr-"),
        _ => false,
    };
    if bypass_vad {
        info!("🎙️ X-ASR mode: VAD will be bypassed for continuous streaming");
    }

    let transcription_receiver = manager
        .start_recording(mic_device, system_device, auto_save, follow_mic, follow_system, bypass_vad)
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
        spawn_default_device_follower(
            app.clone(),
            state_for_follower,
            follow_mic,
            follow_system,
        );
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
    app.emit("recording-started", serde_json::json!({
        "message": "Recording started with custom devices and parallel processing",
        "devices": [
            mic_name_for_emit.unwrap_or_else(|| "Default Microphone".to_string()),
            sys_name_for_emit.unwrap_or_else(|| "Default System Audio".to_string())
        ],
        "workers": 3
    })).map_err(|e| e.to_string())?;

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

    // Step 1.5: Clean up transcript listener to release microphone
    // Unlisten transcript-update event to prevent lingering references
    {
        use tauri::Listener;
        if let Some(listener_id) = TRANSCRIPT_LISTENER_ID.lock().unwrap().take() {
            app.unlisten(listener_id);
            info!("✅ Transcript-update listener removed");
        }
    }

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
        info!("⏳ Waiting for ALL transcription chunks to be processed (no timeout - preserving every chunk)");

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

        // Wait up to 10 minutes for transcription completion to prevent indefinite hangs
        match tokio::time::timeout(
            tokio::time::Duration::from_secs(600), // 10 minutes max
            task_handle
        ).await {
            Ok(Ok(())) => {
                info!("✅ ALL transcription chunks processed successfully - no data lost");
            }
            Ok(Err(e)) => {
                warn!("⚠️ Transcription task completed with error: {:?}", e);
                // Continue anyway - the worker may have processed most chunks
            }
            Err(_) => {
                warn!("⏱️ Transcription timeout (10 minutes) reached, continuing shutdown to prevent indefinite hang");
                // Continue shutdown even on timeout - better to lose some chunks than hang forever
            }
        }

        // Stop progress monitoring
        progress_task.abort();
    } else {
        info!("ℹ️ No transcription task found to wait for");
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
        crate::api::api::api_get_transcript_config(
            app.clone(),
            app.clone().state(),
            None,
        )
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
    let (meeting_folder, meeting_name) = if let Some(mut manager) = manager_for_cleanup {
        info!("🧹 Performing final cleanup and saving recording data");

        // Extract meeting info BEFORE async operations
        let meeting_folder = manager.get_meeting_folder();
        let meeting_name = manager.get_meeting_name();

        match tokio::time::timeout(
            tokio::time::Duration::from_secs(300), // 5 minutes max for file I/O
            manager.save_recording_only(&app)
        ).await {
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

        (meeting_folder, meeting_name)
    } else {
        info!("ℹ️ No recording manager available for cleanup");
        (None, None)
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
        (Some(path), Some(name)) => (
            Some(path.to_string_lossy().to_string()),
            Some(name.clone()),
        ),
        _ => (None, None),
    };

    info!("📤 Preparing recording metadata for frontend save");
    info!("   folder_path: {:?}", folder_path_str);
    info!("   meeting_name: {:?}", meeting_name_str);

    // Database save removed - frontend will handle this after receiving all transcripts
    info!("ℹ️ Skipping database save in Rust - frontend will save after all transcripts received");

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
        Ok(manager.get_meeting_folder().map(|p| p.to_string_lossy().to_string()))
    } else {
        Ok(None)
    }
}

/// Get accumulated transcript segments from current recording session
/// Used for syncing frontend state after page reload during active recording
#[tauri::command]
pub async fn get_transcript_history() -> Result<Vec<crate::audio::recording_saver::TranscriptSegment>, String> {
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
        let _ = app.emit("mic-mute-changed", serde_json::json!({ "muted": new_state }));
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
        let _ = app.emit("mic-mute-changed", serde_json::json!({ "muted": new_state }));
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
            DeviceEvent::DeviceDisconnected { device_name, device_type } => {
                DeviceEventResponse::DeviceDisconnected {
                    device_name,
                    device_type: format!("{:?}", device_type),
                }
            }
            DeviceEvent::DeviceReconnected { device_name, device_type } => {
                DeviceEventResponse::DeviceReconnected {
                    device_name,
                    device_type: format!("{:?}", device_type),
                }
            }
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
        let disconnected_device = state.get_disconnected_device().map(|(device, device_type)| {
            DisconnectedDeviceInfo {
                name: device.name.clone(),
                device_type: format!("{:?}", device_type),
            }
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
                manager.attempt_device_reconnect(&device_name, monitor_type).await
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
