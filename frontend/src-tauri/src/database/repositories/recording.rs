use sqlx::SqlitePool;

use super::super::models::{Recording, TranscriptSegment};
use super::transcript_segment::TranscriptSegmentsRepository;

/// A recording together with its transcription segments.
pub struct RecordingWithSegments {
    pub recording: Recording,
    pub segments: Vec<TranscriptSegment>,
}

pub struct RecordingsRepository;

impl RecordingsRepository {
    pub async fn get_recordings(pool: &SqlitePool) -> Result<Vec<Recording>, sqlx::Error> {
        sqlx::query_as::<_, Recording>("SELECT * FROM recordings ORDER BY created_at DESC")
            .fetch_all(pool)
            .await
    }

    pub async fn get_recording(
        pool: &SqlitePool,
        id: &str,
    ) -> Result<Option<RecordingWithSegments>, sqlx::Error> {
        let recording = sqlx::query_as::<_, Recording>("SELECT * FROM recordings WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await?;

        match recording {
            Some(recording) => {
                let segments =
                    TranscriptSegmentsRepository::get_segments_by_recording(pool, id).await?;
                Ok(Some(RecordingWithSegments {
                    recording,
                    segments,
                }))
            }
            None => Ok(None),
        }
    }

    pub async fn delete_recording(pool: &SqlitePool, id: &str) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM recordings WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn update_recording_title(
        pool: &SqlitePool,
        id: &str,
        title: &str,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("UPDATE recordings SET title = ?, updated_at = ? WHERE id = ?")
            .bind(title)
            .bind(chrono::Utc::now())
            .bind(id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn update_recording_duration(
        pool: &SqlitePool,
        id: &str,
        duration_ms: i64,
    ) -> Result<bool, sqlx::Error> {
        let result =
            sqlx::query("UPDATE recordings SET duration_ms = ?, updated_at = ? WHERE id = ?")
                .bind(duration_ms)
                .bind(chrono::Utc::now())
                .bind(id)
                .execute(pool)
                .await?;
        Ok(result.rows_affected() > 0)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn create_recording(
        pool: &SqlitePool,
        title: &str,
        duration_ms: Option<i64>,
        audio_path: Option<&str>,
        source: Option<&str>,
        asr_engine: Option<&str>,
        folder_path: Option<&str>,
    ) -> Result<String, sqlx::Error> {
        let id = format!("recording-{}", uuid::Uuid::new_v4());
        let now = chrono::Utc::now();

        sqlx::query(
            "INSERT INTO recordings
             (id, title, created_at, updated_at, duration_ms, audio_path, folder_path, source, asr_engine, language, status)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(title)
        .bind(now)
        .bind(now)
        .bind(duration_ms)
        .bind(audio_path)
        .bind(folder_path)
        .bind(source.unwrap_or("realtime"))
        .bind(asr_engine)
        .bind(Option::<String>::None) // language
        .bind("completed")
        .execute(pool)
        .await?;

        Ok(id)
    }
}
