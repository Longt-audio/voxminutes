//! In-app ASR model download / extraction / management.
//!
//! The shipped app contains no model files. The settings UI lists the
//! downloadable models below; when the user picks one we try each of its
//! download sources in order (emitting `model-download-progress` events),
//! extract/copy the files into the models directory, verify the required
//! files, and the ASR engine can then load it via `sherpa_onnx_load_model`.
//!
//! Models can also be installed from a local file/folder via
//! `import_model_file` (archive, single GGUF file, or a prepared folder).

use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex};
use tauri::{AppHandle, Emitter, Runtime};
use tauri_plugin_dialog::DialogExt;

// ── Model registry ────────────────────────────────────────────────────────────

/// How a model's files are obtained from one source. A model has an ordered
/// list of sources; they are tried in order until one succeeds.
enum ModelSource {
    /// Single .tar.bz2 archive, extracted after download.
    Archive { url: &'static str },
    /// Multiple individual files downloaded from a base URL, no extraction.
    /// The full URL is `{base_url}/{resolve_path}/{file}` — HuggingFace uses
    /// `resolve/main`, ModelScope uses `resolve/master`.
    Files {
        base_url: &'static str,
        resolve_path: &'static str,
        files: &'static [&'static str],
    },
}

struct DownloadableModel {
    /// Stable identifier used by the frontend and by `sherpa_onnx_load_model`.
    id: &'static str,
    display_name: &'static str,
    /// Directory name inside models/ after installation.
    dir_name: &'static str,
    /// Download sources, tried in order (official first, mirrors as fallback).
    sources: &'static [ModelSource],
    /// Approximate total size, used as the progress fallback.
    size_bytes: u64,
    /// Files that must exist in dir_name after installation.
    required_files: &'static [&'static str],
}

const OPUS_MT_FILES: &[&str] = &[
    "onnx/encoder_model_int8.onnx",
    "onnx/decoder_model_merged_int8.onnx",
    "source.spm",
    "target.spm",
    "tokenizer.json",
    "generation_config.json",
];

/// HuggingFace-style resolve path (also used by hf-mirror.com).
const HF_RESOLVE: &str = "resolve/main";
/// ModelScope resolve path.
const MS_RESOLVE: &str = "resolve/master";

