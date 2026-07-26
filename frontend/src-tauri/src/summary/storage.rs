//! Per-recording summary markdown files, stored in the recording's folder
//! next to the audio (no database changes).

use std::path::PathBuf;

use crate::database::repositories::recording::RecordingsRepository;
use crate::state::AppState;

fn summary_filename(source: &str) -> &'static str {
    if source == "realtime" {
        "_summary_realtime.md"
    } else {
        "_summary_offline.md"
    }
}

async fn summary_path(
    pool: &sqlx::SqlitePool,
    recording_id: &str,
    source: &str,
) -> Result<PathBuf, String> {
    let recording = RecordingsRepository::get_recording(pool, recording_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Recording not found: {}", recording_id))?
        .recording;
    let folder = recording
        .folder_path
        .filter(|f| !f.trim().is_empty())
        .ok_or_else(|| format!("Recording {} has no folder path", recording_id))?;
    Ok(PathBuf::from(folder).join(summary_filename(source)))
}

/// Write the summary markdown for a recording; returns the full file path.
#[tauri::command]
pub async fn summary_save(
    state: tauri::State<'_, AppState>,
    recording_id: String,
    source: String,
    content: String,
) -> Result<String, String> {
    let path = summary_path(state.db_manager.pool(), &recording_id, &source).await?;
    std::fs::write(&path, content).map_err(|e| format!("Failed to write summary: {}", e))?;
    Ok(path.to_string_lossy().to_string())
}

/// Read the summary markdown for a recording, or `None` when none exists.
#[tauri::command]
pub async fn summary_load(
    state: tauri::State<'_, AppState>,
    recording_id: String,
    source: String,
) -> Result<Option<String>, String> {
    let path = summary_path(state.db_manager.pool(), &recording_id, &source).await?;
    match std::fs::read_to_string(&path) {
        Ok(content) => Ok(Some(content)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("Failed to read summary: {}", e)),
    }
}
