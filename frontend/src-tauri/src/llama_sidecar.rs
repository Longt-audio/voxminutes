//! Shared `llama-helper` sidecar management: a stdin/stdout JSON-lines
//! subprocess over llama-cpp-2. Non-streaming requests answer with one
//! `{"type":"response", text, error}` line; streaming requests additionally
//! emit `{"type":"token","text":...}` lines as tokens are generated.
//!
//! The helper keeps the GGUF model loaded between requests and exits on its
//! own after an idle timeout, so we keep a single long-lived child process in
//! `HELPER` and respawn it lazily when it died. Meeting summaries and LLM
//! translation share the same process (the helper reloads internally when the
//! model path or context size changes).

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::{Arc, LazyLock, Mutex, OnceLock};
use tauri::{AppHandle, Emitter};

/// Long-lived sidecar process; `None` means "not spawned yet / dead".
static HELPER: LazyLock<Mutex<Option<HelperProcess>>> = LazyLock::new(|| Mutex::new(None));

/// Global app handle for emitting `model-loading` events from model load
/// paths that do not have an AppHandle of their own (OPUS-MT lazy load,
/// llama-helper sidecar stdout forwarding). Registered in lib.rs setup.
static APP_HANDLE: OnceLock<AppHandle> = OnceLock::new();

/// Register the global app handle (called once from lib.rs setup).
pub(crate) fn set_app_handle(app: &AppHandle) {
    let _ = APP_HANDLE.set(app.clone());
}

/// Emit a `model-loading` event for the frontend toast.
/// payload: `{model, phase: "start"|"done"|"error", elapsed_ms? (done only), message?}`.
/// No-op before the app handle is registered (e.g. in tests).
pub(crate) fn emit_model_loading(
    model: &str,
    phase: &str,
    elapsed_ms: Option<u64>,
    message: Option<String>,
) {
    let Some(app) = APP_HANDLE.get() else {
        return;
    };
    let mut payload = serde_json::json!({ "model": model, "phase": phase });
    if let Some(ms) = elapsed_ms {
        payload["elapsed_ms"] = serde_json::json!(ms);
    }
    if let Some(m) = message {
        payload["message"] = serde_json::json!(m);
    }
    if let Err(e) = app.emit("model-loading", &payload) {
        log::warn!("model-loading emit failed: {}", e);
    }
}

struct HelperProcess {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    /// Rolling tail of the helper's stderr (drained on a background thread),
    /// appended to error messages for post-mortem context.
    stderr_tail: Arc<Mutex<String>>,
}

impl HelperProcess {
    fn is_alive(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }

    fn stderr_tail(&self) -> String {
        self.stderr_tail.lock().map(|s| s.clone()).unwrap_or_default()
    }
}

impl Drop for HelperProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

// ── Path resolution ───────────────────────────────────────────────────────────

#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
const TARGET_TRIPLE: &str = "x86_64-pc-windows-msvc";
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
const TARGET_TRIPLE: &str = "aarch64-apple-darwin";
#[cfg(all(target_os = "macos", target_arch = "x86_64"))]
const TARGET_TRIPLE: &str = "x86_64-apple-darwin";
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
const TARGET_TRIPLE: &str = "x86_64-unknown-linux-gnu";
#[cfg(not(any(
    all(target_os = "windows", target_arch = "x86_64"),
    all(target_os = "macos", target_arch = "aarch64"),
    all(target_os = "macos", target_arch = "x86_64"),
    all(target_os = "linux", target_arch = "x86_64")
)))]
const TARGET_TRIPLE: &str = "unknown";

#[cfg(windows)]
const EXE_SUFFIX: &str = ".exe";
#[cfg(not(windows))]
const EXE_SUFFIX: &str = "";

/// Resolve the llama-helper executable, in order:
/// 1. Bundled layouts: `<exe_dir>/llama-helper-<triple>.exe`,
///    `<exe_dir>/binaries/llama-helper-<triple>.exe`, and
///    `<exe_dir>/llama-helper.exe` (Tauri externalBin strips the triple).
/// 2. Dev: `<manifest_dir>/binaries/llama-helper-x86_64-pc-windows-msvc.exe`.
/// 3. Dev: workspace `target/{debug,release}/llama-helper.exe` (the app exe
///    lives in the same profile dir under the workspace `target/`).
pub(crate) fn resolve_helper_exe() -> Option<PathBuf> {
    let sidecar_name = format!("llama-helper-{}{}", TARGET_TRIPLE, EXE_SUFFIX);
    let plain_name = format!("llama-helper{}", EXE_SUFFIX);
    let mut candidates: Vec<PathBuf> = Vec::new();

    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            // 1. Bundled layouts. Tauri externalBin strips the target triple
            //    when bundling, so the sidecar lands next to the app exe as
            //    plain `llama-helper.exe`.
            candidates.push(exe_dir.join(&sidecar_name));
            candidates.push(exe_dir.join("binaries").join(&sidecar_name));
            candidates.push(exe_dir.join(&plain_name));
        }
    }

    // 2. Dev: prebuilt sidecar checked into / copied to src-tauri/binaries/.
    candidates.push(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("binaries")
            .join(format!("llama-helper-x86_64-pc-windows-msvc{}", EXE_SUFFIX)),
    );

    // 3. Dev: workspace target dir (exe is target/{profile}/voxminutes.exe).
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            if let Some(target_dir) = exe_dir
                .parent()
                .filter(|p| p.file_name() == Some(std::ffi::OsStr::new("target")))
            {
                candidates.push(exe_dir.join(&plain_name));
                for profile in ["debug", "release"] {
                    candidates.push(target_dir.join(profile).join(&plain_name));
                }
            }
        }
    }

    candidates.into_iter().find(|p| p.is_file())
}

