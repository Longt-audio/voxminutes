use log::{error as log_error, info as log_info, warn as log_warn};
use serde::{Deserialize, Serialize};
use std::sync::{LazyLock, Mutex};
use tauri::{AppHandle, Emitter, Manager, Runtime, WebviewWindowBuilder};
use tauri_plugin_store::StoreExt;

const WINDOW_LABEL: &str = "subtitle-overlay";
const WINDOW_URL: &str = "/subtitle-overlay.html";
const STORE_NAME: &str = "subtitle_overlay.json";
const STORE_KEY: &str = "state";

const DEFAULT_WIDTH: f64 = 800.0;
const DEFAULT_HEIGHT: f64 = 240.0;
const TOP_MARGIN: f64 = 40.0;

/// 悬浮窗句柄保留的**段数上限**（2026-09-29：4 → 50）。
///
/// 原来只留 4 段且超限直接 `remove(0)` 销毁，所以「往上滚看之前的」根本无从谈起
/// （用户反馈「只显示最近几句」）。现在按 ~7.5 段/分钟计，50 段 ≈ 最近 7 分钟，
/// 内存也就 ~25KB，与 5Hz 轮询的 2KB/次相比完全不是瓶颈。
/// 默认**显示**几行由页面端设置决定（默认 5 行），这里只管缓冲能回溯多久。
const MAX_SUBTITLE_SEGMENTS: usize = 50;

/// 窗口高度可由页面按「行数 × 实测行高」调用 `set_subtitle_overlay_height` 调整，
/// 这里给出上下限（原上限 480 只够约 7 行带译文的字幕）。
const MIN_WINDOW_HEIGHT: f64 = 120.0;
const MAX_WINDOW_HEIGHT: f64 = 900.0;

// ── In-memory subtitle segment buffer ────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtitleSegment {
    pub sequence_id: u64,
    pub text: String,
    pub is_partial: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub translation: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SubtitleSegmentInput {
    pub sequence_id: u64,
    pub text: String,
    pub is_partial: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SubtitleTranslationInput {
    pub sequence_id: u64,
    pub translated_text: String,
}

static SUBTITLE_SEGMENTS: LazyLock<Mutex<Vec<SubtitleSegment>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));

pub(crate) fn upsert_subtitle_segment(input: SubtitleSegmentInput) {
    let mut segments = SUBTITLE_SEGMENTS.lock().unwrap();
    if let Some(idx) = segments
        .iter()
        .position(|s| s.sequence_id == input.sequence_id)
    {
        segments[idx].text = input.text;
        segments[idx].is_partial = input.is_partial;
    } else {
        segments.push(SubtitleSegment {
            sequence_id: input.sequence_id,
            text: input.text,
            is_partial: input.is_partial,
            translation: None,
        });
    }
    segments.sort_by_key(|s| s.sequence_id);
    while segments.len() > MAX_SUBTITLE_SEGMENTS {
        segments.remove(0);
    }
}

pub(crate) fn upsert_subtitle_translation(input: SubtitleTranslationInput) {
    let mut segments = SUBTITLE_SEGMENTS.lock().unwrap();
    if let Some(idx) = segments
        .iter()
        .position(|s| s.sequence_id == input.sequence_id)
    {
        segments[idx].translation = Some(input.translated_text);
    } else {
        segments.push(SubtitleSegment {
            sequence_id: input.sequence_id,
            text: String::new(),
            is_partial: false,
            translation: Some(input.translated_text),
        });
        segments.sort_by_key(|s| s.sequence_id);
        while segments.len() > MAX_SUBTITLE_SEGMENTS {
            segments.remove(0);
        }
    }
}

pub fn clear_subtitle_segments_internal() {
    let mut segments = SUBTITLE_SEGMENTS.lock().unwrap();
    segments.clear();
}

#[tauri::command]
pub fn push_subtitle_segment(update: SubtitleSegmentInput) {
    upsert_subtitle_segment(update);
}

#[tauri::command]
pub fn push_subtitle_translation(update: SubtitleTranslationInput) {
    upsert_subtitle_translation(update);
}

#[tauri::command]
pub fn get_subtitle_segments() -> Vec<SubtitleSegment> {
    SUBTITLE_SEGMENTS.lock().unwrap().clone()
}

