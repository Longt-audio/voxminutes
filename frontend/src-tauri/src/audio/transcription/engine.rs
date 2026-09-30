use super::provider::TranscriptionProvider;
use super::remote_asr_provider::RemoteAsrProvider;
use super::worker::TranscriptUpdate;
use super::x_asr_provider::XAsrProvider;
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, Runtime};

pub enum TranscriptionEngine {
    Provider(Arc<dyn TranscriptionProvider>),
}

impl TranscriptionEngine {
    pub async fn is_model_loaded(&self) -> bool {
        match self {
            Self::Provider(provider) => provider.is_model_loaded().await,
        }
    }

    pub async fn get_current_model(&self) -> Option<String> {
        match self {
            Self::Provider(provider) => provider.get_current_model().await,
        }
    }

    pub fn provider_name(&self) -> &str {
        match self {
            Self::Provider(provider) => provider.provider_name(),
        }
    }

    /// Set chunk context for the next transcription call.
    /// Used by streaming providers to emit partial results with correct metadata.
    pub fn set_chunk_context(
        &self,
        sequence_id: u64,
        chunk_start_time: f64,
        audio_start_time: f64,
        audio_end_time: f64,
        duration: f64,
    ) {
        match self {
            Self::Provider(provider) => provider.set_chunk_context(
                sequence_id,
                chunk_start_time,
                audio_start_time,
                audio_end_time,
                duration,
            ),
        }
    }
}

/// 内置默认远程网关地址（生产网关，已上线）。
/// remote_asr_config.json 的 endpoint 字段只存「用户自定义值」：空串 = 未自定义 = 用本默认。
pub const DEFAULT_REMOTE_ENDPOINT: &str = "https://api.voxmin.top";

static REMOTE_ASR_ENDPOINT: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());
static REMOTE_ASR_MODEL: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());
/// 远程 ASR 模型的 mode（来自网关模型目录：streaming / batch）。空串表示未设置，需回退到模型名后缀判断。
static REMOTE_ASR_MODE: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());
/// 远程「离线识别」模型（历史记录重识别/导入转写用，非流式）。
/// 与实时转录的 REMOTE_ASR_MODEL 分开：实时只放流式模型，离线只放非流式模型（2026-09-17 起的产品约定）。
static REMOTE_ASR_OFFLINE_MODEL: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());
static REMOTE_TRANSLATE_MODEL: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());
/// 远程「会议总结」模型。
///
/// 为什么与 REMOTE_TRANSLATE_MODEL 分开（2026-09-23，与 REMOTE_ASR_OFFLINE_MODEL 同一先例）：
/// 逐句翻译与会议总结对 LLM 的要求相反 —— 翻译要**高频、低温、输出便宜**（qwen-flash），
/// 总结要**低频、可推理、质量好**（deepseek-flash）。网关后台已能给每个模型标用途
/// （both / translate / summary），于是两边的**合法模型集不再相交**：豆包机器翻译只翻译、
/// deepseek-flash 只总结。共用一个持久化字段时，在总结里选一个就会把翻译的选择改掉，
/// 反之亦然（两侧的「失配自动纠正」还会互相打架）。故分开存。
/// 空串 = 未设置 → 总结侧回退到 translate 选择（兼容旧配置，与 offline 的回退同思路）。
static REMOTE_SUMMARY_MODEL: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());
static REMOTE_TTS_MODEL: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());
static REMOTE_LICENSE: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());
static REMOTE_ENABLED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// 设置远程服务总开关（开启 + 已配置 endpoint 才真正生效）。
pub fn set_remote_enabled(v: bool) {
    REMOTE_ENABLED.store(v, std::sync::atomic::Ordering::SeqCst);
}

/// 远程服务是否可用（开关开启 && 已配置服务器地址——有内置默认地址兜底，
/// 「已配置」实际恒成立，可用性只看总开关）。
/// ASR / 翻译端据此切换到远程路径。
pub fn remote_enabled() -> bool {
    REMOTE_ENABLED.load(std::sync::atomic::Ordering::SeqCst) && is_remote_asr_configured()
}

/// 远程开关的原始状态（不含「是否已配置 endpoint」判断，供设置页回显）。
pub fn remote_enabled_raw() -> bool {
    REMOTE_ENABLED.load(std::sync::atomic::Ordering::SeqCst)
}

pub fn set_remote_asr_config(endpoint: &str, model_name: &str) {
    if let Ok(mut e) = REMOTE_ASR_ENDPOINT.lock() {
        *e = endpoint.to_string();
    }
    if let Ok(mut m) = REMOTE_ASR_MODEL.lock() {
        *m = model_name.to_string();
    }
    let config = RemoteAsrPersistedConfig {
        endpoint: endpoint.to_string(),
        model: model_name.to_string(),
        asr_mode: get_remote_asr_mode(),
        asr_offline_model: REMOTE_ASR_OFFLINE_MODEL
            .lock()
            .map(|m| m.clone())
            .unwrap_or_default(),
        translate_model: get_remote_translate_model(),
        summary_model: get_remote_summary_model(),
        tts_model: get_remote_tts_model(),
    };
    if let Err(e) = save_remote_asr_config_to_disk(&config) {
        warn!("Failed to persist remote ASR config: {}", e);
    }
}