/// Find the downloaded GGUF for a model directory: the single `*.gguf` file
/// inside `<models>/<dir_name>/`.
pub(crate) fn find_gguf_model(dir_name: &str) -> Option<PathBuf> {
    let dir = crate::sherpa_onnx_engine::commands::resolved_models_dir().join(dir_name);
    let entries = std::fs::read_dir(&dir).ok()?;
    entries
        .flatten()
        .map(|e| e.path())
        .find(|p| p.extension().map(|e| e == "gguf").unwrap_or(false))
}

// ── Sidecar process management ────────────────────────────────────────────────

fn spawn_helper(exe: &Path) -> Result<HelperProcess, String> {
    let mut cmd = Command::new(exe);
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("启动本地推理引擎失败 ({}): {}", exe.display(), e))?;

    let stdin = child.stdin.take().ok_or("无法打开本地推理引擎的 stdin")?;
    let stdout = child.stdout.take().ok_or("无法打开本地推理引擎的 stdout")?;
    let stderr = child.stderr.take();

    // Drain stderr on a background thread so a chatty helper can never
    // deadlock on a full pipe; keep a bounded tail for error messages.
    let stderr_tail: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));
    if let Some(stderr) = stderr {
        let tail = stderr_tail.clone();
        std::thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines() {
                let Ok(line) = line else { break };
                if let Ok(mut buf) = tail.lock() {
                    const MAX_TAIL: usize = 4000;
                    buf.push_str(&line);
                    buf.push('\n');
                    while buf.len() > MAX_TAIL {
                        let mut idx = buf.len() - MAX_TAIL;
                        while !buf.is_char_boundary(idx) {
                            idx += 1;
                        }
                        buf.drain(..idx);
                    }
                }
            }
        });
    }

    Ok(HelperProcess {
        child,
        stdin,
        stdout: BufReader::new(stdout),
        stderr_tail,
    })
}

pub(crate) fn kill_helper() {
    if let Ok(mut guard) = HELPER.lock() {
        // Dropping the process kills the child (see Drop impl).
        let _ = guard.take();
    }
}

