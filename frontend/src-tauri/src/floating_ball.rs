use log::{error as log_error, info as log_info};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, Runtime, WebviewWindowBuilder};

const WINDOW_LABEL: &str = "floating";
const WINDOW_URL: &str = "/floating-ball.html";
const COMPACT_SIZE: f64 = 56.0;
const EXPANDED_SIZE: f64 = 160.0;
/// 拖拽起点（物理像素）：mousedown 时记录，拖动全程以此 + JS 累计位移（screenX 相对按下点）
/// 计算目标位置——绝对定位，不回读窗口位置（set_position 是异步派发，回读会拿到旧值累积误差）。
static BALL_DRAG_ORIGIN: Mutex<Option<tauri::PhysicalPosition<i32>>> = Mutex::new(None);

async fn get_or_create_window<R: Runtime>(
    app: &AppHandle<R>,
) -> Result<tauri::WebviewWindow<R>, String> {
    if let Some(window) = app.get_webview_window(WINDOW_LABEL) {
        return Ok(window);
    }

    let window =
        WebviewWindowBuilder::new(app, WINDOW_LABEL, tauri::WebviewUrl::App(WINDOW_URL.into()))
            .title("")
            .inner_size(COMPACT_SIZE, COMPACT_SIZE)
            .resizable(false)
            .maximizable(false)
            .minimizable(false)
            .closable(false)
            .skip_taskbar(true)
            .always_on_top(true)
            .transparent(true)
            .decorations(false)
            .shadow(false)
            // accept_first_mouse：悬浮球的用途就是「用户正在别的 App（Zoom/浏览器/PPT）里时也能用」，
            // 此时 VoxMinutes 不是前台 App —— 第一次点击只会被系统吞掉用于激活 App，页面收不到 mousedown
            // → 表现为「球点不动、也拖不动」。主窗口在 tauri.conf.json 里早已设了 acceptFirstMouse:true，
            // 悬浮窗当初漏了（2026-09-22 排查：click 完全无反应 + hover 无手型）。
            .accept_first_mouse(true)
            .visible(false)
            .build()
            .map_err(|e| format!("Failed to create floating ball window: {}", e))?;

    // 双保险：显式确保窗口**不**忽略鼠标事件。若该标志为 true，球可见但完全点不到、
    // 也不会有 hover 手型 —— 这是「拖不动」的另一种可能根因，在这里显式关掉并留日志。
    if let Err(e) = window.set_ignore_cursor_events(false) {
        log_error!(
            "Failed to clear ignore_cursor_events on floating ball: {}",
            e
        );
    }

    // 窗口被销毁时广播状态，让主窗口的悬浮球开关同步熄灭
    let app_handle = app.clone();
    window.on_window_event(move |event| {
        if let tauri::WindowEvent::CloseRequested { .. } = event {
            let _ = app_handle.emit(
                "floating-ball-state",
                serde_json::json!({ "visible": false }),
            );
        }
    });

    log_info!("Floating ball window created");
    Ok(window)
}

pub async fn show_floating_ball_internal<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    log_info!("Showing floating ball window...");

    let window = get_or_create_window(app).await?;
    window
        .show()
        .map_err(|e| format!("Failed to show floating ball window: {}", e))?;
    // 保险：macOS 在 hide/show 循环后偶尔丢 always-on-top 层级（球掉到主窗口后面，看起来「消失」）
    let _ = window.set_always_on_top(true);
    // 保险：球若不在屏幕内（手抖微拖累积/屏幕配置变化），钳制回所在屏幕（完整可见）
    ensure_ball_on_screen(app, &window);
    let _ = app.emit(
        "floating-ball-state",
        serde_json::json!({ "visible": true }),
    );
    Ok(())
}