/// 设置远程服务（server_url + license + model）；license 存 OS keychain，不落明文 JSON。
/// model_name 为空或占位名 "remote" 时不清空/覆盖现有模型选择：用户中心、欢迎弹窗
/// 等只存「地址 + 授权码」的入口不得把已选模型重置成占位名（2026-09-20 日志：
/// set_remote_config 把 mimo 选择覆盖成 "remote"，若非随后重新选择，
/// 录制会拿着占位名走非流式路径打网关必然失败）。
/// endpoint 传空串时不覆盖已存地址（UI 只传授权码、不传地址；地址改动走 set_remote_endpoint）。
pub fn set_remote_config(endpoint: &str, license: &str, model_name: &str) {
    if !endpoint.is_empty() {
        if let Ok(mut e) = REMOTE_ASR_ENDPOINT.lock() {
            *e = endpoint.to_string();
        }
    }
    if let Ok(mut l) = REMOTE_LICENSE.lock() {
        *l = license.to_string();
    }
    let effective_model = if model_name.is_empty() || model_name == "remote" {
        // 占位名/空值：保留现有选择（无现有选择时保持空）
        get_remote_asr_model()
    } else {
        model_name.to_string()
    };
    if let Ok(mut m) = REMOTE_ASR_MODEL.lock() {
        *m = effective_model.clone();
    }
    save_license_to_keyring(license);
    let config = RemoteAsrPersistedConfig {
        endpoint: get_remote_asr_endpoint(),
        model: effective_model,
        asr_mode: get_remote_asr_mode(),
        asr_offline_model: REMOTE_ASR_OFFLINE_MODEL
            .lock()
            .map(|m| m.clone())
            .unwrap_or_default(),
        translate_model: get_remote_translate_model(),
        summary_model: get_remote_summary_model(),
        tts_model: get_remote_tts_model(),
    };
    if let Err(e) = save_remote_asr_config_to_disk(&config) {
        warn!("Failed to persist remote config: {}", e);
    }
}

/// 只更新远程服务器地址（设置页「高级」入口；空串 = 清除自定义值、恢复内置默认）。
/// 不动 license / 模型选择；持久化到 remote_asr_config.json（endpoint 字段只存自定义值）。
pub fn set_remote_endpoint(endpoint: &str) {
    if let Ok(mut e) = REMOTE_ASR_ENDPOINT.lock() {
        *e = endpoint.to_string();
    }
    let config = RemoteAsrPersistedConfig {
        endpoint: endpoint.to_string(),
        model: get_remote_asr_model(),
        asr_mode: get_remote_asr_mode(),
        asr_offline_model: REMOTE_ASR_OFFLINE_MODEL
            .lock()
            .map(|m| m.clone())
            .unwrap_or_default(),
        translate_model: get_remote_translate_model(),
        summary_model: get_remote_summary_model(),
        tts_model: get_remote_tts_model(),
    };
    if let Err(e) = save_remote_asr_config_to_disk(&config) {
        warn!("Failed to persist remote endpoint: {}", e);
    }
}

pub fn get_remote_asr_endpoint() -> String {
    REMOTE_ASR_ENDPOINT
        .lock()
        .map(|e| e.clone())
        .unwrap_or_default()
}

/// 生效的远程服务器地址：用户自定义值，未自定义（空）时回落内置默认地址。
/// 所有实际发起请求/健康检查的消费点一律用这个，不要用 get_remote_asr_endpoint()。
pub fn effective_remote_endpoint() -> String {
    let stored = get_remote_asr_endpoint();
    if stored.is_empty() {
        DEFAULT_REMOTE_ENDPOINT.to_string()
    } else {
        stored
    }
}

pub fn get_remote_asr_model() -> String {
    REMOTE_ASR_MODEL
        .lock()
        .map(|m| m.clone())
        .unwrap_or_default()
}

