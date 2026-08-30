use sqlx::SqlitePool;

use super::super::models::TranscriptSegment;

/// A single transcript search hit (segment joined with its recording title).
#[derive(Debug, sqlx::FromRow)]
pub struct SearchResult {
    pub id: String,
    pub recording_id: String,
    pub title: String,
    pub text: String,
    pub start_ms: i64,
}

pub struct TranscriptSegmentsRepository;

impl TranscriptSegmentsRepository {
    pub async fn get_segments_by_recording(
        pool: &SqlitePool,
        recording_id: &str,
    ) -> Result<Vec<TranscriptSegment>, sqlx::Error> {
        sqlx::query_as::<_, TranscriptSegment>(
            "SELECT * FROM transcript_segments WHERE recording_id = ? ORDER BY start_ms ASC",
        )
        .bind(recording_id)
        .fetch_all(pool)
        .await
    }

    pub async fn insert_segments(
        pool: &SqlitePool,
        recording_id: &str,
        segments: &[TranscriptSegment],
    ) -> Result<(), sqlx::Error> {
        let mut tx = pool.begin().await?;
        for segment in segments {
            sqlx::query(
                "INSERT INTO transcript_segments
                 (id, recording_id, text, start_ms, end_ms, speaker, source, created_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(&segment.id)
            .bind(recording_id)
            .bind(&segment.text)
            .bind(segment.start_ms)
            .bind(segment.end_ms)
            .bind(segment.speaker.as_deref())
            .bind(segment.source.as_deref())
            .bind(segment.created_at.0)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    pub async fn search_segments(
        pool: &SqlitePool,
        query: &str,
    ) -> Result<Vec<SearchResult>, sqlx::Error> {
        let pattern = format!("%{}%", query);
        sqlx::query_as::<_, SearchResult>(
            "SELECT ts.id, ts.recording_id, r.title, ts.text, ts.start_ms
             FROM transcript_segments ts
             JOIN recordings r ON r.id = ts.recording_id
             WHERE ts.text LIKE ?
             ORDER BY r.created_at DESC, ts.start_ms ASC",
        )
        .bind(pattern)
        .fetch_all(pool)
        .await
    }

    pub async fn update_segment_text(
        pool: &SqlitePool,
        segment_id: &str,
        text: &str,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("UPDATE transcript_segments SET text = ? WHERE id = ?")
            .bind(text)
            .bind(segment_id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }
}
