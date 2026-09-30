// translation/custom_api.rs
//
// 自定义 API 翻译（custom-api 引擎）：使用用户在会议总结 API 设置中配置的
// OpenAI 兼容 / Anthropic 端点（settings 表 key `summary.api_config`，总结与
// 翻译共享同一份配置）做流式翻译。底层协议（URL 拼接、鉴权头、SSE 解析）
// 直接复用 summary/client.rs 的 stream_completion，prompt 构建与输出清洗
// 复用 llm.rs，与 remote.rs（网关翻译）保持同一套语义。

use std::sync::atomic::AtomicBool;

use crate::summary::SummaryApiConfig;

/// 用自定义 API 配置翻译一段文本。
/// `on_token` 提供时按流式增量回调原始 delta（未清洗），返回值始终是
/// postprocess 清洗后的完整译文（与 remote::translate_remote 一致）。
pub async fn translate_custom_api(
    config: &SummaryApiConfig,
    text: &str,
    direction: &str,
    asr_mode: bool,
    context: Option<&str>,
    mut on_token: Option<&mut (dyn FnMut(&str) + Send)>,
) -> Result<String, String> {
    let Some((src, tgt)) = super::llm::parse_direction(direction) else {
        return Err(format!("不支持的翻译方向: {}", direction));
    };
    if config.endpoint.trim().is_empty() {
        return Err("自定义 API 未配置端点，请先在会议总结的 API 设置中填写端点地址。".to_string());
    }
    if !matches!(config.protocol.as_str(), "openai" | "anthropic") {
        return Err(format!(
            "自定义 API 协议不支持（{}），仅支持 OpenAI 兼容 / Anthropic。",
            config.protocol
        ));
    }

    let prompt = super::llm::build_prompt(text, src, tgt, asr_mode, context);
    // 与 remote.rs 一致：按原文长度估算 max_tokens，低温采样
    let max_tokens = Some((text.chars().count() * 2).clamp(64, 1024) as u32);
    let cancel = AtomicBool::new(false);
    let out = crate::summary::client::stream_completion(
        config,
        &prompt,
        max_tokens,
        Some(0.3),
        &cancel,
        |delta| {
            if let Some(ref mut cb) = on_token {
                cb(&delta);
            }
        },
        // 自定义 API 翻译不展示思维链（那是总结页的体验需求）；
        // 这里传 None，行为与改造前一致。
        None::<fn(String)>,
    )
    .await?;
    // out.truncated 这里不单独处理：翻译按原文长度估 max_tokens，截断时译文
    // 仍会作为定稿落地（比没有强）；空译文的兜底告警在 translation/mod.rs。

    Ok(super::llm::postprocess(&out.text, src, tgt))
}