/// 远程实时 ASR 模型的「显式指定」入口（2026-09-28 竞态修复）。
///
/// 背景：前端模型选择走全局 store，持久化到 remote_asr_config.json 是异步 fire-and-forget；
/// 用户「刚切完模型就点开始录音」时，持久化尚未落盘/未同步到 REMOTE_ASR_MODEL，
/// 引擎解析配置占位名（qwen3-asr-remote）时拿到的还是**上一次**持久化的旧模型 ——
/// 整场录音跑在旧模型上，UI 显示新模型（生产实测：UI 显示 qwen、实际跑了 4.8 分钟 doubao）。
///
/// 修法：开始录音 / 音频测试 / 热切换时，前端把**当前真实选择**（useRemoteModelChoice('asr').value）
/// 一并传给后端，后端调本函数以此为权威：
///   · 空串 / 占位名（"remote"、"qwen3-asr-remote*"）→ 忽略，保留现有选择（坑 #40 同款保护）；
///   · 与当前生效模型不一致 → WARN（这就是「配置滞后」的实锤日志）并以显式指定为准，
///     同步写内存 + 落盘（不再依赖先前的异步持久化）。
/// 返回值：true = 采用了显式指定（含本来就一致的情况）；false = 入参是占位名/空，未动。
/// `apply_explicit_remote_asr_model` 的纯判定部分（便于单测、不触磁盘/全局态）：
/// 入参是真实模型 id 时返回 Some(应采用的模型)；占位名/空串返回 None（不动现有选择）。
pub(crate) fn resolve_explicit_remote_asr_model(explicit: &str) -> Option<String> {
    let explicit = explicit.trim();
    if explicit.is_empty() || explicit == "remote" || explicit.starts_with("qwen3-asr-remote") {
        return None;
    }
    Some(explicit.to_string())
}

pub fn apply_explicit_remote_asr_model(explicit: &str) -> bool {
    let Some(explicit) = resolve_explicit_remote_asr_model(explicit) else {
        return false;
    };
    let current = get_remote_asr_model();
    if current == explicit {
        return true;
    }
    warn!(
        "远程 ASR 模型配置滞后：配置里是 '{}'，调用方显式指定 '{}' —— 以显式指定为准并同步持久化",
        current, explicit
    );
    if let Err(e) = set_remote_models(Some(explicit), None, None, None, None, None) {
        warn!("同步显式远程 ASR 模型失败: {}", e);
    }
    true
}

pub fn get_remote_asr_mode() -> String {
    REMOTE_ASR_MODE
        .lock()
        .map(|m| m.clone())
        .unwrap_or_default()
}

/// 远程离线识别模型（历史记录重识别/导入转写）。未单独设置时回退到实时选择（兼容旧配置）；
/// 但实时选择若是流式模型（-realtime/-streaming）则返回空——离线重识别必须走非流式模型
/// （网关批量端点拒绝流式模型；流式按流式价计费且整段上传会产生重复文本）。
pub fn get_remote_asr_offline_model() -> String {
    let offline = REMOTE_ASR_OFFLINE_MODEL
        .lock()
        .map(|m| m.clone())
        .unwrap_or_default();
    if !offline.is_empty() {
        return offline;
    }
    let realtime = get_remote_asr_model();
    if is_remote_asr_streaming_model(&realtime) {
        return String::new();
    }
    realtime
}

/// 判断远程 ASR 模型是否为「流式」：模型 id 带 -realtime / -streaming 后缀。
/// 仅作为「未设置 mode 时」的兜底判断，与网关模型目录的 mode=streaming 约定一致。
/// 注意豆包等新供应商的命名带版本号后缀（如 doubao-asr-streaming-2.0），
/// 因此 -streaming 用「包含」匹配（2026-09-20：设置页漏传 asr_mode 时豆包被
/// 误判成非流式，音频测试走批量端点 400）。
pub fn is_remote_asr_streaming_model(model_name: &str) -> bool {
    model_name.ends_with("-realtime") || model_name.contains("-streaming")
}

/// 远程 ASR 是否走流式：优先用前端传来的 mode（网关模型目录下发），
/// 未设置时回退到模型名后缀（兼容旧配置 / 旧调用路径）。
pub fn is_remote_asr_streaming() -> bool {
    let mode = get_remote_asr_mode();
    if !mode.is_empty() {
        return mode == "streaming";
    }
    is_remote_asr_streaming_model(&get_remote_asr_model())
}

pub fn get_remote_translate_model() -> String {
    REMOTE_TRANSLATE_MODEL
        .lock()
        .map(|m| m.clone())
        .unwrap_or_default()
}

/// 会议总结用的远程模型。未单独设置时回退到 translate 选择（兼容旧配置 ——
/// 该字段是 2026-09-23 才拆出来的，此前总结与翻译共用 translate）。
pub fn get_remote_summary_model() -> String {
    let m = REMOTE_SUMMARY_MODEL
        .lock()
        .map(|m| m.clone())
        .unwrap_or_default();
    if !m.is_empty() {
        return m;
    }
    get_remote_translate_model()
}

pub fn get_remote_tts_model() -> String {
    REMOTE_TTS_MODEL
        .lock()
        .map(|m| m.clone())
        .unwrap_or_default()
}

pub fn get_remote_license() -> String {
    REMOTE_LICENSE.lock().map(|l| l.clone()).unwrap_or_default()
}

