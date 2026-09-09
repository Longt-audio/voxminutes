use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex as StdMutex;
use tauri::Manager;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

// Performance optimization: Conditional logging macros for hot paths
#[cfg(debug_assertions)]
macro_rules! perf_debug {
    ($($arg:tt)*) => {
        log::debug!($($arg)*)
    };
}

#[cfg(not(debug_assertions))]
macro_rules! perf_debug {
    ($($arg:tt)*) => {};
}

#[cfg(debug_assertions)]
macro_rules! perf_trace {
    ($($arg:tt)*) => {
        log::trace!($($arg)*)
    };
}

#[cfg(not(debug_assertions))]
macro_rules! perf_trace {
    ($($arg:tt)*) => {};
}

// Make these macros available to other modules
pub(crate) use perf_debug;
pub(crate) use perf_trace;

// ── Feature flags ──────────────────────────────────────────────────────────────
// Set to true to re-enable hidden models in UI and engine.
pub const FEATURE_SENSEVOICE_ENABLED: bool = true;
pub const FEATURE_XASR_960MS_ENABLED: bool = false;

pub mod api;
pub mod audio;
mod sherpa_onnx_engine;
pub mod config;
pub mod database;
mod llama_sidecar;
pub mod model_download;
pub mod notifications;
pub mod state;
pub mod summary;
pub mod translation;
pub mod tray;
pub mod subtitle_overlay;
pub mod remote_messages;
mod tts;

pub mod bundle_paths;
pub mod utils;
pub mod win_short_path;
#[cfg(target_os = "windows")]
pub mod win_job_object;

use audio::{list_audio_devices, AudioDevice, trigger_audio_permission};
use log::{error as log_error, info as log_info};
use notifications::commands::NotificationManagerState;
use std::sync::Arc;
use tauri::{AppHandle, Runtime};
use tokio::sync::RwLock;

static RECORDING_FLAG: AtomicBool = AtomicBool::new(false);

/// Receive log messages from the frontend webview and forward them to the
/// unified Rust logger. This makes browser-side errors/info available in the
/// single application log file for post-mortem debugging.
#[tauri::command]
fn frontend_log(level: String, message: String, file: Option<String>, line: Option<u32>) {
    let location = match (file, line) {
        (Some(f), Some(l)) => format!("[{}:{}] ", f, l),
        (Some(f), None) => format!("[{}] ", f),
        _ => String::new(),
    };
    let full = format!("{}{}", location, message);
    match level.to_lowercase().as_str() {
        "trace" => log::trace!("{}", full),
        "debug" => log::debug!("{}", full),
        "warn" => log::warn!("{}", full),
        "error" => log::error!("{}", full),
        _ => log::info!("{}", full),
    }
}

#[tauri::command]
fn get_audio_processing_flags() -> serde_json::Value {
    serde_json::json!({
        "agc": audio::is_agc_enabled(),
        "rnnoise": audio::is_rnnoise_enabled(),
        "ebu": audio::is_ebu_enabled(),
    })
}

#[tauri::command]
fn set_audio_processing_flags(agc: Option<bool>, rnnoise: Option<bool>, ebu: Option<bool>) {
    if let Some(v) = agc {
        audio::AGC_ENABLED.store(v, Ordering::Relaxed);
        log_info!("Audio processing flag: AGC = {}", v);
    }
    if let Some(v) = rnnoise {
        audio::RNNOISE_APPLY_ENABLED.store(v, Ordering::Relaxed);
        log_info!("Audio processing flag: RNNoise = {}", v);
    }
    if let Some(v) = ebu {
        audio::EBU_R128_ENABLED.store(v, Ordering::Relaxed);
        log_info!("Audio processing flag: EBU R128 = {}", v);
    }
}

// MVP: the Python backend is no longer started (all data APIs are served
// natively by `api::api` from SQLite).

// Global ASR language preference (default "auto" for automatic detection)
static LANGUAGE_PREFERENCE: std::sync::LazyLock<StdMutex<String>> =
    std::sync::LazyLock::new(|| StdMutex::new("auto".to_string()));

#[derive(Debug, Deserialize)]
struct RecordingArgs {
    save_path: String,
}

#[derive(Debug, Serialize, Clone)]
struct TranscriptionStatus {
    chunks_in_queue: usize,
    is_processing: bool,
    last_activity_ms: u64,
}

