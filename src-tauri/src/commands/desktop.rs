use tauri::{AppHandle, Manager, WebviewWindow};

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

#[tauri::command]
pub fn set_compact_mode(app: AppHandle, enabled: bool) -> Result<(), String> {
    desktop::set_compact_mode(&app, enabled)
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