/// 只设置授权码（供自动注册后写入），并持久化到 keychain。
pub fn set_remote_license(license: &str) {
    if let Ok(mut l) = REMOTE_LICENSE.lock() {
        *l = license.to_string();
    }
    save_license_to_keyring(license);
}

/// 设置各能力的远程模型（模型选择器），并持久化到磁盘。
/// `asr_mode` 由前端从网关 /v1/models 的 mode 字段透传，用于决定远程 ASR 走流式还是非流式。
/// `asr_offline` 是历史记录离线重识别专用的远程模型（非流式），与实时选择分开存。
pub fn set_remote_models(
    asr: Option<String>,
    asr_mode: Option<String>,
    translate: Option<String>,
    tts: Option<String>,
    asr_offline: Option<String>,
    summary: Option<String>,
) -> Result<(), String> {
    if let Some(m) = asr {
        if let Ok(mut g) = REMOTE_ASR_MODEL.lock() {
            *g = m;
        }
    }
    if let Some(m) = asr_mode {
        if let Ok(mut g) = REMOTE_ASR_MODE.lock() {
            *g = m;
        }
    }
    if let Some(m) = translate {
        if let Ok(mut g) = REMOTE_TRANSLATE_MODEL.lock() {
            *g = m;
        }
    }
    if let Some(m) = tts {
        if let Ok(mut g) = REMOTE_TTS_MODEL.lock() {
            *g = m;
        }
    }
    if let Some(m) = asr_offline {
        if let Ok(mut g) = REMOTE_ASR_OFFLINE_MODEL.lock() {
            *g = m;
        }
    }
    if let Some(m) = summary {
        if let Ok(mut g) = REMOTE_SUMMARY_MODEL.lock() {
            *g = m;
        }
    }
    let config = RemoteAsrPersistedConfig {
        endpoint: get_remote_asr_endpoint(),
        model: get_remote_asr_model(),
        asr_mode: get_remote_asr_mode(),
        asr_offline_model: REMOTE_ASR_OFFLINE_MODEL
            .lock()
            .map(|m| m.clone())
            .unwrap_or_default(),
        translate_model: get_remote_translate_model(),
        summary_model: get_remote_summary_model(),
        tts_model: get_remote_tts_model(),
    };
    save_remote_asr_config_to_disk(&config)
}

/// 远程 ASR 是否已配置：基于 effective 地址（内置默认兜底，实际恒为 true；
/// 远程可用性只看 remote.enabled 总开关）。
pub fn is_remote_asr_configured() -> bool {
    !effective_remote_endpoint().is_empty()
}

/// 网关 /v1 根地址（基于 effective 地址；有内置默认兜底，实际永不返回 None）。
/// ASR / 翻译 / TTS / 总结都拼在其后。
pub fn remote_api_base() -> Option<String> {
    let endpoint = effective_remote_endpoint();
    if endpoint.is_empty() {
        return None;
    }
    Some(format!("{}/v1", endpoint.trim_end_matches('/')))
}

/// 健康检查地址（网关 /health，无需鉴权）。
pub fn remote_health_url(endpoint: &str) -> String {
    format!("{}/health", endpoint.trim_end_matches('/'))
}

// ── Persistence ──────────────────────────────────────────────────────
// Saves remote ASR endpoint+model to a JSON file in the OS config directory,
// and restores it on the next app launch.

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RemoteAsrPersistedConfig {
    endpoint: String,
    model: String,
    #[serde(default)]
    asr_mode: String,
    #[serde(default)]
    asr_offline_model: String,
    #[serde(default)]
    translate_model: String,
    #[serde(default)]
    summary_model: String,
    #[serde(default)]
    tts_model: String,
}

fn get_remote_asr_config_path() -> Option<PathBuf> {
    let mut path = dirs::config_dir()?;
    path.push("voxminutes");
    path.push("remote_asr_config.json");
    Some(path)
}

fn save_remote_asr_config_to_disk(config: &RemoteAsrPersistedConfig) -> Result<(), String> {
    let path = get_remote_asr_config_path()
        .ok_or_else(|| "Could not determine config directory".to_string())?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }
    let json = serde_json::to_string_pretty(config)
        .map_err(|e| format!("Failed to serialize remote ASR config: {}", e))?;
    // Use sync write — called from Tauri command handler which may be on a sync thread.
    std::fs::write(&path, json).map_err(|e| format!("Failed to write remote ASR config: {}", e))?;
    info!("Persisted remote ASR config to {}", path.display());
    Ok(())
}

// ── License 存 OS keychain ────────────────────────────────────────────
// Windows 走凭据管理器，macOS 走钥匙串，Linux 走 Secret Service（libsecret）。
// 读取在启动时做一次并缓存到 REMOTE_LICENSE，热路径不再访问 keychain。

const KEYRING_SERVICE: &str = "voxminutes";
const KEYRING_USER: &str = "remote-license";

