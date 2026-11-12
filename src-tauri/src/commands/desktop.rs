use tauri::{AppHandle, Manager, PhysicalPosition, WebviewWindow};

use crate::{desktop, services, AppState};

#[tauri::command]
pub fn show_window(app: AppHandle) {
    desktop::activate_main_window(&app);
}

#[tauri::command]
pub fn hide_window(app: AppHandle) {
    desktop::hide_main_window(&app);
}

#[tauri::command]
pub fn toggle_window(app: AppHandle) {
    desktop::toggle_main_window(&app);
}

#[tauri::command]
pub fn minimize_window(window: WebviewWindow) {
    let _ = window.minimize();
}

#[tauri::command]
pub fn toggle_maximize_window(window: WebviewWindow) {
    if window.is_maximized().unwrap_or(false) {
        let _ = window.unmaximize();
    } else {
        let _ = window.maximize();
    }
}

#[tauri::command]
pub fn window_maximized(window: WebviewWindow) -> bool {
    window.is_maximized().unwrap_or(false)
}

/// 切换窗口形态：normal（完整）/ compact（简洁）/ capsule（胶囊悬浮）
#[tauri::command]
pub fn set_window_mode(app: AppHandle, mode: String) -> Result<(), String> {
    desktop::set_window_mode(&app, &mode)
}

/// 当前窗口形态（前端启动时同步用）
#[tauri::command]
pub fn get_window_mode(app: AppHandle) -> Result<serde_json::Value, String> {
    use tauri::Manager;
    let mirror = app
        .try_state::<desktop::ModeMirror>()
        .ok_or("状态未初始化")?;
    let mode = mirror.mode();
    let docked = mirror.docked();
    Ok(serde_json::json!({ "mode": mode, "capsuleDocked": docked }))
}

/// 胶囊收起/展开（贴边细条 ⇄ 完整胶囊），仅胶囊形态有效。
/// only_if_outside=true 时（自动回收路径），若光标仍在窗口内则跳过收起。
#[tauri::command]
pub fn set_capsule_docked(
    app: AppHandle,
    docked: bool,
    side: Option<String>,
    only_if_outside: Option<bool>,
) -> Result<(), String> {
    if docked && only_if_outside.unwrap_or(false) {
        use tauri::Manager;
        if let Some(window) = app.get_webview_window("main") {
            if let (Ok(cursor), Ok(rect)) = (app.cursor_position(), window.outer_position()) {
                let size = window.outer_size().unwrap_or_default();
                let cx = cursor.x as i32;
                let cy = cursor.y as i32;
                let inside = cx >= rect.x - 24
                    && cx <= rect.x + size.width as i32 + 24
                    && cy >= rect.y - 24
                    && cy <= rect.y + size.height as i32 + 24;
                if inside {
                    return Ok(()); // 光标仍在窗口附近，不回收
                }
            }
        }
    }
    desktop::set_capsule_docked(&app, docked, side)
}

/// 置顶开关：仅迷你模式记忆该偏好；普通模式始终不置顶
#[tauri::command]
pub fn set_always_on_top(app: AppHandle, enabled: bool) -> Result<(), String> {
    let Some(window) = app.get_webview_window("main") else {
        return Err("主窗口不存在".into());
    };
    let _ = window.set_always_on_top(enabled);
    let is_compact = match app.state::<AppState>().db.lock() {
        Ok(conn) => services::settings::get(&conn, "window_mode").as_deref() == Some("compact"),
        Err(_) => false,
    };
    if is_compact {
        if let Ok(conn) = app.state::<AppState>().db.lock() {
            let _ = services::settings::set(
                &conn,
                "compact_on_top",
                if enabled { "true" } else { "false" },
            );
        }
    }
    Ok(())
}

#[tauri::command]
pub fn set_launch_at_login(app: AppHandle, enabled: bool) -> Result<(), String> {
    use tauri_plugin_autostart::ManagerExt;
    let auto = app.autolaunch();
    if enabled {
        auto.enable().map_err(|e| format!("设置开机启动失败: {e}"))
    } else {
        auto.disable().map_err(|e| format!("关闭开机启动失败: {e}"))
    }
}

#[tauri::command]
pub fn start_window_drag(window: WebviewWindow) {
    let _ = window.start_dragging();
}

/// JS 驱动拖拽：把窗口移动到指定物理坐标（胶囊拖拽用，精确控制吸附时机）
#[tauri::command]
pub fn move_window_to(window: WebviewWindow, x: f64, y: f64) {
    let _ = window.set_position(PhysicalPosition::new(x as i32, y as i32));
}

/// 窗口当前物理坐标
#[tauri::command]
pub fn get_window_position(window: WebviewWindow) -> Result<(i32, i32), String> {
    window
        .outer_position()
        .map(|p| (p.x, p.y))
        .map_err(|e| e.to_string())
}

/// 窗口当前物理坐标与尺寸 (x, y, w, h)
#[tauri::command]
pub fn get_window_rect(window: WebviewWindow) -> Result<(i32, i32, i32, i32), String> {
    let pos = window.outer_position().map_err(|e| e.to_string())?;
    let size = window.outer_size().map_err(|e| e.to_string())?;
    Ok((pos.x, pos.y, size.width as i32, size.height as i32))
}

/// 窗口所在显示器绝对边界 (left, top, width, height)（吸附判定用）
#[tauri::command]
pub fn get_screen_size(window: WebviewWindow) -> (i32, i32, i32, i32) {
    window
        .current_monitor()
        .ok()
        .flatten()
        .map(|m| {
            let pos = m.position();
            let size = m.size();
            (pos.x, pos.y, size.width as i32, size.height as i32)
        })
        .unwrap_or((0, 0, 1920, 1080))
}

/// 前端查询全局快捷键是否注册成功（用于提示降级状态）
#[tauri::command]
pub fn shortcut_status(app: AppHandle) -> Result<serde_json::Value, String> {
    use tauri_plugin_global_shortcut::GlobalShortcutExt;
    let current = {
        match app.try_state::<desktop::ShortcutStateMirror>() {
            Some(m) => m
                .current
                .lock()
                .map(|g| g.as_ref().map(|s| s.to_string()))
                .unwrap_or(None),
            None => None,
        }
    };
    let registered = current.is_some();
    let accel = {
        match app.state::<AppState>().db.lock() {
            Ok(conn) => services::settings::get(&conn, "global_shortcut")
                .unwrap_or_else(|| desktop::DEFAULT_SHORTCUT.into()),
            Err(_) => desktop::DEFAULT_SHORTCUT.into(),
        }
    };
    let _ = app.global_shortcut(); // 确认插件已初始化
    Ok(serde_json::json!({
        "shortcut": accel,
        "registered": registered,
    }))
}

#[tauri::command]
pub fn apply_saved_always_on_top(app: AppHandle) {
    let (compact, on_top) = match app.state::<AppState>().db.lock() {
        Ok(conn) => (
            services::settings::get(&conn, "window_mode").as_deref() == Some("compact"),
            services::settings::get(&conn, "compact_on_top").as_deref() == Some("true"),
        ),
        Err(_) => return,
    };
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.set_always_on_top(compact && on_top);
    }
}
