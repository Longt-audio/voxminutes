use log::info;
use tauri::{AppHandle, Emitter, Manager};

use super::manager::DatabaseManager;
use crate::state::AppState;

/// Initialize the database on startup: detect first launch, open/create the
/// SQLite database, run migrations, and register `AppState` with the pool.
pub async fn initialize_database_on_startup(app: &AppHandle) -> Result<(), String> {
    // Must be checked BEFORE the pool is opened, otherwise the connect call
    // creates the file and the check would always report false.
    let is_first_launch = DatabaseManager::is_first_launch(app)
        .await
        .map_err(|e| format!("Failed to check first-launch status: {e}"))?;

    let db_manager = DatabaseManager::new_from_app_handle(app)
        .await
        .map_err(|e| format!("Failed to initialize database manager: {e}"))?;

    app.manage(AppState { db_manager });
    info!("Database initialized successfully");

    let app_handle = app.clone();
    let _ = app_handle.emit("database-initialized", ());

    if is_first_launch {
        info!("First launch detected - will notify window when ready");
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            let _ = app_handle.emit("first-launch-detected", ());
        });
    }

    Ok(())
}