fn save_license_to_keyring(license: &str) {
    match keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER) {
        Ok(entry) => {
            if license.is_empty() {
                let _ = entry.delete_credential();
            } else if let Err(e) = entry.set_password(license) {
                warn!("Failed to save license to keychain: {}", e);
            }
        }
        Err(e) => warn!("Keychain unavailable, license not persisted: {}", e),
    }
}

fn load_license_from_keyring() -> String {
    match keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER) {
        Ok(entry) => entry.get_password().unwrap_or_default(),
        Err(_) => String::new(),
    }
}

/// 远程配置（remote_asr_config.json + keychain 授权码）是否已在启动时加载完成。
/// 流式通道预检 warm-up 必须等这个信号：2026-09-28 实测启动后 0.9s 触发的 warm-up
/// 跑在配置加载之前，报「远程服务未配置（缺少授权码）」空打三炮后放弃，
/// 真正的预检直到用户进设置页才发生。用 watch channel：晚到的等待者借 borrow()
/// 也能看到已置位的值，不存在「查标志与注册之间错过通知」的竞态。
static REMOTE_CONFIG_LOADED_TX: std::sync::LazyLock<tokio::sync::watch::Sender<bool>> =
    std::sync::LazyLock::new(|| tokio::sync::watch::channel(false).0);

/// 标记远程配置加载完成并唤醒等待中的 warm-up（幂等）。
pub fn mark_remote_config_loaded() {
    let _ = REMOTE_CONFIG_LOADED_TX.send(true);
}

/// 等待远程配置加载完成（已加载则立即返回）。带上限的等待由调用方包 timeout。
pub async fn wait_remote_config_loaded() {
    let mut rx = REMOTE_CONFIG_LOADED_TX.subscribe();
    if *rx.borrow() {
        return;
    }
    let _ = rx.wait_for(|loaded| *loaded).await;
}

/// Load the remote config from disk and populate the in-memory statics.
/// Called once during app startup.
pub fn load_remote_asr_config_from_disk() {
    let path = match get_remote_asr_config_path() {
        Some(p) => p,
        None => return,
    };
    if path.exists() {
        match std::fs::read_to_string(&path) {
            Ok(content) => match serde_json::from_str::<RemoteAsrPersistedConfig>(&content) {
                Ok(config) => {
                    if let Ok(mut e) = REMOTE_ASR_ENDPOINT.lock() {
                        *e = config.endpoint.clone();
                    }
                    if let Ok(mut m) = REMOTE_ASR_MODEL.lock() {
                        *m = config.model.clone();
                    }
                    if let Ok(mut m) = REMOTE_ASR_MODE.lock() {
                        *m = config.asr_mode.clone();
                    }
                    if let Ok(mut m) = REMOTE_ASR_OFFLINE_MODEL.lock() {
                        *m = config.asr_offline_model.clone();
                    }
                    if let Ok(mut m) = REMOTE_TRANSLATE_MODEL.lock() {
                        *m = config.translate_model.clone();
                    }
                    if let Ok(mut m) = REMOTE_SUMMARY_MODEL.lock() {
                        *m = config.summary_model.clone();
                    }
                    if let Ok(mut m) = REMOTE_TTS_MODEL.lock() {
                        *m = config.tts_model.clone();
                    }
                    info!(
                        "Loaded remote config from {}: {} (model: {})",
                        path.display(),
                        config.endpoint,
                        config.model
                    );
                }
                Err(e) => {
                    warn!(
                        "Failed to parse remote config from {}: {}",
                        path.display(),
                        e
                    );
                }
            },
            Err(e) => {
                warn!(
                    "Failed to read remote config from {}: {}",
                    path.display(),
                    e
                );
            }
        }
    }

    // license 从 OS keychain 读回（与 JSON 解耦）
    let license = load_license_from_keyring();
    if !license.is_empty() {
        if let Ok(mut l) = REMOTE_LICENSE.lock() {
            *l = license;
        }
    }
}

