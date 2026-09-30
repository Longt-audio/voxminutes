//! 会议总结 PDF 导出（打印绕道）：内容在主窗口渲染成 HTML，经本模块存到全局槽位，
//! 弹出 `/print-summary.html` 预览窗口（页面加载后 invoke 拉回内容），再触发系统打印
//! 对话框让用户「存为 PDF」。
//!
//! 为什么不开窗口直接静默出 PDF：Tauri 核心没有静默生成 PDF 的能力（tauri#12284 仍 open），
//! 而内嵌排版引擎（typst）会让安装包 +10~25MB。打印对话框里选「存储为 PDF」是零体积代价的
//! 方案，排版质量由系统 WebView + CSS 保证（中文天然支持）。
//!
//! 平台差异：macOS 的 WKWebView 里 `window.print()` 静默无效，必须走原生 `WebviewWindow::print()`
//! （该 API 仅 macOS 有，故 cfg 门控）；Windows 的 WebView2 里 `window.print()` 可用，页面 JS 直接调。

use log::{error as log_error, info as log_info};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::{AppHandle, Manager, Runtime, WebviewWindowBuilder};

const WINDOW_LABEL: &str = "summary-print";
const WINDOW_URL: &str = "/print-summary.html";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryPrintPayload {
    pub title: String,
    /// 已渲染好的正文 HTML（主窗口用本地 marked 产出；打印页只负责注入与排版）
    pub html: String,
}

static PENDING: Mutex<Option<SummaryPrintPayload>> = Mutex::new(None);

/// 打开（或重建）总结打印预览窗口。重复导出时关旧窗建新窗，避免旧文档残留。
#[tauri::command]
pub async fn open_summary_print_window<R: Runtime>(
    app: AppHandle<R>,
    title: String,
    html: String,
) -> Result<(), String> {
    if let Ok(mut slot) = PENDING.lock() {
        *slot = Some(SummaryPrintPayload {
            title: title.clone(),
            html,
        });
    }

    if let Some(existing) = app.get_webview_window(WINDOW_LABEL) {
        let _ = existing.close();
    }

    let window_title = if title.trim().is_empty() {
        "会议总结".to_string()
    } else {
        title
    };
    WebviewWindowBuilder::new(
        &app,
        WINDOW_LABEL,
        tauri::WebviewUrl::App(WINDOW_URL.into()),
    )
    .title(&window_title)
    .inner_size(840.0, 1000.0)
    .resizable(true)
    .center()
    .build()
    .map_err(|e| {
        log_error!("Failed to create summary print window: {}", e);
        format!("无法打开打印预览窗口: {}", e)
    })?;

    log_info!("Summary print window opened: {}", window_title);
    Ok(())
}

/// 打印页加载后拉取待打印内容（拉模型，不依赖事件时序）。
#[tauri::command]
pub fn get_pending_summary_print() -> Option<SummaryPrintPayload> {
    PENDING.lock().ok().and_then(|p| p.clone())
}

/// 触发系统打印对话框。仅在 macOS 上有实际实现（WKWebView 的 window.print() 无效）；
/// 其他平台由页面 JS 直接 window.print()，本命令是 no-op。
#[tauri::command]
pub fn print_window<R: Runtime>(window: tauri::WebviewWindow<R>) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        window.print().map_err(|e| {
            log_error!("Native print failed: {}", e);
            format!("打印失败: {}", e)
        })?;
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = window;
    }
    Ok(())
}
