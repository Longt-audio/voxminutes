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
    /// 打开一个连接池（不含迁移）。
    async fn connect(db_path: &std::path::Path) -> Result<SqlitePool, sqlx::Error> {
        // Use SqliteConnectOptions::filename so the path is passed directly
        // (no URL parsing) — the app data dir contains a space on macOS
        // ("Application Support"), which breaks `sqlite:` URL parsing.
        let options = SqliteConnectOptions::new()
            .filename(db_path)
            .create_if_missing(true)
            // 被别的进程短暂占用时等待而不是立刻报错（Windows 上杀软扫描、
            // 上一个实例还没退干净都会造成瞬时占用）
            .busy_timeout(std::time::Duration::from_secs(10));
        SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await
    }

    /// Open (or create) the app's SQLite database and run migrations.
    pub async fn new_from_app_handle(app: &tauri::AppHandle) -> Result<Self, sqlx::Error> {
        let app_data_dir = app
            .path()
            .app_data_dir()
            .map_err(|e| sqlx::Error::Protocol(format!("failed to resolve app data dir: {e}")))?;
        fs::create_dir_all(&app_data_dir).map_err(sqlx::Error::Io)?;

        let db_path = app_data_dir.join(DB_FILE_NAME);

        // 启动期排障：把「文件在不在、多大」先记下来。
        // （2026-09-30 Windows 真机「双击就消失」排查时补的 —— 当时日志停在
        //   "Opening SQLite database" 之后什么都没有，完全无法判断卡在哪一步。）
        match fs::metadata(&db_path) {
            Ok(m) => log::info!(
                "SQLite file exists: {} ({} bytes)",
                db_path.display(),
                m.len()
            ),
            Err(_) => log::info!("SQLite file does not exist yet: {}", db_path.display()),
        }
        log::info!("Opening SQLite database at {}", db_path.display());

        let pool = Self::connect(&db_path).await?;
        log::info!("SQLite connection pool ready ({} connections)", pool.size());

        log::info!("Running database migrations…");
        match sqlx::migrate!("./migrations").run(&pool).await {
            Ok(_) => {
                log::info!("Database migrations applied");
                Ok(DatabaseManager { pool })
            }
            Err(e) => {
                // 迁移失败时**不要**让进程带着 panic 消失（用户看到的是「双击没反应」）。
                // 把疑似不兼容/损坏的库改名留档，再用全新的库重试一次 ——
                // 录音与转写本体在用户的录音目录里（每个会议文件夹各有 transcripts.json），
                // 数据库只是索引，所以「重建索引」比「打不开」伤害小得多，而且已留 .bak 可人工恢复。
                log::error!("Database migration failed: {e}");
                pool.close().await;

                let stamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
                let backup = db_path.with_extension(format!("sqlite.bak-{stamp}"));
                log::warn!(
                    "Renaming unusable database to {} and retrying with a fresh one",
                    backup.display()
                );
                fs::rename(&db_path, &backup).map_err(sqlx::Error::Io)?;
                // WAL / SHM 必须一起挪走，否则新库会读到旧库的 WAL，等于没换
                for suffix in ["-wal", "-shm"] {
                    let side = std::path::PathBuf::from(format!("{}{}", db_path.display(), suffix));
                    if side.exists() {
                        let _ = fs::rename(
                            &side,
                            std::path::PathBuf::from(format!("{}{}", backup.display(), suffix)),
                        );
                    }
                }

                let fresh = Self::connect(&db_path).await?;
                sqlx::migrate!("./migrations").run(&fresh).await?;
                log::warn!("Fresh database created after migration failure (old one kept as .bak)");
                Ok(DatabaseManager { pool: fresh })
            }
        }
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