pub async fn validate_transcription_model_ready<R: Runtime>(
    app: &AppHandle<R>,
) -> Result<(), String> {
    let mut config =
        match crate::api::api::api_get_transcript_config(app.clone(), app.clone().state(), None)
            .await
        {
            Ok(Some(config)) => {
                info!(
                    "📝 Found transcript config - provider: {}, model: {}",
                    config.provider, config.model
                );
                config
            }
            Ok(None) | Err(_) => {
                info!("📝 No transcript config found, defaulting to sherpaonnx");
                crate::api::api::TranscriptConfig {
                    provider: "sherpaonnx".to_string(),
                    model: "sense-voice".to_string(),
                    api_key: None,
                }
            }
        };

    let is_xasr = config.model.starts_with("x-asr-");
    // 显式选择优先（与翻译引擎口径一致，见 HANDOVER 坑 #8）：只有配置里明确选了远程才走远程；
    // 远程总开关只决定「远程是否可选」。存了远程选择但总开关已关 → 回落本地 SenseVoice。
    let is_remote_selected = config.model == "qwen3-asr-remote"
        || config.model.starts_with("qwen3-asr-remote")
        || config.provider == "remote-qwen3-asr";
    if is_remote_selected && !remote_enabled() {
        warn!("远程 ASR 已选择但总开关已关，回落本地 SenseVoice");
        config.provider = "sherpaonnx".to_string();
        config.model = "sense-voice".to_string();
    }
    let is_remote = config.model == "qwen3-asr-remote"
        || config.model.starts_with("qwen3-asr-remote")
        || config.provider == "remote-qwen3-asr";

    if is_xasr {
        info!("🔍 Validating X-ASR model: {}", config.model);
        if !crate::sherpa_onnx_engine::commands::is_xasr_engine_loaded() {
            crate::sherpa_onnx_engine::commands::sherpa_onnx_load_model(config.model.clone())
                .await
                .map_err(|e| format!("X-ASR model loading failed: {}", e))?;
        }
        info!("✅ X-ASR ready (Rust-native)");
        return Ok(());
    }

    if is_remote {
        let endpoint = effective_remote_endpoint();
        if endpoint.is_empty() {
            // 内置默认兜底下实际不可达，保留作防御
            return Err(
                "Remote ASR endpoint not configured. Please set the remote ASR URL in Settings."
                    .to_string(),
            );
        }
        let model_name = get_remote_asr_model();
        // 配置占位名解析兜底：远程已选但真实模型为空（配置损坏/被占位名覆盖）时
        // 明确报错，而不是拿空模型名去打网关（必然 400，且排查困难）。
        if model_name.is_empty() {
            return Err(
                "未选择远程识别模型。请在录音弹窗或「设置 → 远程服务」里选择识别模型。".to_string(),
            );
        }
        let is_streaming = is_remote_asr_streaming();
        if is_streaming {
            info!(
                "🔍 Validating remote STREAMING ASR at: {} model={}",
                endpoint, model_name
            );
        } else {
            info!(
                "🔍 Validating remote ASR at: {} model={}",
                endpoint, model_name
            );
        }
        // 流式：先打 /health（HTTP），再**真实握手一次 WS**。只查 /health 发现不了 WS 通道问题
        // （TLS feature 缺失 / 鉴权 / 路由）—— 2026-09-22 事故就是「测试连接 OK、这里打 ✅，
        // 一开录却 0 字符」。非流式本身就是 HTTP，校验完整健康即可。
        if is_streaming {
            if !super::remote_asr_provider::check_remote_asr_health(&endpoint).await {
                return Err(format!(
                    "Cannot connect to remote ASR at {}. Please check the server is running.",
                    endpoint
                ));
            }
            super::remote_asr_streaming_provider::probe_streaming_channel()
                .await
                .map_err(|e| format!("远程流式识别通道不可用：{}", e))?;
        } else {
            let provider = RemoteAsrProvider::new(endpoint.clone(), model_name);
            if !provider.check_health().await {
                return Err(format!(
                    "Cannot connect to remote ASR at {}. Please check the server is running.",
                    endpoint
                ));
            }
        }
        info!("✅ Remote ASR ready");
        return Ok(());
    }

    if config.provider != "sherpaonnx" {
        return Err(format!(
            "Provider '{}' is not supported. Use 'sherpaonnx', 'x-asr', or 'remote-qwen3-asr'.",
            config.provider
        ));
    }

    info!("🔍 Validating Sherpa-ONNX model...");
    if let Err(e) = crate::sherpa_onnx_engine::commands::sherpa_onnx_init().await {
        return Err(format!("Failed to init Sherpa-ONNX: {}", e));
    }
    if !crate::sherpa_onnx_engine::commands::sherpa_onnx_is_model_loaded()
        .await
        .unwrap_or(false)
    {
        info!("🔧 Auto-loading Sherpa-ONNX model: {}", config.model);
        crate::sherpa_onnx_engine::commands::sherpa_onnx_load_model(config.model.clone())
            .await
            .map_err(|e| format!("Failed to load Sherpa-ONNX model '{}': {}", config.model, e))?;
    }
    info!("✅ Sherpa-ONNX model ready");
    Ok(())
}

