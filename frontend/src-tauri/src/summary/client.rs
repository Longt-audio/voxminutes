//! HTTP client for OpenAI-compatible and Anthropic summary endpoints:
//! connectivity checks, model listing, and SSE streaming generation with
//! per-request cancellation.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex};
use tauri::{AppHandle, Emitter, Runtime};

use super::SummaryApiConfig;

const ANTHROPIC_VERSION: &str = "2023-06-01";
const DEFAULT_MAX_TOKENS: u32 = 4096;
/// 远程总结流的读空闲上限（秒）：超过这么久没有任何 SSE 数据即判链路假死。
/// 思考模式下思维链是持续输出的，所以这个窗口只会在真假死时命中；
/// 取 45s 是给「思考结束 → 正文首 token」之间留足余量。
const STREAM_IDLE_TIMEOUT_SECS: u64 = 45;
const DEFAULT_TEMPERATURE: f32 = 0.7;

/// Per-request cancellation flags; presence of an entry means a generation
/// for that request id is running. Entries are removed when the stream ends.
/// Shared with `local.rs` (llama-helper sidecar generation).
pub(crate) static CANCEL: LazyLock<Mutex<HashMap<String, Arc<AtomicBool>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Payload of the `summary-stream` event emitted to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SummaryStreamEvent {
    request_id: String,
    /// "token" | "done" | "error"
    kind: String,
    text: String,
    /// 仅 kind="done" 时有意义：true = 输出被 max_tokens 截断
    /// （OpenAI finish_reason=length / Anthropic stop_reason=max_tokens）。
    /// DeepSeek 思考模式下思维链算在 completion 配额里，max_tokens 顶满时
    /// 正文被截断——以前用户拿到半截总结毫无提示。default：老版本客户端
    /// 反序列化时没有该字段也不报错（契约见 2026-09-27 前后端约定）。
    #[serde(default)]
    truncated: bool,
}

pub(crate) fn emit_event<R: Runtime>(
    app: &AppHandle<R>,
    request_id: &str,
    kind: &str,
    text: String,
) {
    emit_event_full(app, request_id, kind, text, false);
}

/// done 事件专用：带截断标记（契约：truncated 仅 kind="done" 时有意义）。
pub(crate) fn emit_done<R: Runtime>(
    app: &AppHandle<R>,
    request_id: &str,
    text: String,
    truncated: bool,
) {
    emit_event_full(app, request_id, "done", text, truncated);
}

fn emit_event_full<R: Runtime>(
    app: &AppHandle<R>,
    request_id: &str,
    kind: &str,
    text: String,
    truncated: bool,
) {
    let _ = app.emit(
        "summary-stream",
        SummaryStreamEvent {
            request_id: request_id.to_string(),
            kind: kind.to_string(),
            text,
            truncated,
        },
    );
}

/// Strip trailing slashes so API paths can be appended with a single `/`.
fn base_url(config: &SummaryApiConfig) -> String {
    config.endpoint.trim_end_matches('/').to_string()
}

fn http_client(total_timeout: Option<std::time::Duration>) -> Result<reqwest::Client, String> {
    let mut builder =
        reqwest::Client::builder().connect_timeout(std::time::Duration::from_secs(15));
    if let Some(t) = total_timeout {
        builder = builder.timeout(t);
    }
    builder.build().map_err(|e| e.to_string())
}