#[tauri::command]
pub fn clear_subtitle_segments() {
    clear_subtitle_segments_internal();
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtitleOverlayState {
    #[serde(default)]
    pub visible: bool,
    #[serde(default)]
    pub x: f64,
    #[serde(default)]
    pub y: f64,
}

impl Default for SubtitleOverlayState {
    fn default() -> Self {
        Self {
            visible: false,
            x: 0.0,
            y: 0.0,
        }
    }
}

fn default_position<R: Runtime>(app: &AppHandle<R>) -> (f64, f64) {
    let monitor = app
        .primary_monitor()
        .ok()
        .flatten()
        .or_else(|| app.available_monitors().ok().into_iter().flatten().next());

    if let Some(m) = monitor {
        let scale = m.scale_factor();
        let screen_x = m.position().x as f64 / scale;
        let screen_y = m.position().y as f64 / scale;
        let screen_w = m.size().width as f64 / scale;
        let x = screen_x + (screen_w - DEFAULT_WIDTH) / 2.0;
        let y = screen_y + TOP_MARGIN;
        (x.max(screen_x), y.max(screen_y))
    } else {
        (100.0, TOP_MARGIN)
    }
}

fn clamp_to_monitor<R: Runtime>(app: &AppHandle<R>, x: f64, y: f64) -> (f64, f64) {
    let monitor = app
        .available_monitors()
        .ok()
        .into_iter()
        .flatten()
        .find(|m| {
            let pos = m.position();
            let size = m.size();
            let sx = pos.x as f64;
            let sy = pos.y as f64;
            let sw = size.width as f64 / m.scale_factor();
            let sh = size.height as f64 / m.scale_factor();
            x >= sx && x < sx + sw && y >= sy && y < sy + sh
        })
        .or_else(|| app.primary_monitor().ok().flatten())
        .or_else(|| app.available_monitors().ok().into_iter().flatten().next());

    if let Some(m) = monitor {
        let scale = m.scale_factor();
        let screen_x = m.position().x as f64 / scale;
        let screen_y = m.position().y as f64 / scale;
        let screen_w = m.size().width as f64 / scale;
        let screen_h = m.size().height as f64 / scale;

        let mut px = x;
        let mut py = y;
        if px + DEFAULT_WIDTH > screen_x + screen_w {
            px = screen_x + screen_w - DEFAULT_WIDTH;
        }
        if py + DEFAULT_HEIGHT > screen_y + screen_h {
            py = screen_y + screen_h - DEFAULT_HEIGHT;
        }
        if px < screen_x {
            px = screen_x;
        }
        if py < screen_y {
            py = screen_y;
        }
        (px, py)
    } else {
        (x.max(0.0), y.max(0.0))
    }
}

async fn load_state<R: Runtime>(app: &AppHandle<R>) -> SubtitleOverlayState {
    match app.store(STORE_NAME) {
        Ok(store) => {
            if let Some(value) = store.get(STORE_KEY) {
                match serde_json::from_value::<SubtitleOverlayState>(value.clone()) {
                    Ok(state) => return state,
                    Err(e) => log_warn!("Failed to deserialize subtitle overlay state: {}", e),
                }
            }
        }
        Err(e) => log_warn!("Failed to access subtitle overlay store: {}", e),
    }

    let (x, y) = default_position(app);
    SubtitleOverlayState {
        visible: false,
        x,
        y,
    }
}

async fn save_state<R: Runtime>(app: &AppHandle<R>, state: &SubtitleOverlayState) {
    match app.store(STORE_NAME) {
        Ok(store) => match serde_json::to_value(state) {
            Ok(value) => {
                store.set(STORE_KEY, value);
                if let Err(e) = store.save() {
                    log_warn!("Failed to persist subtitle overlay state: {}", e);
                }
            }
            Err(e) => log_warn!("Failed to serialize subtitle overlay state: {}", e),
        },
        Err(e) => log_warn!("Failed to access subtitle overlay store: {}", e),
    }
}

async fn get_or_create_window<R: Runtime>(
    app: &AppHandle<R>,
) -> Result<tauri::WebviewWindow<R>, String> {
    if let Some(window) = app.get_webview_window(WINDOW_LABEL) {
        return Ok(window);
    }

    let mut state = load_state(app).await;
    if state.x <= 0.0 && state.y <= 0.0 {
        let (x, y) = default_position(app);
        state.x = x;
        state.y = y;
    }
    let (x, y) = clamp_to_monitor(app, state.x, state.y);
    state.x = x;
    state.y = y;

    let window =
        WebviewWindowBuilder::new(app, WINDOW_LABEL, tauri::WebviewUrl::App(WINDOW_URL.into()))
            .title("")
            .inner_size(DEFAULT_WIDTH, DEFAULT_HEIGHT)
            .min_inner_size(400.0, MIN_WINDOW_HEIGHT)
            .max_inner_size(1200.0, MAX_WINDOW_HEIGHT)
            .resizable(true)
            .maximizable(false)
            .minimizable(false)
            .closable(true)
            .skip_taskbar(true)
            .always_on_top(true)
            .transparent(true)
            .decorations(false)
            .shadow(false)
            // 非前台时第一次点击也能被接收（2026-09-29）：不设这一项时，
            // 窗口不在前台时点滚动条/按钮会被系统吞掉（悬浮球为此专门修过，
            // 见 floating_ball.rs 的注释）—— 现在悬浮窗要支持上滑翻看历史，
            // 滚动条必须一次点中。
            .accept_first_mouse(true)
            .position(x, y)
            .visible(false)
            .build()
            .map_err(|e| format!("Failed to create subtitle overlay window: {}", e))?;

    save_state(app, &state).await;
    Ok(window)
}

pub async fn show_subtitle_overlay_internal<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    log_info!("Showing subtitle overlay window...");

    if app.get_webview_window(WINDOW_LABEL).is_none() {
        let _ = get_or_create_window(app).await?;
    }

    if let Some(window) = app.get_webview_window(WINDOW_LABEL) {
        let mut state = load_state(app).await;
        let (x, y) = clamp_to_monitor(app, state.x, state.y);
        let _ = window.set_position(tauri::Position::Logical(tauri::LogicalPosition { x, y }));
        state.x = x;
        state.y = y;
        state.visible = true;

        window
            .show()
            .map_err(|e| format!("Failed to show subtitle overlay window: {}", e))?;

        save_state(app, &state).await;
        let _ = app.emit(
            "subtitle-window-state",
            serde_json::json!({ "visible": true }),
        );
        log_info!("Subtitle overlay window shown at ({}, {})", x, y);
    }

    Ok(())
}

