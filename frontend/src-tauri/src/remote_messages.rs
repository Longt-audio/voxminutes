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

/// 本地缓存路径（应用数据目录下；带 lang 时按语言分文件，如 notice_documents.en.json）。
fn notice_cache_path(lang: Option<&str>) -> Option<std::path::PathBuf> {
    let mut p = dirs::config_dir()?;
    p.push("voxminutes");
    match lang {
        Some(l) if !l.is_empty() => p.push(format!("notice_documents.{l}.json")),
        _ => p.push("notice_documents.json"),
    }
    Some(p)
}

/// 读本地缓存（离线也能显示上次的文档列表）。
/// 带 lang 且该语言尚无缓存时回退读无后缀的旧缓存，保证老用户升级后离线仍可读。
fn read_notice_cache(lang: Option<&str>) -> Vec<NoticeDocument> {
    for p in [notice_cache_path(lang), lang.and_then(|_| notice_cache_path(None))]
        .into_iter()
        .flatten()
    {
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
fn write_notice_cache(lang: Option<&str>, items: &[NoticeDocument]) {
    if let Some(p) = notice_cache_path(lang) {
        if let Some(parent) = p.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let json = serde_json::json!({ "items": items });
        let _ = std::fs::write(&p, serde_json::to_string_pretty(&json).unwrap_or_default());
    }
}

/// 追加 ?lang= 查询参数（界面语言 zh/en/ko/ja，网关按它返回对应语言内容）。
/// 只允许语言码字符（字母/数字/连字符），其他一律忽略。
fn with_lang(url: String, lang: &Option<String>) -> String {
    if let Some(l) = lang {
        let l = l.trim();
        if !l.is_empty() && l.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            return format!("{url}?lang={l}");
        }
    }
    url
}

/// 拉取启动文档列表：网关返回最新版本，比本地新则更新缓存并返回；离线时返回本地缓存。
/// lang = 界面语言，透传网关 ?lang=；不带 lang 行为与旧版一致。
#[tauri::command]
pub async fn fetch_notice_documents(lang: Option<String>) -> Result<serde_json::Value, String> {
    let lang_ref = lang.as_deref().filter(|l| !l.trim().is_empty());
    let cached = read_notice_cache(lang_ref);
    let base = match crate::audio::transcription::remote_api_base() {
        Some(b) => b,
        None => {
            // 网关未配置：返回本地缓存（离线可读）
            return Ok(serde_json::json!({ "items": cached }));
        }
    };
    let client = reqwest::Client::new();
    let url = with_lang(format!("{}/notice-document", base), &lang);
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
    // 拉取成功即以网关为准：只要版本集合有差异（新增/删除/更新）就覆盖缓存并返回。
    // 注意不能只比「最大版本号」——后台删除文档后 remote_max 不变，旧缓存会一直残留。
    let same = remote.len() == cached.len()
        && remote.iter().all(|r| {
            cached
                .iter()
                .any(|c| c.version == r.version && c.updated_at == r.updated_at)
        });
    if !same {
        write_notice_cache(lang_ref, &remote);
        return Ok(serde_json::json!({ "items": remote }));
    }
    Ok(serde_json::json!({ "items": cached }))
}

/// 欢迎弹窗的欢迎词（网关推送，无需鉴权）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WelcomeMessage {
    pub title: String,
    pub body: String,
    #[serde(default)]
    pub updated_at: String,
}

/// 拉取欢迎词：GET {server}/v1/welcome（5s 超时），lang 透传 ?lang=（界面语言）。
/// 远程服务未配置 server 或请求失败时返回 Err，前端回退到内置 i18n 默认欢迎词。
#[tauri::command]
pub async fn fetch_welcome(lang: Option<String>) -> Result<serde_json::Value, String> {
    let base = crate::audio::transcription::remote_api_base()
        .ok_or_else(|| "远程服务未配置".to_string())?;
    let client = reqwest::Client::new();
    let url = with_lang(format!("{}/welcome", base), &lang);
    let resp = client
        .get(&url)
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
        .map_err(|e| format!("请求失败: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("网关返回 {}", resp.status()));
    }
    let json: serde_json::Value = resp.json().await.map_err(|e| format!("解析失败: {e}"))?;
    let msg: WelcomeMessage =
        serde_json::from_value(json.clone()).map_err(|e| format!("解析失败: {e}"))?;
    Ok(serde_json::to_value(msg).unwrap_or(json))
}

/// 重要信息推送（客户端顶部横幅，无需鉴权）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportantNotice {
    pub text: String,
    #[serde(default)]
    pub updated_at: String,
    #[serde(default)]
    pub lang: Option<String>,
}

/// 拉取重要信息推送：GET {server}/v1/important-notice（5s 超时），lang 透传 ?lang=（界面语言）。
/// text 为空串 = 不显示；远程未配置或请求失败时返回 Err，前端静默不渲染。
/// 不带 lang 时网关可能返回四段式「中文|English|한국어|日本語」，由前端 pickLangSegment 兜底切分。
#[tauri::command]
pub async fn fetch_important_notice(lang: Option<String>) -> Result<serde_json::Value, String> {
    let base = crate::audio::transcription::remote_api_base()
        .ok_or_else(|| "远程服务未配置".to_string())?;
    let client = reqwest::Client::new();
    let url = with_lang(format!("{}/important-notice", base), &lang);
    let resp = client
        .get(&url)
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
        .map_err(|e| format!("请求失败: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("网关返回 {}", resp.status()));
    }
    let json: serde_json::Value = resp.json().await.map_err(|e| format!("解析失败: {e}"))?;
    let notice: ImportantNotice =
        serde_json::from_value(json.clone()).map_err(|e| format!("解析失败: {e}"))?;
    Ok(serde_json::to_value(notice).unwrap_or(json))
}

/// 拉取网关推送消息（公告 + 最新版本）。网关未配置/不可达时返回错误。
/// lang = 界面语言，透传 ?lang=（公告正文/版本 notes 按语言返回；客户端另对短文本做四段式切分兜底）。
#[tauri::command]
pub async fn fetch_remote_messages(lang: Option<String>) -> Result<serde_json::Value, String> {
    let base = crate::audio::transcription::remote_api_base()
        .ok_or_else(|| "远程服务未配置".to_string())?;
    let client = reqwest::Client::new();

    let ann_url = with_lang(format!("{}/announcements", base), &lang);
    let ver_url = with_lang(format!("{}/latest-version", base), &lang);

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