pub async fn get_or_init_transcription_engine<R: Runtime>(
    app: &AppHandle<R>,
) -> Result<TranscriptionEngine, String> {
    let mut config =
        match crate::api::api::api_get_transcript_config(app.clone(), app.clone().state(), None)
            .await
        {
            Ok(Some(config)) => {
                // provider/model 对远程是**前端占位名**（remote-qwen3-asr / qwen3-asr-remote），
                // 真正生效的模型在 remote 配置里。不打印真实模型会把排查带偏：
                // 2026-09-22 用户反馈「选了豆包、日志却显示 qwen3-asr-remote」正是这条日志。
                if config.provider == "remote-qwen3-asr"
                    || config.model.starts_with("qwen3-asr-remote")
                {
                    info!(
                        "📝 Transcript config - provider: {}, model: {}（远程实时模型: {}）",
                        config.provider,
                        config.model,
                        get_remote_asr_model()
                    );
                } else {
                    info!(
                        "📝 Transcript config - provider: {}, model: {}",
                        config.provider, config.model
                    );
                }
                config
            }
            Ok(None) | Err(_) => {
                info!("📝 No transcript config found, defaulting to sherpaonnx");
                crate::api::api::TranscriptConfig {
                    provider: "sherpaonnx".to_string(),
                    model: "sense-voice".to_string(),
                    api_key: None,
                }
            }
        };

    let is_xasr = config.model.starts_with("x-asr-");
    // 显式选择优先（与翻译引擎口径一致，见 HANDOVER 坑 #8）：只有配置里明确选了远程才走远程；
    // 远程总开关只决定「远程是否可选」。存了远程选择但总开关已关 → 回落本地 SenseVoice。
    let is_remote_selected = config.model == "qwen3-asr-remote"
        || config.model.starts_with("qwen3-asr-remote")
        || config.provider == "remote-qwen3-asr";
    if is_remote_selected && !remote_enabled() {
        warn!("远程 ASR 已选择但总开关已关，回落本地 SenseVoice");
        config.provider = "sherpaonnx".to_string();
        config.model = "sense-voice".to_string();
    }
    let is_remote = config.model == "qwen3-asr-remote"
        || config.model.starts_with("qwen3-asr-remote")
        || config.provider == "remote-qwen3-asr";

    if is_xasr {
        info!(
            "🦊 Initializing X-ASR streaming transcription engine (Rust-native): {}",
            config.model
        );
        // Load the X-ASR OnlineRecognizer engine if not already loaded
        if !crate::sherpa_onnx_engine::commands::is_xasr_engine_loaded() {
            crate::sherpa_onnx_engine::commands::sherpa_onnx_load_model(config.model.clone())
                .await
                .map_err(|e| format!("Failed to load X-ASR model: {}", e))?;
        }
        let engine = crate::sherpa_onnx_engine::commands::get_or_init_xasr_engine()
            .map_err(|e| format!("X-ASR engine not ready: {}", e))?;
        let provider = XAsrProvider::new_with_engine(config.model.clone(), engine);
        info!("✅ X-ASR provider ready (Rust-native OnlineRecognizer)");
        return Ok(TranscriptionEngine::Provider(Arc::new(provider)));
    }

    if is_remote {
        let endpoint = effective_remote_endpoint();
        let model_name = get_remote_asr_model();
        if model_name.is_empty() {
            return Err(
                "未选择远程识别模型。请在录音弹窗或「设置 → 远程服务」里选择识别模型。".to_string(),
            );
        }
        let is_streaming = is_remote_asr_streaming();

        if is_streaming {
            info!(
                "🦊 Initializing remote STREAMING ASR engine at: {} model={}",
                endpoint, model_name
            );
            // 语言语义：'auto' = 交给上游自动检测语言，而不是强制中文。
            // 百炼 realtime 不接受字面 'auto'（400 InvalidParameter，整场 0 字符）；
            // 实测省略 language 字段时上游按发音自动检测（中英混说各识别各的），
            // 因此 'auto'/空 → 不传 language；zh/en 等具体语言才透传。
            let language =
                crate::get_language_preference_internal().unwrap_or_else(|| "auto".to_string());
            let language = if language.trim().is_empty() || language == "auto" {
                String::new()
            } else {
                language
            };
            let provider = super::remote_asr_streaming_provider::RemoteAsrStreamingProvider::new(
                endpoint.clone(),
                model_name.clone(),
                language,
            );
            // 连通性预检在 validate_transcription_model_ready（会真握一次 WS）；
            // is_model_loaded 这里只确认「远程已配置」，不是健康探测。
            let _ = provider.is_model_loaded().await;
            info!("✅ Remote streaming ASR provider ready");
            return Ok(TranscriptionEngine::Provider(Arc::new(provider)));
        }

        let provider = RemoteAsrProvider::new(endpoint.clone(), model_name.clone());
        provider.check_health().await;
        let detected_model = provider.detect_model_name().await;

        let provider = if !detected_model.is_empty() && detected_model != model_name {
            info!("🔄 Using detected model: {}", detected_model);
            RemoteAsrProvider::new(endpoint, detected_model)
        } else {
            provider
        };

        // The final provider instance (especially after model-name detection) must
        // have a cached successful health check, otherwise the worker will see
        // `is_model_loaded() == false` and skip every audio chunk.
        provider.check_health().await;

        info!("✅ Remote ASR provider ready");
        return Ok(TranscriptionEngine::Provider(Arc::new(provider)));
    }

    if config.provider != "sherpaonnx" {
        return Err(format!(
            "Provider '{}' is not supported. Use 'sherpaonnx', 'x-asr', or 'remote-qwen3-asr'.",
            config.provider
        ));
    }

    info!("🦊 Initializing Sherpa-ONNX native transcription engine");
    if !crate::sherpa_onnx_engine::commands::sherpa_onnx_is_model_loaded()
        .await
        .unwrap_or(false)
    {
        info!("🔧 Auto-loading Sherpa-ONNX model: {}", config.model);
        crate::sherpa_onnx_engine::commands::sherpa_onnx_load_model(config.model.clone())
            .await
            .map_err(|e| format!("Failed to load Sherpa-ONNX model: {}", e))?;
    }
    let engine = crate::sherpa_onnx_engine::commands::get_or_init_engine()
        .map_err(|e| format!("Sherpa-ONNX engine not ready: {}", e))?;
    let provider =
        crate::audio::transcription::sherpa_onnx_provider::SherpaOnnxProvider::new(engine);
    info!("✅ Sherpa-ONNX provider ready");
    Ok(TranscriptionEngine::Provider(Arc::new(provider)))
}