const MODELS: &[DownloadableModel] = &[
    DownloadableModel {
        id: "sense-voice",
        display_name: "SenseVoice 多语言模型（中/英/日/韩/粤）",
        dir_name: "sherpa-onnx-sense-voice-zh-en-ja-ko-yue-2024-07-17",
        sources: &[
            ModelSource::Archive {
                url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-sense-voice-zh-en-ja-ko-yue-2024-07-17.tar.bz2",
            },
            ModelSource::Archive {
                url: "https://gh-proxy.com/https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-sense-voice-zh-en-ja-ko-yue-2024-07-17.tar.bz2",
            },
        ],
        size_bytes: 896_000_000,
        required_files: &["model.onnx", "tokens.txt"],
    },
    DownloadableModel {
        id: "x-asr-480ms",
        display_name: "X-ASR 流式模型（中英，带标点，480ms）",
        dir_name: "sherpa-onnx-x-asr-480ms-streaming-zipformer-transducer-zh-en-punct-2026-06-05",
        sources: &[
            ModelSource::Archive {
                url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-x-asr-480ms-streaming-zipformer-transducer-zh-en-punct-2026-06-05.tar.bz2",
            },
            ModelSource::Archive {
                url: "https://gh-proxy.com/https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-x-asr-480ms-streaming-zipformer-transducer-zh-en-punct-2026-06-05.tar.bz2",
            },
        ],
        size_bytes: 584_100_000,
        required_files: &["encoder.onnx", "decoder.onnx", "joiner.onnx", "tokens.txt"],
    },
    DownloadableModel {
        id: "opus-mt-zh-en",
        display_name: "OPUS-MT 翻译模型（中 → 英）",
        dir_name: "opus-mt-zh-en",
        sources: &[
            ModelSource::Files {
                base_url: "https://huggingface.co/Xenova/opus-mt-zh-en",
                resolve_path: HF_RESOLVE,
                files: OPUS_MT_FILES,
            },
            ModelSource::Files {
                base_url: "https://hf-mirror.com/Xenova/opus-mt-zh-en",
                resolve_path: HF_RESOLVE,
                files: OPUS_MT_FILES,
            },
            ModelSource::Files {
                base_url: "https://modelscope.cn/models/Xenova/opus-mt-zh-en",
                resolve_path: MS_RESOLVE,
                files: OPUS_MT_FILES,
            },
        ],
        size_bytes: 120_000_000,
        required_files: &["encoder_model_int8.onnx", "decoder_model_merged_int8.onnx", "tokenizer.json"],
    },
    DownloadableModel {
        id: "opus-mt-en-zh",
        display_name: "OPUS-MT 翻译模型（英 → 中）",
        dir_name: "opus-mt-en-zh",
        sources: &[
            ModelSource::Files {
                base_url: "https://huggingface.co/Xenova/opus-mt-en-zh",
                resolve_path: HF_RESOLVE,
                files: OPUS_MT_FILES,
            },
            ModelSource::Files {
                base_url: "https://hf-mirror.com/Xenova/opus-mt-en-zh",
                resolve_path: HF_RESOLVE,
                files: OPUS_MT_FILES,
            },
            // ModelScope URL verified 2026-07-26 via
            // `curl -sIL "https://modelscope.cn/models/Xenova/opus-mt-en-zh/resolve/master/onnx/encoder_model_int8.onnx"` → 200.
            ModelSource::Files {
                base_url: "https://modelscope.cn/models/Xenova/opus-mt-en-zh",
                resolve_path: MS_RESOLVE,
                files: OPUS_MT_FILES,
            },
        ],
        size_bytes: 120_000_000,
        required_files: &["encoder_model_int8.onnx", "decoder_model_merged_int8.onnx", "tokenizer.json"],
    },
    // Local meeting-summary LLM (GGUF, runs via the llama helper).
    DownloadableModel {
        id: "qwen2.5-3b-instruct-q4_k_m",
        display_name: "Qwen2.5-3B-Instruct（会议总结）",
        dir_name: "qwen2.5-3b-instruct",
        sources: &[
            ModelSource::Files {
                base_url: "https://huggingface.co/Qwen/Qwen2.5-3B-Instruct-GGUF",
                resolve_path: HF_RESOLVE,
                files: &["qwen2.5-3b-instruct-q4_k_m.gguf"],
            },
            ModelSource::Files {
                base_url: "https://hf-mirror.com/Qwen/Qwen2.5-3B-Instruct-GGUF",
                resolve_path: HF_RESOLVE,
                files: &["qwen2.5-3b-instruct-q4_k_m.gguf"],
            },
            // ModelScope URL verified 2026-07-26 via
            // `curl -sIL "https://modelscope.cn/models/Qwen/Qwen2.5-3B-Instruct-GGUF/resolve/master/qwen2.5-3b-instruct-q4_k_m.gguf"` → 200.
            ModelSource::Files {
                base_url: "https://modelscope.cn/models/Qwen/Qwen2.5-3B-Instruct-GGUF",
                resolve_path: MS_RESOLVE,
                files: &["qwen2.5-3b-instruct-q4_k_m.gguf"],
            },
        ],
        // Real size: final content-length of the HF resolve URL, verified
        // 2026-07-21 via `curl -sIL "https://huggingface.co/Qwen/Qwen2.5-3B-Instruct-GGUF/resolve/main/qwen2.5-3b-instruct-q4_k_m.gguf"`.
        size_bytes: 2_104_932_768,
        required_files: &["qwen2.5-3b-instruct-q4_k_m.gguf"],
    },
    // Local meeting-summary LLM. Note: there is NO official
    // `Qwen/Qwen3-4B-Instruct-2507-GGUF` repo (verified 2026-07-21: hf-mirror
    // returns the same 401/404 pattern as for a nonexistent repo), so this
    // uses bartowski's ungated mirror.
    DownloadableModel {
        id: "qwen3-4b-instruct-2507-q4_k_m",
        display_name: "Qwen3-4B-Instruct-2507（会议总结）",
        dir_name: "qwen3-4b-instruct-2507",
        sources: &[
            ModelSource::Files {
                base_url: "https://huggingface.co/bartowski/Qwen_Qwen3-4B-Instruct-2507-GGUF",
                resolve_path: HF_RESOLVE,
                files: &["Qwen_Qwen3-4B-Instruct-2507-Q4_K_M.gguf"],
            },
            ModelSource::Files {
                base_url: "https://hf-mirror.com/bartowski/Qwen_Qwen3-4B-Instruct-2507-GGUF",
                resolve_path: HF_RESOLVE,
                files: &["Qwen_Qwen3-4B-Instruct-2507-Q4_K_M.gguf"],
            },
        ],
        // Real size: final content-length of the resolve URL, verified
        // 2026-07-21 via `curl -sIL "https://hf-mirror.com/bartowski/Qwen_Qwen3-4B-Instruct-2507-GGUF/resolve/main/Qwen_Qwen3-4B-Instruct-2507-Q4_K_M.gguf"`.
        size_bytes: 2_497_280_736,
        required_files: &["Qwen_Qwen3-4B-Instruct-2507-Q4_K_M.gguf"],
    },
    // Local meeting-summary LLM. Google's official gemma repos are
    // license-gated; bartowski's mirror (renamed with a `google_` prefix)
    // is ungated.
    DownloadableModel {
        id: "gemma-3-4b-it-q4_k_m",
        display_name: "Gemma-3-4B-it（会议总结）",
        dir_name: "gemma-3-4b-it",
        sources: &[
            ModelSource::Files {
                base_url: "https://huggingface.co/bartowski/google_gemma-3-4b-it-GGUF",
                resolve_path: HF_RESOLVE,
                files: &["google_gemma-3-4b-it-Q4_K_M.gguf"],
            },
            ModelSource::Files {
                base_url: "https://hf-mirror.com/bartowski/google_gemma-3-4b-it-GGUF",
                resolve_path: HF_RESOLVE,
                files: &["google_gemma-3-4b-it-Q4_K_M.gguf"],
            },
        ],
        // Real size: final content-length of the resolve URL, verified
        // 2026-07-21 via `curl -sIL "https://hf-mirror.com/bartowski/google_gemma-3-4b-it-GGUF/resolve/main/google_gemma-3-4b-it-Q4_K_M.gguf"`.
        size_bytes: 2_489_758_112,
        required_files: &["google_gemma-3-4b-it-Q4_K_M.gguf"],
    },
    // Hy-MT2 LLM translation model (GGUF, runs via the llama helper).
    DownloadableModel {
        id: "hy-mt2-1.8b-q4_k_m",
        display_name: "Hy-MT2-1.8B（高质量翻译）",
        dir_name: "hy-mt2-1.8b",
        sources: &[
            ModelSource::Files {
                base_url: "https://huggingface.co/tencent/Hy-MT2-1.8B-GGUF",
                resolve_path: HF_RESOLVE,
                files: &["Hy-MT2-1.8B-Q4_K_M.gguf"],
            },
            ModelSource::Files {
                base_url: "https://hf-mirror.com/tencent/Hy-MT2-1.8B-GGUF",
                resolve_path: HF_RESOLVE,
                files: &["Hy-MT2-1.8B-Q4_K_M.gguf"],
            },
        ],
        // Real size: final content-length of the resolve URL, verified
        // 2026-07-25 via `curl -sIL "https://huggingface.co/tencent/Hy-MT2-1.8B-GGUF/resolve/main/Hy-MT2-1.8B-Q4_K_M.gguf"`.
        size_bytes: 1_133_080_448,
        required_files: &["Hy-MT2-1.8B-Q4_K_M.gguf"],
    },
];