/// 把物理像素矩形 (x, y, w, h) 钳制进「包含其中心、或重叠面积最大」的显示器边界内，
/// 保证整个窗口完整可见。返回钳制后的 (x, y)；无需移动时返回原值。
/// （坐标均为物理像素：outer_position/outer_size 与 Monitor::position/size 一致）
fn clamp_rect_to_monitor(
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    monitors: &[tauri::Monitor],
    context: &str,
) -> (i32, i32) {
    if monitors.is_empty() {
        return (x, y);
    }
    let cx = x + w / 2;
    let cy = y + h / 2;
    let pick = monitors.iter().max_by_key(|m| {
        let mp = m.position();
        let ms = m.size();
        let (mw, mh) = (ms.width as i32, ms.height as i32);
        let contains_center = cx >= mp.x && cx < mp.x + mw && cy >= mp.y && cy < mp.y + mh;
        let overlap_x = (x + w).min(mp.x + mw) - x.max(mp.x);
        let overlap_y = (y + h).min(mp.y + mh) - y.max(mp.y);
        let area = overlap_x.max(0) as i64 * overlap_y.max(0) as i64;
        (contains_center, area)
    });
    let Some(m) = pick else { return (x, y) };
    let mp = m.position();
    let ms = m.size();
    let (mw, mh) = (ms.width as i32, ms.height as i32);
    if w > mw || h > mh {
        return (x, y); // 窗口比屏幕还大（不会发生：最大 160px）
    }
    let nx = x.clamp(mp.x, mp.x + mw - w);
    let ny = y.clamp(mp.y, mp.y + mh - h);
    if nx != x || ny != y {
        log_info!(
            "Floating ball {}: clamp ({},{}) {}x{} -> ({},{}) within monitor ({},{}) {}x{}",
            context,
            x,
            y,
            w,
            h,
            nx,
            ny,
            mp.x,
            mp.y,
            mw,
            mh
        );
    }
    (nx, ny)
}

/// 显示时校验：窗口必须完整落在某块屏幕内（旧版只要求任一像素在屏内——
/// 球被顶出屏幕顶、只剩一条全透明边在屏内时会误判为「在屏内」永不复位）。
/// 不满足则钳制回屏幕；拿不到显示器信息才兜底复位到主窗口右上角内侧 20px。
fn ensure_ball_on_screen<R: Runtime>(app: &AppHandle<R>, window: &tauri::WebviewWindow<R>) {
    let (Ok(pos), Ok(size), Ok(monitors)) = (
        window.outer_position(),
        window.outer_size(),
        window.available_monitors(),
    ) else {
        return;
    };
    if monitors.is_empty() {
        log_info!("Floating ball: no monitor info, resetting to main window top-right");
        if let Some(main_window) = app.get_webview_window("main") {
            if let (Ok(main_pos), Ok(main_size)) =
                (main_window.outer_position(), main_window.outer_size())
            {
                let margin = 20i32;
                let x = main_pos.x + main_size.width as i32 - size.width as i32 - margin;
                let y = main_pos.y + margin;
                let _ = window.set_position(tauri::Position::Physical(
                    tauri::PhysicalPosition::new(x, y),
                ));
            }
        }
        return;
    }
    let (nx, ny) = clamp_rect_to_monitor(
        pos.x,
        pos.y,
        size.width as i32,
        size.height as i32,
        &monitors,
        "show",
    );
    if nx != pos.x || ny != pos.y {
        let _ = window.set_position(tauri::Position::Physical(tauri::PhysicalPosition::new(
            nx, ny,
        )));
    }
}

pub async fn hide_floating_ball_internal<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    log_info!("Hiding floating ball window...");

    if let Some(window) = app.get_webview_window(WINDOW_LABEL) {
        window
            .hide()
            .map_err(|e| format!("Failed to hide floating ball window: {}", e))?;
    }
    let _ = app.emit(
        "floating-ball-state",
        serde_json::json!({ "visible": false }),
    );
    Ok(())
}

#[tauri::command]
pub async fn show_floating_ball<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    show_floating_ball_internal(&app).await
}

#[tauri::command]
pub async fn hide_floating_ball<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    hide_floating_ball_internal(&app).await
}

#[tauri::command]
pub async fn toggle_floating_ball<R: Runtime>(app: AppHandle<R>) -> Result<bool, String> {
    let visible = app
        .get_webview_window(WINDOW_LABEL)
        .and_then(|w| w.is_visible().ok())
        .unwrap_or(false);
    if visible {
        hide_floating_ball_internal(&app).await?;
        Ok(false)
    } else {
        show_floating_ball_internal(&app).await?;
        Ok(true)
    }
}

