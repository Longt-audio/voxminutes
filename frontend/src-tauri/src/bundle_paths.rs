use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager, Runtime};

/// Resolve the project root from the current executable location.
/// For dev layout: target/{debug|release}/voxminutes.exe -> project_root
fn project_root_from_exe() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf)) // target/{debug|release}
        .and_then(|p| p.parent().map(Path::to_path_buf)) // target
        .and_then(|p| p.parent().map(Path::to_path_buf)) // project_root
}

/// Find a directory by walking up from CWD looking for sibling `frontend/`.
fn find_local_sibling_dir(name: &str) -> Option<PathBuf> {
    let cwd = std::env::current_dir().ok()?;
    let mut current = cwd.clone();
    for _ in 0..5 {
        let candidate = current.join(name);
        if candidate.exists() && current.join("frontend").exists() {
            return Some(candidate.canonicalize().unwrap_or(candidate));
        }
        current = current.parent()?.to_path_buf();
    }
    let parent_candidate = cwd.parent()?.join(name);
    if parent_candidate.exists() {
        return Some(parent_candidate.canonicalize().unwrap_or(parent_candidate));
    }
    None
}

/// Resolve a bundled resource directory using multiple fallback strategies.
///
/// Search order:
/// 1. Portable folder layout: <exe_dir>/<name> (e.g. backend/, models/)
/// 2. Project-local path (dev): exe's project root directory
/// 3. Sibling directory from CWD (covers running from project root)
/// 4. Tauri resource_dir() (installed bundle)
/// 5. App data dir: app_data_dir/<name>
pub fn resolve_bundle_dir<R: Runtime>(app: &AppHandle<R>, name: &str) -> Option<PathBuf> {
    // 1. Portable layout: backend/ and models/ next to the executable.
    //    This must come before the dev-layout check because the portable
    //    package may live inside the project tree (e.g. dist/VoxMinutes
    //    under the repo root), and walking up from the exe would otherwise
    //    wrongly pick the dev backend/models from the project root.
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            let candidate = exe_dir.join(name);
            if candidate.exists() {
                log::info!("Using portable {} directory: {}", name, candidate.display());
                return Some(candidate);
            }
        }
    }

    // 2. Dev layout: project root
    if let Some(root) = project_root_from_exe() {
        let candidate = root.join(name);
        if candidate.exists() {
            log::info!("Using project-local {} directory: {}", name, candidate.display());
            return Some(candidate);
        }
    }

    // 3. Sibling directory from CWD (covers running from project root)
    if let Some(candidate) = find_local_sibling_dir(name) {
        log::info!("Using local sibling {} directory: {}", name, candidate.display());
        return Some(candidate);
    }

    // 4. Tauri resource dir (bundled installer)
    if let Ok(resource_dir) = app.path().resource_dir() {
        let candidate = resource_dir.join(name);
        if candidate.exists() {
            log::info!("Using bundled resource {} directory: {}", name, candidate.display());
            return Some(candidate);
        }
    }

    // 5. App data dir fallback
    let app_data_dir = app.path().app_data_dir().ok()?;
    let candidate = app_data_dir.join(name);
    std::fs::create_dir_all(&candidate).ok();
    log::info!("Using app data {} directory: {}", name, candidate.display());
    Some(candidate)
}

// ---------------------------------------------------------------------------
// ASCII model staging
//
// sherpa-onnx (the bundled C-API / onnxruntime build) cannot open model files
// whose path contains non-ASCII (e.g. Chinese) characters on Windows. When the
// app lives under such a path, we expose the models under a guaranteed-ASCII
// directory (C:\ProgramData\VoxMinutes\models) — preferring an instant NTFS directory
// junction and falling back to copying the ASR model subdirs — and point both
// the Rust engines and the Python backend (via VOXMINUTES_MODELS_DIR) at it.
// Pure-ASCII install paths are used as-is (no staging, zero overhead).
// ---------------------------------------------------------------------------

static STAGED_MODELS_DIR: std::sync::Mutex<Option<PathBuf>> = std::sync::Mutex::new(None);

