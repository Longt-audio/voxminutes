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
                 (id, recording_id, text, start_ms, end_ms, speaker, source, translation, created_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(&segment.id)
            .bind(recording_id)
            .bind(&segment.text)
            .bind(segment.start_ms)
            .bind(segment.end_ms)
            .bind(segment.speaker.as_deref())
            .bind(segment.source.as_deref())
            .bind(&segment.translation)
            .bind(segment.created_at.0)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    /// 用给定段落**整体替换**某条录音的段落（同事务：先删后插）。
    ///
    /// 为什么需要（2026-09-24）：录音停止时写历史库原来只有**前端**一条路径
    /// （`api_save_transcript` → 裸 INSERT，且每次 `create_recording` 都新建一行）。
    /// 于是「不在录音页时从托盘停止录音」→ 历史里**完全没有这条记录**
    /// （audio.mp4 / transcripts.json 都在磁盘上，库里却没有行）。
    ///
    /// 现在两个写入方都用这个函数：Rust 在停止时先写原始段落，前端随后用
    /// 「按段落合并」的版本整体替换。谁后写谁的版本生效，重复调用也不会撞主键、
    /// 不会留下两份。
    pub async fn replace_segments(
        pool: &SqlitePool,
        recording_id: &str,
        segments: &[TranscriptSegment],
    ) -> Result<(), sqlx::Error> {
        let mut tx = pool.begin().await?;
        sqlx::query("DELETE FROM transcript_segments WHERE recording_id = ?")
            .bind(recording_id)
            .execute(&mut *tx)
            .await?;
        for segment in segments {
            sqlx::query(
                "INSERT INTO transcript_segments
                 (id, recording_id, text, start_ms, end_ms, speaker, source, translation, created_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(&segment.id)
            .bind(recording_id)
            .bind(&segment.text)
            .bind(segment.start_ms)
            .bind(segment.end_ms)
            .bind(segment.speaker.as_deref())
            .bind(segment.source.as_deref())
            .bind(&segment.translation)
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

#[cfg(test)]
mod history_persist_tests {
    use super::*;
    use crate::database::models::DateTimeUtc;
    use crate::database::repositories::recording::RecordingsRepository;

    /// 最小可用的内存库（只建这两个表；与 migrations 的定义保持一致）。
    async fn memory_pool() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::query(
            "CREATE TABLE recordings (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                duration_ms INTEGER,
                audio_path TEXT,
                folder_path TEXT,
                source TEXT NOT NULL DEFAULT 'realtime',
                asr_engine TEXT,
                language TEXT,
                status TEXT NOT NULL DEFAULT 'completed'
            )",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "CREATE TABLE transcript_segments (
                id TEXT PRIMARY KEY,
                recording_id TEXT NOT NULL,
                text TEXT NOT NULL,
                start_ms INTEGER NOT NULL,
                end_ms INTEGER,
                speaker TEXT,
                source TEXT NOT NULL DEFAULT 'realtime',
                translation TEXT NOT NULL DEFAULT '',
                created_at TEXT NOT NULL
            )",
        )
        .execute(&pool)
        .await
        .unwrap();
        pool
    }

    fn seg(id: &str, text: &str, start_ms: i64) -> TranscriptSegment {
        TranscriptSegment {
            id: id.to_string(),
            recording_id: String::new(),
            text: text.to_string(),
            start_ms,
            end_ms: Some(start_ms + 500),
            speaker: None,
            source: Some("realtime".to_string()),
            translation: String::new(),
            created_at: DateTimeUtc(chrono::Utc::now()),
        }
    }

    /// 2026-09-24：录音停止有两个写入方（Rust 原始段落 → 前端合并版本），
    /// 必须「同一行 + 整体替换」，否则历史里会出现两条记录或重复段落。
    #[tokio::test]
    async fn replace_is_idempotent_and_folder_lookup_reuses_row() {
        let pool = memory_pool().await;

        let id = RecordingsRepository::create_recording(
            &pool,
            "会议",
            Some(60_000),
            None,
            Some("realtime"),
            None,
            Some("/tmp/vox-rec-1"),
        )
        .await
        .unwrap();

        // Rust 停止时写过之后，前端按 folder_path 必须能找到同一行（而不是新建第二条）
        let found = RecordingsRepository::get_by_folder_path(&pool, "/tmp/vox-rec-1")
            .await
            .unwrap()
            .expect("按 folder 应能查到刚建的行");
        assert_eq!(found.id, id);
        assert!(RecordingsRepository::get_by_folder_path(&pool, "/tmp/不存在")
            .await
            .unwrap()
            .is_none());

        // 第一次写 3 段
        TranscriptSegmentsRepository::replace_segments(
            &pool,
            &id,
            &[seg("seg_1", "a", 0), seg("seg_2", "b", 1000), seg("seg_3", "c", 2000)],
        )
        .await
        .unwrap();
        assert_eq!(TranscriptSegmentsRepository::get_segments_by_recording(&pool, &id).await.unwrap().len(), 3);

        // 前端随后写「合并版本」（2 段）→ 整体替换，旧的第 3 段必须消失
        TranscriptSegmentsRepository::replace_segments(
            &pool,
            &id,
            &[seg("seg_1", "a", 0), seg("seg_2", "b", 1000)],
        )
        .await
        .unwrap();
        assert_eq!(TranscriptSegmentsRepository::get_segments_by_recording(&pool, &id).await.unwrap().len(), 2);

        // 幂等重放（例如前端重试/重复保存）：不能撞主键、不能变 4 段
        TranscriptSegmentsRepository::replace_segments(
            &pool,
            &id,
            &[seg("seg_1", "a", 0), seg("seg_2", "b", 1000)],
        )
        .await
        .unwrap();
        let after = TranscriptSegmentsRepository::get_segments_by_recording(&pool, &id).await.unwrap();
        assert_eq!(after.len(), 2, "重复 replace 不应产生重复段落");
        assert_eq!(after[0].text, "a");
        assert_eq!(after[1].text, "b");
    }
}