fn apply_auth(req: reqwest::RequestBuilder, config: &SummaryApiConfig) -> reqwest::RequestBuilder {
    if config.protocol == "anthropic" {
        req.header("x-api-key", &config.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
    } else if config.api_key.is_empty() {
        req
    } else {
        req.header("Authorization", format!("Bearer {}", config.api_key))
    }
}

/// Truncate an HTTP error body for inclusion in an error message.
fn snippet(body: &str) -> String {
    const MAX_CHARS: usize = 300;
    let trimmed = body.trim();
    if trimmed.chars().count() > MAX_CHARS {
        format!("{}…", trimmed.chars().take(MAX_CHARS).collect::<String>())
    } else {
        trimmed.to_string()
    }
}

/// GET the provider's model list; both protocols answer with `data[].id`.
async fn fetch_models(config: &SummaryApiConfig) -> Result<Vec<String>, String> {
    let url = if config.protocol == "anthropic" {
        format!("{}/v1/models", base_url(config))
    } else {
        format!("{}/models", base_url(config))
    };
    let client = http_client(Some(std::time::Duration::from_secs(15)))?;
    let response = apply_auth(client.get(&url), config)
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(format!("HTTP {}: {}", status.as_u16(), snippet(&body)));
    }
    let body: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Invalid response: {}", e))?;
    let ids = body
        .get("data")
        .and_then(|d| d.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|m| m.get("id").and_then(|i| i.as_str()).map(String::from))
                .collect()
        })
        .unwrap_or_default();
    Ok(ids)
}

#[tauri::command]
pub async fn summary_test_connection(config: SummaryApiConfig) -> Result<String, String> {
    let models = fetch_models(&config).await?;
    Ok(format!("Connection OK ({} models available)", models.len()))
}

#[tauri::command]
pub async fn summary_list_models(config: SummaryApiConfig) -> Result<Vec<String>, String> {
    fetch_models(&config).await
}

/// What to do with one parsed SSE `data:` payload.
enum SseAction {
    Skip,
    Token(String),
    Stop,
}

/// 一个 SSE `data:` 载荷的解析结果。
struct ParsedSse {
    action: SseAction,
    /// true = 上游因 max_tokens 顶满而截断（OpenAI finish_reason=length /
    /// Anthropic message_delta 的 stop_reason=max_tokens）。DeepSeek 思考模式下
    /// 思维链算在 completion 配额里，顶满时正文被截断——必须捕获并透传给前端，
    /// 否则用户拿到半截总结毫无提示。
    truncated: bool,
}

/// Parse a single SSE `data:` payload from either protocol.
fn parse_sse_data(data: &str, is_openai: bool) -> Result<ParsedSse, String> {
    let skip = |truncated: bool| ParsedSse {
        action: SseAction::Skip,
        truncated,
    };
    if is_openai && data == "[DONE]" {
        return Ok(ParsedSse {
            action: SseAction::Stop,
            truncated: false,
        });
    }
    let json: serde_json::Value = match serde_json::from_str(data) {
        Ok(v) => v,
        // Keepalive comments and non-JSON payloads are ignored.
        Err(_) => return Ok(skip(false)),
    };
    // Provider error payloads (both protocols) surface as stream errors.
    if let Some(err) = json.get("error") {
        if !err.is_null() {
            let msg = err
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("unknown provider error");
            return Err(format!("Provider error: {}", msg));
        }
    }
    if is_openai {
        // finish_reason 通常出现在最后一个（空 delta）chunk 上，与 content 分开判断，
        // 兼容「content 与 finish_reason 同帧」的上游
        let truncated = json
            .pointer("/choices/0/finish_reason")
            .and_then(|f| f.as_str())
            == Some("length");
        match json
            .pointer("/choices/0/delta/content")
            .and_then(|c| c.as_str())
        {
            // Skip role-only deltas and finish_reason-only chunks.
            Some(s) if !s.is_empty() => Ok(ParsedSse {
                action: SseAction::Token(s.to_string()),
                truncated,
            }),
            _ => Ok(skip(truncated)),
        }
    } else {
        match json.get("type").and_then(|t| t.as_str()) {
            Some("content_block_delta") => {
                match json.pointer("/delta/text").and_then(|t| t.as_str()) {
                    Some(s) if !s.is_empty() => Ok(ParsedSse {
                        action: SseAction::Token(s.to_string()),
                        truncated: false,
                    }),
                    _ => Ok(skip(false)),
                }
            }
            Some("message_stop") => Ok(ParsedSse {
                action: SseAction::Stop,
                truncated: false,
            }),
            // message_delta 携带 delta.stop_reason：max_tokens = 输出被截断
            Some("message_delta") => Ok(skip(
                json.pointer("/delta/stop_reason").and_then(|s| s.as_str())
                    == Some("max_tokens"),
            )),
            // message_start / content_block_start / content_block_stop /
            // ping carry no user-visible text.
            _ => Ok(skip(false)),
        }
    }
}

