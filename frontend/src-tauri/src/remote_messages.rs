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
    /// 展示渠道：footer = 页面底部短信息；startup = 启动时长信息弹窗
    #[serde(default)]
    pub channel: Option<String>,
    pub title: String,
    pub body: String,
    /// 可选图片（data URL 或 http 链接），仅 startup 渠道显示
    #[serde(default)]
    pub image: Option<String>,
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

/// 启动文档（版本化多文档）：后台每推一篇，客户端本地缓存，离线可读。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoticeDocument {
    pub version: i64,
    pub title: String,
    pub body: String,
    #[serde(default)]
    pub images: Vec<String>,
    #[serde(default)]
    pub updated_at: String,
}

/// 本地缓存路径（应用数据目录下的 notice_documents.json）。
fn notice_cache_path() -> Option<std::path::PathBuf> {
    let mut p = dirs::config_dir()?;
    p.push("voxminutes");
    p.push("notice_documents.json");
    Some(p)
}

/// 读本地缓存（离线也能显示上次的文档列表）。
fn read_notice_cache() -> Vec<NoticeDocument> {
    if let Some(p) = notice_cache_path() {
        if let Ok(content) = std::fs::read_to_string(&p) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(arr) = v.get("items").and_then(|i| i.as_array()) {
                    return arr
                        .iter()
                        .filter_map(|d| serde_json::from_value::<NoticeDocument>(d.clone()).ok())
                        .collect();
                }
            }
        }
    }
    Vec::new()
}

/// 写本地缓存。
fn write_notice_cache(items: &[NoticeDocument]) {
    if let Some(p) = notice_cache_path() {
        if let Some(parent) = p.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let json = serde_json::json!({ "items": items });
        let _ = std::fs::write(&p, serde_json::to_string_pretty(&json).unwrap_or_default());
    }
}

/// 拉取启动文档列表：网关返回最新版本，比本地新则更新缓存并返回；离线时返回本地缓存。
#[tauri::command]
pub async fn fetch_notice_documents() -> Result<serde_json::Value, String> {
    let cached = read_notice_cache();
    let base = match crate::audio::transcription::remote_api_base() {
        Some(b) => b,
        None => {
            // 网关未配置：返回本地缓存（离线可读）
            return Ok(serde_json::json!({ "items": cached }));
        }
    };
    let client = reqwest::Client::new();
    let url = format!("{}/notice-document", base);
    let remote: Vec<NoticeDocument> = match client
        .get(&url)
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
        _ => {
            // 网关不可达：返回本地缓存（离线可读）
            return Ok(serde_json::json!({ "items": cached }));
        }
    };
    // 比较版本：网关最新 > 本地最新 → 更新缓存
    let remote_max = remote.iter().map(|d| d.version).max().unwrap_or(0);
    let local_max = cached.iter().map(|d| d.version).max().unwrap_or(0);
    if remote_max > local_max {
        write_notice_cache(&remote);
        return Ok(serde_json::json!({ "items": remote }));
    }
    Ok(serde_json::json!({ "items": if cached.is_empty() { remote } else { cached } }))
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