/// Build the SSE partial-result emitter callback used by streaming remote ASR providers.
/// Centralized here to avoid duplicating the same ~35 lines in model-selection branches.
fn build_sse_emitter<R: Runtime>(
    app: AppHandle<R>,
    chunk_context: Arc<std::sync::Mutex<Option<super::remote_asr_provider::ChunkContext>>>,
) -> Arc<dyn Fn(&str, bool) + Send + Sync> {
    Arc::new(move |text: &str, is_partial: bool| {
        let ctx = match chunk_context.lock().ok() {
            Some(guard) => guard,
            None => {
                warn!("SSE partial callback: chunk_context lock failed");
                return;
            }
        };
        let ctx = match &*ctx {
            Some(c) => c,
            None => {
                warn!("SSE partial callback: chunk_context is None");
                return;
            }
        };
        let update = TranscriptUpdate {
            text: text.to_string(),
            timestamp: format_timestamp_simple(),
            source: "Audio".to_string(),
            sequence_id: ctx.sequence_id,
            chunk_start_time: ctx.chunk_start_time,
            is_partial,
            confidence: 0.9,
            audio_start_time: ctx.audio_start_time,
            audio_end_time: ctx.audio_end_time,
            duration: ctx.duration,
            paragraph_id: None,
        };
        match app.emit("transcript-update", &update) {
            Ok(_) => {
                if is_partial {
                    debug!(
                        "🔵 Emitted partial transcript seq={} text_len={}",
                        ctx.sequence_id,
                        text.len()
                    );
                } else {
                    info!(
                        "🔵 Emitted final transcript seq={} text_len={}",
                        ctx.sequence_id,
                        text.len()
                    );
                }
            }
            Err(e) => warn!("SSE partial callback: emit failed: {}", e),
        }
    })
}

/// Simple timestamp formatter for use in callback closures
/// (cannot call worker::format_current_timestamp because it's in a different module)
fn format_timestamp_simple() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let hours = (now.as_secs() / 3600) % 24;
    let minutes = (now.as_secs() / 60) % 60;
    let seconds = now.as_secs() % 60;
    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
}

#[cfg(test)]
mod tests {
    use super::resolve_explicit_remote_asr_model;

    #[test]
    fn explicit_model_real_ids_are_authoritative() {
        // 真实模型 id（用户当前选择）→ 采用
        assert_eq!(
            resolve_explicit_remote_asr_model("doubao-asr-streaming-2.0"),
            Some("doubao-asr-streaming-2.0".to_string())
        );
        assert_eq!(
            resolve_explicit_remote_asr_model("qwen-audio-3.0-asr-flash-realtime"),
            Some("qwen-audio-3.0-asr-flash-realtime".to_string())
        );
        // 前后空白不影响判定
        assert_eq!(
            resolve_explicit_remote_asr_model("  deepgram-nova-3-streaming  "),
            Some("deepgram-nova-3-streaming".to_string())
        );
    }

    #[test]
    fn explicit_model_placeholders_are_ignored() {
        // 占位名/空串 → None：不得覆盖现有选择（坑 #40 同款保护；
        // 2026-09-28 竞态修复后，占位名绝不能再写回 REMOTE_ASR_MODEL）
        assert_eq!(resolve_explicit_remote_asr_model(""), None);
        assert_eq!(resolve_explicit_remote_asr_model("   "), None);
        assert_eq!(resolve_explicit_remote_asr_model("remote"), None);
        assert_eq!(resolve_explicit_remote_asr_model("qwen3-asr-remote"), None);
        assert_eq!(
            resolve_explicit_remote_asr_model("qwen3-asr-remote-streaming"),
            None
        );
    }
}
