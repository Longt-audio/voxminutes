use log::{info, warn};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{AppHandle, Runtime};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_store::StoreExt;

use anyhow::Result;
#[cfg(target_os = "macos")]
use log::error;

#[cfg(target_os = "macos")]
use crate::audio::capture::AudioCaptureBackend;

fn default_file_format() -> String {
    "mp4".to_string()
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RecordingPreferences {
    #[serde(rename = "recordingsFolder")]
    pub save_folder: PathBuf,
    #[serde(rename = "autoSave")]
    pub auto_save: bool,
    // 前端 setRecordingPreferences 只发 recordingsFolder/autoSave（/defaultAsrModel），
    // 没有 serde default 时整条命令会因 missing field `file_format` 反序列化失败，
    // 设置页改目录直接报错（2026-09-28 排查确认）。
    #[serde(default = "default_file_format")]
    pub file_format: String,
    #[serde(default)]
    pub preferred_mic_device: Option<String>,
    #[serde(default)]
    pub preferred_system_device: Option<String>,
    #[serde(default)]
    #[serde(rename = "defaultAsrModel")]
    pub default_asr_model: Option<String>,
    #[cfg(target_os = "macos")]
    #[serde(default)]
    pub system_audio_backend: Option<String>,
}

impl Default for RecordingPreferences {
    fn default() -> Self {
        Self {
            save_folder: get_default_recordings_folder(),
            auto_save: true,
            file_format: "mp4".to_string(),
            preferred_mic_device: None,
            preferred_system_device: None,
            default_asr_model: Some("x-asr-480ms".to_string()),
            #[cfg(target_os = "macos")]
            system_audio_backend: Some("coreaudio".to_string()),
        }
    }
}

/// Get the default recordings folder — uses home directory "recordings" on all platforms
pub fn get_default_recordings_folder() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("recordings")
}

/// Resolve the effective recordings base folder: the user's configured
/// `save_folder`, falling back to the default when preferences can't be loaded
/// or the configured path is empty. Used by every place that creates a new
/// meeting folder (recording / import / merge) so the setting is honored
/// consistently.
pub async fn resolved_recordings_folder<R: Runtime>(app: &AppHandle<R>) -> PathBuf {
    match load_recording_preferences(app).await {
        Ok(prefs) if !prefs.save_folder.as_os_str().is_empty() => prefs.save_folder,
        Ok(_) => {
            warn!("Configured recordings folder is empty, using default");
            get_default_recordings_folder()
        }
        Err(e) => {
            warn!(
                "Failed to load recording preferences, using default recordings folder: {}",
                e
            );
            get_default_recordings_folder()
        }
    }
}

/// Ensure the recordings directory exists
pub fn ensure_recordings_directory(path: &PathBuf) -> Result<()> {
    if !path.exists() {
        std::fs::create_dir_all(path)?;
        info!("Created recordings directory: {:?}", path);
    }
    Ok(())
}

/// Validate that `path` is usable as the recordings base folder:
/// non-empty, creatable, and writable. Called before persisting preferences so
/// a bad destination is rejected at settings time instead of failing (or
/// silently losing recordings) at recording time.
pub fn validate_recordings_folder(path: &PathBuf) -> Result<()> {
    if path.as_os_str().is_empty() {
        return Err(anyhow::anyhow!("录音保存目录不能为空"));
    }
    ensure_recordings_directory(path)?;
    // 目录存在 ≠ 可写（只读挂载/权限收紧）。写一个探测文件再删掉来验证。
    let probe = path.join(".voxminutes_write_test");
    std::fs::write(&probe, b"test")
        .map_err(|e| anyhow::anyhow!("录音保存目录不可写 {}: {}", path.display(), e))?;
    let _ = std::fs::remove_file(&probe);
    Ok(())
}

/// Generate a unique filename for a recording
pub fn generate_recording_filename(format: &str) -> String {
    let now = chrono::Utc::now();
    let timestamp = now.format("%Y%m%d_%H%M%S");
    format!("recording_{}.{}", timestamp, format)
}

/// Load recording preferences from store
pub async fn load_recording_preferences<R: Runtime>(
    app: &AppHandle<R>,
) -> Result<RecordingPreferences> {
    // Try to load from Tauri store
    let store = match app.store("recording_preferences.json") {
        Ok(store) => store,
        Err(e) => {
            warn!("Failed to access store: {}, using defaults", e);
            return Ok(RecordingPreferences::default());
        }
    };

    // Try to get the preferences from store
    let prefs = if let Some(value) = store.get("preferences") {
        match serde_json::from_value::<RecordingPreferences>(value.clone()) {
            Ok(p) => {
                info!("Loaded recording preferences from store");
                #[cfg(target_os = "macos")]
                {
                    let mut p = p;
                    let backend = crate::audio::capture::get_current_backend();
                    p.system_audio_backend = Some(backend.to_string());
                    p
                }
                #[cfg(not(target_os = "macos"))]
                {
                    p
                }
            }
            Err(e) => {
                warn!("Failed to deserialize preferences: {}, using defaults", e);
                RecordingPreferences::default()
            }
        }
    } else {
        info!("No stored preferences found, using defaults");
        RecordingPreferences::default()
    };

    info!("Loaded recording preferences: save_folder={:?}, auto_save={}, format={}, mic={:?}, system={:?}",
          prefs.save_folder, prefs.auto_save, prefs.file_format,
          prefs.preferred_mic_device, prefs.preferred_system_device);
    Ok(prefs)
}

