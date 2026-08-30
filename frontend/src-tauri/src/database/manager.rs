use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use std::fs;
use tauri::Manager;

/// SQLite database file name inside the app data directory.
pub const DB_FILE_NAME: &str = "meeting_minutes.sqlite";

#[derive(Clone)]
pub struct DatabaseManager {
    pool: SqlitePool,
}

impl DatabaseManager {
    /// Open (or create) the app's SQLite database and run migrations.
    pub async fn new_from_app_handle(app: &tauri::AppHandle) -> Result<Self, sqlx::Error> {
        let app_data_dir = app
            .path()
            .app_data_dir()
            .map_err(|e| sqlx::Error::Protocol(format!("failed to resolve app data dir: {e}")))?;
        fs::create_dir_all(&app_data_dir).map_err(sqlx::Error::Io)?;

        let db_path = app_data_dir.join(DB_FILE_NAME);

        log::info!("Opening SQLite database at {}", db_path.display());
        // Use SqliteConnectOptions::filename so the path is passed directly
        // (no URL parsing) — the app data dir contains a space on macOS
        // ("Application Support"), which breaks `sqlite:` URL parsing.
        let options = SqliteConnectOptions::new()
            .filename(&db_path)
            .create_if_missing(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await?;

        sqlx::migrate!("./migrations").run(&pool).await?;

        Ok(DatabaseManager { pool })
    }

    /// Whether the SQLite database has not been created yet (first launch).
    pub async fn is_first_launch(app: &tauri::AppHandle) -> Result<bool, sqlx::Error> {
        let app_data_dir = app
            .path()
            .app_data_dir()
            .map_err(|e| sqlx::Error::Protocol(format!("failed to resolve app data dir: {e}")))?;
        Ok(!app_data_dir.join(DB_FILE_NAME).exists())
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// Checkpoint the WAL and close the connection pool. Called on shutdown.
    pub async fn cleanup(&self) -> Result<(), sqlx::Error> {
        log::info!("Checkpointing SQLite WAL...");
        match sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
            .execute(&self.pool)
            .await
        {
            Ok(_) => log::info!("WAL checkpoint complete"),
            Err(e) => log::warn!("WAL checkpoint failed (non-fatal): {e}"),
        }
        self.pool.close().await;
        log::info!("SQLite connection pool closed");
        Ok(())
    }
}
