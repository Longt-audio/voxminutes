// Merge recordings module - combine >=2 recordings into one new recording.
// Audio files are concatenated with FFmpeg; transcript segments are copied
// with their timestamps shifted by each preceding recording's audio duration,
// so the merged recording behaves like a normal one (incl. offline
// re-transcription via the meeting folder's audio.mp4 + metadata.json).

use anyhow::{anyhow, Result};
use log::{info, warn};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Runtime};
use uuid::Uuid;

use super::audio_processing::create_meeting_folder;
use super::decoder::probe_audio_duration;
use super::ffmpeg::find_ffmpeg_path;
use super::recording_preferences::resolved_recordings_folder;
use super::retranscription::{find_audio_file, is_retranscription_in_progress};
use crate::database::models::Recording;
use crate::database::repositories::recording::{RecordingWithSegments, RecordingsRepository};
use crate::state::AppState;
use crate::win_short_path::{to_short_path, to_short_path_string};

/// Global flag to track if a merge is in progress (mutually exclusive with
/// retranscription; retranscription.rs checks `is_merge_in_progress`).
static MERGE_IN_PROGRESS: AtomicBool = AtomicBool::new(false);

struct MergeGuard;

impl MergeGuard {
    fn acquire() -> Result<Self, String> {
        if MERGE_IN_PROGRESS
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return Err("A merge operation is already in progress".to_string());
        }
        Ok(MergeGuard)
    }
}

impl Drop for MergeGuard {
    fn drop(&mut self) {
        MERGE_IN_PROGRESS.store(false, Ordering::SeqCst);
    }
}

pub fn is_merge_in_progress() -> bool {
    MERGE_IN_PROGRESS.load(Ordering::SeqCst)
}

/// Default marker template (Chinese) used when the frontend does not pass one.
/// Placeholders: {n} = part index (2-based), {title} = source recording title,
/// {time} = source recording's original local start time.
const DEFAULT_MARKER_TEMPLATE: &str = "—— 第 {n} 段会议「{title}」，原开始时间 {time} ——";

/// A source recording prepared for merging.
struct MergeSource {
    recording: Recording,
    segments: Vec<crate::database::models::TranscriptSegment>,
    audio_path: PathBuf,
    /// Audio duration in milliseconds (probed / metadata fallback).
    duration_ms: i64,
}

/// Compute cumulative start offsets (ms) for each part plus the total duration.
fn compute_offsets_ms(durations_ms: &[i64]) -> (Vec<i64>, i64) {
    let mut offsets = Vec::with_capacity(durations_ms.len());
    let mut total: i64 = 0;
    for d in durations_ms {
        offsets.push(total);
        total += d;
    }
    (offsets, total)
}

/// Render a junction marker text from a template with {n}/{title}/{time}
/// placeholders.
fn render_marker_text(template: &str, n: usize, title: &str, time: &str) -> String {
    template
        .replace("{n}", &n.to_string())
        .replace("{title}", title)
        .replace("{time}", time)
}

/// Determine a recording's audio duration in ms: probe the file, then fall
/// back to metadata.json's duration_seconds, then the DB duration_ms.
fn resolve_duration_ms(
    source_folder: &Path,
    audio_path: &Path,
    recording: &Recording,
) -> Result<i64> {
    if let Ok(d) = probe_audio_duration(audio_path) {
        if d > 0.0 {
            return Ok((d * 1000.0).round() as i64);
        }
    }
    // metadata.json fallback
    let meta_path = source_folder.join("metadata.json");
    if let Ok(content) = std::fs::read_to_string(&meta_path) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(d) = json.get("duration_seconds").and_then(|v| v.as_f64()) {
                if d > 0.0 {
                    return Ok((d * 1000.0).round() as i64);
                }
            }
        }
    }
    if let Some(d) = recording.duration_ms {
        if d > 0 {
            return Ok(d);
        }
    }
    Err(anyhow!("无法确定工程「{}」的音频时长", recording.title))
}