/// Save recording preferences to store
pub async fn save_recording_preferences<R: Runtime>(
    app: &AppHandle<R>,
    preferences: &RecordingPreferences,
) -> Result<()> {
    info!("Saving recording preferences: save_folder={:?}, auto_save={}, format={}, mic={:?}, system={:?}",
          preferences.save_folder, preferences.auto_save, preferences.file_format,
          preferences.preferred_mic_device, preferences.preferred_system_device);

    // 先验证目标目录（可创建 + 可写），失败则不落盘——避免把坏路径存进去之后
    // 录音时才发现（旧行为是把目录建不起来当警告吞掉）。
    validate_recordings_folder(&preferences.save_folder)?;

    // Get or create store
    let store = app
        .store("recording_preferences.json")
        .map_err(|e| anyhow::anyhow!("Failed to access store: {}", e))?;

    // Serialize preferences to JSON value
    let prefs_value = serde_json::to_value(preferences)
        .map_err(|e| anyhow::anyhow!("Failed to serialize preferences: {}", e))?;

    // Save to store
    store.set("preferences", prefs_value);

    // Persist to disk
    store
        .save()
        .map_err(|e| anyhow::anyhow!("Failed to save store to disk: {}", e))?;

    info!("Successfully persisted recording preferences to disk");

    // Save backend preference to global config
    #[cfg(target_os = "macos")]
    if let Some(backend_str) = &preferences.system_audio_backend {
        if let Some(backend) = AudioCaptureBackend::from_string(backend_str) {
            info!("Setting audio capture backend to: {:?}", backend);
            crate::audio::capture::set_current_backend(backend);
        }
    }

    Ok(())
}

/// Tauri commands for recording preferences
#[tauri::command]
pub async fn get_recording_preferences<R: Runtime>(
    app: AppHandle<R>,
) -> Result<RecordingPreferences, String> {
    load_recording_preferences(&app)
        .await
        .map_err(|e| format!("Failed to load recording preferences: {}", e))
}

#[tauri::command]
pub async fn set_recording_preferences<R: Runtime>(
    app: AppHandle<R>,
    preferences: RecordingPreferences,
) -> Result<(), String> {
    // 前端只发送它管理的字段（recordingsFolder / autoSave [/ defaultAsrModel]），
    // 其余字段（file_format、设备偏好、macOS 采集后端）经 serde default 进来的是
    // 默认值而非用户已有值——直接整体覆盖会在「改目录」时把其他偏好抹掉。
    // 这里与已存偏好合并：只采纳前端真正负责的字段。
    let mut merged = load_recording_preferences(&app)
        .await
        .unwrap_or_else(|_| RecordingPreferences::default());
    merged.save_folder = preferences.save_folder;
    merged.auto_save = preferences.auto_save;
    if preferences.default_asr_model.is_some() {
        merged.default_asr_model = preferences.default_asr_model;
    }
    save_recording_preferences(&app, &merged)
        .await
        .map_err(|e| format!("Failed to save recording preferences: {}", e))
}

#[tauri::command]
pub async fn get_default_recordings_folder_path() -> Result<String, String> {
    let path = get_default_recordings_folder();
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn open_recordings_folder<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    let preferences = load_recording_preferences(&app)
        .await
        .map_err(|e| format!("Failed to load preferences: {}", e))?;

    // Ensure directory exists before trying to open it
    ensure_recordings_directory(&preferences.save_folder)
        .map_err(|e| format!("Failed to create directory: {}", e))?;

    let folder_path = preferences.save_folder.to_string_lossy().to_string();

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&folder_path)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&folder_path)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        std::process::Command::new("xdg-open")
            .arg(&folder_path)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }

    info!("Opened recordings folder: {}", folder_path);
    Ok(())
}

#[tauri::command]
pub async fn select_recording_folder<R: Runtime>(
    app: AppHandle<R>,
) -> Result<Option<String>, String> {
    let path = app.dialog().file().blocking_pick_folder();

    match path {
        Some(p) => {
            let path_str = p.to_string();
            info!("User selected recordings folder: {}", path_str);
            Ok(Some(path_str))
        }
        None => {
            info!("User cancelled folder selection");
            Ok(None)
        }
    }
}

// Backend selection commands