#[tauri::command]
async fn start_recording<R: Runtime>(
    app: AppHandle<R>,
    mic_device_name: Option<String>,
    system_device_name: Option<String>,
    meeting_name: Option<String>,
) -> Result<(), String> {
    log_info!("🔥 CALLED start_recording with meeting: {:?}", meeting_name);
    log_info!(
        "📋 Backend received parameters - mic: {:?}, system: {:?}, meeting: {:?}",
        mic_device_name,
        system_device_name,
        meeting_name
    );

    if is_recording().await {
        return Err("Recording already in progress".to_string());
    }

    // Route to the correct function: if no devices specified, use defaults
    let recording_result = match (mic_device_name.clone(), system_device_name.clone()) {
        (None, None) => {
            log_info!("No devices specified, starting with system defaults");
            audio::recording_commands::start_recording_with_meeting_name(app.clone(), meeting_name.clone())
                .await
        }
        _ => {
            audio::recording_commands::start_recording_with_devices_and_meeting(
                app.clone(),
                mic_device_name,
                system_device_name,
                meeting_name.clone(),
            )
            .await
        }
    };

    match recording_result
    {
        Ok(_) => {
            RECORDING_FLAG.store(true, Ordering::SeqCst);
            tray::update_tray_menu(&app);

            log_info!("Recording started successfully");

            let notification_manager_state = app.state::<NotificationManagerState<R>>();
            if let Err(e) = notifications::commands::show_recording_started_notification(
                &app,
                &notification_manager_state,
                meeting_name.clone(),
            )
            .await
            {
                log_error!(
                    "Failed to show recording started notification: {}",
                    e
                );
            } else {
                log_info!("Successfully showed recording started notification");
            }

            Ok(())
        }
        Err(e) => {
            log_error!("Failed to start audio recording: {}", e);
            Err(format!("Failed to start recording: {}", e))
        }
    }
}

#[tauri::command]
async fn stop_recording<R: Runtime>(app: AppHandle<R>, args: RecordingArgs) -> Result<(), String> {
    log_info!("Attempting to stop recording...");

    if !audio::recording_commands::is_recording().await {
        log_info!("Recording is already stopped");
        return Ok(());
    }

    match audio::recording_commands::stop_recording(
        app.clone(),
        audio::recording_commands::RecordingArgs {
            save_path: args.save_path.clone(),
        },
    )
    .await
    {
        Ok(_) => {
            RECORDING_FLAG.store(false, Ordering::SeqCst);
            tray::update_tray_menu(&app);
            subtitle_overlay::clear_subtitle_segments_internal();

            if let Some(parent) = std::path::Path::new(&args.save_path).parent() {
                if !parent.exists() {
                    log_info!("Creating directory: {:?}", parent);
                    if let Err(e) = std::fs::create_dir_all(parent) {
                        let err_msg = format!("Failed to create save directory: {}", e);
                        log_error!("{}", err_msg);
                        return Err(err_msg);
                    }
                }
            }

            let notification_manager_state = app.state::<NotificationManagerState<R>>();
            if let Err(e) = notifications::commands::show_recording_stopped_notification(
                &app,
                &notification_manager_state,
            )
            .await
            {
                log_error!(
                    "Failed to show recording stopped notification: {}",
                    e
                );
            } else {
                log_info!("Successfully showed recording stopped notification");
            }

            Ok(())
        }
        Err(e) => {
            log_error!("Failed to stop audio recording: {}", e);
            RECORDING_FLAG.store(false, Ordering::SeqCst);
            tray::update_tray_menu(&app);
            Err(format!("Failed to stop recording: {}", e))
        }
    }
}

#[tauri::command]
async fn is_recording() -> bool {
    audio::recording_commands::is_recording().await
}

#[tauri::command]
fn get_transcription_status() -> TranscriptionStatus {
    TranscriptionStatus {
        chunks_in_queue: 0,
        is_processing: false,
        last_activity_ms: 0,
    }
}

#[tauri::command]
fn read_audio_file(file_path: String) -> Result<Vec<u8>, String> {
    match std::fs::read(&file_path) {
        Ok(data) => Ok(data),
        Err(e) => Err(format!("Failed to read audio file: {}", e)),
    }
}

#[tauri::command]
async fn save_transcript(file_path: String, content: String) -> Result<(), String> {
    log_info!("Saving transcript to: {}", file_path);

    if let Some(parent) = std::path::Path::new(&file_path).parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create directory: {}", e))?;
        }
    }

    std::fs::write(&file_path, content)
        .map_err(|e| format!("Failed to write transcript: {}", e))?;

    log_info!("Transcript saved successfully");
    Ok(())
}

// Audio level monitoring commands
#[tauri::command]
async fn start_audio_level_monitoring<R: Runtime>(
    app: AppHandle<R>,
    device_names: Vec<String>,
) -> Result<(), String> {
    log_info!(
        "Starting audio level monitoring for devices: {:?}",
        device_names
    );

    audio::simple_level_monitor::start_monitoring(app, device_names)
        .await
        .map_err(|e| format!("Failed to start audio level monitoring: {}", e))
}

#[tauri::command]
async fn stop_audio_level_monitoring() -> Result<(), String> {
    log_info!("Stopping audio level monitoring");

    audio::simple_level_monitor::stop_monitoring()
        .await
        .map_err(|e| format!("Failed to stop audio level monitoring: {}", e))
}

#[tauri::command]
async fn is_audio_level_monitoring() -> bool {
    audio::simple_level_monitor::is_monitoring()
}

