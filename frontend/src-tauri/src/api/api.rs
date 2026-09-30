use log::{debug as log_debug, error as log_error, info as log_info, warn as log_warn};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Runtime};
use uuid::Uuid;

use crate::{
    database::{
        models::{Recording, TranscriptSegment as DbTranscriptSegment},
        repositories::{
            recording::{RecordingWithSegments, RecordingsRepository},
            setting::SettingsRepository,
            transcript_segment::{SearchResult, TranscriptSegmentsRepository},
        },
    },
    state::AppState,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

/// Legacy transcript segment shape used for the transcripts.json file contract
/// (audio import / offline retranscription flows) and by `audio::common`.
/// Database persistence uses `database::models::TranscriptSegment` instead.
#[derive(Debug, Serialize, Deserialize)]
pub struct TranscriptSegment {
    pub id: String,
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
    #[serde(
        default,
        rename = "display_time",
        skip_serializing_if = "Option::is_none"
    )]
    pub display_time: Option<String>,
    // Recording-relative timestamps for playback synchronization
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_start_time: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_end_time: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<f64>,
    /// 说话人匿名编号（"1"/"2"/…；离线识别开启说话人分离时填充）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub speaker: Option<String>,
    /// 段落最终译文（实时内嵌翻译随段落持久化；无译文时为 None）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub translation: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RecordingListItem {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder_path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RecordingMetadata {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
    pub duration_ms: Option<i64>,
    pub audio_path: Option<String>,
    pub folder_path: Option<String>,
    pub source: Option<String>,
    pub asr_engine: Option<String>,
    pub language: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RecordingDetails {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
    pub duration_ms: Option<i64>,
    pub audio_path: Option<String>,
    pub folder_path: Option<String>,
    pub source: Option<String>,
    pub asr_engine: Option<String>,
    pub language: Option<String>,
    pub status: Option<String>,
    pub segments: Vec<RecordingSegment>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RecordingSegment {
    pub id: String,
    pub text: String,
    pub start_ms: i64,
    pub end_ms: Option<i64>,
    pub speaker: Option<String>,
    pub source: Option<String>,
    /// 段落最终译文（空串 = 无译文）
    pub translation: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteRecordingRequest {
    pub recording_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SaveRecordingTitleRequest {
    pub recording_id: String,
    pub title: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SaveRecordingRequest {
    pub recording_title: String,
    pub segments: Vec<RecordingSegmentInput>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RecordingSegmentInput {
    pub id: String,
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
    #[serde(
        default,
        rename = "display_time",
        skip_serializing_if = "Option::is_none"
    )]
    pub display_time: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_ms: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_ms: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub speaker: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// 段落最终译文（前端合并段落时从实时翻译 store 带出）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub translation: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelConfig {
    pub provider: String,
    pub model: String,
    #[serde(rename = "whisperModel")]
    pub whisper_model: String,
    #[serde(rename = "apiKey")]
    pub api_key: Option<String>,
    #[serde(rename = "ollamaEndpoint")]
    pub ollama_endpoint: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TranscriptConfig {
    pub provider: String,
    pub model: String,
    #[serde(rename = "apiKey")]
    pub api_key: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaginatedSegmentsResponse {
    pub segments: Vec<RecordingSegment>,
    pub total_count: i64,
    pub has_more: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchTranscriptsResponse {
    pub results: Vec<SearchTranscriptResult>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchTranscriptResult {
    pub id: String,
    pub recording_id: String,
    pub title: String,
    pub text: String,
    pub start_ms: i64,
}

// API Commands for Tauri

#[tauri::command]
pub async fn api_get_recordings<R: Runtime>(
    _app: AppHandle<R>,
    state: tauri::State<'_, AppState>,
    auth_token: Option<String>,
) -> Result<Vec<RecordingListItem>, String> {
    log_info!(
        "api_get_recordings called with auth_token(native): {}",
        auth_token.is_some()
    );
    let pool = state.db_manager.pool();
    let recordings: Result<Vec<Recording>, sqlx::Error> =
        RecordingsRepository::get_recordings(pool).await;

    match recordings {
        Ok(recording_models) => {
            log_info!("Successfully got {} recordings", recording_models.len());

            let result: Vec<RecordingListItem> = recording_models
                .into_iter()
                .map(|r| RecordingListItem {
                    id: r.id,
                    title: r.title,
                    created_at: r.created_at.0.to_rfc3339(),
                    updated_at: r.updated_at.0.to_rfc3339(),
                    folder_path: r.folder_path,
                })
                .collect();
            Ok(result)
        }
        Err(e) => {
            log_error!("Error getting recordings: {}", e);
            Err(e.to_string())
        }
    }
}

#[tauri::command]
pub async fn api_get_model_config<R: Runtime>(
    _app: AppHandle<R>,
    state: tauri::State<'_, AppState>,
    _auth_token: Option<String>,
) -> Result<Option<ModelConfig>, String> {
    log_info!("api_get_model_config called (native)");
    let pool = state.db_manager.pool();

    let provider = SettingsRepository::get(pool, "model.provider")
        .await
        .map_err(|e| e.to_string())?
        .unwrap_or_else(|| "sherpaonnx".to_string());
    let model = SettingsRepository::get(pool, "model.name")
        .await
        .map_err(|e| e.to_string())?
        .unwrap_or_else(|| "sense-voice".to_string());
    let whisper_model = SettingsRepository::get(pool, "model.whisper_model")
        .await
        .map_err(|e| e.to_string())?
        .unwrap_or_else(|| "base".to_string());
    let api_key = SettingsRepository::get(pool, "model.api_key")
        .await
        .map_err(|e| e.to_string())?;
    let ollama_endpoint = SettingsRepository::get(pool, "model.ollama_endpoint")
        .await
        .map_err(|e| e.to_string())?;

    Ok(Some(ModelConfig {
        provider,
        model,
        whisper_model,
        api_key,
        ollama_endpoint,
    }))
}

#[tauri::command]
pub async fn api_save_model_config<R: Runtime>(
    _app: AppHandle<R>,
    state: tauri::State<'_, AppState>,
    provider: String,
    model: String,
    whisper_model: String,
    api_key: Option<String>,
    ollama_endpoint: Option<String>,
    _auth_token: Option<String>,
) -> Result<serde_json::Value, String> {
    log_info!(
        "api_save_model_config called (native): provider='{}', model='{}', whisperModel='{}', ollamaEndpoint={:?}",
        &provider,
        &model,
        &whisper_model,
        &ollama_endpoint
    );
    let pool = state.db_manager.pool();

    SettingsRepository::set(pool, "model.provider", &provider)
        .await
        .map_err(|e| e.to_string())?;
    SettingsRepository::set(pool, "model.name", &model)
        .await
        .map_err(|e| e.to_string())?;
    SettingsRepository::set(pool, "model.whisper_model", &whisper_model)
        .await
        .map_err(|e| e.to_string())?;
    if let Some(endpoint) = ollama_endpoint.as_deref() {
        SettingsRepository::set(pool, "model.ollama_endpoint", endpoint)
            .await
            .map_err(|e| e.to_string())?;
    }
    if let Some(key) = api_key.as_deref() {
        if !key.is_empty() {
            SettingsRepository::set(pool, "model.api_key", key)
                .await
                .map_err(|e| e.to_string())?;
        }
    }

    log_info!("Successfully saved model configuration to settings");
    Ok(serde_json::json!({
        "status": "success",
        "message": "Model configuration saved successfully"
    }))
}

#[tauri::command]
pub async fn api_delete_recording<R: Runtime>(
    _app: AppHandle<R>,
    state: tauri::State<'_, AppState>,
    recording_id: String,
    auth_token: Option<String>,
) -> Result<serde_json::Value, String> {
    log_info!(
        "api_delete_recording called for recording_id(native): {}, auth_token: {}",
        recording_id,
        auth_token.is_some()
    );

    let pool = state.db_manager.pool();

    match RecordingsRepository::delete_recording(pool, &recording_id).await {
        Ok(true) => {
            log_info!("Successfully deleted recording {}", recording_id);
            Ok(serde_json::json!({
                "status": "success",
                "message": "Recording deleted successfully"
            }))
        }
        Ok(false) => {
            log_warn!("Recording not found or already deleted: {}", recording_id);
            Err(format!(
                "Recording not found or could not be deleted: {}",
                recording_id
            ))
        }
        Err(e) => {
            log_error!("Error deleting recording {}: {}", recording_id, e);
            Err(format!("Failed to delete recording: {}", e))
        }
    }
}

#[tauri::command]
pub async fn api_get_recording<R: Runtime>(
    _app: AppHandle<R>,
    recording_id: String,
    state: tauri::State<'_, AppState>,
    auth_token: Option<String>,
) -> Result<RecordingDetails, String> {
    log_info!(
        "api_get_recording called(native) for recording_id: {}, auth_token: {}",
        recording_id,
        auth_token.is_some()
    );

    let pool = state.db_manager.pool();

    match RecordingsRepository::get_recording(pool, &recording_id).await {
        Ok(Some(RecordingWithSegments {
            recording,
            segments,
        })) => {
            log_info!("Successfully retrieved recording {}", recording_id);
            Ok(RecordingDetails {
                id: recording.id,
                title: recording.title,
                created_at: recording.created_at.0.to_rfc3339(),
                updated_at: recording.updated_at.0.to_rfc3339(),
                duration_ms: recording.duration_ms,
                audio_path: recording.audio_path,
                folder_path: recording.folder_path,
                source: recording.source,
                asr_engine: recording.asr_engine,
                language: recording.language,
                status: recording.status,
                segments: segments
                    .into_iter()
                    .map(|s| RecordingSegment {
                        id: s.id,
                        text: s.text,
                        start_ms: s.start_ms,
                        end_ms: s.end_ms,
                        speaker: s.speaker,
                        source: s.source,
                        translation: s.translation,
                    })
                    .collect(),
            })
        }
        Ok(None) => {
            log_warn!("Recording not found: {}", recording_id);
            Err(format!("Recording not found: {}", recording_id))
        }
        Err(e) => {
            log_error!("Error retrieving recording {}: {}", recording_id, e);
            Err(format!("Failed to retrieve recording: {}", e))
        }
    }
}

#[tauri::command]
pub async fn api_get_recording_metadata<R: Runtime>(
    _app: AppHandle<R>,
    recording_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<RecordingMetadata, String> {
    log_info!(
        "api_get_recording_metadata called for recording_id: {}",
        recording_id
    );

    let pool = state.db_manager.pool();

    match RecordingsRepository::get_recording(pool, &recording_id).await {
        Ok(Some(RecordingWithSegments { recording, .. })) => {
            log_info!("Successfully retrieved recording metadata {}", recording_id);
            Ok(RecordingMetadata {
                id: recording.id,
                title: recording.title,
                created_at: recording.created_at.0.to_rfc3339(),
                updated_at: recording.updated_at.0.to_rfc3339(),
                duration_ms: recording.duration_ms,
                audio_path: recording.audio_path,
                folder_path: recording.folder_path,
                source: recording.source,
                asr_engine: recording.asr_engine,
                language: recording.language,
                status: recording.status,
            })
        }
        Ok(None) => {
            log_warn!("Recording not found: {}", recording_id);
            Err(format!("Recording not found: {}", recording_id))
        }
        Err(e) => {
            log_error!(
                "Error retrieving recording metadata {}: {}",
                recording_id,
                e
            );
            Err(format!("Failed to retrieve recording metadata: {}", e))
        }
    }
}

#[tauri::command]
pub async fn api_get_recording_segments<R: Runtime>(
    _app: AppHandle<R>,
    recording_id: String,
    limit: i64,
    offset: i64,
    source: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<PaginatedSegmentsResponse, String> {
    log_info!(
        "api_get_recording_segments called for recording_id: {}, limit: {}, offset: {}, source: {:?}",
        recording_id,
        limit,
        offset,
        source
    );

    let pool = state.db_manager.pool();

    let segments = TranscriptSegmentsRepository::get_segments_by_recording(pool, &recording_id)
        .await
        .map_err(|e| {
            log_error!(
                "Error retrieving segments for recording {}: {}",
                recording_id,
                e
            );
            format!("Failed to retrieve segments: {}", e)
        })?;

    let filtered: Vec<RecordingSegment> = segments
        .into_iter()
        .filter(|s| {
            source
                .as_ref()
                .map(|src| s.source.as_deref() == Some(src.as_str()))
                .unwrap_or(true)
        })
        .map(|s| RecordingSegment {
            id: s.id,
            text: s.text,
            start_ms: s.start_ms,
            end_ms: s.end_ms,
            speaker: s.speaker,
            source: s.source,
            translation: s.translation,
        })
        .collect();

    let total_count = filtered.len() as i64;
    let paginated: Vec<RecordingSegment> = filtered
        .into_iter()
        .skip(offset.max(0) as usize)
        .take(limit.max(0) as usize)
        .collect();

    let has_more = (offset + paginated.len() as i64) < total_count;

    Ok(PaginatedSegmentsResponse {
        segments: paginated,
        total_count,
        has_more,
    })
}

#[tauri::command]
pub async fn api_save_recording_title<R: Runtime>(
    _app: AppHandle<R>,
    state: tauri::State<'_, AppState>,
    recording_id: String,
    title: String,
    auth_token: Option<String>,
) -> Result<serde_json::Value, String> {
    log_info!(
        "api_save_recording_title called for recording_id: {}, auth_token: {}",
        recording_id,
        auth_token.is_some()
    );
    let pool = state.db_manager.pool();
    match RecordingsRepository::update_recording_title(pool, &recording_id, &title).await {
        Ok(true) => {
            log_info!("Successfully saved recording title");
            Ok(serde_json::json!({"message": "Recording title saved successfully"}))
        }
        Ok(false) => {
            log_error!("No recording found with id {}", recording_id);
            Err(format!("No recording found with id {}", recording_id))
        }
        Err(e) => {
            log_error!("Failed to update recording {}", e);
            Err(format!("Failed to update recording: {}", e))
        }
    }
}

#[tauri::command]
pub async fn api_save_transcript<R: Runtime>(
    _app: AppHandle<R>,
    state: tauri::State<'_, AppState>,
    recording_title: String,
    segments: Vec<serde_json::Value>,
    folder_path: Option<String>,
    auth_token: Option<String>,
) -> Result<serde_json::Value, String> {
    log_info!(
        "api_save_transcript called for recording: {}, segments: {}, folder_path: {:?}, auth_token: {}",
        recording_title,
        segments.len(),
        folder_path,
        auth_token.is_some()
    );

    if let Some(first) = segments.first() {
        log_debug!(
            "First segment data: {}",
            serde_json::to_string_pretty(first).unwrap_or_default()
        );
    }

    let segments_to_save: Vec<RecordingSegmentInput> = segments
        .into_iter()
        .map(serde_json::from_value)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| {
            log_error!("Failed to parse transcript segments: {}", e);
            format!(
                "Invalid transcript data format: {}. Please check the data structure.",
                e
            )
        })?;

    if let Some(first_seg) = segments_to_save.first() {
        log_debug!(
            "First parsed segment: text='{}', start_ms={:?}, end_ms={:?}",
            first_seg.text.chars().take(50).collect::<String>(),
            first_seg.start_ms,
            first_seg.end_ms
        );
    }

    let pool = state.db_manager.pool();

    // 复用同一文件夹已有的历史行（2026-09-24）：Rust 在停止录音时已经写过一行
    // （保证「不在录音页时从托盘停止」也有历史记录），前端这次是**按段落合并**的
    // 增强版本，必须写回同一行，否则历史列表会出现两条同名记录。
    let existing = match folder_path.as_deref().filter(|p| !p.is_empty()) {
        Some(folder) => RecordingsRepository::get_by_folder_path(pool, folder).await.unwrap_or(None),
        None => None,
    };
    let recording_id = match existing {
        Some(rec) => {
            let _ =
                RecordingsRepository::update_recording_title(pool, &rec.id, &recording_title).await;
            log_info!(
                "api_save_transcript: reusing existing recording {} for folder {:?}",
                rec.id,
                folder_path
            );
            rec.id
        }
        None => RecordingsRepository::create_recording(
            pool,
            &recording_title,
            None,
            None,
            None,
            None,
            folder_path.as_deref(),
        )
        .await
        .map_err(|e| {
            log_error!("Error creating recording for '{}': {}", recording_title, e);
            format!("Failed to create recording: {}", e)
        })?,
    };

    let db_segments: Vec<DbTranscriptSegment> = segments_to_save
        .into_iter()
        .map(|s| DbTranscriptSegment {
            id: if s.id.is_empty() {
                format!("segment-{}", Uuid::new_v4())
            } else {
                s.id
            },
            recording_id: recording_id.clone(),
            text: s.text,
            start_ms: s.start_ms.unwrap_or(0),
            end_ms: s.end_ms,
            speaker: s.speaker,
            source: s.source,
            translation: s.translation.unwrap_or_default(),
            created_at: crate::database::models::DateTimeUtc(chrono::Utc::now()),
        })
        .collect();

    // 整体替换（而不是裸 INSERT）：Rust 停止时写的原始段落会被这里覆盖成合并版本
    TranscriptSegmentsRepository::replace_segments(pool, &recording_id, &db_segments)
        .await
        .map_err(|e| {
            log_error!("Error saving segments for '{}': {}", recording_title, e);
            format!("Failed to save segments: {}", e)
        })?;

    // Persist recording duration: prefer the authoritative value written by the
    // recorder into metadata.json; fall back to the last segment's end time.
    let duration_ms = folder_path
        .as_deref()
        .and_then(|fp| {
            let meta_path = std::path::Path::new(fp).join("metadata.json");
            let content = std::fs::read_to_string(meta_path).ok()?;
            let json: serde_json::Value = serde_json::from_str(&content).ok()?;
            json.get("duration_seconds")?
                .as_f64()
                .map(|s| (s * 1000.0) as i64)
        })
        .or_else(|| db_segments.iter().filter_map(|s| s.end_ms).max());
    if let Some(d) = duration_ms {
        if let Err(e) =
            RecordingsRepository::update_recording_duration(pool, &recording_id, d).await
        {
            log_warn!(
                "Failed to update duration for recording {}: {}",
                recording_id,
                e
            );
        }
    }

    log_info!(
        "Successfully saved transcript and created recording with id: {}",
        recording_id
    );
    Ok(serde_json::json!({
        "status": "success",
        "message": "Transcript saved successfully",
        "recording_id": recording_id
    }))
}

#[tauri::command]
pub async fn api_search_transcripts<R: Runtime>(
    _app: AppHandle<R>,
    state: tauri::State<'_, AppState>,
    query: String,
) -> Result<SearchTranscriptsResponse, String> {
    log_info!("api_search_transcripts called with query: {}", query);

    let pool = state.db_manager.pool();
    let results: Vec<SearchResult> = TranscriptSegmentsRepository::search_segments(pool, &query)
        .await
        .map_err(|e| {
            log_error!("Error searching transcripts: {}", e);
            format!("Failed to search transcripts: {}", e)
        })?;

    Ok(SearchTranscriptsResponse {
        results: results
            .into_iter()
            .map(|r| SearchTranscriptResult {
                id: r.id,
                recording_id: r.recording_id,
                title: r.title,
                text: r.text,
                start_ms: r.start_ms,
            })
            .collect(),
    })
}

#[tauri::command]
pub async fn api_get_transcript_config<R: Runtime>(
    _app: AppHandle<R>,
    state: tauri::State<'_, AppState>,
    _auth_token: Option<String>,
) -> Result<Option<TranscriptConfig>, String> {
    log_info!("api_get_transcript_config called (native)");
    let pool = state.db_manager.pool();

    let provider = SettingsRepository::get(pool, "transcript.provider")
        .await
        .map_err(|e| e.to_string())?
        .unwrap_or_else(|| "sherpaonnx".to_string());
    let model = SettingsRepository::get(pool, "transcript.model")
        .await
        .map_err(|e| e.to_string())?
        .unwrap_or_else(|| "sense-voice".to_string());
    let api_key = SettingsRepository::get(pool, "transcript.api_key")
        .await
        .map_err(|e| e.to_string())?;

    Ok(Some(TranscriptConfig {
        provider,
        model,
        api_key,
    }))
}

#[tauri::command]
pub async fn api_save_transcript_config<R: Runtime>(
    _app: AppHandle<R>,
    state: tauri::State<'_, AppState>,
    provider: String,
    model: String,
    api_key: Option<String>,
    _auth_token: Option<String>,
    remote_asr_model: Option<String>,
) -> Result<serde_json::Value, String> {
    log_info!(
        "api_save_transcript_config called (native): provider='{}', model='{}'",
        &provider,
        &model
    );

    // 远程 ASR 模型竞态修复（2026-09-28）：前端把「当前真实选择」随保存一并传来，
    // 后端以此为权威同步 REMOTE_ASR_MODEL 并落盘 —— 不依赖模型选择器先前的异步持久化
    // （用户刚切完模型就点开始录音时，异步落盘可能还没完成，引擎会拿旧模型跑整场录音；
    // 生产实测 UI 显示 qwen、实际跑了 4.8 分钟 doubao）。占位名/空值不动现有选择（坑 #40）。
    let is_remote_provider =
        provider == "remote-qwen3-asr" || model.starts_with("qwen3-asr-remote");
    if is_remote_provider {
        if let Some(explicit) = remote_asr_model.as_deref() {
            crate::audio::transcription::apply_explicit_remote_asr_model(explicit);
        }
    }

    let pool = state.db_manager.pool();

    SettingsRepository::set(pool, "transcript.provider", &provider)
        .await
        .map_err(|e| e.to_string())?;
    SettingsRepository::set(pool, "transcript.model", &model)
        .await
        .map_err(|e| e.to_string())?;
    if let Some(key) = api_key.as_deref() {
        if !key.is_empty() {
            SettingsRepository::set(pool, "transcript.api_key", key)
                .await
                .map_err(|e| e.to_string())?;
        }
    }

    log_info!("Successfully saved transcript configuration to settings");
    Ok(serde_json::json!({
        "status": "success",
        "message": "Transcript configuration saved successfully"
    }))
}

#[tauri::command]
pub async fn api_get_api_key<R: Runtime>(
    _app: AppHandle<R>,
    _state: tauri::State<'_, AppState>,
    _provider: Option<String>,
    _auth_token: Option<String>,
) -> Result<Option<String>, String> {
    log_info!("api_get_api_key called (native) - returning None");
    Ok(None)
}

#[tauri::command]
pub async fn api_get_transcript_api_key<R: Runtime>(
    _app: AppHandle<R>,
    _state: tauri::State<'_, AppState>,
    _provider: Option<String>,
    _auth_token: Option<String>,
) -> Result<Option<String>, String> {
    log_info!("api_get_transcript_api_key called (native) - returning None");
    Ok(None)
}

/// Opens the recording's folder in the system file explorer
#[tauri::command]
pub async fn open_recording_folder<R: Runtime>(
    _app: AppHandle<R>,
    state: tauri::State<'_, AppState>,
    recording_id: String,
) -> Result<(), String> {
    log_info!(
        "open_recording_folder called for recording_id: {}",
        recording_id
    );

    let pool = state.db_manager.pool();

    let recording: Option<Recording> = sqlx::query_as(
        "SELECT id, title, created_at, updated_at, duration_ms, audio_path, folder_path, source, asr_engine, language, status FROM recordings WHERE id = ?",
    )
    .bind(&recording_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("Database error: {}", e))?;

    match recording {
        Some(r) => {
            if let Some(folder_path) = r.folder_path {
                log_info!("Opening recording folder: {}", folder_path);

                let path = std::path::Path::new(&folder_path);
                if !path.exists() {
                    log_warn!("Folder path does not exist: {}", folder_path);
                    return Err(format!("Recording folder not found: {}", folder_path));
                }

                #[cfg(target_os = "macos")]
                {
                    std::process::Command::new("open")
                        .arg(&folder_path)
                        .spawn()
                        .map_err(|e| format!("Failed to open folder: {}", e))?;
                }

                #[cfg(target_os = "windows")]
                {
                    std::process::Command::new("explorer")
                        .arg(&folder_path)
                        .spawn()
                        .map_err(|e| format!("Failed to open folder: {}", e))?;
                }

                #[cfg(target_os = "linux")]
                {
                    std::process::Command::new("xdg-open")
                        .arg(&folder_path)
                        .spawn()
                        .map_err(|e| format!("Failed to open folder: {}", e))?;
                }

                log_info!("Successfully opened folder: {}", folder_path);
                Ok(())
            } else {
                log_warn!("Recording {} has no folder_path set", recording_id);
                Err("Recording folder path not available for this recording".to_string())
            }
        }
        None => {
            log_warn!("Recording not found: {}", recording_id);
            Err("Recording not found".to_string())
        }
    }
}

/// 用系统默认程序打开 URL。
///
/// Windows 必须走 `ShellExecuteW`（shell32），**不能**用 `cmd /C start <url>`：
/// cmd 会把 URL 里的 `&` 当成命令分隔符 —— 推送消息/更新公告里的链接普遍带查询串，
/// 结果是「点了没反应」甚至执行半条命令（把远端下发的字符串当命令跑，安全面也不可接受），
/// 而且 `cmd.exe` 还会闪一个黑框。ShellExecuteW 把 URL 作为参数直接交给系统 shell，
/// 不做二次解析，是 Windows 上的标准做法。
#[cfg(target_os = "windows")]
fn open_url_with_system(url: &str) -> Result<(), String> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    #[link(name = "shell32")]
    extern "system" {
        fn ShellExecuteW(
            hwnd: *mut std::ffi::c_void,
            lp_operation: *const u16,
            lp_file: *const u16,
            lp_parameters: *const u16,
            lp_directory: *const u16,
            n_show_cmd: i32,
        ) -> *mut std::ffi::c_void;
    }

    /// ShellExecuteW 的 nShowCmd：正常显示窗口
    const SW_SHOWNORMAL: i32 = 1;

    let to_wide = |s: &str| -> Vec<u16> {
        OsStr::new(s)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    };
    let operation = to_wide("open");
    let file = to_wide(url);

    // SAFETY: 两个宽字符串都以 NUL 结尾，且在整个调用期间保持存活；
    // 其余参数按 MSDN 允许传 NULL。
    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            operation.as_ptr(),
            file.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        )
    } as isize;

    // MSDN：返回值 <= 32 表示失败（各错误码含义见 ShellExecuteW 文档）
    if result <= 32 {
        Err(format!("系统无法打开链接（ShellExecuteW 返回 {result}）"))
    } else {
        Ok(())
    }
}

/// macOS：`open`；Linux：`xdg-open`。
/// 参数以 argv 直接传递（不经 shell），URL 里的 `&`/空格不会被二次解析。
#[cfg(not(target_os = "windows"))]
fn open_url_with_system(url: &str) -> Result<(), String> {
    use std::process::Command;

    let program = if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    let status = Command::new(program)
        .arg(url)
        .status()
        .map_err(|e| format!("Failed to open URL: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{program} 退出码非 0: {status}"))
    }
}

#[tauri::command]
pub async fn open_external_url(url: String) -> Result<(), String> {
    let url = url.trim();
    if url.is_empty() {
        return Err("链接为空".to_string());
    }
    open_url_with_system(url)
}

// ── Export ────────────────────────────────────────────────────────────────────

/// 导出目录解析顺序：
/// 1. explicit `output_dir` argument
/// 2. `export.default_dir` setting
/// 3. the recording's own `folder_path`
async fn resolve_export_dir(
    pool: &sqlx::SqlitePool,
    output_dir: Option<String>,
    folder_path: Option<&str>,
) -> Result<String, String> {
    match output_dir.filter(|d| !d.trim().is_empty()) {
        Some(d) => Ok(d),
        None => {
            let setting_dir = SettingsRepository::get_export_dir(pool)
                .await
                .map_err(|e| e.to_string())?;
            if !setting_dir.trim().is_empty() {
                Ok(setting_dir)
            } else {
                folder_path
                    .filter(|f| !f.trim().is_empty())
                    .map(|f| f.to_string())
                    .ok_or_else(|| {
                        "No export directory available: set one in Settings or pass output_dir"
                            .to_string()
                    })
            }
        }
    }
}

/// Export a recording's transcript to TXT / SRT / Markdown and return the
/// path of the written file.
///
/// Output directory resolution order: see `resolve_export_dir`.
#[tauri::command]
pub async fn api_export_recording<R: Runtime>(
    _app: AppHandle<R>,
    state: tauri::State<'_, AppState>,
    recording_id: String,
    format: String,
    source: Option<String>,
    output_dir: Option<String>,
) -> Result<serde_json::Value, String> {
    log_info!(
        "api_export_recording called: id={}, format={}, source={:?}, output_dir={:?}",
        recording_id,
        format,
        source,
        output_dir
    );

    let pool = state.db_manager.pool();
    let RecordingWithSegments {
        recording,
        segments,
    } = RecordingsRepository::get_recording(pool, &recording_id)
        .await
        .map_err(|e| format!("Failed to load recording: {}", e))?
        .ok_or_else(|| format!("Recording not found: {}", recording_id))?;

    // Filter by result source: 'realtime' = everything except offline_asr,
    // 'offline_asr' = only offline re-transcription segments, None = all.
    let segments: Vec<_> = match source.as_deref() {
        Some("realtime") => segments
            .into_iter()
            .filter(|s| s.source.as_deref() != Some("offline_asr"))
            .collect(),
        Some("offline_asr") => segments
            .into_iter()
            .filter(|s| s.source.as_deref() == Some("offline_asr"))
            .collect(),
        _ => segments.into_iter().collect(),
    };

    if segments.is_empty() {
        return Err("No segments to export for the selected source".to_string());
    }

    let normalized = format.to_lowercase();
    let (content, ext) = match normalized.as_str() {
        "txt" => (render_txt(&recording, &segments), "txt"),
        "srt" => (render_srt(&segments), "srt"),
        "markdown" | "md" => (render_markdown(&recording, &segments), "md"),
        other => {
            return Err(format!(
                "Unsupported export format: {}. Use txt, srt or markdown.",
                other
            ))
        }
    };

    let dir = resolve_export_dir(pool, output_dir, recording.folder_path.as_deref()).await?;

    let dir_path = std::path::Path::new(&dir);
    std::fs::create_dir_all(dir_path)
        .map_err(|e| format!("Failed to create export directory: {}", e))?;

    // Suffix per source so realtime / offline exports don't overwrite each other.
    let source_suffix = match source.as_deref() {
        Some("realtime") => "_realtime",
        Some("offline_asr") => "_offline",
        _ => "",
    };
    let file_name = format!(
        "{}{}.{}",
        sanitize_filename(&recording.title),
        source_suffix,
        ext
    );
    let file_path = dir_path.join(&file_name);
    std::fs::write(&file_path, content)
        .map_err(|e| format!("Failed to write export file: {}", e))?;

    log_info!(
        "Exported recording {} to {}",
        recording_id,
        file_path.display()
    );
    Ok(serde_json::json!({
        "status": "success",
        "path": file_path.to_string_lossy(),
    }))
}

/// 将 AI 总结内容导出为 Markdown 文件（`<标题>_AI总结.md`），返回写入的文件路径。
/// 输出目录解析顺序与 api_export_recording 相同（见 `resolve_export_dir`）。
#[tauri::command]
pub async fn summary_export_markdown<R: Runtime>(
    _app: AppHandle<R>,
    state: tauri::State<'_, AppState>,
    recording_id: String,
    content: String,
    output_dir: Option<String>,
) -> Result<String, String> {
    let pool = state.db_manager.pool();
    let recording = RecordingsRepository::get_recording(pool, &recording_id)
        .await
        .map_err(|e| format!("Failed to load recording: {}", e))?
        .ok_or_else(|| format!("Recording not found: {}", recording_id))?
        .recording;

    let dir = resolve_export_dir(pool, output_dir, recording.folder_path.as_deref()).await?;
    let dir_path = std::path::Path::new(&dir);
    std::fs::create_dir_all(dir_path)
        .map_err(|e| format!("Failed to create export directory: {}", e))?;

    let file_name = format!("{}_AI总结.md", sanitize_filename(&recording.title));
    let file_path = dir_path.join(&file_name);
    std::fs::write(&file_path, content)
        .map_err(|e| format!("Failed to write summary export: {}", e))?;

    log_info!(
        "Exported summary for {} to {}",
        recording_id,
        file_path.display()
    );
    Ok(file_path.to_string_lossy().to_string())
}

/// `mm:ss` (or `hh:mm:ss` past one hour) for TXT / Markdown timestamps.
fn ms_to_clock(ms: i64) -> String {
    let total_seconds = ms.max(0) / 1000;
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;
    if hours > 0 {
        format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
    } else {
        format!("{:02}:{:02}", minutes, seconds)
    }
}

/// `hh:mm:ss,mmm` for SRT cues.
fn ms_to_srt(ms: i64) -> String {
    let ms = ms.max(0);
    format!(
        "{:02}:{:02}:{:02},{:03}",
        ms / 3_600_000,
        (ms % 3_600_000) / 60_000,
        (ms % 60_000) / 1000,
        ms % 1000
    )
}

/// Windows 保留设备名判断：CON / PRN / AUX / NUL / COM1-9 / LPT1-9。
/// 这些名字（**含带扩展名的形式**，如 `CON.txt`）不能作为文件名，
/// 创建时会报「文件名、目录名或卷标语法不正确」。判定只看第一个点之前的部分。
fn is_windows_reserved_name(name: &str) -> bool {
    let base = name.split('.').next().unwrap_or(name).trim();
    let upper = base.to_ascii_uppercase();
    if matches!(upper.as_str(), "CON" | "PRN" | "AUX" | "NUL") {
        return true;
    }
    // COM1..COM9 / LPT1..LPT9（COM0/LPT0 不是保留名）
    upper.len() == 4
        && (upper.starts_with("COM") || upper.starts_with("LPT"))
        && matches!(upper.as_bytes()[3], b'1'..=b'9')
}

fn sanitize_filename(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect();
    let trim_edge = |s: &str| -> String {
        // Windows 不允许文件名以空格或点结尾（资源管理器会静默截断，
        // 表现为「导出成功但按标题找不到文件」）
        s.trim().trim_end_matches(|c| c == '.' || c == ' ').to_string()
    };
    let trimmed = trim_edge(&cleaned);
    let mut stem: String = if trimmed.is_empty() {
        "recording".to_string()
    } else {
        trimmed.chars().take(80).collect()
    };
    // 80 字截断后可能又以点/空格结尾，再清一次
    stem = trim_edge(&stem);
    if stem.is_empty() {
        stem = "recording".to_string();
    }
    // Windows 保留设备名兜底：加下划线前缀（macOS/Linux 上加前缀也无害，保持跨平台一致）
    if is_windows_reserved_name(&stem) {
        stem.insert(0, '_');
    }
    stem
}

fn render_txt(recording: &Recording, segments: &[DbTranscriptSegment]) -> String {
    let mut out = format!("{}\n\n", recording.title);
    for s in segments {
        match &s.speaker {
            Some(sp) if !sp.is_empty() => out.push_str(&format!(
                "[{}] {}: {}\n",
                ms_to_clock(s.start_ms),
                sp,
                s.text
            )),
            _ => out.push_str(&format!("[{}] {}\n", ms_to_clock(s.start_ms), s.text)),
        }
        if !s.translation.trim().is_empty() {
            out.push_str(&format!("  ↳ {}\n", s.translation));
        }
    }
    out
}

fn render_srt(segments: &[DbTranscriptSegment]) -> String {
    let mut out = String::new();
    for (i, s) in segments.iter().enumerate() {
        // Segments without an end time get a 2s default cue duration.
        let end = s.end_ms.unwrap_or(s.start_ms + 2000);
        // 双语字幕：原文一行 + 译文一行
        let cue_text = if s.translation.trim().is_empty() {
            s.text.clone()
        } else {
            format!("{}\n{}", s.text, s.translation)
        };
        out.push_str(&format!(
            "{}\n{} --> {}\n{}\n\n",
            i + 1,
            ms_to_srt(s.start_ms),
            ms_to_srt(end),
            cue_text
        ));
    }
    out
}

fn render_markdown(recording: &Recording, segments: &[DbTranscriptSegment]) -> String {
    let duration = recording
        .duration_ms
        .map(ms_to_clock)
        .unwrap_or_else(|| "未知".to_string());
    let mut out = format!(
        "# {}\n\n- 创建时间：{}\n- 时长：{}\n\n---\n\n",
        recording.title,
        recording.created_at.0.format("%Y-%m-%d %H:%M:%S"),
        duration
    );
    for s in segments {
        match &s.speaker {
            Some(sp) if !sp.is_empty() => out.push_str(&format!(
                "- **[{}] {}**：{}\n",
                ms_to_clock(s.start_ms),
                sp,
                s.text
            )),
            _ => out.push_str(&format!("- **[{}]** {}\n", ms_to_clock(s.start_ms), s.text)),
        }
        if !s.translation.trim().is_empty() {
            out.push_str(&format!("  > {}\n", s.translation));
        }
    }
    out
}

// ── Segment editing & generic settings ───────────────────────────────────────

// ── 说话人命名（离线识别说话人分离） ──────────────────────────────────────────

/// 读会议文件夹 metadata.json 里记录的「离线识别实际使用的模型」。
#[tauri::command]
pub async fn api_get_retranscribed_model<R: Runtime>(
    _app: AppHandle<R>,
    folder_path: String,
) -> Result<String, String> {
    let path = std::path::PathBuf::from(&folder_path).join("metadata.json");
    if !path.exists() {
        return Ok(String::new());
    }
    let raw = std::fs::read_to_string(&path).map_err(|e| format!("读取元数据失败: {}", e))?;
    let value: serde_json::Value =
        serde_json::from_str(&raw).unwrap_or_else(|_| serde_json::json!({}));
    Ok(value
        .get("retranscribed_model")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string())
}

/// 读会议文件夹 metadata.json 里的离线识别摘要（模型 / 音频时长 / 识别耗时）。
///
/// 2026-09-22 新增：历史页需要在**离开再回来**之后仍能显示「离线识别模型 + 音频时长 +
/// 识别用时」（用户反馈：希望识别完成后能看到音频时间长度和识别耗时）。这些信息由
/// retranscription 写入 metadata.json，这里一次读回。
#[tauri::command]
pub async fn api_get_offline_recognition_info<R: Runtime>(
    _app: AppHandle<R>,
    folder_path: String,
) -> Result<serde_json::Value, String> {
    let path = std::path::PathBuf::from(&folder_path).join("metadata.json");
    if !path.exists() {
        return Ok(serde_json::json!({}));
    }
    let raw = std::fs::read_to_string(&path).map_err(|e| format!("读取元数据失败: {}", e))?;
    let value: serde_json::Value =
        serde_json::from_str(&raw).unwrap_or_else(|_| serde_json::json!({}));
    Ok(serde_json::json!({
        "model": value.get("retranscribed_model").and_then(|v| v.as_str()).unwrap_or(""),
        "duration_seconds": value.get("duration_seconds").and_then(|v| v.as_f64()).unwrap_or(0.0),
        "elapsed_seconds": value.get("retranscribe_elapsed_seconds").and_then(|v| v.as_f64()).unwrap_or(0.0),
        "retranscribed_at": value.get("retranscribed_at").and_then(|v| v.as_str()).unwrap_or(""),
        // 上游告警（内容风控部分拦截 / 档位降级 / 部分分片失败）：离线 tab 需要**持久**
        // 展示它们，因为完成时的 toast 一闪而过，用户回头只看到「结果不完整」。
        "warnings": value.get("retranscription_warnings").cloned().unwrap_or_else(|| serde_json::json!([])),
    }))
}

/// 读会议文件夹里的说话人命名表（speakers.json，{speakerId: 自定义名}）。
#[tauri::command]
pub async fn api_get_speaker_names<R: Runtime>(
    _app: AppHandle<R>,
    folder_path: String,
) -> Result<serde_json::Value, String> {
    let path = std::path::PathBuf::from(&folder_path).join("speakers.json");
    if !path.exists() {
        return Ok(serde_json::json!({}));
    }
    let raw = std::fs::read_to_string(&path).map_err(|e| format!("读取说话人配置失败: {}", e))?;
    let value: serde_json::Value =
        serde_json::from_str(&raw).unwrap_or_else(|_| serde_json::json!({}));
    Ok(value)
}

/// 保存某个说话人的自定义名（name 为空 = 恢复默认「说话人N」）。
#[tauri::command]
pub async fn api_set_speaker_name<R: Runtime>(
    _app: AppHandle<R>,
    folder_path: String,
    speaker_id: String,
    name: String,
) -> Result<(), String> {
    let dir = std::path::PathBuf::from(&folder_path);
    let path = dir.join("speakers.json");
    let mut map: serde_json::Map<String, serde_json::Value> = if path.exists() {
        let raw = std::fs::read_to_string(&path).unwrap_or_default();
        serde_json::from_str(&raw).unwrap_or_default()
    } else {
        serde_json::Map::new()
    };
    let trimmed = name.trim().to_string();
    if trimmed.is_empty() {
        map.remove(&speaker_id);
    } else {
        map.insert(speaker_id, serde_json::json!(trimmed));
    }
    let tmp = dir.join(".speakers.json.tmp");
    std::fs::write(
        &tmp,
        serde_json::to_string_pretty(&map).map_err(|e| e.to_string())?,
    )
    .map_err(|e| format!("写入说话人配置失败: {}", e))?;
    std::fs::rename(&tmp, &path).map_err(|e| format!("保存说话人配置失败: {}", e))?;
    Ok(())
}

/// Update the text of a single transcript segment (used by the history detail
/// page's inline editing).
#[tauri::command]
pub async fn api_update_segment_text<R: Runtime>(
    _app: AppHandle<R>,
    state: tauri::State<'_, AppState>,
    segment_id: String,
    text: String,
) -> Result<serde_json::Value, String> {
    let pool = state.db_manager.pool();
    match TranscriptSegmentsRepository::update_segment_text(pool, &segment_id, &text).await {
        Ok(true) => Ok(serde_json::json!({"status": "success"})),
        Ok(false) => Err(format!("Segment not found: {}", segment_id)),
        Err(e) => Err(format!("Failed to update segment: {}", e)),
    }
}

/// Read all rows of the key/value settings table as a JSON object.
#[tauri::command]
pub async fn api_get_settings<R: Runtime>(
    _app: AppHandle<R>,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let pool = state.db_manager.pool();
    let rows = SettingsRepository::get_all(pool)
        .await
        .map_err(|e| e.to_string())?;
    let map: serde_json::Map<String, serde_json::Value> = rows
        .into_iter()
        .map(|s| (s.key, serde_json::Value::String(s.value)))
        .collect();
    Ok(serde_json::Value::Object(map))
}

/// Save one settings key. `value == null` deletes the key.
#[tauri::command]
pub async fn api_save_setting<R: Runtime>(
    _app: AppHandle<R>,
    state: tauri::State<'_, AppState>,
    key: String,
    value: Option<String>,
) -> Result<serde_json::Value, String> {
    let pool = state.db_manager.pool();
    match value {
        Some(v) => SettingsRepository::set(pool, &key, &v)
            .await
            .map_err(|e| e.to_string())?,
        None => SettingsRepository::delete(pool, &key)
            .await
            .map_err(|e| e.to_string())?,
    }
    Ok(serde_json::json!({"status": "success"}))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::models::DateTimeUtc;

    fn recording() -> Recording {
        Recording {
            id: "rec-1".to_string(),
            title: "周会".to_string(),
            created_at: DateTimeUtc(chrono::Utc::now()),
            updated_at: DateTimeUtc(chrono::Utc::now()),
            duration_ms: Some(60_000),
            audio_path: None,
            folder_path: None,
            source: Some("realtime".to_string()),
            asr_engine: None,
            language: None,
            status: Some("completed".to_string()),
        }
    }

    fn seg(text: &str, translation: &str, start_ms: i64) -> DbTranscriptSegment {
        DbTranscriptSegment {
            id: format!("seg_{}", start_ms),
            recording_id: "rec-1".to_string(),
            text: text.to_string(),
            start_ms,
            end_ms: Some(start_ms + 1500),
            speaker: None,
            source: Some("realtime".to_string()),
            translation: translation.to_string(),
            created_at: DateTimeUtc(chrono::Utc::now()),
        }
    }

    #[test]
    fn render_txt_appends_translation_line() {
        let out = render_txt(
            &recording(),
            &[seg("你好，世界", "Hello, world", 0), seg("没有译文的段", "", 2000)],
        );
        assert!(out.contains("[00:00] 你好，世界\n  ↳ Hello, world\n"));
        assert!(out.contains("[00:02] 没有译文的段\n"));
        assert!(!out.contains("没有译文的段\n  ↳"));
    }

    #[test]
    fn render_srt_bilingual_cue() {
        let out = render_srt(&[seg("你好，世界", "Hello, world", 0), seg("再见", "", 2000)]);
        assert!(out.contains("00:00:00,000 --> 00:00:01,500\n你好，世界\nHello, world\n\n"));
        assert!(out.contains("00:00:02,000 --> 00:00:03,500\n再见\n\n"));
    }

    #[test]
    fn render_markdown_appends_translation_quote() {
        let out = render_markdown(
            &recording(),
            &[seg("你好，世界", "Hello, world", 0), seg("再见", "", 2000)],
        );
        assert!(out.contains("- **[00:00]** 你好，世界\n  > Hello, world\n"));
        assert!(out.contains("- **[00:02]** 再见\n"));
        assert!(!out.contains("再见\n  >"));
    }
}