/// 一次流式生成的结果：正文 + 是否被 max_tokens 截断。
/// truncated 只反映上游的 finish_reason/stop_reason；**取消不算截断**
/// （取消路径提前返回时该字段恒为 false，避免误标）。
pub(crate) struct StreamOutput {
    pub text: String,
    pub truncated: bool,
}

/// 远程总结：走自建网关 /v1/chat/completions（OpenAI 兼容 SSE），复用远程服务
/// 的 server_url + license，不接触上游 API key。
async fn stream_completion_gateway<F: FnMut(String), G: FnMut(String)>(
    config: &SummaryApiConfig,
    prompt: &str,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
    cancel: &AtomicBool,
    mut on_token: F,
    mut on_thinking: Option<G>,
) -> Result<StreamOutput, String> {
    use futures_util::StreamExt;
    let api_base = crate::audio::transcription::remote_api_base()
        .ok_or_else(|| "远程服务未配置（缺少服务器地址）".to_string())?;
    let license = crate::audio::transcription::get_remote_license();
    if license.is_empty() {
        return Err("远程服务未配置（缺少授权码）".to_string());
    }

    let body = serde_json::json!({
        "model": config.model,
        // 标明用途：网关据此在积分流水里写「会议总结 · 模型名」（不上行到上游）
        "feature": "summary",
        "stream": true,
        "max_tokens": max_tokens.unwrap_or(DEFAULT_MAX_TOKENS),
        "temperature": temperature.unwrap_or(DEFAULT_TEMPERATURE),
        "messages": [{ "role": "user", "content": prompt }],
    });

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/chat/completions", api_base))
        .bearer_auth(&license)
        // 任务会话（2026-09-22）：一次总结 = 网关侧一个独立任务，
        // 这样用户中心能把「会议总结」和逐句翻译分开显示。见 src/task_session.rs。
        .header("x-vox-session", crate::task_session::current_session())
        // 请求 id：总结是单笔大额调用，超时重发的重复扣费最容易被用户发现
        .header("x-vox-request-id", crate::task_session::new_request_id())
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("远程总结请求失败: {}", e))?;
    let status = resp.status();
    if !status.is_success() {
        let detail = resp.text().await.unwrap_or_default();
        return Err(format!("远程总结错误 (HTTP {}): {}", status, detail));
    }

    let mut accumulated = String::new();
    // 上游是否因 max_tokens 顶满截断（finish_reason=length）。
    let mut truncated = false;
    let mut buf = String::new();
    let mut stream = resp.bytes_stream();
    loop {
        // 读空闲看门狗（2026-09-24）：远程总结原来没有任何读超时 —— 链路假死时
        // 界面会一直停在「正在生成…」（用户实测过一次「没有结果」）。
        // 思考模式的思维链是持续输出的，所以 45s 无任何数据基本等于链路/上游假死。
        let next = match tokio::time::timeout(
            std::time::Duration::from_secs(STREAM_IDLE_TIMEOUT_SECS),
            stream.next(),
        )
        .await
        {
            Ok(n) => n,
            Err(_) => {
                return Err(format!(
                    "远程总结超时（{}s 无数据，疑似链路假死）",
                    STREAM_IDLE_TIMEOUT_SECS
                ))
            }
        };
        let Some(chunk) = next else { break };
        if cancel.load(Ordering::SeqCst) {
            // 取消不算截断：truncated 恒为 false
            return Ok(StreamOutput {
                text: accumulated,
                truncated: false,
            });
        }
        let chunk = chunk.map_err(|e| format!("远程总结流错误: {}", e))?;
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
                return Ok(StreamOutput {
                    text: accumulated,
                    truncated,
                });
            }
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(data) {
                // max_tokens 顶满时正文被截断（思考模式下思维链会吃掉输出预算）
                if v["choices"][0]["finish_reason"].as_str() == Some("length") {
                    truncated = true;
                }
                // 思考模式的思维链走 `reasoning_content`，与 `content` **同级**
                // （DeepSeek / MiMo 官方口径）。以前这里只读 content、把思维链整个丢掉，
                // 于是「会议总结开启思考」时前 10 多秒界面**毫无输出**，用户以为卡死。
                // 现在把思维链作为独立的 thinking 流上报，让界面能显示"正在思考"。
                if let Some(think) = v["choices"][0]["delta"]["reasoning_content"].as_str() {
                    if !think.is_empty() {
                        if let Some(cb) = on_thinking.as_mut() {
                            cb(think.to_string());
                        }
                    }
                }
                if let Some(delta) = v["choices"][0]["delta"]["content"].as_str() {
                    let delta = delta.to_string();
                    accumulated.push_str(&delta);
                    on_token(delta);
                }
            }
        }
    }
    Ok(StreamOutput {
        text: accumulated,
        truncated,
    })
}