#[tauri::command]
async fn get_audio_devices() -> Result<Vec<AudioDevice>, String> {
    list_audio_devices()
        .await
        .map_err(|e| format!("Failed to list audio devices: {}", e))
}

#[tauri::command]
async fn trigger_microphone_permission() -> Result<bool, String> {
    trigger_audio_permission()
        .map_err(|e| format!("Failed to trigger microphone permission: {}", e))
}

#[derive(Serialize)]
struct DefaultDevicesInfo {
    microphone: Option<String>,
    speaker: Option<String>,
}

#[tauri::command]
fn get_default_audio_devices() -> DefaultDevicesInfo {
    use audio::devices::{default_input_device, default_output_device};
    DefaultDevicesInfo {
        microphone: default_input_device().ok().map(|d| d.name),
        speaker: default_output_device().ok().map(|d| d.name),
    }
}

#[tauri::command]
fn open_system_sound_settings() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg("x-apple.systempreferences:com.apple.preference.sound")
            .spawn()
            .map_err(|e| format!("Failed: {}", e))?;
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/c", "start", "ms-settings:sound"])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map_err(|e| format!("Failed: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("sh")
            .arg("-c")
            .arg("gnome-control-center sound 2>/dev/null || pavucontrol 2>/dev/null || true")
            .spawn()
            .map_err(|e| format!("Failed: {}", e))?;
    }

    Ok(())
}

#[tauri::command]
async fn start_recording_with_devices<R: Runtime>(
    app: AppHandle<R>,
    mic_device_name: Option<String>,
    system_device_name: Option<String>,
) -> Result<(), String> {
    start_recording_with_devices_and_meeting(app, mic_device_name, system_device_name, None).await
}

#[tauri::command]
async fn start_recording_with_devices_and_meeting<R: Runtime>(
    app: AppHandle<R>,
    mic_device_name: Option<String>,
    system_device_name: Option<String>,
    meeting_name: Option<String>,
) -> Result<(), String> {
    log_info!("🚀 CALLED start_recording_with_devices_and_meeting - Mic: {:?}, System: {:?}, Meeting: {:?}",
             mic_device_name, system_device_name, meeting_name);

    let meeting_name_for_notification = meeting_name.clone();

    let recording_result = match (mic_device_name.clone(), system_device_name.clone()) {
        (None, None) => {
            log_info!(
                "No devices specified, starting with defaults and meeting: {:?}",
                meeting_name
            );
            audio::recording_commands::start_recording_with_meeting_name(app.clone(), meeting_name)
                .await
        }
        _ => {
            log_info!(
                "Starting with specified devices: mic={:?}, system={:?}, meeting={:?}",
                mic_device_name,
                system_device_name,
                meeting_name
            );
            audio::recording_commands::start_recording_with_devices_and_meeting(
                app.clone(),
                mic_device_name,
                system_device_name,
                meeting_name,
            )
            .await
        }
    };

    match recording_result {
        Ok(_) => {
            log_info!("Recording started successfully via tauri command");

            let notification_manager_state = app.state::<NotificationManagerState<R>>();
            if let Err(e) = notifications::commands::show_recording_started_notification(
                &app,
                &notification_manager_state,
                meeting_name_for_notification.clone(),
            )
            .await
            {
                log_error!(
                    "Failed to show recording started notification: {}",
                    e
                );
            }

            Ok(())
        }
        Err(e) => {
            log_error!("Failed to start recording via tauri command: {}", e);
            Err(e)
        }
    }
}

#[tauri::command]
async fn focus_main_window<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        if let Err(e) = window.show() {
            return Err(format!("Failed to show main window: {}", e));
        }
        if let Err(e) = window.set_focus() {
            return Err(format!("Failed to focus main window: {}", e));
        }
        Ok(())
    } else {
        Err("Main window not found".to_string())
    }
}

#[tauri::command]
async fn start_window_drag<R: Runtime>(window: tauri::WebviewWindow<R>) -> Result<(), String> {
    window.start_dragging().map_err(|e| format!("{}", e))
}

#[tauri::command]
async fn set_language_preference(language: String) -> Result<(), String> {
    let mut lang_pref = LANGUAGE_PREFERENCE
        .lock()
        .map_err(|e| format!("Failed to set language preference: {}", e))?;
    log_info!("Setting language preference to: {}", language);
    *lang_pref = language;
    Ok(())
}

#[tauri::command]
async fn set_remote_asr_endpoint(endpoint: String, model_name: Option<String>) -> Result<(), String> {
    let model = model_name.unwrap_or_else(|| "Qwen/Qwen3-ASR-1.7B".to_string());
    log_info!("Setting remote ASR endpoint: {} (model: {})", endpoint, model);
    audio::transcription::set_remote_asr_config(&endpoint, &model);
    Ok(())
}

#[tauri::command]
async fn check_remote_asr_health_cmd(endpoint: String) -> Result<bool, String> {
    log_info!("Checking remote ASR health at: {}", endpoint);
    let healthy = audio::transcription::check_remote_asr_health(&endpoint).await;
    Ok(healthy)
}

