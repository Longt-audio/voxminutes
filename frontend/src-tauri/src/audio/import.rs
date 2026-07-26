// Audio file import module - allows importing external audio files as new meetings.
// This module only creates the meeting folder, copies the audio file, and writes
// metadata/transcript files. Actual transcription is performed later by the user
// via the "offline retranscription" flow, keeping the behavior identical to
// recording-based meetings.

use crate::api::TranscriptSegment;
use crate::audio::decoder::{decode_audio_file, probe_audio_duration};
use crate::state::AppState;
use anyhow::{anyhow, Result};
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter, Manager, Runtime};
use tauri_plugin_dialog::DialogExt;
use uuid::Uuid;

use super::audio_processing::create_meeting_folder;
use super::common::write_transcripts_json;
use super::constants::AUDIO_EXTENSIONS;
use super::recording_preferences::get_default_recordings_folder;

/// Global flag to track if import is in progress
static IMPORT_IN_PROGRESS: AtomicBool = AtomicBool::new(false);

/// Global flag to signal cancellation
static IMPORT_CANCELLED: AtomicBool = AtomicBool::new(false);

/// RAII guard for IMPORT_IN_PROGRESS flag
struct ImportGuard;

impl ImportGuard {
    fn acquire() -> Result<Self, String> {
        if IMPORT_IN_PROGRESS
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return Err("Import already in progress".to_string());
        }
        Ok(ImportGuard)
    }
}

impl Drop for ImportGuard {
    fn drop(&mut self) {
        IMPORT_IN_PROGRESS.store(false, Ordering::SeqCst);
    }
}

