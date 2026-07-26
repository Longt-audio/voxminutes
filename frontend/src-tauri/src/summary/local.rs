//! Local meeting-summary generation via the `llama-helper` sidecar: a
//! stdin/stdout JSON-lines subprocess over llama-cpp-2 (no token streaming —
//! one `{"type":"response", text, error}` line per completed generation).
//!
//! The sidecar process itself is managed by the shared `llama_sidecar`
//! module (single long-lived child, respawned lazily when it died).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Runtime};

use super::client::{emit_event, CANCEL};
use crate::llama_sidecar::{self, GenerateParams};

/// Pick the summary model directory for a request. An explicit `model_id`
/// must be one of `SUMMARY_MODEL_IDS`; without one, fall back to the first
/// installed model in priority order.
fn resolve_model_dir(model_id: Option<&str>) -> Result<&'static str, String> {
    match model_id {
        Some(id) => {
            if !crate::model_download::SUMMARY_MODEL_IDS.contains(&id) {
                return Err(format!("未知的本地总结模型: {}", id));
            }
            crate::model_download::summary_model_dir_name(id)
                .ok_or_else(|| format!("未知的本地总结模型: {}", id))
        }
        None => crate::model_download::SUMMARY_MODEL_IDS
            .iter()
            .find(|id| crate::model_download::summary_model_installed(id))
            .and_then(|id| crate::model_download::summary_model_dir_name(id))
            .ok_or_else(|| "本地总结模型未下载，请先在设置页下载".to_string()),
    }
}

/// 按模型目录名判断模型家族，用于选择聊天模板。
/// 注册表目录名约定：qwen2.5-3b-instruct / qwen3-4b-instruct-2507 / gemma-3-4b-it。
fn model_family(dir_name: &str) -> &'static str {
    if dir_name.contains("gemma") {
        "gemma"
    } else {
        // qwen 及未知家族默认走 ChatML（<|im_start|>）模板
        "qwen"
    }
}

/// 裸 prompt（模板 + 转写拼接）包装为模型对应的聊天模板，并给出停止 token。
/// 实测：不包装时 Qwen 系列会在总结完成后继续复读自身输出直到 max_tokens。
fn wrap_chat_prompt(dir_name: &str, prompt: &str) -> (String, Vec<String>) {
    match model_family(dir_name) {
        "gemma" => (
            format!("<start_of_turn>user\n{prompt}<end_of_turn>\n<start_of_turn>model\n"),
            vec!["<end_of_turn>".to_string()],
        ),
        _ => (
            format!(
                "<|im_start|>system\nYou are a helpful assistant.<|im_end|>\n<|im_start|>user\n{prompt}<|im_end|>\n<|im_start|>assistant\n"
            ),
            vec!["<|im_end|>".to_string()],
        ),
    }
}

async fn run_local_generation<R: Runtime>(
    app: &AppHandle<R>,
    request_id: &str,
    prompt: &str,
    max_tokens: Option<u32>,
    model_id: Option<&str>,
    cancel: &AtomicBool,
) {
    let dir_name = match resolve_model_dir(model_id) {
        Ok(d) => d,
        Err(msg) => {
            emit_event(app, request_id, "error", msg);
            return;
        }
    };
    let model_path = match llama_sidecar::find_gguf_model(dir_name) {
        Some(p) => p,
        None => {
            emit_event(
                app,
                request_id,
                "error",
                "本地总结模型未下载，请先在设置页下载".to_string(),
            );
            return;
        }
    };
    let helper_exe = match llama_sidecar::resolve_helper_exe() {
        Some(p) => p,
        None => {
            emit_event(
                app,
                request_id,
                "error",
                "本地总结引擎（llama-helper）未找到，请重新安装应用".to_string(),
            );
            return;
        }
    };
    if cancel.load(Ordering::SeqCst) {
        emit_event(app, request_id, "done", String::new());
        return;
    }

    let (wrapped_prompt, stop_tokens) = wrap_chat_prompt(dir_name, prompt);
    let params = GenerateParams {
        model_path: model_path.to_string_lossy().to_string(),
        prompt: wrapped_prompt,
        max_tokens: max_tokens.unwrap_or(2048),
        context_size: 8192,
        temperature: 0.3,
        top_k: 40,
        top_p: 0.9,
        repeat_penalty: None,
        frequency_penalty: None,
        stop_tokens,
        stream: false,
    };
    let handle = tokio::task::spawn_blocking(move || {
        llama_sidecar::blocking_generate(&helper_exe, params, None)
    });

    // Poll for completion or cancellation: the sidecar answers with a single
    // line only when generation finishes, so cancellation cannot interrupt
    // the blocking read — kill the child instead (it respawns next time).
    let mut cancelled = false;
    let result = loop {
        if cancel.load(Ordering::SeqCst) {
            llama_sidecar::kill_helper();
            cancelled = true;
            break Ok(String::new());
        }
        if handle.is_finished() {
            break match handle.await {
                Ok(r) => r,
                Err(e) => Err(format!("本地总结任务失败: {}", e)),
            };
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    };

    if cancelled {
        // Cancellation ends as a normal `done`; the sidecar does not stream,
        // so there is no partial text to deliver.
        emit_event(app, request_id, "done", String::new());
        return;
    }
    match result {
        Ok(text) => {
            // Non-streaming backend, streaming UX: deliver the whole text as
            // one token, then finalize with `done`.
            emit_event(app, request_id, "token", text.clone());
            emit_event(app, request_id, "done", text);
        }
        Err(e) => emit_event(app, request_id, "error", e),
    }
}

/// Start a local (llama-helper) summary generation in the background;
/// returns immediately. Progress is reported via `summary-stream` events
/// keyed by `request_id`, same as `summary_generate`. `model_id` selects one
/// of `model_download::SUMMARY_MODEL_IDS`; `None` picks the first installed.
#[tauri::command]
pub async fn summary_local_generate<R: Runtime>(
    app: AppHandle<R>,
    request_id: String,
    prompt: String,
    max_tokens: Option<u32>,
    model_id: Option<String>,
) -> Result<(), String> {
    let flag = Arc::new(AtomicBool::new(false));
    {
        let mut map = CANCEL.lock().map_err(|e| e.to_string())?;
        map.insert(request_id.clone(), flag.clone());
    }
    tauri::async_runtime::spawn(async move {
        run_local_generation(&app, &request_id, &prompt, max_tokens, model_id.as_deref(), &flag)
            .await;
        if let Ok(mut map) = CANCEL.lock() {
            map.remove(&request_id);
        }
    });
    Ok(())
}