#[tauri::command]
async fn get_remote_asr_config() -> Result<serde_json::Value, String> {
    let endpoint = audio::transcription::get_remote_asr_endpoint();
    let model = audio::transcription::get_remote_asr_model();
    Ok(serde_json::json!({
        "endpoint": endpoint,
        "model": model,
        "configured": !endpoint.is_empty()
    }))
}

/// 设置远程服务（server_url + license + model）；license 存 OS keychain。
#[tauri::command]
async fn set_remote_config(
    server_url: String,
    license: String,
    model_name: Option<String>,
) -> Result<(), String> {
    let model = model_name.unwrap_or_else(|| "remote".to_string());
    log_info!("Setting remote config: {} (model: {})", server_url, model);
    audio::transcription::set_remote_config(&server_url, &license, &model);
    Ok(())
}

#[tauri::command]
async fn get_remote_config() -> Result<serde_json::Value, String> {
    let endpoint = audio::transcription::get_remote_asr_endpoint();
    let model = audio::transcription::get_remote_asr_model();
    let license = audio::transcription::get_remote_license();
    Ok(serde_json::json!({
        "serverUrl": endpoint,
        "license": license,
        "model": model,
        "configured": !endpoint.is_empty()
    }))
}

/// 设置三种能力各自的远程模型（模型选择器）。
#[tauri::command]
async fn set_remote_model_choice(
    asr: Option<String>,
    asr_mode: Option<String>,
    translate: Option<String>,
    tts: Option<String>,
) -> Result<(), String> {
    audio::transcription::set_remote_models(asr, asr_mode, translate, tts)
}

/// 读回三种能力的远程模型（模型选择器回显）。
#[tauri::command]
async fn get_remote_model_choice() -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "asr": audio::transcription::get_remote_asr_model(),
        "translate": audio::transcription::get_remote_translate_model(),
        "tts": audio::transcription::get_remote_tts_model(),
    }))
}

/// 远程服务总开关（持久化到 tauri store 的 settings.json）。
#[tauri::command]
async fn set_remote_enabled(app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    audio::transcription::set_remote_enabled(enabled);
    use tauri_plugin_store::StoreExt;
    let store = app
        .store("settings.json")
        .map_err(|e| format!("打开设置存储失败: {}", e))?;
    store.set("remote.enabled".to_string(), serde_json::json!(enabled));
    store.save().map_err(|e| format!("保存设置失败: {}", e))?;
    log_info!("Remote service enabled: {}", enabled);
    Ok(())
}

#[tauri::command]
async fn get_remote_enabled() -> Result<bool, String> {
    Ok(audio::transcription::remote_enabled_raw())
}

