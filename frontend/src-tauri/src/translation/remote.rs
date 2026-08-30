// translation/remote.rs
//
// 远程翻译：走开发者网关的 OpenAI 兼容 chat completions（SSE 流式）。
// 复用 llm.rs 的 build_prompt（纠错翻译指令）与 postprocess（去回声清洗），
// 只把底层生成从 llama-helper 换成 HTTP。

use futures_util::StreamExt;

pub async fn translate_remote(
    text: &str,
    direction: &str,
    asr_mode: bool,
    mut on_token: Option<&mut (dyn FnMut(&str) + Send)>,
) -> Result<String, String> {
    let Some((src, tgt)) = super::llm::parse_direction(direction) else {
        return Err(format!("不支持的翻译方向: {}", direction));
    };
    let api_base = crate::audio::transcription::remote_api_base()
        .ok_or_else(|| "远程服务未配置（缺少服务器地址）".to_string())?;
    let license = crate::audio::transcription::get_remote_license();
    if license.is_empty() {
        return Err("远程服务未配置（缺少授权码）".to_string());
    }

    let prompt = super::llm::build_prompt(text, src, tgt, asr_mode);
    let translate_model = crate::audio::transcription::get_remote_translate_model();
    let model_name = if translate_model.is_empty() {
        "remote".to_string()
    } else {
        translate_model
    };
    let body = serde_json::json!({
        "model": model_name,
        "messages": [{ "role": "user", "content": prompt }],
        "stream": true,
        "temperature": 0.3,
        "max_tokens": (text.chars().count() * 2).clamp(64, 1024) as u32,
    });

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/chat/completions", api_base))
        .bearer_auth(&license)
        .json(&body)
        .timeout(std::time::Duration::from_secs(120))
        .send()
        .await
        .map_err(|e| format!("远程翻译请求失败: {}", e))?;

    let status = resp.status();
    if !status.is_success() {
        let detail = resp.text().await.unwrap_or_default();
        return Err(format!("远程翻译错误 (HTTP {}): {}", status, detail));
    }

    let mut raw = String::new();
    let mut buf = String::new();
    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("远程翻译流错误: {}", e))?;
        buf.push_str(&String::from_utf8_lossy(&chunk));
        while let Some(pos) = buf.find('\n') {
            let line = buf[..pos].trim().to_string();
            buf.drain(..=pos);
            let data = match line.strip_prefix("data:") {
                Some(d) => d.trim(),
                None => continue,
            };
            if data.is_empty() {
                continue;
            }
            if data == "[DONE]" {
                break;
            }
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(data) {
                if let Some(delta) = v["choices"][0]["delta"]["content"].as_str() {
                    raw.push_str(delta);
                    if let Some(ref mut cb) = on_token {
                        cb(delta);
                    }
                }
            }
        }
    }

    Ok(super::llm::postprocess(&raw, src, tgt))
}