/// Concatenate audio files into `output` using FFmpeg's concat demuxer.
/// First tries stream copy (fast, same-codec inputs); falls back to
/// re-encoding to AAC 48kHz mono so mixed-format sources still merge.
fn concat_audio_files(inputs: &[PathBuf], output: &Path) -> Result<()> {
    let ffmpeg_path = find_ffmpeg_path()
        .ok_or_else(|| anyhow!("FFmpeg not found. Please install FFmpeg to merge recordings."))?;
    info!("Using FFmpeg at: {:?}", ffmpeg_path);

    // Write the concat list file next to the output.
    let list_file = output
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(".merge_concat_list.txt");
    let mut list_content = String::new();
    for input in inputs {
        let canonical = input.canonicalize()?;
        let short = to_short_path_string(&canonical);
        list_content.push_str(&format!("file '{}'\n", short.replace('\'', "'\\''")));
    }
    std::fs::write(&list_file, &list_content)?;

    let output_arg = match (output.parent(), output.file_name()) {
        (Some(parent), Some(name)) => to_short_path(parent).join(name),
        _ => output.to_path_buf(),
    }
    .to_string_lossy()
    .to_string();
    let list_arg = to_short_path_string(&list_file);

    let run = |extra_codec_args: &[&str]| -> Result<std::process::Output> {
        let mut command = std::process::Command::new(&ffmpeg_path);
        command.args(["-f", "concat", "-safe", "0", "-i", &list_arg]);
        command.args(extra_codec_args);
        command.args(["-y", &output_arg]);

        // Hide console window on Windows to prevent CMD popup.
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            command.creation_flags(CREATE_NO_WINDOW);
        }

        Ok(command.output()?)
    };

    // Attempt 1: stream copy (no re-encode).
    let copy_result = run(&["-c", "copy"]);
    let copy_ok = matches!(&copy_result, Ok(o) if o.status.success()) && output.exists();
    if !copy_ok {
        if let Ok(o) = &copy_result {
            warn!(
                "FFmpeg concat stream-copy failed, falling back to re-encode: {}",
                String::from_utf8_lossy(&o.stderr)
            );
        }
        let _ = std::fs::remove_file(output);
        // Attempt 2: re-encode to a uniform AAC/48kHz/mono track.
        let reencode = run(&["-c:a", "aac", "-b:a", "128k", "-ar", "48000", "-ac", "1"])?;
        if !reencode.status.success() {
            let _ = std::fs::remove_file(&list_file);
            return Err(anyhow!(
                "FFmpeg concat failed: {}",
                String::from_utf8_lossy(&reencode.stderr)
            ));
        }
    }

    let _ = std::fs::remove_file(&list_file);

    if !output.exists() {
        return Err(anyhow!(
            "Merged audio file was not created: {}",
            output.display()
        ));
    }
    Ok(())
}

/// Write metadata.json for the merged meeting folder.
fn write_merge_metadata(
    folder: &Path,
    meeting_id: &str,
    meeting_name: &str,
    created_at: &str,
    duration_seconds: f64,
    merged_from: &[String],
) -> Result<()> {
    let metadata_path = folder.join("metadata.json");
    let temp_path = folder.join(".metadata.json.tmp");
    let now = chrono::Utc::now().to_rfc3339();

    let json = serde_json::json!({
        "version": "1.0",
        "meeting_id": meeting_id,
        "meeting_name": meeting_name,
        "created_at": created_at,
        "completed_at": now,
        "duration_seconds": duration_seconds,
        "devices": { "microphone": null, "system_audio": null },
        "audio_file": "audio.mp4",
        "transcript_file": "transcripts.json",
        "sample_rate": 48000,
        "status": "completed",
        "source": "merged",
        "merged_from": merged_from,
    });

    std::fs::write(&temp_path, serde_json::to_string_pretty(&json)?)?;
    std::fs::rename(&temp_path, &metadata_path)?;
    Ok(())
}

/// Write transcripts_offline.json for the merged offline segments (same shape
/// as retranscription.rs's offline file).
fn write_merged_offline_transcripts_json(
    folder: &Path,
    segments: &[crate::api::TranscriptSegment],
) -> Result<()> {
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

    std::fs::write(&temp_path, serde_json::to_string_pretty(&json)?)?;
    std::fs::rename(&temp_path, &transcript_path)?;
    Ok(())
}

/// Thin Tauri wrapper: concurrency guards + user-configured recordings folder.
/// All real work lives in `merge_recordings_core` (unit-testable).
#[tauri::command]
pub async fn api_merge_recordings<R: Runtime>(
    app: AppHandle<R>,
    state: tauri::State<'_, AppState>,
    recording_ids: Vec<String>,
    title: Option<String>,
    delete_sources: bool,
    marker_template: Option<String>,
) -> Result<serde_json::Value, String> {
    info!(
        "api_merge_recordings called: {} recordings, delete_sources={}",
        recording_ids.len(),
        delete_sources
    );

    if is_retranscription_in_progress() {
        return Err(
            "A re-transcription is in progress. Please wait for it to finish before merging."
                .to_string(),
        );
    }
    let _guard = MergeGuard::acquire()?;

    // 合并产物建在用户配置的录音目录下（与实时录音/导入一致），
    // 此前硬编码默认目录导致设置不生效。
    let base_folder = resolved_recordings_folder(&app).await;
    merge_recordings_core(
        state.db_manager.pool(),
        &base_folder,
        &recording_ids,
        title,
        delete_sources,
        marker_template.as_deref(),
    )
    .await
}