#[tauri::command]
pub async fn get_floating_ball_state<R: Runtime>(app: AppHandle<R>) -> Result<bool, String> {
    Ok(app
        .get_webview_window(WINDOW_LABEL)
        .and_then(|w| w.is_visible().ok())
        .unwrap_or(false))
}

/// 悬浮球诊断上报（排查「球收不到鼠标事件」用）：页面把布局信息 / 首次收到的鼠标事件
/// 写进 app 日志（前缀 `[floatprobe]`），页面侧按事件类型只上报一次，不会刷日志。
/// 排查手段：`grep floatprobe <日志文件>` —— 若一条 mouseenter 都没有，说明事件根本没到页面
/// （窗口级问题：点击穿透 / z 序 / 非前台 App 吞掉首击），而不是 JS 拖拽逻辑的问题。
#[tauri::command]
pub fn floating_ball_diag(msg: String) {
    log_info!("[floatprobe] {}", msg);
}

/// 收起（56×56，只有球体，避免透明区域挡下层点击）/ 展开（160×160，菜单）
/// ⚠️ macOS setContentSize 以窗口**左下角**为锚：直接 resize 展开时窗口向上生长 104px，
/// 球在屏幕上向上跳 52px——球贴近屏幕顶/底时会被顶出屏幕（菜单随之不可见，
/// 但窗口还剩一条全透明边在屏内，is_visible=true、开关亮着，表现为「球消失了」）。
/// 球在收起/展开两种布局下都位于窗口正中心（56/2 = 52+28 = 160/2），所以 resize 后
/// 把窗口中心对齐回原屏幕中心，球即原地不动；再把目标矩形钳制进屏幕，菜单永不裁出屏外。
/// 注意 set_size/set_position 在 macOS 都是异步派发（主队列 FIFO，先 size 后 position，
/// 顺序有保证），因此钳制必须基于「目标矩形」计算，不能 resize 后回读（会读到旧值）。
#[tauri::command]
pub async fn set_floating_ball_compact<R: Runtime>(
    app: AppHandle<R>,
    compact: bool,
) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(WINDOW_LABEL) {
        let logical = if compact { COMPACT_SIZE } else { EXPANDED_SIZE };
        // resize 前的窗口矩形（物理像素），作为球心锚点
        let anchor = (window.outer_position().ok(), window.outer_size().ok());
        window
            .set_size(tauri::LogicalSize::new(logical, logical))
            .map_err(|e| format!("{}", e))?;
        if let (Some(pos), Some(size)) = anchor {
            let scale = window.scale_factor().unwrap_or(1.0);
            let target = (logical * scale).round() as i32;
            let cx = pos.x + size.width as i32 / 2;
            let cy = pos.y + size.height as i32 / 2;
            let intended = (cx - target / 2, cy - target / 2);
            let (nx, ny) = match window.available_monitors() {
                Ok(monitors) => clamp_rect_to_monitor(
                    intended.0, intended.1, target, target, &monitors, "resize",
                ),
                Err(_) => intended,
            };
            if (nx, ny) != (pos.x, pos.y) {
                let _ = window.set_position(tauri::Position::Physical(
                    tauri::PhysicalPosition::new(nx, ny),
                ));
            }
        }
        Ok(())
    } else {
        Err("Floating ball window not found".to_string())
    }
}

/// —— 拖拽（mig 同款交互：按下记录起点，移动超阈值才算拖动，点击不触发拖动）——
/// JS 侧用 screenX 累计「相对按下点」的总位移传进来，这里换算成物理像素后
/// 以起点 + 总位移为目标位置（绝对定位、无累积误差），并钳制进屏幕——球永远拖不出屏幕。
/// 不用原生 start_dragging：它从 IPC 调用时依赖 NSApp.currentEvent 是否为真实 mousedown，
/// 不可靠时会静默失败、手势退化成点击（表现为「想拖动却弹出了菜单」，2026-09-21 实测）。