const ASR_MODEL_SUBDIRS: &[&str] = &[
    "sherpa-onnx-sense-voice-zh-en-ja-ko-yue-2024-07-17",
];

/// X-ASR model directory prefix — used to locate the actual directory name
/// which includes a date suffix (e.g. sherpa-onnx-x-asr-480ms-streaming-zipformer-transducer-zh-en-punct-2026-06-05).
const XASR_MODEL_DIR_PREFIX: &str = "sherpa-onnx-x-asr-480ms-streaming-zipformer-transducer-zh-en-punct";

/// Find the X-ASR model directory in the given base path by matching the prefix.
fn find_xasr_dir_in(base: &Path) -> Option<PathBuf> {
    let entries = std::fs::read_dir(base).ok()?;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str.starts_with(XASR_MODEL_DIR_PREFIX) {
            let dir = entry.path();
            if dir.is_dir()
                && dir.join("encoder.onnx").exists()
                && dir.join("decoder.onnx").exists()
                && dir.join("joiner.onnx").exists()
                && dir.join("tokens.txt").exists()
            {
                return Some(dir);
            }
        }
    }
    None
}

/// The ASCII-safe models directory selected for native engines (set by
/// `stage_models_dir_for_native`). Used to pass VOXMINUTES_MODELS_DIR to the backend.
pub fn staged_models_dir() -> Option<PathBuf> {
    STAGED_MODELS_DIR.lock().ok().and_then(|g| g.clone())
}

fn set_staged_models_dir(p: &Path) {
    if let Ok(mut g) = STAGED_MODELS_DIR.lock() {
        *g = Some(p.to_path_buf());
    }
}

/// Strip the Windows verbatim prefix (\\?\) if present.
fn strip_verbatim(p: &Path) -> PathBuf {
    let s = p.to_string_lossy();
    match s.strip_prefix(r"\\?\") {
        Some(rest) => PathBuf::from(rest),
        None => PathBuf::from(s.as_ref()),
    }
}

fn path_is_ascii(p: &Path) -> bool {
    p.as_os_str().to_string_lossy().is_ascii()
}

/// Hash a path into a short deterministic token. Used to create a unique
/// fallback staging directory when the default one is locked or corrupted.
fn compute_path_hash(path: &Path) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    path.to_string_lossy().hash(&mut hasher);
    format!("{:08x}", hasher.finish())
}

fn staged_has_models(staged: &Path) -> bool {
    ASR_MODEL_SUBDIRS.iter().any(|d| staged.join(d).exists())
        || find_xasr_dir_in(staged).is_some()
}

#[cfg(target_os = "windows")]
fn is_reparse_point(path: &Path) -> bool {
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
    std::fs::symlink_metadata(path)
        .map(|m| m.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0)
        .unwrap_or(false)
}

/// Remove a staging entry without following junctions/symlinks and deleting
/// their targets. Broken junctions are removed as entries.
fn remove_staging_safely(path: &Path) -> std::io::Result<()> {
    #[cfg(target_os = "windows")]
    {
        if is_reparse_point(path) {
            return std::fs::remove_dir(path);
        }
    }
    std::fs::remove_dir_all(path)
}

/// Check whether an existing staging entry can be reused for `original`.
/// - Junctions/symlinks are reused only when they point at `original`.
/// - Directory copies are reused if they contain the required model files.
fn is_valid_staging(staged: &Path, original: &Path) -> bool {
    if !staged.exists() {
        return false;
    }

    #[cfg(target_os = "windows")]
    {
        if is_reparse_point(staged) {
            let Ok(staged_target) = std::fs::canonicalize(staged) else {
                return false;
            };
            let Ok(original_canon) = std::fs::canonicalize(original) else {
                return false;
            };
            return strip_verbatim(&staged_target) == strip_verbatim(&original_canon);
        }
    }

    staged_has_models(staged)
}