/// Core merge logic, independent of the Tauri runtime: takes a DB pool and
/// the recordings base folder explicitly so integration tests can point both
/// at a tempdir.
pub(crate) async fn merge_recordings_core(
    pool: &sqlx::SqlitePool,
    base_folder: &Path,
    recording_ids: &[String],
    title: Option<String>,
    delete_sources: bool,
    marker_template: Option<&str>,
) -> Result<serde_json::Value, String> {
    if recording_ids.len() < 2 {
        return Err("At least two recordings are required to merge".to_string());
    }

    // 1. Load + validate all sources, then sort by created_at ascending.
    let mut sources: Vec<MergeSource> = Vec::with_capacity(recording_ids.len());
    for id in recording_ids {
        let RecordingWithSegments {
            recording,
            segments,
        } = RecordingsRepository::get_recording(pool, id)
            .await
            .map_err(|e| format!("Failed to load recording {}: {}", id, e))?
            .ok_or_else(|| format!("Recording not found: {}", id))?;

        let folder = recording
            .folder_path
            .clone()
            .filter(|f| !f.trim().is_empty())
            .ok_or_else(|| format!("工程「{}」没有关联的会议文件夹，无法合并", recording.title))?;
        let folder_path = PathBuf::from(&folder);
        let audio_path = find_audio_file(&folder_path).map_err(|_| {
            format!(
                "工程「{}」的文件夹中找不到音频文件，无法合并",
                recording.title
            )
        })?;

        sources.push(MergeSource {
            duration_ms: 0, // filled after sorting
            recording,
            segments,
            audio_path,
        });
    }
    sources.sort_by(|a, b| a.recording.created_at.0.cmp(&b.recording.created_at.0));

    // Resolve durations in sorted order, then build the cumulative offsets.
    for source in sources.iter_mut() {
        let folder = PathBuf::from(source.recording.folder_path.as_deref().unwrap_or_default());
        source.duration_ms = resolve_duration_ms(&folder, &source.audio_path, &source.recording)
            .map_err(|e| e.to_string())?;
    }
    let durations: Vec<i64> = sources.iter().map(|s| s.duration_ms).collect();
    let (offsets_ms, total_ms) = compute_offsets_ms(&durations);

    let first = &sources[0].recording;
    let merged_title = title
        .filter(|t| !t.trim().is_empty())
        .unwrap_or_else(|| format!("合并：{} 等{}段", first.title, sources.len()));
    let merged_id = format!("recording-{}", Uuid::new_v4());

    // 2. Create the new meeting folder（文件夹名固定 Rec_ 前缀，语言无关）
    let meeting_folder = create_meeting_folder(
        &base_folder.to_path_buf(),
        super::audio_processing::MEETING_FOLDER_PREFIX,
        false,
    )
    .map_err(|e| format!("Failed to create merged meeting folder: {}", e))?;
    let merged_audio_path = meeting_folder.join("audio.mp4");

    // From here on, clean up the folder on failure.
    let merge_result = run_merge(
        pool,
        &sources,
        &offsets_ms,
        total_ms,
        &merged_id,
        &merged_title,
        &meeting_folder,
        &merged_audio_path,
        marker_template.unwrap_or(DEFAULT_MARKER_TEMPLATE),
        delete_sources,
    )
    .await;

    if let Err(e) = &merge_result {
        warn!("Merge failed, cleaning up folder: {}", e);
        let _ = std::fs::remove_dir_all(&meeting_folder);
    }

    merge_result
}

