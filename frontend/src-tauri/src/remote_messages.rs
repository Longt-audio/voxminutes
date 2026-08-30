// remote_messages.rs
//
// 远程推送消息：从网关拉取使用 tips / 公告 / 最新版本信息（无需鉴权）。
// 用于帮助页实时更新、更新提醒、使用 tips。网关未配置时返回错误，前端静默忽略。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteMessage {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub title: String,
    pub body: String,
    pub severity: String,
    #[serde(default)]
    pub dismissible: bool,
    #[serde(default)]
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteLatestVersion {
    pub version: String,
    pub release_notes: String,
    pub download_url: String,
    pub published_at: String,
}

/// 拉取网关推送消息（公告 + 最新版本）。网关未配置/不可达时返回错误。
#[tauri::command]
pub async fn fetch_remote_messages() -> Result<serde_json::Value, String> {
    let base = crate::audio::transcription::remote_api_base()
        .ok_or_else(|| "远程服务未配置".to_string())?;
    let client = reqwest::Client::new();

    let ann_url = format!("{}/announcements", base);
    let ver_url = format!("{}/latest-version", base);

    let announcements: Vec<RemoteMessage> = match client
        .get(&ann_url)
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => {
            let json: serde_json::Value = resp.json().await.unwrap_or_default();
            json.get("items")
                .cloned()
                .and_then(|v| serde_json::from_value(v).ok())
                .unwrap_or_default()
        }
        _ => Vec::new(),
    };

    let latest_version: Option<RemoteLatestVersion> = match client
        .get(&ver_url)
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => resp.json().await.ok(),
        _ => None,
    };

    Ok(serde_json::json!({
        "announcements": announcements,
        "latestVersion": latest_version,
    }))
}