/// Run one streaming completion, invoking `on_token` per text chunk.
/// Returns the full accumulated text plus the upstream truncation flag.
/// A cancelled stream returns the text accumulated so far (treated as a normal
/// completion by the caller; truncated is always false on cancellation).
/// pub(crate)：translation/custom_api.rs 的自定义 API 翻译复用同一套
/// OpenAI 兼容 / Anthropic SSE 协议实现。
pub(crate) async fn stream_completion<F: FnMut(String), G: FnMut(String)>(
    config: &SummaryApiConfig,
    prompt: &str,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
    cancel: &AtomicBool,
    on_token: F,
    on_thinking: Option<G>,
) -> Result<StreamOutput, String> {
    use futures_util::StreamExt;

    if config.protocol == "gateway" {
        return stream_completion_gateway(
            config,
            prompt,
            max_tokens,
            temperature,
            cancel,
            on_token,
            on_thinking,
        )
        .await;
    }
    // 非 gateway 协议（直连上游）目前不区分思维链：那里也没有"思考模式"开关，
    // 思维链若存在会混在 content 里，行为与改造前一致。
    drop(on_thinking);

    let is_openai = config.protocol == "openai";
    let (url, body) = if is_openai {
        (
            format!("{}/chat/completions", base_url(config)),
            serde_json::json!({
                "model": config.model,
                "stream": true,
                "max_tokens": max_tokens.unwrap_or(DEFAULT_MAX_TOKENS),
                "temperature": temperature.unwrap_or(DEFAULT_TEMPERATURE),
                "messages": [{ "role": "user", "content": prompt }],
            }),
        )
    } else {
        let mut body = serde_json::json!({
            "model": config.model,
            "stream": true,
            "max_tokens": max_tokens.unwrap_or(DEFAULT_MAX_TOKENS),
            "messages": [{ "role": "user", "content": prompt }],
        });
        if let Some(t) = temperature {
            body["temperature"] = serde_json::json!(t);
        }
        (format!("{}/v1/messages", base_url(config)), body)
    };

    // No total timeout: generation streams may legitimately run for minutes.
    let client = http_client(None)?;
    let response = apply_auth(client.post(&url), config)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(format!("HTTP {}: {}", status.as_u16(), snippet(&body)));
    }

    let mut on_token = on_token;
    let mut accumulated = String::new();
    // 上游是否因 max_tokens 顶满截断（OpenAI finish_reason=length /
    // Anthropic stop_reason=max_tokens，见 parse_sse_data）
    let mut truncated = false;
    let mut buf: Vec<u8> = Vec::new();
    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.next().await {
        if cancel.load(Ordering::SeqCst) {
            // Cancellation ends as a normal `done` with whatever accumulated.
            // 取消不算截断：truncated 恒为 false
            return Ok(StreamOutput {
                text: accumulated,
                truncated: false,
            });
        }
        let chunk = chunk.map_err(|e| format!("Stream interrupted: {}", e))?;
        buf.extend_from_slice(&chunk);
        // Process complete lines only; a chunk boundary can split a line.
        while let Some(pos) = buf.iter().position(|b| *b == b'\n') {
            let line: Vec<u8> = buf.drain(..=pos).collect();
            let line = String::from_utf8_lossy(&line);
            let line = line.trim();
            if let Some(data) = line.strip_prefix("data:") {
                let parsed = parse_sse_data(data.trim(), is_openai)?;
                truncated |= parsed.truncated;
                match parsed.action {
                    SseAction::Skip => {}
                    SseAction::Token(text) => {
                        accumulated.push_str(&text);
                        on_token(text);
                    }
                    SseAction::Stop => {
                        return Ok(StreamOutput {
                            text: accumulated,
                            truncated,
                        });
                    }
                }
            }
        }
    }

    // Stream ended without an explicit stop marker (provider closed early).
    Ok(StreamOutput {
        text: accumulated,
        truncated,
    })
}

