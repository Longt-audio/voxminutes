//! Summary LLM endpoint configuration, persisted as JSON in the settings
//! table under the key `summary.api_config`.

use serde::{Deserialize, Serialize};

use crate::database::repositories::setting::SettingsRepository;
use crate::state::AppState;

/// Settings-table key holding the JSON-serialized `SummaryApiConfig`.
pub(crate) const SUMMARY_CONFIG_KEY: &str = "summary.api_config";

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SummaryApiConfig {
    /// "openai" | "anthropic"
    pub protocol: String,
    /// Base URL, e.g. http://192.168.1.10:8000/v1 (openai) or
    /// https://api.anthropic.com (anthropic).
    pub endpoint: String,
    /// May be empty for LAN endpoints.
    pub api_key: String,
    pub model: String,
}

#[tauri::command]
pub async fn summary_get_config(
    state: tauri::State<'_, AppState>,
) -> Result<Option<SummaryApiConfig>, String> {
    let raw = SettingsRepository::get(state.db_manager.pool(), SUMMARY_CONFIG_KEY)
        .await
        .map_err(|e| e.to_string())?;
    match raw {
        None => Ok(None),
        Some(json) => serde_json::from_str(&json)
            .map(Some)
            .map_err(|e| format!("Invalid summary config in settings: {}", e)),
    }
}

#[tauri::command]
pub async fn summary_save_config(
    state: tauri::State<'_, AppState>,
    config: SummaryApiConfig,
) -> Result<(), String> {
    let json = serde_json::to_string(&config).map_err(|e| e.to_string())?;
    SettingsRepository::set(state.db_manager.pool(), SUMMARY_CONFIG_KEY, &json)
        .await
        .map_err(|e| e.to_string())
}