/// Get available audio capture backends for the current platform
#[tauri::command]
pub async fn get_available_audio_backends() -> Result<Vec<String>, String> {
    #[cfg(target_os = "macos")]
    {
        let backends = crate::audio::capture::get_available_backends();
        Ok(backends.iter().map(|b| b.to_string()).collect())
    }

    #[cfg(not(target_os = "macos"))]
    {
        // Only ScreenCaptureKit available on non-macOS
        Ok(vec!["screencapturekit".to_string()])
    }
}

/// Get current audio capture backend
#[tauri::command]
pub async fn get_current_audio_backend() -> Result<String, String> {
    #[cfg(target_os = "macos")]
    {
        let backend = crate::audio::capture::get_current_backend();
        Ok(backend.to_string())
    }

    #[cfg(not(target_os = "macos"))]
    {
        Ok("screencapturekit".to_string())
    }
}

/// Set audio capture backend
#[tauri::command]
pub async fn set_audio_backend(backend: String) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        use crate::audio::capture::AudioCaptureBackend;
        use crate::audio::permissions::{
            check_screen_recording_permission, request_screen_recording_permission,
        };

        let backend_enum = AudioCaptureBackend::from_string(&backend)
            .ok_or_else(|| format!("Invalid backend: {}", backend))?;

        // If switching to Core Audio, log information about Audio Capture permission
        if backend_enum == AudioCaptureBackend::CoreAudio {
            info!("🔐 Core Audio backend requires Audio Capture permission (macOS 14.4+)");
            info!("📍 Permission dialog will appear automatically when recording starts");

            // Check if permission is already granted (this is informational only)
            if !check_screen_recording_permission() {
                warn!("⚠️  Audio Capture permission may not be granted");

                // Attempt to open System Settings (opens System Settings)
                if let Err(e) = request_screen_recording_permission() {
                    error!("Failed to open System Settings: {}", e);
                }

                return Err(
                    "Core Audio requires Audio Capture permission. \
                    The permission dialog will appear when you start recording. \
                    If already denied, enable it in System Settings → Privacy & Security → Audio Capture, \
                    then restart the app.".to_string()
                );
            }

            info!(
                "✅ Core Audio backend selected - permission check will occur at recording start"
            );
        }

        info!("Setting audio backend to: {:?}", backend_enum);
        crate::audio::capture::set_current_backend(backend_enum);
        Ok(())
    }

    #[cfg(not(target_os = "macos"))]
    {
        if backend != "screencapturekit" {
            return Err(format!(
                "Backend {} not available on this platform",
                backend
            ));
        }
        Ok(())
    }
}

/// Get backend information (name and description)
#[derive(Serialize)]
pub struct BackendInfo {
    pub id: String,
    pub name: String,
    pub description: String,
}

#[tauri::command]
pub async fn get_audio_backend_info() -> Result<Vec<BackendInfo>, String> {
    #[cfg(target_os = "macos")]
    {
        use crate::audio::capture::AudioCaptureBackend;

        let backends = vec![
            BackendInfo {
                id: AudioCaptureBackend::ScreenCaptureKit.to_string(),
                name: AudioCaptureBackend::ScreenCaptureKit.name().to_string(),
                description: AudioCaptureBackend::ScreenCaptureKit
                    .description()
                    .to_string(),
            },
            BackendInfo {
                id: AudioCaptureBackend::CoreAudio.to_string(),
                name: AudioCaptureBackend::CoreAudio.name().to_string(),
                description: AudioCaptureBackend::CoreAudio.description().to_string(),
            },
        ];
        Ok(backends)
    }

    #[cfg(not(target_os = "macos"))]
    {
        Ok(vec![BackendInfo {
            id: "screencapturekit".to_string(),
            name: "ScreenCaptureKit".to_string(),
            description: "Default system audio capture".to_string(),
        }])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 前端 setRecordingPreferences 只发 {recordingsFolder, autoSave} 两个字段，
    /// file_format 必须有 serde default，否则整条保存命令反序列化失败。
    #[test]
    fn deserialize_frontend_partial_payload() {
        let value = serde_json::json!({
            "recordingsFolder": "/tmp/vox-test-recordings",
            "autoSave": true
        });
        let prefs: RecordingPreferences =
            serde_json::from_value(value).expect("partial payload must deserialize");
        assert_eq!(prefs.save_folder, PathBuf::from("/tmp/vox-test-recordings"));
        assert!(prefs.auto_save);
        assert_eq!(prefs.file_format, "mp4");
        assert_eq!(prefs.preferred_mic_device, None);
    }

    /// 校验：空路径直接拒绝；不存在的路径会被创建；创建后可写。
    #[test]
    fn validate_recordings_folder_cases() {
        assert!(validate_recordings_folder(&PathBuf::new()).is_err());

        let tmp = std::env::temp_dir().join(format!(
            "vox-pref-test-{}",
            std::process::id()
        ));
        let nested = tmp.join("a").join("b");
        let _ = std::fs::remove_dir_all(&tmp);
        validate_recordings_folder(&nested).expect("nested path should be created");
        assert!(nested.is_dir());
        // 探测文件已清理
        assert!(!nested.join(".voxminutes_write_test").exists());
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