async fn run_generation<R: Runtime>(
    app: &AppHandle<R>,
    request_id: &str,
    config: &SummaryApiConfig,
    prompt: &str,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
    cancel: &AtomicBool,
) {
    let started = std::time::Instant::now();
    let mut first_token_logged = false;
    let result = stream_completion(
        config,
        prompt,
        max_tokens,
        temperature,
        cancel,
        |text| {
            // 首 token 延迟：排查「点了总结半天没动静」时区分是建连慢还是生成慢
            if !first_token_logged {
                first_token_logged = true;
                log::info!(
                    "⏱️ 总结收到首个 token request_id={} 距开始 {:.1}s",
                    request_id,
                    started.elapsed().as_secs_f64()
                );
            }
            emit_event(app, request_id, "token", text);
        },
        // 思维链单独发一类事件（kind="thinking"）：界面可因此显示"正在思考…"，
        // 而不是在思考期间完全空白（DeepSeek/MiMo 思考模式下会长达十几秒）。
        Some(|text: String| {
            emit_event(app, request_id, "thinking", text);
        }),
    )
    .await;
    // Exactly one terminal event per request: `done` (full accumulated text,
    // possibly partial on cancellation) or `error` (message).
    // done 事件带截断标记（契约：truncated 仅 kind="done" 时有意义）。
    match result {
        Ok(out) => {
            let chars = out.text.chars().count();
            let elapsed = started.elapsed().as_secs_f64();
            if cancel.load(Ordering::SeqCst) {
                log::info!(
                    "⏹️ 总结已取消 request_id={} 已生成正文 {} 字 耗时 {:.1}s",
                    request_id,
                    chars,
                    elapsed
                );
            } else {
                log::info!(
                    "✅ 总结完成 request_id={} 正文 {} 字 耗时 {:.1}s 空正文={}",
                    request_id,
                    chars,
                    elapsed,
                    out.text.trim().is_empty()
                );
            }
            if out.truncated {
                // 排查用：思考模式下思维链算在 completion 配额里，max_tokens 顶满时
                // 正文被截断，没有这行日志只能从「总结看起来像被砍了一半」反推。
                log::warn!(
                    "⚠️ 总结输出被 max_tokens 截断 request_id={} 已生成正文 {} 字（finish_reason=length / stop_reason=max_tokens）",
                    request_id,
                    out.text.chars().count()
                );
            }
            emit_done(app, request_id, out.text, out.truncated);
        }
        Err(err) => {
            log::warn!(
                "⚠️ 总结生成失败 request_id={} 耗时 {:.1}s: {}",
                request_id,
                started.elapsed().as_secs_f64(),
                err
            );
            emit_event(app, request_id, "error", err);
        }
    }
}