/// Try to create or reuse a staging directory at `staged` pointing to `original`.
/// Returns `Some(staged)` on success, `None` if the location cannot be used.
fn try_stage_at(staged: &Path, original: &Path) -> Option<PathBuf> {
    if is_valid_staging(staged, original) {
        log::info!("Reusing existing ASCII model staging: {}", staged.display());
        return Some(staged.to_path_buf());
    }

    if staged.exists() {
        if let Err(e) = remove_staging_safely(staged) {
            log::warn!(
                "Failed to remove existing staging at {}: {}. Trying another location.",
                staged.display(),
                e
            );
            return None;
        }
    }

    if let Some(parent) = staged.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            log::warn!("Failed to create staging parent {}: {}", parent.display(), e);
            return None;
        }
    }

    #[cfg(target_os = "windows")]
    {
        if try_make_junction(staged, original) {
            log::info!(
                "Staged models via junction: {} -> {}",
                staged.display(),
                original.display()
            );
            return Some(staged.to_path_buf());
        }
        log::warn!("Junction creation failed at {}", staged.display());
    }

    match copy_asr_models(original, staged) {
        Ok(()) => {
            log::info!("Staged models via copy: {}", staged.display());
            Some(staged.to_path_buf())
        }
        Err(e) => {
            log::warn!("Copy staging failed at {}: {}", staged.display(), e);
            None
        }
    }
}

/// Resolve a models directory to an ASCII-safe path for native sherpa-onnx.
/// Returns the input (with any `\\?\` prefix removed) unchanged when it is
/// already ASCII; otherwise stages to C:\ProgramData\VoxMinutes\models, falling back
/// to a unique per-path directory if the default location is unusable.
pub fn stage_models_dir_for_native(original: &Path) -> PathBuf {
    let original = strip_verbatim(original);

    if path_is_ascii(&original) {
        set_staged_models_dir(&original);
        return original;
    }

    log::warn!(
        "Models path contains non-ASCII characters ({}); sherpa-onnx cannot open such paths. Staging to ASCII dir.",
        original.display()
    );

    // If we already have a valid ASCII staging for this exact original, reuse it.
    if let Some(existing) = staged_models_dir() {
        if path_is_ascii(&existing) && is_valid_staging(&existing, &original) {
            return existing;
        }
    }

    let primary = PathBuf::from(r"C:\ProgramData\VoxMinutes\models");
    if let Some(staged) = try_stage_at(&primary, &original) {
        set_staged_models_dir(&staged);
        return staged;
    }

    let fallback = PathBuf::from(format!(
        r"C:\ProgramData\VoxMinutes\models_{}",
        compute_path_hash(&original)
    ));
    log::warn!(
        "Default staging location {} is unusable; trying fallback: {}",
        primary.display(),
        fallback.display()
    );
    if let Some(staged) = try_stage_at(&fallback, &original) {
        set_staged_models_dir(&staged);
        return staged;
    }

    log::error!(
        "Failed to stage models to ASCII dir. Falling back to original path (ASR/TTS may not work under a non-ASCII path)."
    );
    set_staged_models_dir(&original);
    original
}

#[cfg(target_os = "windows")]
fn try_make_junction(link: &Path, target: &Path) -> bool {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let status = std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(link)
        .arg(target)
        .creation_flags(CREATE_NO_WINDOW)
        .status();
    matches!(status, Ok(s) if s.success()) && staged_has_models(link)
}

fn copy_asr_models(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for sub in ASR_MODEL_SUBDIRS {
        let from = src.join(sub);
        if from.exists() {
            copy_dir_recursive(&from, &dst.join(sub))?;
        }
    }
    // Copy X-ASR model directory by prefix match
    if let Some(xasr_dir) = find_xasr_dir_in(src) {
        let dir_name = xasr_dir.file_name().unwrap();
        copy_dir_recursive(&xasr_dir, &dst.join(dir_name))?;
    }
    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let target = dst.join(entry.file_name());
        if path.is_dir() {
            copy_dir_recursive(&path, &target)?;
        } else {
            // Skip re-copying same-size files so repeated launches stay cheap.
            let need = match (std::fs::metadata(&path), std::fs::metadata(&target)) {
                (Ok(a), Ok(b)) => a.len() != b.len(),
                _ => true,
            };
            if need {
                std::fs::copy(&path, &target)?;
            }
        }
    }
    Ok(())
}
