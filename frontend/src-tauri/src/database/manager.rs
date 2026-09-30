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

        let migrator = sqlx::migrate!("./migrations");
        log::info!("Running database migrations…");
        match migrator.run(&pool).await {
            Ok(_) => {
                log::info!("Database migrations applied");
                return Ok(DatabaseManager { pool });
            }
            // ① 最常见的一种「假故障」：迁移文件内容其实没变，只是**校验和对不上**。
            //    sqlx 的校验和是对文件字节算的，**行尾 CRLF / LF 不同就会不一致**
            //    （Windows 上 git 检出把 LF 转成 CRLF，Linux/macOS/CI 是 LF）。
            //    2026-09-30 真机实测：日志报
            //      "migration 20260717000000 was previously applied but has been modified"
            //    用户就永久卡在启动页。这里把**库里记录的校验和刷新成当前二进制内嵌的**，
            //    再重跑一次迁移即可 —— 基础迁移是 CREATE TABLE IF NOT EXISTS，本身幂等，
            //    所以刷新只影响「要不要再执行一遍」，不会改坏结构。
            Err(sqlx::migrate::MigrateError::VersionMismatch(version)) => {
                log::warn!(
                    "Migration {version} checksum mismatch (usually CRLF/LF difference). \
                     Refreshing the stored checksum and retrying."
                );
                match migrator.iter().find(|m| m.version == version) {
                    Some(m) => {
                        sqlx::query("UPDATE _sqlx_migrations SET checksum = ? WHERE version = ?")
                            .bind(m.checksum.as_ref())
                            .bind(m.version)
                            .execute(&pool)
                            .await?;
                        migrator.run(&pool).await?;
                        log::warn!("Checksum mismatch repaired; migrations applied");
                        return Ok(DatabaseManager { pool });
                    }
                    None => {
                        log::error!(
                            "Migration {version} is recorded in the database but missing from this build"
                        );
                    }
                }
            }
            Err(e) => log::error!("Database migration failed: {e}"),
        }

        // ② 兜底：走到这里说明「刷新校验和」也救不了（结构性不兼容 / 库损坏）。
        //    把疑似坏库改名留档，再用全新的库重试一次 ——
        //    录音与转写本体在用户的录音目录里（每个会议文件夹各有 transcripts.json），
        //    数据库只是索引，所以「重建索引」比「打不开」伤害小得多，且已留 .bak 可人工恢复。
        pool.close().await;
        // ⚠️ close() 返回后 Windows 未必立刻释放文件句柄（实测 rename 撞上
        //    os error 32「另一个程序正在使用此文件」），所以这里必须重试。
        let stamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let backup = db_path.with_extension(format!("sqlite.bak-{stamp}"));
        log::warn!(
            "Renaming unusable database to {} and retrying with a fresh one",
            backup.display()
        );
        let mut renamed = false;
        for attempt in 1..=10 {
            match fs::rename(&db_path, &backup) {
                Ok(()) => {
                    renamed = true;
                    break;
                }
                Err(e) => {
                    if attempt == 10 {
                        log::error!("Giving up renaming the database after 10 attempts: {e}");
                        return Err(sqlx::Error::Io(e));
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
                }
            }
        }
        if !renamed {
            return Err(sqlx::Error::Io(std::io::Error::other(
                "could not move the unusable database aside",
            )));
        }
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
        migrator.run(&fresh).await?;
        log::warn!("Fresh database created after migration failure (old one kept as .bak)");
        Ok(DatabaseManager { pool: fresh })
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