#[allow(clippy::too_many_arguments)]
async fn run_merge(
    pool: &sqlx::SqlitePool,
    sources: &[MergeSource],
    offsets_ms: &[i64],
    total_ms: i64,
    merged_id: &str,
    merged_title: &str,
    meeting_folder: &Path,
    merged_audio_path: &Path,
    marker_template: &str,
    delete_sources: bool,
) -> Result<serde_json::Value, String> {
    // 3. Concatenate audio (blocking FFmpeg call → spawn_blocking).
    let inputs: Vec<PathBuf> = sources.iter().map(|s| s.audio_path.clone()).collect();
    let output = merged_audio_path.to_path_buf();
    tokio::task::spawn_blocking(move || concat_audio_files(&inputs, &output))
        .await
        .map_err(|e| format!("Audio merge task panicked: {}", e))?
        .map_err(|e| format!("音频拼接失败：{}", e))?;
    info!(
        "Merged {} audio files → {}",
        sources.len(),
        merged_audio_path.display()
    );

    // 4. New recording row + copied segments in one transaction.
    let first = &sources[0].recording;
    let now = chrono::Utc::now();

    let mut conn = pool
        .acquire()
        .await
        .map_err(|e| format!("DB error: {}", e))?;
    let mut tx = sqlx::Connection::begin(&mut *conn)
        .await
        .map_err(|e| format!("Failed to start transaction: {}", e))?;

    sqlx::query(
        "INSERT INTO recordings
         (id, title, created_at, updated_at, duration_ms, audio_path, folder_path, source, asr_engine, language, status)
         VALUES (?, ?, ?, ?, ?, ?, ?, 'realtime', ?, ?, 'completed')",
    )
    .bind(merged_id)
    .bind(merged_title)
    .bind(first.created_at.0)
    .bind(now)
    .bind(total_ms)
    .bind(merged_audio_path.to_string_lossy().to_string())
    .bind(meeting_folder.to_string_lossy().to_string())
    .bind(&first.asr_engine)
    .bind(&first.language)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Failed to create merged recording: {}", e))?;

    // 5. Copy segments with shifted timestamps. Realtime group (source !=
    //    'offline_asr') gets junction marker segments; the offline group is
    //    copied verbatim (re-transcription regenerates it anyway).
    let mut realtime_json_segments: Vec<crate::api::TranscriptSegment> = Vec::new();
    let mut offline_json_segments: Vec<crate::api::TranscriptSegment> = Vec::new();

    for (idx, source) in sources.iter().enumerate() {
        let offset = offsets_ms[idx];

        let realtime: Vec<_> = source
            .segments
            .iter()
            .filter(|s| s.source.as_deref() != Some("offline_asr"))
            .collect();
        let offline: Vec<_> = source
            .segments
            .iter()
            .filter(|s| s.source.as_deref() == Some("offline_asr"))
            .collect();

        // Junction marker in the realtime group (from the 2nd recording on).
        if idx > 0 {
            let local_time = source
                .recording
                .created_at
                .0
                .with_timezone(&chrono::Local)
                .format("%Y-%m-%d %H:%M")
                .to_string();
            let marker_text = render_marker_text(
                marker_template,
                idx + 1,
                &source.recording.title,
                &local_time,
            );
            let marker_source = realtime
                .first()
                .and_then(|s| s.source.clone())
                .unwrap_or_else(|| "realtime".to_string());
            let marker_id = format!("segment-{}", Uuid::new_v4());
            sqlx::query(
                "INSERT INTO transcript_segments (id, recording_id, text, start_ms, end_ms, speaker, source, translation, created_at)
                 VALUES (?, ?, ?, ?, ?, NULL, ?, '', ?)",
            )
            .bind(&marker_id)
            .bind(merged_id)
            .bind(&marker_text)
            .bind(offset)
            .bind(offset)
            .bind(&marker_source)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Failed to insert marker segment: {}", e))?;

            realtime_json_segments.push(crate::api::TranscriptSegment {
                id: marker_id,
                text: marker_text,
                timestamp: Some(now.to_rfc3339()),
                display_time: None,
                audio_start_time: Some(offset as f64 / 1000.0),
                audio_end_time: Some(offset as f64 / 1000.0),
                duration: Some(0.0),
                speaker: None,
                translation: None,
            });
        }

        for seg in &realtime {
            let new_id = format!("segment-{}", Uuid::new_v4());
            let new_start = seg.start_ms + offset;
            let new_end = seg.end_ms.map(|e| e + offset);
            sqlx::query(
                "INSERT INTO transcript_segments (id, recording_id, text, start_ms, end_ms, speaker, source, translation, created_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(&new_id)
            .bind(merged_id)
            .bind(&seg.text)
            .bind(new_start)
            .bind(new_end)
            .bind(&seg.speaker)
            .bind(&seg.source)
            .bind(&seg.translation)
            .bind(seg.created_at.0)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Failed to copy transcript segment: {}", e))?;

            let start_s = new_start as f64 / 1000.0;
            let end_s = new_end.map(|e| e as f64 / 1000.0);
            realtime_json_segments.push(crate::api::TranscriptSegment {
                id: new_id,
                text: seg.text.clone(),
                timestamp: Some(seg.created_at.0.to_rfc3339()),
                display_time: None,
                audio_start_time: Some(start_s),
                audio_end_time: end_s,
                duration: end_s.map(|e| e - start_s),
                speaker: seg.speaker.clone(),
                translation: Some(seg.translation.clone()).filter(|t| !t.is_empty()),
            });
        }

        for seg in &offline {
            let new_id = format!("segment-{}", Uuid::new_v4());
            let new_start = seg.start_ms + offset;
            let new_end = seg.end_ms.map(|e| e + offset);
            sqlx::query(
                "INSERT INTO transcript_segments (id, recording_id, text, start_ms, end_ms, speaker, source, translation, created_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(&new_id)
            .bind(merged_id)
            .bind(&seg.text)
            .bind(new_start)
            .bind(new_end)
            .bind(&seg.speaker)
            .bind(&seg.source)
            .bind(&seg.translation)
            .bind(seg.created_at.0)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Failed to copy offline transcript segment: {}", e))?;

            let start_s = new_start as f64 / 1000.0;
            let end_s = new_end.map(|e| e as f64 / 1000.0);
            offline_json_segments.push(crate::api::TranscriptSegment {
                id: new_id,
                text: seg.text.clone(),
                timestamp: Some(seg.created_at.0.to_rfc3339()),
                display_time: None,
                audio_start_time: Some(start_s),
                audio_end_time: end_s,
                duration: end_s.map(|e| e - start_s),
                speaker: seg.speaker.clone(),
                translation: Some(seg.translation.clone()).filter(|t| !t.is_empty()),
            });
        }
    }

    tx.commit()
        .await
        .map_err(|e| format!("Failed to commit merge transaction: {}", e))?;

    // Release the pooled connection before the source-deletion phase below —
    // otherwise a pool with max_connections=1 (e.g. in tests) deadlocks.
    // (commit() consumed `tx`, ending its borrow of `conn`.)
    drop(conn);

    // 6. Write transcripts.json / transcripts_offline.json / metadata.json.
    if let Err(e) = super::common::write_transcripts_json(meeting_folder, &realtime_json_segments) {
        warn!("Failed to write merged transcripts.json: {}", e);
    }
    if !offline_json_segments.is_empty() {
        if let Err(e) =
            write_merged_offline_transcripts_json(meeting_folder, &offline_json_segments)
        {
            warn!("Failed to write merged transcripts_offline.json: {}", e);
        }
    }
    let merged_from: Vec<String> = sources.iter().map(|s| s.recording.id.clone()).collect();
    if let Err(e) = write_merge_metadata(
        meeting_folder,
        merged_id,
        merged_title,
        &first.created_at.0.to_rfc3339(),
        total_ms as f64 / 1000.0,
        &merged_from,
    ) {
        warn!("Failed to write merged metadata.json: {}", e);
    }

    // 7. Optionally delete the source recordings (DB row → CASCADE segments,
    //    then the on-disk meeting folder). Folder removal failures only warn.
    if delete_sources {
        for source in sources {
            if let Err(e) = RecordingsRepository::delete_recording(pool, &source.recording.id).await
            {
                warn!(
                    "Failed to delete source recording {} from DB: {}",
                    source.recording.id, e
                );
                continue;
            }
            if let Some(folder) = &source.recording.folder_path {
                let folder_path = PathBuf::from(folder);
                if folder_path.exists() && folder_path != meeting_folder {
                    if let Err(e) = std::fs::remove_dir_all(&folder_path) {
                        warn!(
                            "Failed to remove source recording folder {}: {}",
                            folder_path.display(),
                            e
                        );
                    }
                }
            }
        }
    }

    info!(
        "Merge complete: {} sources → {} ({}ms total)",
        sources.len(),
        merged_id,
        total_ms
    );

    // 8. Return the new recording (same shape as api_get_recording's metadata).
    Ok(serde_json::json!({
        "id": merged_id,
        "title": merged_title,
        "created_at": first.created_at.0.to_rfc3339(),
        "updated_at": now.to_rfc3339(),
        "duration_ms": total_ms,
        "audio_path": merged_audio_path.to_string_lossy(),
        "folder_path": meeting_folder.to_string_lossy(),
        "source": "realtime",
        "asr_engine": first.asr_engine,
        "language": first.language,
        "status": "completed",
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, Utc};
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};

    // ── pure helpers ────────────────────────────────────────────────────────

    #[test]
    fn compute_offsets_accumulates_durations() {
        let (offsets, total) = compute_offsets_ms(&[1000, 2000, 500]);
        assert_eq!(offsets, vec![0, 1000, 3000]);
        assert_eq!(total, 3500);
    }

    #[test]
    fn compute_offsets_empty() {
        let (offsets, total) = compute_offsets_ms(&[]);
        assert!(offsets.is_empty());
        assert_eq!(total, 0);
    }

    #[test]
    fn render_marker_replaces_all_placeholders() {
        let text = render_marker_text(
            "—— 第 {n} 段会议「{title}」，原开始时间 {time} ——",
            2,
            "晨会",
            "2026-01-01 10:00",
        );
        assert_eq!(
            text,
            "—— 第 2 段会议「晨会」，原开始时间 2026-01-01 10:00 ——"
        );
    }

    #[test]
    fn render_marker_without_placeholders_is_unchanged() {
        assert_eq!(render_marker_text("plain", 3, "t", "x"), "plain");
    }

    // ── integration helpers ─────────────────────────────────────────────────

    /// ffmpeg is required to synthesize test audio; tests skip (pass) without it.
    fn ffmpeg_or_skip() -> Option<PathBuf> {
        let path = find_ffmpeg_path();
        if path.is_none() {
            eprintln!("ffmpeg not found, skipping merge integration test");
        }
        path
    }

    /// Create an isolated SQLite pool (tempdir file) with migrations applied.
    async fn test_pool(dir: &Path) -> SqlitePool {
        let options = SqliteConnectOptions::new()
            .filename(dir.join("test.sqlite"))
            .create_if_missing(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .expect("connect test db");
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("run migrations");
        pool
    }

    /// Create a fake meeting folder: 1s sine audio.mp4 + metadata.json +
    /// transcripts.json, mirroring what the recorder writes.
    fn make_meeting_folder(base: &Path, name: &str, freq: u32) -> PathBuf {
        let ffmpeg = find_ffmpeg_path().expect("checked by caller");
        let folder = base.join(name);
        std::fs::create_dir_all(&folder).unwrap();

        let audio = folder.join("audio.mp4");
        let output = std::process::Command::new(ffmpeg)
            .args([
                "-f",
                "lavfi",
                "-i",
                &format!("sine=frequency={}:duration=1", freq),
                "-c:a",
                "aac",
                "-ar",
                "48000",
                "-ac",
                "1",
                "-y",
            ])
            .arg(&audio)
            .output()
            .expect("run ffmpeg sine");
        assert!(
            output.status.success(),
            "ffmpeg sine failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        std::fs::write(
            folder.join("metadata.json"),
            serde_json::to_string_pretty(&serde_json::json!({
                "version": "1.0",
                "meeting_id": null,
                "created_at": "2026-01-01T10:00:00Z",
                "duration_seconds": 1.0,
                "audio_file": "audio.mp4",
                "transcript_file": "transcripts.json",
                "status": "completed",
                "source": "realtime"
            }))
            .unwrap(),
        )
        .unwrap();
        std::fs::write(
            folder.join("transcripts.json"),
            serde_json::to_string_pretty(&serde_json::json!({
                "version": "1.0",
                "segments": [],
                "total_segments": 0
            }))
            .unwrap(),
        )
        .unwrap();
        folder
    }

    /// Insert one recording with 1 realtime + 1 offline segment.
    async fn insert_source_recording(
        pool: &SqlitePool,
        id: &str,
        title: &str,
        created_at: DateTime<Utc>,
        folder: &Path,
    ) {
        sqlx::query(
            "INSERT INTO recordings
             (id, title, created_at, updated_at, duration_ms, audio_path, folder_path, source, asr_engine, language, status)
             VALUES (?, ?, ?, ?, 1000, NULL, ?, 'realtime', 'sense-voice', 'zh', 'completed')",
        )
        .bind(id)
        .bind(title)
        .bind(created_at)
        .bind(created_at)
        .bind(folder.to_string_lossy().to_string())
        .execute(pool)
        .await
        .unwrap();

        // realtime segment: start 100ms, end 900ms
        sqlx::query(
            "INSERT INTO transcript_segments (id, recording_id, text, start_ms, end_ms, speaker, source, created_at)
             VALUES (?, ?, ?, 100, 900, NULL, 'realtime', ?)",
        )
        .bind(format!("{}-rt", id))
        .bind(id)
        .bind(format!("{} 实时段", title))
        .bind(created_at)
        .execute(pool)
        .await
        .unwrap();

        // offline segment: start 0ms, end 1000ms
        sqlx::query(
            "INSERT INTO transcript_segments (id, recording_id, text, start_ms, end_ms, speaker, source, created_at)
             VALUES (?, ?, ?, 0, 1000, NULL, 'offline_asr', ?)",
        )
        .bind(format!("{}-off", id))
        .bind(id)
        .bind(format!("{} 离线段", title))
        .bind(created_at)
        .execute(pool)
        .await
        .unwrap();
    }

    async fn segment_count(pool: &SqlitePool, recording_id: &str, offline: bool) -> i64 {
        let op = if offline { "=" } else { "!=" };
        let sql = format!(
            "SELECT COUNT(*) FROM transcript_segments WHERE recording_id = ? AND source {} 'offline_asr'",
            op
        );
        sqlx::query_scalar::<_, i64>(&sql)
            .bind(recording_id)
            .fetch_one(pool)
            .await
            .unwrap()
    }

    struct MergedIds {
        id: String,
        folder: String,
        duration_ms: i64,
    }

    fn parse_result(value: serde_json::Value) -> MergedIds {
        MergedIds {
            id: value["id"].as_str().unwrap().to_string(),
            folder: value["folder_path"].as_str().unwrap().to_string(),
            duration_ms: value["duration_ms"].as_i64().unwrap(),
        }
    }

    // ── integration tests ───────────────────────────────────────────────────

    #[tokio::test]
    async fn merge_recordings_core_merges_audio_and_segments() {
        if ffmpeg_or_skip().is_none() {
            return;
        }
        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path().join("recordings");
        std::fs::create_dir_all(&base).unwrap();
        let pool = test_pool(tmp.path()).await;

        let t1 = DateTime::parse_from_rfc3339("2026-01-01T10:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let t2 = DateTime::parse_from_rfc3339("2026-01-01T11:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let folder1 = make_meeting_folder(&base, "会议A_2026-01-01_10-00", 440);
        let folder2 = make_meeting_folder(&base, "会议B_2026-01-01_11-00", 550);
        insert_source_recording(&pool, "recording-a", "会议A", t1, &folder1).await;
        insert_source_recording(&pool, "recording-b", "会议B", t2, &folder2).await;

        // Expected offset of the 2nd recording = probed duration of the 1st.
        let dur1_ms =
            (probe_audio_duration(&folder1.join("audio.mp4")).unwrap() * 1000.0).round() as i64;

        let result = merge_recordings_core(
            &pool,
            &base,
            &["recording-b".to_string(), "recording-a".to_string()], // unordered on purpose
            None,
            false,
            Some("MARKER n={n} title={title} time={time}"),
        )
        .await
        .expect("merge should succeed");
        let merged = parse_result(result);

        // New recording row exists, duration ≈ 2 × 1s, keeps 1st created_at.
        let row = RecordingsRepository::get_recording(&pool, &merged.id)
            .await
            .unwrap()
            .expect("merged recording row");
        assert!(
            (merged.duration_ms - dur1_ms * 2).abs() <= 300,
            "duration_ms {} should be ≈ {}",
            merged.duration_ms,
            dur1_ms * 2
        );
        assert_eq!(row.recording.duration_ms, Some(merged.duration_ms));
        assert_eq!(row.recording.created_at.0, t1);
        assert_eq!(row.recording.title, "合并：会议A 等2段");
        assert_eq!(row.recording.source.as_deref(), Some("realtime"));
        assert_eq!(row.recording.status.as_deref(), Some("completed"));

        // Realtime group: 2 copied segments + 1 junction marker; offline: 2.
        assert_eq!(segment_count(&pool, &merged.id, false).await, 3);
        assert_eq!(segment_count(&pool, &merged.id, true).await, 2);

        // The 2nd recording's realtime segment is shifted by ≈ dur1.
        let rt_b = sqlx::query_as::<_, (String, i64, Option<i64>)>(
            "SELECT text, start_ms, end_ms FROM transcript_segments
             WHERE recording_id = ? AND source = 'realtime' AND text LIKE '%会议B%' AND text NOT LIKE 'MARKER%'",
        )
        .bind(&merged.id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(
            (rt_b.1 - (100 + dur1_ms)).abs() <= 2,
            "shifted start_ms {} should be ≈ {}",
            rt_b.1,
            100 + dur1_ms
        );
        assert_eq!(rt_b.2, Some(rt_b.1 + 800));

        // Offline group shifted too.
        let off_b = sqlx::query_scalar::<_, i64>(
            "SELECT start_ms FROM transcript_segments
             WHERE recording_id = ? AND source = 'offline_asr' AND text LIKE '%会议B%'",
        )
        .bind(&merged.id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!((off_b - dur1_ms).abs() <= 2);

        // Junction marker: rendered from template, placed at the boundary.
        let marker = sqlx::query_as::<_, (String, i64, String)>(
            "SELECT text, start_ms, source FROM transcript_segments
             WHERE recording_id = ? AND text LIKE 'MARKER%'",
        )
        .bind(&merged.id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(marker.0.contains("n=2"));
        assert!(marker.0.contains("title=会议B"));
        assert!((marker.1 - dur1_ms).abs() <= 2);
        assert_eq!(marker.2, "realtime"); // 沿用实时组的 source

        // Merged audio exists and is ≈ 2s long.
        let merged_audio = PathBuf::from(&merged.folder).join("audio.mp4");
        assert!(merged_audio.exists());
        let merged_secs = probe_audio_duration(&merged_audio).unwrap();
        assert!(
            (merged_secs - 2.0).abs() <= 0.3,
            "merged audio duration {} should be ≈ 2s",
            merged_secs
        );

        // transcripts.json (realtime group) and metadata.json on disk.
        let transcripts: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(PathBuf::from(&merged.folder).join("transcripts.json"))
                .unwrap(),
        )
        .unwrap();
        assert_eq!(transcripts["total_segments"].as_u64(), Some(3));

        let metadata: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(PathBuf::from(&merged.folder).join("metadata.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(metadata["source"].as_str(), Some("merged"));
        assert_eq!(metadata["status"].as_str(), Some("completed"));
        assert_eq!(metadata["audio_file"].as_str(), Some("audio.mp4"));
        let merged_from = metadata["merged_from"].as_array().unwrap();
        assert_eq!(merged_from.len(), 2);
        // Sorted by created_at: recording-a first even though passed second.
        assert_eq!(merged_from[0].as_str(), Some("recording-a"));

        // Sources preserved (delete_sources = false).
        assert!(RecordingsRepository::get_recording(&pool, "recording-a")
            .await
            .unwrap()
            .is_some());
        assert!(folder1.exists() && folder2.exists());
    }

    #[tokio::test]
    async fn merge_recordings_core_delete_sources_removes_rows_and_folders() {
        if ffmpeg_or_skip().is_none() {
            return;
        }
        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path().join("recordings");
        std::fs::create_dir_all(&base).unwrap();
        let pool = test_pool(tmp.path()).await;

        let t1 = DateTime::parse_from_rfc3339("2026-01-01T10:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let t2 = DateTime::parse_from_rfc3339("2026-01-01T11:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let folder1 = make_meeting_folder(&base, "会议A_2026-01-01_10-00", 440);
        let folder2 = make_meeting_folder(&base, "会议B_2026-01-01_11-00", 550);
        insert_source_recording(&pool, "recording-a", "会议A", t1, &folder1).await;
        insert_source_recording(&pool, "recording-b", "会议B", t2, &folder2).await;

        let result = merge_recordings_core(
            &pool,
            &base,
            &["recording-a".to_string(), "recording-b".to_string()],
            Some("合并测试".to_string()),
            true, // delete_sources
            None, // falls back to DEFAULT_MARKER_TEMPLATE
        )
        .await
        .expect("merge should succeed");
        let merged = parse_result(result);

        assert_eq!(
            sqlx::query_scalar::<_, Option<String>>("SELECT title FROM recordings WHERE id = ?")
                .bind(&merged.id)
                .fetch_one(&pool)
                .await
                .unwrap()
                .as_deref(),
            Some("合并测试")
        );

        // Source recording rows gone; segments cascaded away.
        for id in ["recording-a", "recording-b"] {
            assert!(RecordingsRepository::get_recording(&pool, id)
                .await
                .unwrap()
                .is_none());
            let remaining = sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM transcript_segments WHERE recording_id = ?",
            )
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
            assert_eq!(remaining, 0, "segments of {} should be cascade-deleted", id);
        }

        // Source folders removed from disk; merged folder intact.
        assert!(!folder1.exists());
        assert!(!folder2.exists());
        assert!(PathBuf::from(&merged.folder).join("audio.mp4").exists());
    }

    #[tokio::test]
    async fn merge_recordings_core_rejects_single_recording() {
        let tmp = tempfile::tempdir().unwrap();
        let pool = test_pool(tmp.path()).await;
        let err = merge_recordings_core(
            &pool,
            tmp.path(),
            &["only-one".to_string()],
            None,
            false,
            None,
        )
        .await
        .unwrap_err();
        assert!(err.contains("At least two"));
    }

    #[tokio::test]
    async fn merge_recordings_core_reports_missing_recording() {
        let tmp = tempfile::tempdir().unwrap();
        let pool = test_pool(tmp.path()).await;
        let err = merge_recordings_core(
            &pool,
            tmp.path(),
            &["ghost-1".to_string(), "ghost-2".to_string()],
            None,
            false,
            None,
        )
        .await
        .unwrap_err();
        assert!(err.contains("Recording not found"));
    }
}
