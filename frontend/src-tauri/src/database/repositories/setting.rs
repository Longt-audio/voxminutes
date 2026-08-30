use sqlx::SqlitePool;

use super::super::models::SettingRow;

pub struct SettingsRepository;

impl SettingsRepository {
    pub async fn get(pool: &SqlitePool, key: &str) -> Result<Option<String>, sqlx::Error> {
        let row: Option<(String,)> = sqlx::query_as("SELECT value FROM settings WHERE key = ?")
            .bind(key)
            .fetch_optional(pool)
            .await?;
        Ok(row.map(|r| r.0))
    }

    pub async fn set(pool: &SqlitePool, key: &str, value: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO settings (key, value) VALUES (?, ?)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        )
        .bind(key)
        .bind(value)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn delete(pool: &SqlitePool, key: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM settings WHERE key = ?")
            .bind(key)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn get_all(pool: &SqlitePool) -> Result<Vec<SettingRow>, sqlx::Error> {
        sqlx::query_as::<_, SettingRow>("SELECT key, value FROM settings")
            .fetch_all(pool)
            .await
    }

    pub async fn get_export_dir(pool: &SqlitePool) -> Result<String, sqlx::Error> {
        Ok(Self::get(pool, "export.default_dir").await?.unwrap_or_default())
    }

    pub async fn save_model_config(
        pool: &SqlitePool,
        provider: &str,
        model: &str,
        whisper_model: &str,
        api_key: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        Self::set(pool, "model.provider", provider).await?;
        Self::set(pool, "model.name", model).await?;
        Self::set(pool, "model.whisper_model", whisper_model).await?;
        if let Some(key) = api_key {
            if !key.is_empty() {
                Self::set(pool, "model.api_key", key).await?;
            }
        }
        Ok(())
    }

    pub async fn save_transcript_config(
        pool: &SqlitePool,
        provider: &str,
        model: &str,
    ) -> Result<(), sqlx::Error> {
        Self::set(pool, "transcript.provider", provider).await?;
        Self::set(pool, "transcript.model", model).await?;
        Ok(())
    }
}