/// Start a streaming generation in the background; returns immediately.
/// Progress is reported via `summary-stream` events keyed by `request_id`.
#[tauri::command]
pub async fn summary_generate<R: Runtime>(
    app: AppHandle<R>,
    request_id: String,
    config: SummaryApiConfig,
    prompt: String,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
) -> Result<(), String> {
    if config.protocol != "openai" && config.protocol != "anthropic" && config.protocol != "gateway"
    {
        return Err(format!("Unknown summary protocol: {}", config.protocol));
    }
    // 入口日志只记模型/协议与 prompt 长度，不打 prompt 正文与授权信息
    log::info!(
        "📝 远程总结开始 request_id={} 协议={} 模型={} prompt {} 字",
        request_id,
        config.protocol,
        config.model,
        prompt.chars().count()
    );
    let flag = Arc::new(AtomicBool::new(false));
    {
        let mut map = CANCEL.lock().map_err(|e| e.to_string())?;
        map.insert(request_id.clone(), flag.clone());
    }
    tauri::async_runtime::spawn(async move {
        run_generation(
            &app,
            &request_id,
            &config,
            &prompt,
            max_tokens,
            temperature,
            &flag,
        )
        .await;
        if let Ok(mut map) = CANCEL.lock() {
            map.remove(&request_id);
        }
    });
    Ok(())
}

/// Cancel an in-flight generation (best effort; takes effect on the next
/// streamed chunk). Idempotent: cancelling an unknown request is a no-op.
#[tauri::command]
pub fn summary_cancel(request_id: String) -> Result<(), String> {
    let map = CANCEL.lock().map_err(|e| e.to_string())?;
    if let Some(flag) = map.get(&request_id) {
        log::info!("⏹️ 收到总结取消请求 request_id={}", request_id);
        flag.store(true, Ordering::SeqCst);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openai_finish_reason_length_marks_truncated() {
        // 思考模式顶满 max_tokens 的典型收尾 chunk：空 delta + finish_reason=length
        let p = parse_sse_data(
            r#"{"choices":[{"delta":{},"finish_reason":"length"}]}"#,
            true,
        )
        .unwrap();
        assert!(p.truncated);
        assert!(matches!(p.action, SseAction::Skip));
        // 正常收尾不标截断
        let p = parse_sse_data(
            r#"{"choices":[{"delta":{},"finish_reason":"stop"}]}"#,
            true,
        )
        .unwrap();
        assert!(!p.truncated);
        // content 与 finish_reason 同帧时两者都不能丢
        let p = parse_sse_data(
            r#"{"choices":[{"delta":{"content":"尾"},"finish_reason":"length"}]}"#,
            true,
        )
        .unwrap();
        assert!(p.truncated);
        match p.action {
            SseAction::Token(t) => assert_eq!(t, "尾"),
            _ => panic!("应为 Token"),
        }
    }

    #[test]
    fn anthropic_max_tokens_marks_truncated() {
        let p = parse_sse_data(
            r#"{"type":"message_delta","delta":{"stop_reason":"max_tokens"}}"#,
            false,
        )
        .unwrap();
        assert!(p.truncated);
        // 正常结束不标截断
        let p = parse_sse_data(
            r#"{"type":"message_delta","delta":{"stop_reason":"end_turn"}}"#,
            false,
        )
        .unwrap();
        assert!(!p.truncated);
        let p = parse_sse_data(r#"{"type":"message_stop"}"#, false).unwrap();
        assert!(matches!(p.action, SseAction::Stop));
        assert!(!p.truncated);
    }

    #[test]
    fn openai_token_chunks_are_not_truncated() {
        let p = parse_sse_data(r#"{"choices":[{"delta":{"content":"你好"}}]}"#, true).unwrap();
        assert!(!p.truncated);
        match p.action {
            SseAction::Token(t) => assert_eq!(t, "你好"),
            _ => panic!("应为 Token"),
        }
        let p = parse_sse_data("[DONE]", true).unwrap();
        assert!(matches!(p.action, SseAction::Stop));
        assert!(!p.truncated);
    }
}