/// 从网关 /v1/models 拉取当前启用的 ASR/翻译/TTS 模型名（kind → id）。
#[tauri::command]
async fn get_remote_models() -> Result<serde_json::Value, String> {
    let base = audio::transcription::remote_api_base()
        .ok_or_else(|| "远程服务未配置".to_string())?;
    let license = audio::transcription::get_remote_license();
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{}/models", base))
        .bearer_auth(license)
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
        .map_err(|e| format!("请求失败: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("网关返回 HTTP {}", resp.status()));
    }
    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

    let mut asr = String::new();
    let mut translate = String::new();
    let mut tts = String::new();
    if let Some(data) = json.get("data").and_then(|d| d.as_array()) {
        for m in data {
            let kind = m.get("kind").and_then(|k| k.as_str()).unwrap_or("");
            let id = m.get("id").and_then(|i| i.as_str()).unwrap_or("");
            match kind {
                "asr" => asr = id.to_string(),
                "translate" => translate = id.to_string(),
                "tts" => tts = id.to_string(),
                _ => {}
            }
        }
    }
    Ok(serde_json::json!({ "asr": asr, "translate": translate, "tts": tts }))
}

/// 拉取网关 /v1/models 全量列表（供模型选择器下拉）。
#[tauri::command]
async fn list_remote_models() -> Result<serde_json::Value, String> {
    let base = audio::transcription::remote_api_base()
        .ok_or_else(|| "远程服务未配置".to_string())?;
    let license = audio::transcription::get_remote_license();
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{}/models", base))
        .bearer_auth(license)
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
        .map_err(|e| format!("请求失败: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("网关返回 HTTP {}", resp.status()));
    }
    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    Ok(json.get("data").cloned().unwrap_or_else(|| serde_json::json!([])))
}

/// 一键清除所有模型后台，释放内存：
/// - ASR（sherpa-onnx：SenseVoice / X-ASR）
/// - 翻译（OPUS-MT 引擎）
/// - Hy-MT2 翻译 + 会议总结（llama-helper sidecar，直接 kill 释放 GGUF 模型）
#[tauri::command]
fn clear_all_model_backends() -> Result<(), String> {
    sherpa_onnx_engine::commands::unload_all_engines();
    translation::unload_opus_engines();
    llama_sidecar::kill_helper();
    log_info!("All model backends cleared");
    Ok(())
}

/// 拉取积分余额（网关 /v1/usage）。
#[tauri::command]
async fn get_remote_usage() -> Result<serde_json::Value, String> {
    let base = audio::transcription::remote_api_base()
        .ok_or_else(|| "远程服务未配置".to_string())?;
    let license = audio::transcription::get_remote_license();
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{}/usage", base))
        .bearer_auth(license)
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
        .map_err(|e| format!("请求失败: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("网关返回 HTTP {}", resp.status()));
    }
    resp.json().await.map_err(|e| e.to_string())
}

/// 读取稳定设备标识：macOS IOPlatformUUID / Windows MachineGuid / Linux machine-id。
fn read_device_id() -> Result<String, String> {
    #[cfg(target_os = "macos")]
    {
        let out = std::process::Command::new("ioreg")
            .args(["-rd1", "-c", "IOPlatformExpertDevice"])
            .output()
            .map_err(|e| format!("ioreg 失败: {e}"))?;
        let s = String::from_utf8_lossy(&out.stdout);
        let marker = "\"IOPlatformUUID\" = \"";
        if let Some(idx) = s.find(marker) {
            let rest = &s[idx + marker.len()..];
            if let Some(end) = rest.find('"') {
                return Ok(rest[..end].to_string());
            }
        }
        return Err("未找到 IOPlatformUUID".to_string());
    }
    #[cfg(target_os = "windows")]
    {
        let out = std::process::Command::new("reg")
            .args(["query", r"HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Cryptography", "/v", "MachineGuid"])
            .output()
            .map_err(|e| format!("reg 失败: {e}"))?;
        let s = String::from_utf8_lossy(&out.stdout);
        for line in s.lines() {
            if line.contains("MachineGuid") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if let Some(v) = parts.last() {
                    return Ok(v.to_string());
                }
            }
        }
        return Err("未找到 MachineGuid".to_string());
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let s = std::fs::read_to_string("/etc/machine-id").map_err(|e| format!("machine-id 失败: {e}"))?;
        return Ok(s.trim().to_string());
    }
}

/// 读取设备标识（供前端/自动注册使用）。
#[tauri::command]
fn get_device_id() -> Result<String, String> {
    read_device_id()
}

/// 设备绑定自动注册：POST /v1/register，成功则把授权码写入 keychain + 内存。
#[tauri::command]
async fn register_device() -> Result<serde_json::Value, String> {
    let base = audio::transcription::remote_api_base()
        .ok_or_else(|| "远程服务未配置".to_string())?;
    let device_id = read_device_id()?;
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{base}/register"))
        .json(&serde_json::json!({ "device_id": device_id }))
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| format!("请求失败: {e}"))?;
    let status = resp.status();
    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        let msg = json
            .get("error")
            .and_then(|e| e.get("message"))
            .and_then(|m| m.as_str())
            .unwrap_or("注册失败");
        return Err(msg.to_string());
    }
    if let Some(key) = json.get("api_key").and_then(|k| k.as_str()) {
        audio::transcription::set_remote_license(key);
    }
    Ok(json)
}

/// 兑换码充值：POST /v1/redeem。
#[tauri::command]
async fn redeem_code(code: String) -> Result<serde_json::Value, String> {
    let base = audio::transcription::remote_api_base()
        .ok_or_else(|| "远程服务未配置".to_string())?;
    let license = audio::transcription::get_remote_license();
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{base}/redeem"))
        .bearer_auth(license)
        .json(&serde_json::json!({ "code": code }))
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| format!("请求失败: {e}"))?;
    let status = resp.status();
    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        let msg = json
            .get("error")
            .and_then(|e| e.get("message"))
            .and_then(|m| m.as_str())
            .unwrap_or("兑换失败");
        return Err(msg.to_string());
    }
    Ok(json)
}

/// 用户自己的积分流水：GET /v1/ledger。
#[tauri::command]
async fn get_remote_ledger() -> Result<serde_json::Value, String> {
    let base = audio::transcription::remote_api_base()
        .ok_or_else(|| "远程服务未配置".to_string())?;
    let license = audio::transcription::get_remote_license();
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{base}/ledger"))
        .bearer_auth(license)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| format!("请求失败: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("网关返回 HTTP {}", resp.status()));
    }
    resp.json().await.map_err(|e| e.to_string())
}

/// 用户按模型消耗汇总：GET /v1/usage/by-model。
#[tauri::command]
async fn get_remote_usage_by_model() -> Result<serde_json::Value, String> {
    let base = audio::transcription::remote_api_base()
        .ok_or_else(|| "远程服务未配置".to_string())?;
    let license = audio::transcription::get_remote_license();
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{base}/usage/by-model"))
        .bearer_auth(license)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| format!("请求失败: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("网关返回 HTTP {}", resp.status()));
    }
    resp.json().await.map_err(|e| e.to_string())
}

