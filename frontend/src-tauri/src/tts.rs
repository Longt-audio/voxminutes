// tts.rs
//
// 文本转语音（TTS）：走共享网关 `/v1/audio/speech`（非流式，返回音频二进制）。
// 供翻译页「播放原文 / 播放译文」与「语音合成」页使用；支持把合成出的音频
// 保存到用户选择的位置。
//
// 参数：
// - voice: 音色（可选；缺省时由网关用上供应商默认音色）
// - model: 指定远程 TTS 模型（可选；缺省时用用户在「远程服务」里选择的 TTS 模型）

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use serde::Serialize;
use tauri_plugin_dialog::DialogExt;

/// TTS 合成结果（音频以 base64 回传，前端用 Blob URL 播放 / 下载）。
#[derive(Debug, Serialize)]
pub struct TtsSynthesisResult {
    pub audio_base64: String,
    pub content_type: String,
}

/// 合成一段文本为语音（远程网关 TTS）。
#[tauri::command]
pub async fn tts_synthesize(
    text: String,
    voice: Option<String>,
    model: Option<String>,
) -> Result<TtsSynthesisResult, String> {
    let base = crate::audio::transcription::remote_api_base()
        .ok_or_else(|| "远程服务未配置（缺少服务器地址）".to_string())?;
    let license = crate::audio::transcription::get_remote_license();
    if license.is_empty() {
        return Err("远程服务未配置（缺少授权码）".to_string());
    }

    let text = text.trim().to_string();
    if text.is_empty() {
        return Err("TTS 文本为空".to_string());
    }

    // 模型：显式传入优先（语音合成页按 tab 指定）；否则用用户选择的远程 TTS 模型；
    // 都没有时交给网关走默认上游。
    let chosen = model
        .filter(|m| !m.trim().is_empty())
        .unwrap_or_else(|| crate::audio::transcription::get_remote_tts_model());
    let mut body = serde_json::json!({ "input": text });
    if !chosen.is_empty() {
        body["model"] = serde_json::json!(chosen);
    }
    if let Some(v) = voice {
        let v = v.trim().to_string();
        if !v.is_empty() {
            body["voice"] = serde_json::json!(v);
        }
    }

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/audio/speech", base))
        .bearer_auth(&license)
        .json(&body)
        .timeout(std::time::Duration::from_secs(120))
        .send()
        .await
        .map_err(|e| format!("TTS 请求失败: {}", e))?;

    let status = resp.status();
    let content_type = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("audio/mpeg")
        .to_string();

    if !status.is_success() {
        let detail = resp.text().await.unwrap_or_default();
        let msg = extract_gateway_error(&detail).unwrap_or(detail);
        return Err(format!("TTS 失败 (HTTP {}): {}", status.as_u16(), msg));
    }

    let bytes = resp
        .bytes()
        .await
        .map_err(|e| format!("TTS 响应读取失败: {}", e))?;
    if bytes.is_empty() {
        return Err("TTS 返回了空音频".to_string());
    }

    Ok(TtsSynthesisResult {
        audio_base64: BASE64.encode(&bytes),
        content_type,
    })
}

/// 保存 TTS 音频到用户选择的位置（弹出保存对话框）。
/// 返回写入的文件路径；用户取消时返回 None。
#[tauri::command]
pub async fn save_tts_audio(
    app: tauri::AppHandle,
    audio_base64: String,
    suggested_name: String,
    mime: String,
) -> Result<Option<String>, String> {
    let bytes = BASE64
        .decode(&audio_base64)
        .map_err(|e| format!("音频解码失败: {}", e))?;
    if bytes.is_empty() {
        return Err("音频数据为空".to_string());
    }

    let ext = extension_from_mime(&mime);
    let file_name = if suggested_name
        .to_lowercase()
        .ends_with(&format!(".{}", ext))
    {
        suggested_name
    } else {
        format!("{}.{}", suggested_name.trim_end_matches('.'), ext)
    };

    let app_clone = app.clone();
    let path = tokio::task::spawn_blocking(move || {
        app_clone
            .dialog()
            .file()
            .set_file_name(file_name)
            .add_filter("Audio", &[ext.as_str()])
            .blocking_save_file()
    })
    .await
    .map_err(|e| format!("保存对话框失败: {}", e))?;

    match path {
        Some(p) => {
            let path = p.into_path().map_err(|e| e.to_string())?;
            if let Some(parent) = path.parent() {
                if !parent.exists() {
                    std::fs::create_dir_all(parent)
                        .map_err(|e| format!("创建目录失败: {}", e))?;
                }
            }
            std::fs::write(&path, &bytes).map_err(|e| format!("写入音频失败: {}", e))?;
            log::info!("TTS 音频已保存到 {}", path.display());
            Ok(Some(path.to_string_lossy().to_string()))
        }
        None => Ok(None),
    }
}

/// 从网关的错误响应体里尽量抠出可读信息（形如 `{"error":{"message":...,"detail":...}}`）。
/// 优先把 message 和 detail 合并返回——detail 常含「可用音色列表」等可操作信息。
fn extract_gateway_error(detail: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(detail).ok()?;
    let msg = v
        .get("error")
        .and_then(|e| e.get("message"))
        .and_then(|m| m.as_str())
        .map(|s| s.to_string());
    let det = v
        .get("error")
        .and_then(|e| e.get("detail"))
        .and_then(|d| d.as_str())
        .or_else(|| v.get("detail").and_then(|m| m.as_str()))
        .map(|s| s.to_string());
    match (msg, det) {
        (Some(m), Some(d)) => Some(format!("{}: {}", m, d)),
        (Some(m), None) => Some(m),
        (None, Some(d)) => Some(d),
        (None, None) => None,
    }
}

/// 根据 MIME 类型推导文件扩展名（保存对话框默认文件名用）。
fn extension_from_mime(mime: &str) -> String {
    match mime.split(';').next().unwrap_or("").trim().to_lowercase().as_str() {
        "audio/wav" | "audio/x-wav" | "audio/wave" => "wav".to_string(),
        "audio/mp4" | "audio/mp4a-latm" | "audio/x-m4a" => "m4a".to_string(),
        "audio/ogg" | "application/ogg" => "ogg".to_string(),
        "audio/flac" | "audio/x-flac" => "flac".to_string(),
        "audio/aac" => "aac".to_string(),
        "audio/webm" => "webm".to_string(),
        _ => "mp3".to_string(),
    }
}
