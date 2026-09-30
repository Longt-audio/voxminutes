use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};

/// Wrapper around `chrono::DateTime<Utc>` stored as TEXT (RFC3339) in SQLite.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(transparent)]
pub struct DateTimeUtc(pub DateTime<Utc>);

impl From<NaiveDateTime> for DateTimeUtc {
    fn from(naive: NaiveDateTime) -> Self {
        DateTimeUtc(DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc))
    }
}

/// A single recording (live session or offline transcription job).
#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct Recording {
    pub id: String,
    pub title: String,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
    pub duration_ms: Option<i64>,
    pub audio_path: Option<String>,
    pub folder_path: Option<String>,
    pub source: Option<String>,
    pub asr_engine: Option<String>,
    pub language: Option<String>,
    pub status: Option<String>,
}

/// A single transcription segment belonging to a recording.
#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct TranscriptSegment {
    pub id: String,
    pub recording_id: String,
    pub text: String,
    pub start_ms: i64,
    pub end_ms: Option<i64>,
    pub speaker: Option<String>,
    pub source: Option<String>,
    /// 段落最终译文（实时内嵌翻译；空串 = 无译文）
    #[sqlx(default)]
    pub translation: String,
    pub created_at: DateTimeUtc,
}

/// A key/value row from the `settings` table.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SettingRow {
    pub key: String,
    pub value: String,
}