/// Best-effort sidecar shutdown on app exit (called from lib.rs RunEvent::Exit).
/// Uses try_lock so a mid-generation helper cannot stall application shutdown;
/// a busy helper keeps running but exits on its own idle timeout.
pub fn shutdown_helper() {
    if let Ok(mut guard) = HELPER.try_lock() {
        if let Some(mut helper) = guard.take() {
            let _ = writeln!(helper.stdin, "{}", r#"{"type":"shutdown"}"#);
            let _ = helper.stdin.flush();
            let _ = helper.child.kill();
        }
    }
}

fn parse_helper_response(line: &str) -> Result<HelperLine, String> {
    let json: serde_json::Value = serde_json::from_str(line.trim())
        .map_err(|e| format!("本地推理引擎返回了无效数据: {}", e))?;
    match json.get("type").and_then(|t| t.as_str()) {
        Some("token") => Ok(HelperLine::Token(
            json.get("text")
                .and_then(|t| t.as_str())
                .unwrap_or_default()
                .to_string(),
        )),
        Some("response") => {
            if let Some(err) = json.get("error").and_then(|e| e.as_str()) {
                if !err.is_empty() {
                    return Ok(HelperLine::Done(Err(format!(
                        "本地推理生成失败: {}",
                        err
                    ))));
                }
            }
            Ok(HelperLine::Done(Ok(json
                .get("text")
                .and_then(|t| t.as_str())
                .unwrap_or_default()
                .to_string())))
        }
        Some("error") => Ok(HelperLine::Done(Err(format!(
            "本地推理引擎错误: {}",
            json.get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("未知错误")
        )))),
        // Model load progress line (may precede any response for a request);
        // forwarded to the frontend as a `model-loading` event.
        Some("model-loading") => {
            let model_path = json
                .get("model_path")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            let model = Path::new(model_path)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(model_path)
                .to_string();
            Ok(HelperLine::ModelLoading {
                model,
                phase: json
                    .get("phase")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
                elapsed_ms: json.get("elapsed_ms").and_then(|v| v.as_u64()),
                message: json
                    .get("message")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
            })
        }
        other => Err(format!("本地推理引擎返回了未知消息: {:?}", other)),
    }
}

/// One parsed stdout line from the helper: an incremental token (streaming
/// requests only), a model load progress notification, or the terminal
/// response/error for the request.
enum HelperLine {
    Token(String),
    ModelLoading {
        model: String,
        phase: String,
        elapsed_ms: Option<u64>,
        message: Option<String>,
    },
    Done(Result<String, String>),
}

/// One generation request to the sidecar (model + prompt + full sampling
/// parameters). Penalties are optional and omitted from the request when
/// `None`, keeping backward compatibility with older helper binaries.
/// `stream` asks the helper to emit `{"type":"token"}` lines as it generates.
pub(crate) struct GenerateParams {
    pub model_path: String,
    pub prompt: String,
    pub max_tokens: u32,
    pub context_size: u32,
    pub temperature: f32,
    pub top_k: i32,
    pub top_p: f32,
    pub repeat_penalty: Option<f32>,
    pub frequency_penalty: Option<f32>,
    pub stop_tokens: Vec<String>,
    pub stream: bool,
}

/// Blocking request/response exchange with the sidecar. Runs on
/// `spawn_blocking`; holds the HELPER lock for the whole exchange so
/// concurrent generations are serialized through the single child process.
/// When `params.stream` is set, each `{"type":"token"}` line is forwarded to
/// `on_token` as it arrives; the final response text is returned either way.
pub(crate) fn blocking_generate(
    helper_exe: &Path,
    params: GenerateParams,
    on_token: Option<&mut dyn FnMut(&str)>,
) -> Result<String, String> {
    let mut guard = HELPER
        .lock()
        .map_err(|e| format!("本地推理引擎状态锁失败: {}", e))?;

    let alive = guard.as_mut().map(|h| h.is_alive()).unwrap_or(false);
    if !alive {
        *guard = Some(spawn_helper(helper_exe)?);
    }

    let result = {
        let helper = guard.as_mut().expect("helper just spawned");
        let mut request = serde_json::json!({
            "type": "generate",
            "prompt": params.prompt,
            "max_tokens": params.max_tokens,
            "context_size": params.context_size,
            "model_path": params.model_path,
            "temperature": params.temperature,
            "top_k": params.top_k,
            "top_p": params.top_p,
            "stop_tokens": params.stop_tokens,
        });
        if let Some(p) = params.repeat_penalty {
            request["repeat_penalty"] = serde_json::json!(p);
        }
        if let Some(p) = params.frequency_penalty {
            request["frequency_penalty"] = serde_json::json!(p);
        }
        if params.stream {
            request["stream"] = serde_json::json!(true);
        }

        let exchange = |helper: &mut HelperProcess,
                        on_token: Option<&mut dyn FnMut(&str)>|
         -> Result<String, String> {
            writeln!(helper.stdin, "{}", request)
                .and_then(|_| helper.stdin.flush())
                .map_err(|e| format!("写入本地推理引擎失败: {}", e))?;
            let mut on_token = on_token;
            let mut line = String::new();
            // 跟踪未配对的 model-loading start：helper 加载失败时不会发 done/error
            // 行（错误走 response error 通道），由这里补发 error 关闭前端的加载提示。
            let mut pending_load: Option<String> = None;
            loop {
                line.clear();
                let n = helper
                    .stdout
                    .read_line(&mut line)
                    .map_err(|e| format!("读取本地推理引擎输出失败: {}", e))?;
                if n == 0 {
                    return Err(format!(
                        "本地推理引擎意外退出。{}",
                        helper.stderr_tail()
                    ));
                }
                match parse_helper_response(&line)? {
                    HelperLine::Token(text) => {
                        if let Some(cb) = on_token.as_deref_mut() {
                            cb(&text);
                        }
                    }
                    HelperLine::ModelLoading {
                        model,
                        phase,
                        elapsed_ms,
                        message,
                    } => {
                        if phase == "start" {
                            pending_load = Some(model.clone());
                        } else {
                            pending_load = None;
                        }
                        emit_model_loading(&model, &phase, elapsed_ms, message);
                    }
                    HelperLine::Done(result) => {
                        if result.is_err() {
                            if let Some(model) = pending_load.take() {
                                emit_model_loading(&model, "error", None, None);
                            }
                        }
                        return result;
                    }
                }
            }
        };
        exchange(helper, on_token)
    };

    if result.is_err() {
        // The process is likely wedged; drop it so the next call respawns.
        *guard = None;
    }
    result
}