#[tauri::command]
pub async fn floating_ball_drag_begin<R: Runtime>(
    window: tauri::WebviewWindow<R>,
) -> Result<(), String> {
    if window.label() != WINDOW_LABEL {
        return Err("not the floating ball window".to_string());
    }
    let pos = window.outer_position().map_err(|e| e.to_string())?;
    let size = window.outer_size().map_err(|e| e.to_string())?;
    let scale = window.scale_factor().unwrap_or(1.0);
    let half = (COMPACT_SIZE / 2.0 * scale).round() as i32;
    // 统一存「收起态左上角」而不是当前左上角：球心在两种布局下都是窗口中心，
    // 菜单展开着开始拖动时，阈值触发会收起菜单（窗口左上角因此右移 52px）——
    // 按收起态定位，拖动全程窗口位置连续、无跳变。
    let origin = tauri::PhysicalPosition::new(
        pos.x + size.width as i32 / 2 - half,
        pos.y + size.height as i32 / 2 - half,
    );
    if let Ok(mut guard) = BALL_DRAG_ORIGIN.lock() {
        *guard = Some(origin);
    }
    Ok(())
}

#[tauri::command]
pub async fn floating_ball_drag_to<R: Runtime>(
    window: tauri::WebviewWindow<R>,
    dx: f64,
    dy: f64,
) -> Result<(), String> {
    if window.label() != WINDOW_LABEL {
        return Err("not the floating ball window".to_string());
    }
    let origin = BALL_DRAG_ORIGIN.lock().ok().and_then(|g| *g);
    let Some(origin) = origin else {
        return Ok(()); // 没有起点（mousedown 没来/已被清），忽略
    };
    let scale = window.scale_factor().unwrap_or(1.0);
    let target_x = origin.x + (dx * scale).round() as i32;
    let target_y = origin.y + (dy * scale).round() as i32;
    let size = window.outer_size().map_err(|e| e.to_string())?;
    let (nx, ny) = match window.available_monitors() {
        Ok(monitors) => clamp_rect_to_monitor(
            target_x,
            target_y,
            size.width as i32,
            size.height as i32,
            &monitors,
            "drag",
        ),
        Err(_) => (target_x, target_y),
    };
    let _ = window.set_position(tauri::Position::Physical(tauri::PhysicalPosition::new(
        nx, ny,
    )));
    Ok(())
}

#[tauri::command]
pub async fn floating_ball_drag_end<R: Runtime>(
    window: tauri::WebviewWindow<R>,
) -> Result<(), String> {
    if window.label() != WINDOW_LABEL {
        return Err("not the floating ball window".to_string());
    }
    if let Ok(mut guard) = BALL_DRAG_ORIGIN.lock() {
        if let Some(origin) = guard.take() {
            if let Ok(pos) = window.outer_position() {
                log_info!(
                    "Floating ball drag end: ({},{}) -> ({},{})",
                    origin.x,
                    origin.y,
                    pos.x,
                    pos.y
                );
            }
        }
    }
    Ok(())
}

/// 启动时延迟自动显示悬浮球，并定位到主窗口右上角内侧 20px（物理像素）。
/// 位置不持久化。应在 setup() 中调用。
pub fn show_floating_ball_on_startup<R: Runtime>(app: &AppHandle<R>) {
    let app_handle = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(600)).await;

        if let Err(e) = show_floating_ball_internal(&app_handle).await {
            log_error!("Failed to show floating ball on startup: {}", e);
            return;
        }

        if let Some(main_window) = app_handle.get_webview_window("main") {
            if let Some(floating_window) = app_handle.get_webview_window(WINDOW_LABEL) {
                if let (Ok(main_pos), Ok(main_size)) =
                    (main_window.outer_position(), main_window.outer_size())
                {
                    let float_size = floating_window
                        .outer_size()
                        .unwrap_or(tauri::PhysicalSize::new(56, 56));
                    let margin = 20i32;
                    let x = main_pos.x + main_size.width as i32 - float_size.width as i32 - margin;
                    let y = main_pos.y + margin;
                    let _ = floating_window.set_position(tauri::Position::Physical(
                        tauri::PhysicalPosition::new(x, y),
                    ));
                }
            }
        }
    });
}