const MAX_FILE_SIZE_BYTES: u64 = 20 * 1024 * 1024 * 1024; // 20GB

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioFileInfo {
    pub path: String,
    pub filename: String,
    pub duration_seconds: f64,
    pub size_bytes: u64,
    pub format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportProgress {
    pub stage: String,
    pub progress_percentage: u32,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportResult {
    pub meeting_id: String,
    pub title: String,
    pub segments_count: usize,
    pub duration_seconds: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportError {
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportWarning {
    pub warning: String,
    pub details: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportStarted {
    pub message: String,
}

pub fn is_import_in_progress() -> bool {
    IMPORT_IN_PROGRESS.load(Ordering::SeqCst)
}

pub fn cancel_import() {
    IMPORT_CANCELLED.store(true, Ordering::SeqCst);
}

pub fn validate_audio_file(path: &Path) -> Result<AudioFileInfo> {
    if !path.exists() {
        return Err(anyhow!("File does not exist: {}", path.display()));
    }

    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();

    if !AUDIO_EXTENSIONS.contains(&extension.as_str()) {
        return Err(anyhow!(
            "Unsupported format: .{}. Supported: {}",
            extension,
            AUDIO_EXTENSIONS.join(", ")
        ));
    }

    let metadata = std::fs::metadata(path)
        .map_err(|e| anyhow!("Cannot read file: {}", e))?;
    let size_bytes = metadata.len();

    if size_bytes > MAX_FILE_SIZE_BYTES {
        return Err(anyhow!(
            "File too large: {:.2}GB. Maximum supported size is {}GB",
            size_bytes as f64 / (1024.0 * 1024.0 * 1024.0),
            MAX_FILE_SIZE_BYTES / (1024 * 1024 * 1024)
        ));
    }

    let filename = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Imported Audio")
        .to_string();

    let duration_seconds = match extract_duration_from_metadata(path) {
        Ok(duration) => {
            debug!("Got duration from metadata: {:.2}s (fast path)", duration);
            duration
        }
        Err(e) => {
            warn!("Metadata extraction failed: {}, falling back to full decode", e);
            let decoded = decode_audio_file(path)?;
            decoded.duration_seconds
        }
    };

    Ok(AudioFileInfo {
        path: path.to_string_lossy().to_string(),
        filename,
        duration_seconds,
        size_bytes,
        format: extension.to_uppercase(),
    })
}

fn extract_duration_from_metadata(path: &Path) -> Result<f64> {
    use symphonia::core::formats::FormatOptions;
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::MetadataOptions;
    use symphonia::core::probe::Hint;

    let file = std::fs::File::open(path)
        .map_err(|e| anyhow!("Failed to open audio file: {}", e))?;

    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .map_err(|e| anyhow!("Failed to probe audio format: {}", e))?;

    let format = probed.format;

    use symphonia::core::codecs::CODEC_TYPE_NULL;
    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or_else(|| anyhow!("No audio track found in file"))?;

    let sample_rate = track
        .codec_params
        .sample_rate
        .ok_or_else(|| anyhow!("Unknown sample rate"))?;

    let n_frames = track
        .codec_params
        .n_frames
        .ok_or_else(|| anyhow!("Frame count not available in metadata"))?;

    let duration_seconds = n_frames as f64 / sample_rate as f64;

    debug!(
        "Extracted metadata: {}Hz, {} frames, {:.2}s",
        sample_rate, n_frames, duration_seconds
    );

    Ok(duration_seconds)
}

/// Start import of an audio file.
/// Model/provider arguments are accepted for API compatibility but are ignored:
/// transcription is intentionally NOT performed here; the user triggers it later
/// via the offline retranscription flow.
pub async fn start_import<R: Runtime>(
    app: AppHandle<R>,
    source_path: String,
    title: String,
    _language: Option<String>,
    _model: Option<String>,
    _provider: Option<String>,
) -> Result<ImportResult> {
    let _guard = ImportGuard::acquire().map_err(|e| anyhow!(e))?;

    IMPORT_CANCELLED.store(false, Ordering::SeqCst);

    let result = run_import(app.clone(), source_path, title).await;

    match &result {
        Ok(res) => {
            let _ = app.emit(
                "import-complete",
                serde_json::json!({
                    "meeting_id": res.meeting_id,
                    "title": res.title,
                    "segments_count": res.segments_count,
                    "duration_seconds": res.duration_seconds
                }),
            );
        }
        Err(e) => {
            let _ = app.emit(
                "import-error",
                ImportError {
                    error: e.to_string(),
                },
            );
        }
    }

    result
}

/// Internal function to run import.
/// Creates a meeting folder, copies the audio file, and persists an empty
/// transcript entry so the meeting appears in the list and can be retranscribed
/// later by the user.
async fn run_import<R: Runtime>(
    app: AppHandle<R>,
    source_path: String,
    title: String,
) -> Result<ImportResult> {
    let source = PathBuf::from(&source_path);

    if !source.exists() {
        return Err(anyhow!("Source file not found: {}", source.display()));
    }

    info!("Starting import for '{}' from {}", title, source_path);

    emit_progress(&app, "copying", 10, "创建会议文件夹...");

    if IMPORT_CANCELLED.load(Ordering::SeqCst) {
        return Err(anyhow!("Import cancelled"));
    }

    let base_folder = get_default_recordings_folder();
    let meeting_folder = create_meeting_folder(&base_folder, &title, false)?;

    emit_progress(&app, "copying", 40, "复制音频文件...");

    let dest_filename = format!(
        "audio.{}",
        source
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("mp4")
    );
    let dest_path = meeting_folder.join(&dest_filename);

    let src = source.clone();
    let dst = dest_path.clone();
    tokio::task::spawn_blocking(move || std::fs::copy(&src, &dst))
        .await
        .map_err(|e| anyhow!("Copy task join error: {}", e))?
        .map_err(|e| anyhow!("Failed to copy audio file: {}", e))?;

    info!("Copied audio to: {}", dest_path.display());

    if IMPORT_CANCELLED.load(Ordering::SeqCst) {
        let _ = std::fs::remove_dir_all(&meeting_folder);
        return Err(anyhow!("Import cancelled"));
    }

    emit_progress(&app, "saving", 80, "保存会议信息...");

    let duration_seconds = match probe_audio_duration(&dest_path) {
        Ok(d) => d,
        Err(e) => {
            warn!("Failed to probe audio duration: {}, using 0", e);
            0.0
        }
    };

    let segments: Vec<TranscriptSegment> = Vec::new();

    let app_state = app
        .try_state::<AppState>()
        .ok_or_else(|| anyhow!("App state not available"))?;

    let meeting_id = create_meeting_with_transcripts(
        app_state.db_manager.pool(),
        &title,
        &segments,
        meeting_folder.to_string_lossy().to_string(),
        (duration_seconds * 1000.0) as i64,
    )
    .await?;

    if let Err(e) = write_transcripts_json(&meeting_folder, &segments) {
        warn!("Failed to write transcripts.json: {}", e);
    }

    if let Err(e) = write_import_metadata(
        &meeting_folder,
        &meeting_id,
        &title,
        duration_seconds,
        &dest_filename,
        "import",
    ) {
        warn!("Failed to write metadata.json: {}", e);
    }

    emit_progress(&app, "complete", 100, "导入完成");

    Ok(ImportResult {
        meeting_id,
        title,
        segments_count: 0,
        duration_seconds,
    })
}

fn emit_progress<R: Runtime>(app: &AppHandle<R>, stage: &str, progress: u32, message: &str) {
    let _ = app.emit(
        "import-progress",
        ImportProgress {
            stage: stage.to_string(),
            progress_percentage: progress,
            message: message.to_string(),
        },
    );
}

async fn create_meeting_with_transcripts(
    pool: &sqlx::SqlitePool,
    title: &str,
    segments: &[TranscriptSegment],
    folder_path: String,
    duration_ms: i64,
) -> Result<String> {
    let recording_id = format!("recording-{}", Uuid::new_v4());
    let now = chrono::Utc::now();

    let mut conn = pool.acquire().await.map_err(|e| anyhow!("DB error: {}", e))?;
    let mut tx = sqlx::Connection::begin(&mut *conn)
        .await
        .map_err(|e| anyhow!("Failed to start transaction: {}", e))?;

    sqlx::query(
        "INSERT INTO recordings (id, title, created_at, updated_at, duration_ms, folder_path, source, status)
         VALUES (?, ?, ?, ?, ?, ?, 'import', 'pending')",
    )
    .bind(&recording_id)
    .bind(title)
    .bind(now)
    .bind(now)
    .bind(duration_ms)
    .bind(&folder_path)
    .execute(&mut *tx)
    .await
    .map_err(|e| anyhow!("Failed to create recording: {}", e))?;

    for segment in segments {
        sqlx::query(
            "INSERT INTO transcript_segments (id, recording_id, text, start_ms, end_ms, source, created_at)
             VALUES (?, ?, ?, ?, ?, 'import', ?)",
        )
        .bind(&segment.id)
        .bind(&recording_id)
        .bind(&segment.text)
        .bind(segment.audio_start_time.map(|s| (s * 1000.0) as i64).unwrap_or(0))
        .bind(segment.audio_end_time.map(|e| (e * 1000.0) as i64))
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(|e| anyhow!("Failed to insert transcript segment: {}", e))?;
    }

    tx.commit()
        .await
        .map_err(|e| anyhow!("Failed to commit transaction: {}", e))?;

    info!(
        "Created recording '{}' with {} transcript segments",
        recording_id,
        segments.len()
    );

    Ok(recording_id)
}

fn write_import_metadata(
    folder: &Path,
    meeting_id: &str,
    title: &str,
    duration_seconds: f64,
    audio_filename: &str,
    source: &str,
) -> Result<()> {
    let metadata_path = folder.join("metadata.json");
    let temp_path = folder.join(".metadata.json.tmp");
    let now = chrono::Utc::now().to_rfc3339();

    let json = serde_json::json!({
        "version": "1.0",
        "meeting_id": meeting_id,
        "meeting_name": title,
        "created_at": now,
        "completed_at": now,
        "duration_seconds": duration_seconds,
        "audio_file": audio_filename,
        "transcript_file": "transcripts.json",
        "status": "completed",
        "source": source
    });

    let json_string = serde_json::to_string_pretty(&json)?;
    std::fs::write(&temp_path, &json_string)?;
    std::fs::rename(&temp_path, &metadata_path)?;

    info!("Wrote metadata.json to {}", metadata_path.display());
    Ok(())
}

// ============================================================================
// Tauri Commands
// ============================================================================

#[tauri::command]
pub async fn select_and_validate_audio_command<R: Runtime>(
    app: AppHandle<R>,
) -> Result<Option<AudioFileInfo>, String> {
    info!("Opening file dialog for audio import");

    let app_clone = app.clone();
    let file_path = tokio::task::spawn_blocking(move || {
        app_clone
            .dialog()
            .file()
            .add_filter("Audio Files", &AUDIO_EXTENSIONS.iter().map(|s| *s).collect::<Vec<_>>())
            .blocking_pick_file()
    })
    .await
    .map_err(|e| format!("File dialog task failed: {}", e))?;

    match file_path {
        Some(path) => {
            let path_str = path.to_string();
            info!("User selected: {}", path_str);
            match validate_audio_file(Path::new(&path_str)) {
                Ok(info) => Ok(Some(info)),
                Err(e) => {
                    error!("Validation failed: {}", e);
                    Err(e.to_string())
                }
            }
        }
        None => {
            info!("User cancelled file selection");
            Ok(None)
        }
    }
}

#[tauri::command]
pub async fn validate_audio_file_command(path: String) -> Result<AudioFileInfo, String> {
    info!("Validating audio file: {}", path);
    validate_audio_file(Path::new(&path)).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn start_import_audio_command<R: Runtime>(
    app: AppHandle<R>,
    source_path: String,
    title: String,
    language: Option<String>,
    model: Option<String>,
    provider: Option<String>,
) -> Result<ImportStarted, String> {
    if IMPORT_IN_PROGRESS.load(Ordering::SeqCst) {
        return Err("Import already in progress".to_string());
    }

    tauri::async_runtime::spawn(async move {
        let result = start_import(app, source_path, title, language, model, provider).await;
        if let Err(e) = result {
            error!("Import failed: {}", e);
        }
    });

    Ok(ImportStarted {
        message: "Import started".to_string(),
    })
}

#[tauri::command]
pub async fn cancel_import_command() -> Result<(), String> {
    if !is_import_in_progress() {
        return Err("No import in progress".to_string());
    }
    cancel_import();
    Ok(())
}

#[tauri::command]
pub async fn is_import_in_progress_command() -> bool {
    is_import_in_progress()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_extensions() {
        assert!(AUDIO_EXTENSIONS.contains(&"mp4"));
        assert!(AUDIO_EXTENSIONS.contains(&"wav"));
        assert!(AUDIO_EXTENSIONS.contains(&"mp3"));
        assert!(!AUDIO_EXTENSIONS.contains(&"txt"));
    }

    #[test]
    fn test_cancellation_flag() {
        IMPORT_CANCELLED.store(false, Ordering::SeqCst);
        IMPORT_IN_PROGRESS.store(false, Ordering::SeqCst);
        assert!(!is_import_in_progress());
        cancel_import();
        assert!(IMPORT_CANCELLED.load(Ordering::SeqCst));
        IMPORT_CANCELLED.store(false, Ordering::SeqCst);
    }
}
