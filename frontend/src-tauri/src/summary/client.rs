//! HTTP client for OpenAI-compatible and Anthropic summary endpoints:
//! connectivity checks, model listing, and SSE streaming generation with
//! per-request cancellation.

use serde::Serialize;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex};
use tauri::{AppHandle, Emitter, Runtime};

use super::SummaryApiConfig;

const ANTHROPIC_VERSION: &str = "2023-06-01";
const DEFAULT_MAX_TOKENS: u32 = 4096;
const DEFAULT_TEMPERATURE: f32 = 0.7;

/// Per-request cancellation flags; presence of an entry means a generation
/// for that request id is running. Entries are removed when the stream ends.
/// Shared with `local.rs` (llama-helper sidecar generation).
pub(crate) static CANCEL: LazyLock<Mutex<HashMap<String, Arc<AtomicBool>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Payload of the `summary-stream` event emitted to the frontend.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SummaryStreamEvent {
    request_id: String,
    /// "token" | "done" | "error"
    kind: String,
    text: String,
}

pub(crate) fn emit_event<R: Runtime>(app: &AppHandle<R>, request_id: &str, kind: &str, text: String) {
    let _ = app.emit(
        "summary-stream",
        SummaryStreamEvent {
            request_id: request_id.to_string(),
            kind: kind.to_string(),
            text,
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

fn apply_auth(
    req: reqwest::RequestBuilder,
    config: &SummaryApiConfig,
) -> reqwest::RequestBuilder {
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

/// Parse a single SSE `data:` payload from either protocol.
fn parse_sse_data(data: &str, is_openai: bool) -> Result<SseAction, String> {
    if is_openai && data == "[DONE]" {
        return Ok(SseAction::Stop);
    }
    let json: serde_json::Value = match serde_json::from_str(data) {
        Ok(v) => v,
        // Keepalive comments and non-JSON payloads are ignored.
        Err(_) => return Ok(SseAction::Skip),
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
        match json.pointer("/choices/0/delta/content").and_then(|c| c.as_str()) {
            // Skip role-only deltas and finish_reason-only chunks.
            Some(s) if !s.is_empty() => Ok(SseAction::Token(s.to_string())),
            _ => Ok(SseAction::Skip),
        }
    } else {
        match json.get("type").and_then(|t| t.as_str()) {
            Some("content_block_delta") => {
                match json.pointer("/delta/text").and_then(|t| t.as_str()) {
                    Some(s) if !s.is_empty() => Ok(SseAction::Token(s.to_string())),
                    _ => Ok(SseAction::Skip),
                }
            }
            Some("message_stop") => Ok(SseAction::Stop),
            // message_start / content_block_start / content_block_stop /
            // message_delta / ping carry no user-visible text.
            _ => Ok(SseAction::Skip),
        }
    }
}

/// Run one streaming completion, invoking `on_token` per text chunk.
/// Returns the full accumulated text. A cancelled stream returns the text
/// accumulated so far (treated as a normal completion by the caller).
async fn stream_completion<F: FnMut(String)>(
    config: &SummaryApiConfig,
    prompt: &str,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
    cancel: &AtomicBool,
    on_token: F,
) -> Result<String, String> {
    use futures_util::StreamExt;

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
    let mut buf: Vec<u8> = Vec::new();
    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.next().await {
        if cancel.load(Ordering::SeqCst) {
            // Cancellation ends as a normal `done` with whatever accumulated.
            return Ok(accumulated);
        }
        let chunk = chunk.map_err(|e| format!("Stream interrupted: {}", e))?;
        buf.extend_from_slice(&chunk);
        // Process complete lines only; a chunk boundary can split a line.
        while let Some(pos) = buf.iter().position(|b| *b == b'\n') {
            let line: Vec<u8> = buf.drain(..=pos).collect();
            let line = String::from_utf8_lossy(&line);
            let line = line.trim();
            if let Some(data) = line.strip_prefix("data:") {
                match parse_sse_data(data.trim(), is_openai)? {
                    SseAction::Skip => {}
                    SseAction::Token(text) => {
                        accumulated.push_str(&text);
                        on_token(text);
                    }
                    SseAction::Stop => return Ok(accumulated),
                }
            }
        }
    }

    // Stream ended without an explicit stop marker (provider closed early).
    Ok(accumulated)
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
    let result =
        stream_completion(config, prompt, max_tokens, temperature, cancel, |text| {
            emit_event(app, request_id, "token", text);
        })
        .await;
    // Exactly one terminal event per request: `done` (full accumulated text,
    // possibly partial on cancellation) or `error` (message).
    match result {
        Ok(full) => emit_event(app, request_id, "done", full),
        Err(err) => emit_event(app, request_id, "error", err),
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
    if config.protocol != "openai" && config.protocol != "anthropic" {
        return Err(format!("Unknown summary protocol: {}", config.protocol));
    }
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
        flag.store(true, Ordering::SeqCst);
    }
    Ok(())
}
