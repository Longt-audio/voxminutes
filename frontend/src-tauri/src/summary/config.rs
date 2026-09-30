//! Summary LLM endpoint configuration, persisted as JSON in the settings
//! table under the key `summary.api_config`.

use serde::{Deserialize, Serialize};
use std::sync::{LazyLock, Mutex};

use crate::database::repositories::setting::SettingsRepository;
use crate::state::AppState;

/// Settings-table key holding the JSON-serialized `SummaryApiConfig`.
pub(crate) const SUMMARY_CONFIG_KEY: &str = "summary.api_config";

/// 当前配置的内存缓存：启动恢复、summary_get_config、summary_save_config 时更新，
/// 供同步上下文（如翻译引擎 custom-api 合法性校验、实时翻译分发）读取。
static CURRENT_CONFIG: LazyLock<Mutex<Option<SummaryApiConfig>>> =
    LazyLock::new(|| Mutex::new(None));

/// 读取缓存的当前配置（不访问数据库）。
pub fn cached_config() -> Option<SummaryApiConfig> {
    CURRENT_CONFIG.lock().ok().and_then(|c| c.clone())
}

/// 更新配置缓存（启动恢复与保存时调用）。
pub(crate) fn set_cached_config(config: Option<SummaryApiConfig>) {
    if let Ok(mut guard) = CURRENT_CONFIG.lock() {
        *guard = config;
    }
}

/// custom-api 翻译引擎可用的配置：endpoint 非空且协议为 openai/anthropic。
/// gateway 协议（endpoint 为空）不算自定义 API 配置。
pub fn custom_api_config() -> Option<SummaryApiConfig> {
    cached_config().filter(|c| {
        !c.endpoint.trim().is_empty() && matches!(c.protocol.as_str(), "openai" | "anthropic")
    })
}

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
    let parsed = match raw {
        None => None,
        Some(json) => serde_json::from_str(&json)
            .map(Some)
            .map_err(|e| format!("Invalid summary config in settings: {}", e))?,
    };
    set_cached_config(parsed.clone());
    Ok(parsed)
}

#[tauri::command]
pub async fn summary_save_config(
    state: tauri::State<'_, AppState>,
    config: SummaryApiConfig,
) -> Result<(), String> {
    let json = serde_json::to_string(&config).map_err(|e| e.to_string())?;
    SettingsRepository::set(state.db_manager.pool(), SUMMARY_CONFIG_KEY, &json)
        .await
        .map_err(|e| e.to_string())?;
    // 只记协议与模型：endpoint 可能含内网地址，api_key 绝不落日志
    log::info!(
        "💾 总结 API 配置已保存 协议={} 模型={}",
        config.protocol,
        config.model
    );
    set_cached_config(Some(config));
    Ok(())
}