pub async fn hide_subtitle_overlay_internal<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    log_info!("Hiding subtitle overlay window...");

    if let Some(window) = app.get_webview_window(WINDOW_LABEL) {
        if let Ok(pos) = window.outer_position() {
            let scale = window.scale_factor().unwrap_or(1.0);
            let mut state = load_state(app).await;
            state.x = pos.x as f64 / scale;
            state.y = pos.y as f64 / scale;
            state.visible = false;
            save_state(app, &state).await;
        }

        window
            .hide()
            .map_err(|e| format!("Failed to hide subtitle overlay window: {}", e))?;
        let _ = app.emit(
            "subtitle-window-state",
            serde_json::json!({ "visible": false }),
        );
        log_info!("Subtitle overlay window hidden");
    } else {
        let _ = app.emit(
            "subtitle-window-state",
            serde_json::json!({ "visible": false }),
        );
    }

    Ok(())
}

#[tauri::command]
pub async fn show_subtitle_window<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    show_subtitle_overlay_internal(&app).await
}

#[tauri::command]
pub async fn hide_subtitle_window<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    hide_subtitle_overlay_internal(&app).await
}

#[tauri::command]
pub async fn toggle_subtitle_window<R: Runtime>(app: AppHandle<R>) -> Result<bool, String> {
    let state = load_state(&app).await;
    if state.visible {
        hide_subtitle_overlay_internal(&app).await?;
        Ok(false)
    } else {
        show_subtitle_overlay_internal(&app).await?;
        Ok(true)
    }
}

#[tauri::command]
pub async fn get_subtitle_window_state<R: Runtime>(app: AppHandle<R>) -> Result<bool, String> {
    let state = load_state(&app).await;
    Ok(state.visible)
}

/// 按「显示行数」调整悬浮窗高度（2026-09-29）。
///
/// 为什么由页面端算好高度再传进来：可见行数取决于字号、是否带译文等 CSS 变量，
/// 只有页面自己能量准（`lines × 实测行高 + chrome`）。而页面端**没有**
/// `core:window:allow-set-size` 权限，所以由这个命令代劳。
/// 高度会按上下限夹取；窗口位置不动（左上角锚定，向下长高）。
#[tauri::command]
pub async fn set_subtitle_overlay_height<R: Runtime>(
    app: AppHandle<R>,
    height: f64,
) -> Result<f64, String> {
    let clamped = height.clamp(MIN_WINDOW_HEIGHT, MAX_WINDOW_HEIGHT);
    let Some(window) = app.get_webview_window(WINDOW_LABEL) else {
        // 窗口还没建（首次显示前）：只返回夹取后的值，页面下次轮询会再调一次
        return Ok(clamped);
    };
    let scale = window.scale_factor().unwrap_or(1.0);
    let width = window
        .inner_size()
        .map(|s| s.width as f64 / scale)
        .unwrap_or(DEFAULT_WIDTH);
    window
        .set_size(tauri::Size::Logical(tauri::LogicalSize {
            width,
            height: clamped,
        }))
        .map_err(|e| format!("Failed to resize subtitle overlay: {}", e))?;
    Ok(clamped)
}

#[tauri::command]
pub async fn start_subtitle_drag<R: Runtime>(
    window: tauri::WebviewWindow<R>,
) -> Result<(), String> {
    window.start_dragging().map_err(|e| e.to_string())
}

/// Restore the saved visibility on app startup. Should be called from setup().
pub async fn restore_subtitle_overlay_on_startup<R: Runtime>(app: &AppHandle<R>) {
    let state = load_state(app).await;
    log_info!(
        "Restoring subtitle overlay state: visible={}, pos=({}, {})",
        state.visible,
        state.x,
        state.y
    );
    if state.visible {
        if let Err(e) = show_subtitle_overlay_internal(app).await {
            log_error!("Failed to restore subtitle overlay window: {}", e);
        }
    }
}

/// Persist current position before the window is destroyed.
pub async fn save_subtitle_overlay_position<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window(WINDOW_LABEL) {
        if let Ok(pos) = window.outer_position() {
            let scale = window.scale_factor().unwrap_or(1.0);
            let mut state = load_state(app).await;
            state.x = pos.x as f64 / scale;
            state.y = pos.y as f64 / scale;
            save_state(app, &state).await;
            log_info!(
                "Saved subtitle overlay position: ({}, {})",
                state.x,
                state.y
            );
        }
    }
}