/// 模型测速（网关 /v1/speedtest，仅测延迟，不扣积分）。
#[tauri::command]
async fn run_speed_test(kind: String, model: Option<String>) -> Result<serde_json::Value, String> {
    let base = audio::transcription::remote_api_base()
        .ok_or_else(|| "远程服务未配置".to_string())?;
    let license = audio::transcription::get_remote_license();
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/speedtest", base))
        .bearer_auth(license)
        .json(&serde_json::json!({ "kind": kind, "model": model }))
        .timeout(std::time::Duration::from_secs(60))
        .send()
        .await
        .map_err(|e| format!("请求失败: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("网关返回 HTTP {}", resp.status()));
    }
    resp.json().await.map_err(|e| e.to_string())
}

/// 提交用户反馈（文字 + 可选截图 base64 + 联系方式）到网关 /v1/feedback。
#[tauri::command]
async fn submit_feedback(
    text: String,
    screenshot: Option<String>,
    contact: Option<String>,
) -> Result<(), String> {
    let base = audio::transcription::remote_api_base()
        .ok_or_else(|| "远程服务未配置".to_string())?;
    let license = audio::transcription::get_remote_license();
    let body = serde_json::json!({
        "text": text,
        "screenshot": screenshot,
        "contact": contact,
        "license": license,
    });
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/feedback", base))
        .json(&body)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| format!("提交失败: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("网关返回 HTTP {}", resp.status()));
    }
    Ok(())
}

// Internal helper function to get language preference (for use within Rust code)
pub fn get_language_preference_internal() -> Option<String> {
    LANGUAGE_PREFERENCE.lock().ok().map(|lang| lang.clone())
}

pub fn run() {
    log::set_max_level(log::LevelFilter::Info);

    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .manage(Arc::new(RwLock::new(
            None::<notifications::manager::NotificationManager<tauri::Wry>>,
        )) as NotificationManagerState<tauri::Wry>)
        .manage(audio::init_system_audio_state())
        .setup(|_app| {
            log::info!("Application setup complete");

            // 注册全局 AppHandle，供各模型加载路径发送 model-loading 事件
            llama_sidecar::set_app_handle(_app.handle());

            // Initialize system tray
            if let Err(e) = tray::create_tray(_app.handle()) {
                log::error!("Failed to create system tray: {}", e);
            }

            // Explicitly set the main window icon so Windows taskbar shows the app icon.
            if let Some(main_window) = _app.get_webview_window("main") {
                match tauri::image::Image::from_bytes(include_bytes!("../icons/app_icon.ico")) {
                    Ok(icon) => {
                        if let Err(e) = main_window.set_icon(icon) {
                            log::error!("Failed to set main window icon: {}", e);
                        }
                    }
                    Err(e) => log::error!("Failed to load app_icon.ico for window: {}", e),
                }
            }

            // Initialize notification system with proper defaults
            log::info!("Initializing notification system...");
            let app_for_notif = _app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let notif_state = app_for_notif.state::<NotificationManagerState<tauri::Wry>>();
                match notifications::commands::initialize_notification_manager(app_for_notif.clone()).await {
                    Ok(manager) => {
                        if let Err(e) = manager.set_consent(true).await {
                            log::error!("Failed to set initial consent: {}", e);
                        }
                        if let Err(e) = manager.request_permission().await {
                            log::error!("Failed to request initial permission: {}", e);
                        }

                        let mut state_lock = notif_state.write().await;
                        *state_lock = Some(manager);
                        log::info!("Notification system initialized with default permissions");
                    }
                    Err(e) => {
                        log::error!("Failed to initialize notification manager: {}", e);
                    }
                }
            });

            // Set Sherpa-ONNX models directory
            sherpa_onnx_engine::commands::set_models_directory(&_app.handle());

            // Initialize database (handles first launch detection and conditional setup)
            tauri::async_runtime::block_on(async {
                let init_result = database::setup::initialize_database_on_startup(&_app.handle()).await;

                init_result
            })
            .expect("Failed to initialize database");

            // 读回持久化的翻译设置（引擎/home 语言/目标语言）写入内存态，读不到保持默认值
            {
                use crate::database::repositories::setting::SettingsRepository;
                let app_state = _app.state::<state::AppState>();
                let pool = app_state.db_manager.pool();
                let (saved_engine, saved_lang, saved_home) = tauri::async_runtime::block_on(async {
                    let engine = SettingsRepository::get(pool, "translation.engine")
                        .await
                        .ok()
                        .flatten();
                    let lang = SettingsRepository::get(pool, "translation.target_lang")
                        .await
                        .ok()
                        .flatten();
                    let home = SettingsRepository::get(pool, "translation.home_lang")
                        .await
                        .ok()
                        .flatten();
                    (engine, lang, home)
                });
                if let Some(engine) =
                    saved_engine.filter(|e| matches!(e.as_str(), "opus" | "hymt2" | "remote"))
                {
                    if let Ok(mut guard) = translation::TRANSLATION_ENGINE.lock() {
                        *guard = engine;
                    }
                }
                // home 语言：非法值忽略，保持默认 "zh"
                if let Some(home) =
                    saved_home.filter(|h| matches!(h.as_str(), "en" | "zh" | "ko" | "ja"))
                {
                    if let Ok(mut guard) = translation::HOME_LANG.lock() {
                        *guard = home;
                    }
                }
                // 目标语言：合法的 13 种之一直接读回；存量 "auto" 或非法值
                // 迁移为 home 的默认目标（home != "en" → "en"，home == "en" → "zh"）
                if let Some(lang) = saved_lang {
                    let migrated = if translation::llm::SUPPORTED_TARGET_LANGS.contains(&lang.as_str())
                    {
                        lang
                    } else {
                        translation::default_target_for_home(&translation::home_lang())
                    };
                    if let Ok(mut guard) = translation::TARGET_LANG.lock() {
                        *guard = migrated;
                    }
                }
            }

            // 本地模型全部按需加载：不在启动时预加载翻译引擎（OPUS-MT / Hy-MT2），
            // 避免 sidecar 常驻约 1.5GB 内存。首次使用（翻译/总结/录音）时才加载，
            // 加载/卸载进度通过 model-loading 事件以 toast 提示用户。

            // Restore remote ASR endpoint config from disk (survives app restarts)
            audio::transcription::load_remote_asr_config_from_disk();

            // 恢复远程服务总开关（存于 tauri-plugin-store 的 settings.json）
            {
                use tauri_plugin_store::StoreExt;
                if let Ok(store) = _app.store("settings.json") {
                    let enabled = store
                        .get("remote.enabled")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    audio::transcription::set_remote_enabled(enabled);
                    log::info!("Restored remote.enabled = {}", enabled);
                }
            }

            // 恢复字幕悬浮窗的可见状态
            {
                let app_for_subtitle = _app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    subtitle_overlay::restore_subtitle_overlay_on_startup(&app_for_subtitle).await;
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            focus_main_window,
            start_window_drag,
            subtitle_overlay::push_subtitle_segment,
            subtitle_overlay::push_subtitle_translation,
            subtitle_overlay::get_subtitle_segments,
            subtitle_overlay::clear_subtitle_segments,
            subtitle_overlay::show_subtitle_window,
            subtitle_overlay::hide_subtitle_window,
            subtitle_overlay::toggle_subtitle_window,
            subtitle_overlay::get_subtitle_window_state,
            subtitle_overlay::start_subtitle_drag,
            start_recording,
            stop_recording,
            is_recording,
            get_transcription_status,
            read_audio_file,
            save_transcript,
            // Sherpa-ONNX native ASR commands
            sherpa_onnx_engine::commands::sherpa_onnx_init,
            sherpa_onnx_engine::commands::sherpa_onnx_get_models,
            sherpa_onnx_engine::commands::sherpa_onnx_load_model,
            sherpa_onnx_engine::commands::sherpa_onnx_is_model_loaded,
            sherpa_onnx_engine::commands::sherpa_onnx_get_current_model,
            sherpa_onnx_engine::commands::sherpa_onnx_get_models_directory,
            sherpa_onnx_engine::commands::set_models_directory_custom,
            model_download::get_downloadable_models,
            model_download::download_model,
            model_download::cancel_model_download,
            model_download::delete_model,
            model_download::import_model_file,
            model_download::import_model_folder,
            model_download::summary_local_models,
            get_audio_devices,
            get_default_audio_devices,
            open_system_sound_settings,
            trigger_microphone_permission,
            start_recording_with_devices,
            start_recording_with_devices_and_meeting,
            start_audio_level_monitoring,
            stop_audio_level_monitoring,
            is_audio_level_monitoring,
            audio::recording_commands::pause_recording,
            audio::recording_commands::resume_recording,
            audio::recording_commands::is_recording_paused,
            audio::recording_commands::get_recording_state,
            audio::recording_commands::get_meeting_folder_path,
            audio::recording_commands::get_transcript_history,
            audio::recording_commands::get_recording_meeting_name,
            audio::recording_commands::poll_audio_device_events,
            audio::recording_commands::get_reconnection_status,
            audio::recording_commands::attempt_device_reconnect,
            audio::recording_commands::get_active_audio_output,
            audio::recording_commands::set_mic_mute,
            audio::recording_commands::get_mic_mute,
            audio::recording_commands::toggle_mic_mute,
            audio::incremental_saver::recover_audio_from_checkpoints,
            audio::incremental_saver::cleanup_checkpoints,
            audio::incremental_saver::has_audio_checkpoints,
            api::api_get_recordings,
            api::api_get_model_config,
            api::api_save_model_config,
            api::api_delete_recording,
            api::api_get_recording,
            api::api_get_recording_metadata,
            api::api_get_recording_segments,
            api::api_save_recording_title,
            api::api_save_transcript,
            api::api_search_transcripts,
            api::api_get_transcript_config,
            api::api_save_transcript_config,
            api::api_get_api_key,
            api::api_get_transcript_api_key,
            api::api_export_recording,
            api::summary_export_markdown,
            api::api_update_segment_text,
            api::api_get_settings,
            api::api_save_setting,
            api::open_recording_folder,
            api::open_external_url,
            audio::recording_preferences::get_recording_preferences,
            audio::recording_preferences::set_recording_preferences,
            audio::recording_preferences::get_default_recordings_folder_path,
            audio::recording_preferences::open_recordings_folder,
            audio::recording_preferences::select_recording_folder,
            audio::recording_preferences::get_available_audio_backends,
            audio::recording_preferences::get_current_audio_backend,
            audio::recording_preferences::set_audio_backend,
            audio::recording_preferences::get_audio_backend_info,
            set_language_preference,
            frontend_log,
            translation::commands::translate_text,
            translation::commands::set_translation_enabled,
            translation::commands::get_translation_enabled,
            translation::commands::set_translation_target_lang,
            translation::commands::get_translation_target_lang,
            translation::commands::set_translation_home_lang,
            translation::commands::get_translation_home_lang,
            translation::commands::set_translation_engine,
            translation::commands::get_translation_engine,
            set_remote_asr_endpoint,
            check_remote_asr_health_cmd,
            get_remote_asr_config,
            set_remote_config,
            get_remote_config,
            set_remote_model_choice,
            get_remote_model_choice,
            set_remote_enabled,
            get_remote_enabled,
            get_remote_models,
            list_remote_models,
            clear_all_model_backends,
            get_remote_usage,
            get_device_id,
            register_device,
            redeem_code,
            get_remote_ledger,
            get_remote_usage_by_model,
            submit_feedback,
            tts::tts_synthesize,
            tts::save_tts_audio,
            remote_messages::fetch_remote_messages,
            remote_messages::fetch_notice_documents,
            run_speed_test,
            notifications::commands::get_notification_settings,
            notifications::commands::set_notification_settings,
            notifications::commands::request_notification_permission,
            notifications::commands::show_notification,
            notifications::commands::show_test_notification,
            notifications::commands::is_dnd_active,
            notifications::commands::get_system_dnd_status,
            notifications::commands::set_manual_dnd,
            notifications::commands::set_notification_consent,
            notifications::commands::clear_notifications,
            notifications::commands::is_notification_system_ready,
            notifications::commands::initialize_notification_manager_manual,
            notifications::commands::test_notification_with_auto_consent,
            notifications::commands::get_notification_stats,
            audio::system_audio_commands::start_system_audio_capture_command,
            audio::system_audio_commands::list_system_audio_devices_command,
            audio::system_audio_commands::check_system_audio_permissions_command,
            audio::system_audio_commands::start_system_audio_monitoring,
            audio::system_audio_commands::stop_system_audio_monitoring,
            audio::system_audio_commands::get_system_audio_monitoring_status,
            audio::permissions::check_screen_recording_permission_command,
            audio::permissions::request_screen_recording_permission_command,
            audio::permissions::trigger_system_audio_permission_command,
            database::commands::check_first_launch,
            database::commands::initialize_fresh_database,
            database::commands::get_database_directory,
            database::commands::open_database_folder,
            #[cfg(target_os = "macos")]
            utils::open_system_settings,
            audio::retranscription::start_retranscription_command,
            audio::retranscription::cancel_retranscription_command,
            audio::retranscription::is_retranscription_in_progress_command,
            audio::import::select_and_validate_audio_command,
            audio::import::validate_audio_file_command,
            audio::import::start_import_audio_command,
            audio::import::cancel_import_command,
            audio::import::is_import_in_progress_command,
            audio::quick_transcribe::quick_transcribe,
            audio::quick_transcribe::benchmark_asr,
            audio::quick_transcribe::prepare_auto_test_audio,
            audio::audio_test::start_audio_test,
            audio::audio_test::stop_audio_test,
            audio::audio_test::replay_audio_test,
            get_audio_processing_flags,
            set_audio_processing_flags,
            summary::config::summary_get_config,
            summary::config::summary_save_config,
            summary::client::summary_test_connection,
            summary::client::summary_list_models,
            summary::client::summary_generate,
            summary::client::summary_cancel,
            summary::local::summary_local_generate,
            summary::storage::summary_save,
            summary::storage::summary_load,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app_handle, event| {
            if let tauri::RunEvent::Exit = event {
                log::info!("Application exiting, cleaning up resources...");
                llama_sidecar::shutdown_helper();
                tauri::async_runtime::block_on(async {
                    // Clean up database connection and checkpoint WAL
                    if let Some(app_state) = _app_handle.try_state::<state::AppState>() {
                        log::info!("Starting database cleanup...");
                        if let Err(e) = app_state.db_manager.cleanup().await {
                            log::error!("Failed to cleanup database: {}", e);
                        } else {
                            log::info!("Database cleanup completed successfully");
                        }
                    } else {
                        log::warn!("AppState not available for database cleanup (likely first launch)");
                    }
                });
                log::info!("Application cleanup complete");
            }
        });
}