/// Meeting-summary (local LLM) model ids in priority order. The summary
/// module uses this list to pick the default local model (first installed).
pub const SUMMARY_MODEL_IDS: &[&str] = &[
    "qwen2.5-3b-instruct-q4_k_m",
    "qwen3-4b-instruct-2507-q4_k_m",
    "gemma-3-4b-it-q4_k_m",
];

/// Directory name under the resolved models dir for a summary model id.
pub(crate) fn summary_model_dir_name(model_id: &str) -> Option<&'static str> {
    MODELS.iter().find(|m| m.id == model_id).map(|m| m.dir_name)
}

/// Whether all required files of the given model are installed.
pub(crate) fn summary_model_installed(model_id: &str) -> bool {
    match find_model(model_id) {
        Some(m) => is_installed(
            &crate::sherpa_onnx_engine::commands::resolved_models_dir(),
            m,
        ),
        None => false,
    }
}

/// Hy-MT2 LLM 翻译模型 id（MODELS 注册表中的唯一条目）。
pub const HY_MT2_MODEL_ID: &str = "hy-mt2-1.8b-q4_k_m";

/// Whether the Hy-MT2 translation model is installed.
pub(crate) fn hy_mt2_installed() -> bool {
    match find_model(HY_MT2_MODEL_ID) {
        Some(m) => is_installed(
            &crate::sherpa_onnx_engine::commands::resolved_models_dir(),
            m,
        ),
        None => false,
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalSummaryModel {
    pub id: String,
    pub display_name: String,
    pub installed: bool,
}

/// List the local meeting-summary models with install state, in priority
/// order (`SUMMARY_MODEL_IDS`).
#[tauri::command]
pub fn summary_local_models() -> Vec<LocalSummaryModel> {
    SUMMARY_MODEL_IDS
        .iter()
        .filter_map(|id| {
            find_model(id).map(|m| LocalSummaryModel {
                id: m.id.to_string(),
                display_name: m.display_name.to_string(),
                installed: summary_model_installed(m.id),
            })
        })
        .collect()
}

fn find_model(model_id: &str) -> Option<&'static DownloadableModel> {
    MODELS.iter().find(|m| m.id == model_id)
}

// ── State ─────────────────────────────────────────────────────────────────────

/// Per-model cancellation flags; presence of an entry means a download or
/// import is running for that model. Multiple models may run concurrently;
/// only the same model is mutually exclusive.
static DOWNLOADS: LazyLock<Mutex<HashMap<String, Arc<AtomicBool>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

const SAME_TASK_RUNNING_ERR: &str = "该模型正在下载或导入中";

#[derive(Debug, Clone, Serialize)]
pub struct ModelDownloadInfo {
    pub id: String,
    pub display_name: String,
    pub installed: bool,
    pub downloading: bool,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProgressEvent {
    model_id: String,
    /// downloading | extracting | verifying | done | error | cancelled
    stage: String,
    downloaded_bytes: u64,
    total_bytes: u64,
    percent: f64,
    message: Option<String>,
    /// URL of the source currently being downloaded from (downloads only).
    source_url: Option<String>,
}

fn emit_progress<R: Runtime>(
    app: &AppHandle<R>,
    model_id: &str,
    stage: &str,
    downloaded: u64,
    total: u64,
    message: Option<String>,
    source_url: Option<&str>,
) {
    let percent = if total > 0 {
        (downloaded as f64 / total as f64 * 100.0).min(100.0)
    } else {
        0.0
    };
    let _ = app.emit(
        "model-download-progress",
        ProgressEvent {
            model_id: model_id.to_string(),
            stage: stage.to_string(),
            downloaded_bytes: downloaded,
            total_bytes: total,
            percent,
            message,
            source_url: source_url.map(|s| s.to_string()),
        },
    );
}

// ── Commands ──────────────────────────────────────────────────────────────────

/// List all downloadable models with install/download state.
#[tauri::command]
pub fn get_downloadable_models() -> Result<Vec<ModelDownloadInfo>, String> {
    let models_dir = crate::sherpa_onnx_engine::commands::resolved_models_dir();
    let downloads = DOWNLOADS.lock().map_err(|e| e.to_string())?;

    Ok(MODELS
        .iter()
        .map(|m| ModelDownloadInfo {
            id: m.id.to_string(),
            display_name: m.display_name.to_string(),
            installed: is_installed(&models_dir, m),
            downloading: downloads.contains_key(m.id),
            size_bytes: m.size_bytes,
        })
        .collect())
}

/// Start downloading + extracting a model in the background.
/// Progress is reported via the `model-download-progress` event.
#[tauri::command]
pub async fn download_model<R: Runtime>(app: AppHandle<R>, model_id: String) -> Result<(), String> {
    let model = find_model(&model_id).ok_or_else(|| format!("Unknown model: {}", model_id))?;

    // Per-model exclusivity; different models download concurrently.
    let cancel_flag = register_task(model.id)?;

    let models_dir = crate::sherpa_onnx_engine::commands::resolved_models_dir();
    let app_handle = app.clone();

    tauri::async_runtime::spawn(async move {
        let result = run_download(&app_handle, model, &models_dir, &cancel_flag).await;

        finish_task(model.id);

        match result {
            Ok(()) => emit_progress(&app_handle, model.id, "done", 1, 1, None, None),
            Err(e) if e == "cancelled" => {
                emit_progress(&app_handle, model.id, "cancelled", 0, 1, None, None)
            }
            Err(e) => emit_progress(&app_handle, model.id, "error", 0, 1, Some(e), None),
        }
    });

    Ok(())
}

/// Cancel an in-flight download (best effort; takes effect within ~1 chunk).
#[tauri::command]
pub fn cancel_model_download(model_id: String) -> Result<(), String> {
    let downloads = DOWNLOADS.lock().map_err(|e| e.to_string())?;
    match downloads.get(&model_id) {
        Some(flag) => {
            flag.store(true, Ordering::SeqCst);
            Ok(())
        }
        None => Err(format!("Model {} is not downloading", model_id)),
    }
}

/// Delete an installed model directory.
#[tauri::command]
pub fn delete_model(model_id: String) -> Result<(), String> {
    let model = find_model(&model_id).ok_or_else(|| format!("Unknown model: {}", model_id))?;
    let dir = crate::sherpa_onnx_engine::commands::resolved_models_dir().join(model.dir_name);
    if !dir.exists() {
        return Err(format!("Model {} is not installed", model_id));
    }
    std::fs::remove_dir_all(&dir).map_err(|e| format!("Failed to delete model: {}", e))?;
    log::info!("Deleted model {} at {}", model_id, dir.display());
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelImportResult {
    /// "done" | "cancelled" (failures are returned as Err).
    pub status: String,
    pub message: Option<String>,
}

/// Install a model from a local file or folder picked by the user.
///
/// The import kind is inferred from the model's registry entry:
/// - Archive models (SenseVoice, X-ASR): pick a `.tar.bz2` / `.tar.gz` /
///   `.zip` archive, extracted and verified.
/// - GGUF models (summary LLMs, Hy-MT2): pick a single `.gguf` file, its
///   magic header is checked before copying.
/// - Folder models (OPUS-MT): pick a folder containing the required files.
///
/// Progress is reported via the same `model-download-progress` event as
/// downloads (stages extracting / verifying / done / error / cancelled).
#[tauri::command]
pub async fn import_model_file<R: Runtime>(
    app: AppHandle<R>,
    model_id: String,
) -> Result<ModelImportResult, String> {
    let model = find_model(&model_id).ok_or_else(|| format!("Unknown model: {}", model_id))?;

    // Per-model exclusivity; different models may import/download concurrently.
    let cancel_flag = register_task(model.id)?;

    let result = run_import(&app, model, &cancel_flag).await;

    finish_task(model.id);

    match result {
        Ok(r) => {
            match r.status.as_str() {
                "done" => emit_progress(&app, model.id, "done", 1, 1, None, None),
                "cancelled" => emit_progress(&app, model.id, "cancelled", 0, 1, None, None),
                _ => {}
            }
            Ok(r)
        }
        Err(e) => {
            emit_progress(&app, model.id, "error", 0, 1, Some(e.clone()), None);
            Err(e)
        }
    }
}

// ── Download / extract implementation ─────────────────────────────────────────

fn register_task(model_id: &str) -> Result<Arc<AtomicBool>, String> {
    let mut downloads = DOWNLOADS.lock().map_err(|e| e.to_string())?;
    if downloads.contains_key(model_id) {
        return Err(SAME_TASK_RUNNING_ERR.to_string());
    }
    let flag = Arc::new(AtomicBool::new(false));
    downloads.insert(model_id.to_string(), flag.clone());
    Ok(flag)
}

fn finish_task(model_id: &str) {
    DOWNLOADS.lock().ok().map(|mut d| d.remove(model_id));
}

fn is_installed(models_dir: &Path, model: &DownloadableModel) -> bool {
    let dir = models_dir.join(model.dir_name);
    model.required_files.iter().all(|f| dir.join(f).exists())
}

async fn run_download<R: Runtime>(
    app: &AppHandle<R>,
    model: &DownloadableModel,
    models_dir: &Path,
    cancel: &AtomicBool,
) -> Result<(), String> {
    std::fs::create_dir_all(models_dir)
        .map_err(|e| format!("Failed to create models directory: {}", e))?;

    // Try each source in order; only fail when every source failed.
    let mut failures: Vec<String> = Vec::new();
    for source in model.sources {
        let result = match source {
            ModelSource::Archive { url } => {
                run_archive_download(app, model, models_dir, url, cancel).await
            }
            ModelSource::Files {
                base_url,
                resolve_path,
                files,
            } => {
                run_files_download(app, model, models_dir, base_url, resolve_path, files, cancel)
                    .await
            }
        };
        match result {
            Ok(()) => {
                failures.clear();
                break;
            }
            Err(e) if e == "cancelled" => return Err(e),
            Err(e) => {
                log::warn!("Model {} source failed: {}", model.id, e);
                failures.push(e);
            }
        }
    }

    if !failures.is_empty() {
        return Err(format!("所有下载源均失败：{}", failures.join("；")));
    }

    // Verify required files
    emit_progress(app, model.id, "verifying", 0, 1, None, None);
    for f in model.required_files {
        let p = models_dir.join(model.dir_name).join(f);
        if !p.exists() {
            return Err(format!("Model file missing after download: {}", f));
        }
    }

    log::info!(
        "Model {} installed at {}",
        model.id,
        models_dir.join(model.dir_name).display()
    );
    Ok(())
}

/// Download a single .tar.bz2 archive from `url` and extract it.
async fn run_archive_download<R: Runtime>(
    app: &AppHandle<R>,
    model: &DownloadableModel,
    models_dir: &Path,
    url: &str,
    cancel: &AtomicBool,
) -> Result<(), String> {
    let archive_path = models_dir.join(format!("{}.tar.bz2.part", model.dir_name));

    emit_progress(
        app,
        model.id,
        "downloading",
        0,
        model.size_bytes,
        None,
        Some(url),
    );
    if let Err(e) = download_file(app, model, url, &archive_path, cancel, 0, model.size_bytes).await
    {
        if e == "cancelled" {
            return Err(e);
        }
        // Keep the .part file so the next source (or run) can resume.
        return Err(format!("{}: {}", url, e));
    }

    if cancel.load(Ordering::SeqCst) {
        return Err("cancelled".to_string());
    }

    // Extract
    emit_progress(app, model.id, "extracting", 0, 1, None, Some(url));
    let models_dir_owned = models_dir.to_path_buf();
    let archive = archive_path.clone();
    let extract_result = tokio::task::spawn_blocking(move || extract_tar_bz2(&archive, &models_dir_owned))
        .await
        .map_err(|e| format!("Extraction task failed: {}", e))?;
    let _ = std::fs::remove_file(&archive_path);
    extract_result.map_err(|e| format!("{}: {}", url, e))?;
    Ok(())
}

/// Download multiple individual files from a `{base_url}/{resolve_path}/{file}`
/// source into the model directory (files are stored under their base names).
async fn run_files_download<R: Runtime>(
    app: &AppHandle<R>,
    model: &DownloadableModel,
    models_dir: &Path,
    base_url: &str,
    resolve_path: &str,
    files: &[&str],
    cancel: &AtomicBool,
) -> Result<(), String> {
    let target_dir = models_dir.join(model.dir_name);
    std::fs::create_dir_all(&target_dir)
        .map_err(|e| format!("Failed to create model directory: {}", e))?;

    let total = model.size_bytes;
    let mut completed_bytes: u64 = 0;

    for file in files {
        let local_name = file.rsplit('/').next().unwrap_or(file);
        let dest = target_dir.join(local_name);
        // Skip files that already exist (resume across runs).
        if dest.exists() {
            completed_bytes += std::fs::metadata(&dest).map(|m| m.len()).unwrap_or(0);
            continue;
        }

        let part = target_dir.join(format!("{}.part", local_name));
        let url = format!(
            "{}/{}/{}",
            base_url.trim_end_matches('/'),
            resolve_path,
            file
        );
        if let Err(e) = download_file(app, model, &url, &part, cancel, completed_bytes, total).await
        {
            if e == "cancelled" {
                return Err(e);
            }
            // Keep the .part file so the next source (or run) can resume.
            return Err(format!("{}: {}", url, e));
        }

        let size = std::fs::metadata(&part).map(|m| m.len()).unwrap_or(0);
        std::fs::rename(&part, &dest).map_err(|e| format!("Failed to move file: {}", e))?;
        completed_bytes += size;
        emit_progress(
            app,
            model.id,
            "downloading",
            completed_bytes.min(total),
            total,
            None,
            Some(&url),
        );
    }

    Ok(())
}

async fn download_file<R: Runtime>(
    app: &AppHandle<R>,
    model: &DownloadableModel,
    url: &str,
    dest: &Path,
    cancel: &AtomicBool,
    offset: u64,
    total_hint: u64,
) -> Result<(), String> {
    use futures_util::StreamExt;
    use tokio::io::AsyncWriteExt;

    let client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?;

    // Resume from an existing .part file via a Range request when possible.
    let existing = std::fs::metadata(dest).map(|m| m.len()).unwrap_or(0);
    let mut request = client.get(url);
    if existing > 0 {
        request = request.header(reqwest::header::RANGE, format!("bytes={}-", existing));
    }

    let response = request
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    let status = response.status();
    let resumed = if existing > 0 && status == reqwest::StatusCode::PARTIAL_CONTENT {
        true
    } else if existing > 0 && status == reqwest::StatusCode::RANGE_NOT_SATISFIABLE {
        // The .part file is already complete (e.g. the app crashed before
        // the rename); treat it as downloaded.
        return Ok(());
    } else if status.is_success() {
        // Server does not support Range (or no resume requested): restart.
        false
    } else {
        return Err(format!("HTTP {}", status));
    };

    let base: u64 = if resumed { existing } else { 0 };
    let total = response
        .content_length()
        .map(|l| offset + base + l)
        .unwrap_or(total_hint);

    let mut file = if resumed {
        tokio::fs::OpenOptions::new()
            .append(true)
            .open(dest)
            .await
            .map_err(|e| format!("Failed to open {}: {}", dest.display(), e))?
    } else {
        tokio::fs::File::create(dest)
            .await
            .map_err(|e| format!("Failed to create {}: {}", dest.display(), e))?
    };

    let mut downloaded: u64 = base;
    let mut last_emit = std::time::Instant::now();
    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.next().await {
        if cancel.load(Ordering::SeqCst) {
            drop(file);
            // Keep the .part file so a later retry can resume.
            return Err("cancelled".to_string());
        }
        let chunk = chunk.map_err(|e| format!("Download interrupted: {}", e))?;
        file.write_all(&chunk)
            .await
            .map_err(|e| format!("Write failed: {}", e))?;
        downloaded += chunk.len() as u64;

        // Throttle progress events to ~2 per second.
        if last_emit.elapsed() >= std::time::Duration::from_millis(500) {
            last_emit = std::time::Instant::now();
            emit_progress(
                app,
                model.id,
                "downloading",
                offset + downloaded,
                total,
                None,
                Some(url),
            );
        }
    }

    file.flush().await.map_err(|e| e.to_string())?;
    emit_progress(
        app,
        model.id,
        "downloading",
        offset + downloaded,
        total,
        None,
        Some(url),
    );
    Ok(())
}

/// Extract a `.tar.bz2` archive into `dest_dir` (blocking; call via spawn_blocking).
fn extract_tar_bz2(archive: &Path, dest_dir: &Path) -> Result<(), String> {
    let file = std::fs::File::open(archive)
        .map_err(|e| format!("Failed to open archive: {}", e))?;
    let decoder = bzip2::read::BzDecoder::new(file);
    let mut tar = tar::Archive::new(decoder);
    tar.unpack(dest_dir)
        .map_err(|e| format!("Failed to extract archive: {}", e))?;
    Ok(())
}

/// Extract a `.tar.gz` archive into `dest_dir` (blocking).
fn extract_tar_gz(archive: &Path, dest_dir: &Path) -> Result<(), String> {
    let file = std::fs::File::open(archive)
        .map_err(|e| format!("Failed to open archive: {}", e))?;
    let decoder = flate2::read::GzDecoder::new(file);
    let mut tar = tar::Archive::new(decoder);
    tar.unpack(dest_dir)
        .map_err(|e| format!("Failed to extract archive: {}", e))?;
    Ok(())
}

/// Extract a `.zip` archive into `dest_dir` (blocking).
fn extract_zip(archive: &Path, dest_dir: &Path) -> Result<(), String> {
    let file = std::fs::File::open(archive)
        .map_err(|e| format!("Failed to open archive: {}", e))?;
    let mut zip = zip::ZipArchive::new(file)
        .map_err(|e| format!("Failed to read zip archive: {}", e))?;
    zip.extract(dest_dir)
        .map_err(|e| format!("Failed to extract archive: {}", e))?;
    Ok(())
}

// ── Model import implementation ───────────────────────────────────────────────

/// How a model is imported from local files, inferred from its registry entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ImportKind {
    /// Archive-type model: import a .tar.bz2 / .tar.gz / .zip archive.
    Archive,
    /// Files-type model whose files are all .gguf: import one GGUF file.
    GgufFile,
    /// Anything else: import a prepared folder containing the required files.
    Folder,
}

fn import_kind(model: &DownloadableModel) -> ImportKind {
    match model.sources.first() {
        Some(ModelSource::Archive { .. }) => ImportKind::Archive,
        Some(ModelSource::Files { files, .. })
            if !files.is_empty() && files.iter().all(|f| f.ends_with(".gguf")) =>
        {
            ImportKind::GgufFile
        }
        _ => ImportKind::Folder,
    }
}

async fn run_import<R: Runtime>(
    app: &AppHandle<R>,
    model: &'static DownloadableModel,
    cancel: &AtomicBool,
) -> Result<ModelImportResult, String> {
    let kind = import_kind(model);

    // File/folder picker must run on a blocking thread (see
    // audio/import.rs select_and_validate_audio_command).
    let app_clone = app.clone();
    let picked: Option<String> = tokio::task::spawn_blocking(move || {
        let dialog = app_clone.dialog().file();
        match kind {
            ImportKind::Archive => dialog
                .add_filter("模型压缩包", &["tar.bz2", "tar.gz", "tgz", "zip"])
                .blocking_pick_file()
                .map(|p| p.to_string()),
            ImportKind::GgufFile => dialog
                .add_filter("GGUF 模型", &["gguf"])
                .blocking_pick_file()
                .map(|p| p.to_string()),
            ImportKind::Folder => dialog.blocking_pick_folder().map(|p| p.to_string()),
        }
    })
    .await
    .map_err(|e| format!("File dialog task failed: {}", e))?;

    let picked = match picked {
        Some(p) => PathBuf::from(p),
        None => {
            return Ok(ModelImportResult {
                status: "cancelled".to_string(),
                message: None,
            })
        }
    };

    if cancel.load(Ordering::SeqCst) {
        return Ok(ModelImportResult {
            status: "cancelled".to_string(),
            message: None,
        });
    }

    let models_dir = crate::sherpa_onnx_engine::commands::resolved_models_dir();
    std::fs::create_dir_all(&models_dir)
        .map_err(|e| format!("Failed to create models directory: {}", e))?;

    let app_clone = app.clone();
    let source = picked.clone();
    let models_dir_clone = models_dir.clone();
    tokio::task::spawn_blocking(move || match kind {
        ImportKind::Archive => import_archive(&app_clone, model, &source, &models_dir_clone),
        ImportKind::GgufFile => import_gguf(&app_clone, model, &source, &models_dir_clone),
        ImportKind::Folder => import_folder(&app_clone, model, &source, &models_dir_clone),
    })
    .await
    .map_err(|e| format!("Import task failed: {}", e))??;

    log::info!(
        "Model {} imported from {} into {}",
        model.id,
        picked.display(),
        models_dir.join(model.dir_name).display()
    );
    Ok(ModelImportResult {
        status: "done".to_string(),
        message: None,
    })
}

/// Extract a picked archive into `<models_dir>/<dir_name>/`, verifying the
/// required files. On any failure the target directory is left untouched or
/// fully removed — no half-installed state.
fn import_archive<R: Runtime>(
    app: &AppHandle<R>,
    model: &DownloadableModel,
    archive: &Path,
    models_dir: &Path,
) -> Result<(), String> {
    let name = archive
        .file_name()
        .map(|n| n.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    let temp_dir = models_dir.join(format!(".import-{}", model.dir_name));
    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir)
            .map_err(|e| format!("Failed to clean temp directory: {}", e))?;
    }
    std::fs::create_dir_all(&temp_dir)
        .map_err(|e| format!("Failed to create temp directory: {}", e))?;

    let result = import_archive_inner(app, model, archive, models_dir, &temp_dir, &name);
    if result.is_err() {
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
    result
}

fn import_archive_inner<R: Runtime>(
    app: &AppHandle<R>,
    model: &DownloadableModel,
    archive: &Path,
    models_dir: &Path,
    temp_dir: &Path,
    name: &str,
) -> Result<(), String> {
    emit_progress(app, model.id, "extracting", 0, 1, None, None);
    if name.ends_with(".tar.bz2") {
        extract_tar_bz2(archive, temp_dir)?;
    } else if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
        extract_tar_gz(archive, temp_dir)?;
    } else if name.ends_with(".zip") {
        extract_zip(archive, temp_dir)?;
    } else {
        return Err("不支持的压缩包格式（支持 .tar.bz2 / .tar.gz / .zip）".to_string());
    }

    // Archives usually wrap everything in a single top-level directory.
    let root = descend_single_dir(temp_dir)?;

    emit_progress(app, model.id, "verifying", 0, 1, None, None);
    for f in model.required_files {
        if !root.join(f).exists() {
            return Err(format!("压缩包中缺少必需文件：{}", f));
        }
    }

    let target = models_dir.join(model.dir_name);
    if target.exists() {
        std::fs::remove_dir_all(&target)
            .map_err(|e| format!("Failed to remove old model directory: {}", e))?;
    }
    std::fs::rename(&root, &target)
        .map_err(|e| format!("Failed to move model into place: {}", e))?;
    // Remove the (now empty) temp extraction directory.
    let _ = std::fs::remove_dir_all(temp_dir);
    Ok(())
}

/// Import a single GGUF file: verify the magic header, then copy it into
/// `<models_dir>/<dir_name>/` keeping the original file name.
fn import_gguf<R: Runtime>(
    app: &AppHandle<R>,
    model: &DownloadableModel,
    file: &Path,
    models_dir: &Path,
) -> Result<(), String> {
    emit_progress(app, model.id, "verifying", 0, 1, None, None);
    check_gguf_magic(file)?;

    let file_name = file
        .file_name()
        .ok_or_else(|| "无法获取文件名".to_string())?;

    emit_progress(app, model.id, "extracting", 0, 1, None, None);
    let target_dir = models_dir.join(model.dir_name);
    std::fs::create_dir_all(&target_dir)
        .map_err(|e| format!("Failed to create model directory: {}", e))?;
    let dest = target_dir.join(file_name);
    std::fs::copy(file, &dest).map_err(|e| format!("Failed to copy file: {}", e))?;

    // The copied file must satisfy the model's required files (i.e. the
    // original file name must match the registry entry).
    for f in model.required_files {
        if !target_dir.join(f).exists() {
            let _ = std::fs::remove_file(&dest);
            return Err(format!(
                "导入的文件名与模型要求不匹配，需要文件：{}",
                model.required_files.join(", ")
            ));
        }
    }
    Ok(())
}

/// Import a prepared folder: verify the required files, then copy the whole
/// folder contents into `<models_dir>/<dir_name>/`.
fn import_folder<R: Runtime>(
    app: &AppHandle<R>,
    model: &DownloadableModel,
    folder: &Path,
    models_dir: &Path,
) -> Result<(), String> {
    emit_progress(app, model.id, "verifying", 0, 1, None, None);
    for f in model.required_files {
        if !folder.join(f).exists() {
            return Err(format!("文件夹中缺少必需文件：{}", f));
        }
    }

    emit_progress(app, model.id, "extracting", 0, 1, None, None);
    let target = models_dir.join(model.dir_name);
    if target.exists() {
        std::fs::remove_dir_all(&target)
            .map_err(|e| format!("Failed to remove old model directory: {}", e))?;
    }
    if let Err(e) = copy_dir_all(folder, &target) {
        let _ = std::fs::remove_dir_all(&target);
        return Err(e);
    }

    for f in model.required_files {
        if !target.join(f).exists() {
            let _ = std::fs::remove_dir_all(&target);
            return Err(format!("导入后缺少必需文件：{}", f));
        }
    }
    Ok(())
}

/// If `dir` contains exactly one entry and it is a directory, return that
/// directory (archives often wrap everything in a single top-level folder).
fn descend_single_dir(dir: &Path) -> Result<PathBuf, String> {
    let entries: Vec<std::fs::DirEntry> = std::fs::read_dir(dir)
        .map_err(|e| format!("Failed to read directory: {}", e))?
        .collect::<Result<_, _>>()
        .map_err(|e| format!("Failed to read directory: {}", e))?;
    if entries.len() == 1 {
        let file_type = entries[0].file_type().map_err(|e| e.to_string())?;
        if file_type.is_dir() {
            return Ok(entries[0].path());
        }
    }
    Ok(dir.to_path_buf())
}

/// A valid GGUF file starts with the 4-byte magic `GGUF`.
fn check_gguf_magic(path: &Path) -> Result<(), String> {
    use std::io::Read;
    let mut file =
        std::fs::File::open(path).map_err(|e| format!("Failed to open file: {}", e))?;
    let mut magic = [0u8; 4];
    file.read_exact(&mut magic)
        .map_err(|_| "文件太小，不是有效的 GGUF 模型".to_string())?;
    if &magic != b"GGUF" {
        return Err("所选文件不是有效的 GGUF 模型（magic 校验失败）".to_string());
    }
    Ok(())
}

fn copy_dir_all(src: &Path, dst: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dst).map_err(|e| format!("Failed to create directory: {}", e))?;
    for entry in std::fs::read_dir(src).map_err(|e| format!("Failed to read directory: {}", e))? {
        let entry = entry.map_err(|e| format!("Failed to read directory: {}", e))?;
        let file_type = entry.file_type().map_err(|e| e.to_string())?;
        let dest_path = dst.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_all(&entry.path(), &dest_path)?;
        } else {
            std::fs::copy(entry.path(), &dest_path)
                .map_err(|e| format!("Failed to copy file: {}", e))?;
        }
    }
    Ok(())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gguf_magic_accepts_valid_header() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("model.gguf");
        std::fs::write(&path, b"GGUF\x03\x00\x00\x00rest-of-file").unwrap();
        assert!(check_gguf_magic(&path).is_ok());
    }

    #[test]
    fn gguf_magic_rejects_bad_header() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("model.gguf");
        std::fs::write(&path, b"NOPE-not-a-gguf-file").unwrap();
        assert!(check_gguf_magic(&path).is_err());
    }

    #[test]
    fn gguf_magic_rejects_tiny_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("model.gguf");
        std::fs::write(&path, b"GG").unwrap();
        assert!(check_gguf_magic(&path).is_err());
    }

    #[test]
    fn descend_into_single_wrapping_dir() {
        let dir = tempfile::tempdir().unwrap();
        let wrapped = dir.path().join("model-dir");
        std::fs::create_dir(&wrapped).unwrap();
        std::fs::write(wrapped.join("model.onnx"), b"onnx").unwrap();
        let root = descend_single_dir(dir.path()).unwrap();
        assert_eq!(root, wrapped);
    }

    #[test]
    fn descend_keeps_flat_layout() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("model.onnx"), b"onnx").unwrap();
        std::fs::write(dir.path().join("tokens.txt"), b"tokens").unwrap();
        let root = descend_single_dir(dir.path()).unwrap();
        assert_eq!(root, dir.path());
    }

    #[test]
    fn zip_extract_and_descend() {
        let dir = tempfile::tempdir().unwrap();
        let zip_path = dir.path().join("model.zip");
        {
            use std::io::Write;
            let file = std::fs::File::create(&zip_path).unwrap();
            let mut writer = zip::ZipWriter::new(file);
            let options = zip::write::SimpleFileOptions::default();
            writer.add_directory("model-dir/", options).unwrap();
            writer.start_file("model-dir/model.onnx", options).unwrap();
            writer.write_all(b"onnx").unwrap();
            writer.finish().unwrap();
        }
        let out = dir.path().join("out");
        std::fs::create_dir(&out).unwrap();
        extract_zip(&zip_path, &out).unwrap();
        let root = descend_single_dir(&out).unwrap();
        assert_eq!(root.file_name().unwrap(), "model-dir");
        assert!(root.join("model.onnx").exists());
    }

    #[test]
    fn registry_sources_are_ordered_official_first() {
        // Every model must have at least one source.
        for m in MODELS {
            assert!(!m.sources.is_empty(), "model {} has no sources", m.id);
        }

        // Archive models: GitHub release first, gh-proxy mirror second.
        for id in ["sense-voice", "x-asr-480ms"] {
            let m = find_model(id).unwrap();
            assert_eq!(m.sources.len(), 2);
            match &m.sources[0] {
                ModelSource::Archive { url } => assert!(url.contains("github.com")),
                _ => panic!("{} source 0 should be an archive", id),
            }
            match &m.sources[1] {
                ModelSource::Archive { url } => assert!(url.contains("gh-proxy.com")),
                _ => panic!("{} source 1 should be an archive", id),
            }
        }

        // OPUS-MT models: HF, hf-mirror, ModelScope (resolve/master).
        for id in ["opus-mt-zh-en", "opus-mt-en-zh"] {
            let m = find_model(id).unwrap();
            assert_eq!(m.sources.len(), 3);
            match &m.sources[2] {
                ModelSource::Files {
                    base_url,
                    resolve_path,
                    ..
                } => {
                    assert!(base_url.contains("modelscope.cn"));
                    assert_eq!(*resolve_path, "resolve/master");
                }
                _ => panic!("{} source 2 should be files", id),
            }
        }

        // Qwen2.5: HF, hf-mirror, ModelScope.
        let qwen = find_model("qwen2.5-3b-instruct-q4_k_m").unwrap();
        assert_eq!(qwen.sources.len(), 3);
        match &qwen.sources[2] {
            ModelSource::Files {
                base_url,
                resolve_path,
                ..
            } => {
                assert!(base_url.contains("modelscope.cn"));
                assert_eq!(*resolve_path, "resolve/master");
            }
            _ => panic!("qwen source 2 should be files"),
        }
    }

    #[test]
    fn import_kind_is_inferred_from_registry() {
        assert_eq!(
            import_kind(find_model("sense-voice").unwrap()),
            ImportKind::Archive
        );
        assert_eq!(
            import_kind(find_model("x-asr-480ms").unwrap()),
            ImportKind::Archive
        );
        assert_eq!(
            import_kind(find_model("opus-mt-zh-en").unwrap()),
            ImportKind::Folder
        );
        assert_eq!(
            import_kind(find_model("opus-mt-en-zh").unwrap()),
            ImportKind::Folder
        );
        for id in [
            "qwen2.5-3b-instruct-q4_k_m",
            "qwen3-4b-instruct-2507-q4_k_m",
            "gemma-3-4b-it-q4_k_m",
            "hy-mt2-1.8b-q4_k_m",
        ] {
            assert_eq!(import_kind(find_model(id).unwrap()), ImportKind::GgufFile);
        }
    }
}
