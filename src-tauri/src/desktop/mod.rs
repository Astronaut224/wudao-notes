use tauri::{
    menu::{Menu, MenuItem},
    tray::{TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewWindow, WindowEvent,
};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

use crate::{services, AppState};

pub const DEFAULT_SHORTCUT: &str = "Ctrl+Shift+Space";
pub const COMPACT_SIZE: (f64, f64) = (340.0, 440.0);
pub const NORMAL_SIZE: (f64, f64) = (950.0, 640.0);

/// 当前窗口形态（记录于 settings，重启恢复）
fn current_mode(conn: &rusqlite::Connection) -> String {
    services::settings::get(conn, "window_mode").unwrap_or_else(|| "normal".into())
}

fn get_setting(conn: &rusqlite::Connection, key: &str) -> Option<String> {
    services::settings::get(conn, key)
}

/// 几何串 "x,y,w,h"（物理像素）
fn save_geometry(app: &AppHandle, mode: &str) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    if window.is_maximized().unwrap_or(false) || window.is_minimized().unwrap_or(false) {
        return; // 最大化/最小化状态不覆盖记忆几何
    }
    if let (Ok(pos), Ok(size)) = (window.outer_position(), window.outer_size()) {
        let g = format!("{},{},{},{}", pos.x, pos.y, size.width, size.height);
        if let Ok(conn) = app.state::<AppState>().db.lock() {
            let _ = services::settings::set(&conn, &format!("{mode}_geometry"), &g);
        }
    }
}

fn apply_geometry(window: &WebviewWindow, geom: &str, default_size: (f64, f64)) {
    let parts: Vec<&str> = geom.split(',').collect();
    if parts.len() == 4 {
        if let (Ok(x), Ok(y), Ok(w), Ok(h)) = (
            parts[0].parse::<i32>(),
            parts[1].parse::<i32>(),
            parts[2].parse::<u32>(),
            parts[3].parse::<u32>(),
        ) {
            if w > 0 && h > 0 {
                let _ = window.set_position(PhysicalPosition::new(x, y));
                let _ = window.set_size(PhysicalSize::new(w.max(200), h.max(200)));
                return;
            }
        }
    }
    let _ = window.unmaximize();
    let _ = window.set_size(tauri::LogicalSize::new(default_size.0, default_size.1));
    let _ = window.center();
}

pub fn activate_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

pub fn hide_main_window(app: &AppHandle) {
    let mode = {
        match app.state::<AppState>().db.lock() {
            Ok(conn) => current_mode(&conn),
            Err(_) => "normal".into(),
        }
    };
    save_geometry(app, &mode);
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
}

pub fn toggle_main_window(app: &AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    if window.is_visible().unwrap_or(false) {
        hide_main_window(app);
    } else {
        activate_main_window(app);
    }
}

/// 切换正常/迷你模式：同一窗口 resize，复用同一 WebView
pub fn set_compact_mode(app: &AppHandle, enabled: bool) -> Result<(), String> {
    let Some(window) = app.get_webview_window("main") else {
        return Err("主窗口不存在".into());
    };
    let old_mode = match app.state::<AppState>().db.lock() {
        Ok(conn) => current_mode(&conn),
        Err(_) => return Err("数据库被占用".into()),
    };
    let new_mode = if enabled { "compact" } else { "normal" };
    if old_mode != new_mode {
        save_geometry(app, &old_mode);
    }

    let (geom_key, default_size, min_size) = if enabled {
        (
            "compact_geometry",
            COMPACT_SIZE,
            tauri::LogicalSize::new(300.0, 360.0),
        )
    } else {
        (
            "normal_geometry",
            NORMAL_SIZE,
            tauri::LogicalSize::new(760.0, 520.0),
        )
    };

    let geom = match app.state::<AppState>().db.lock() {
        Ok(conn) => get_setting(&conn, geom_key),
        Err(_) => None,
    };

    let _ = window.set_min_size(Some(min_size));
    let _ = window.unmaximize();
    match geom {
        Some(g) => apply_geometry(&window, &g, default_size),
        None => {
            let _ = window.set_size(tauri::LogicalSize::new(default_size.0, default_size.1));
            if enabled {
                // 迷你窗默认出现在原窗口右上角附近，避免跳到屏幕中央
                if let (Ok(pos), Some(monitor)) = (
                    window.outer_position(),
                    window.current_monitor().ok().flatten(),
                ) {
                    let size = monitor.size();
                    let x = pos.x.clamp(0, size.width as i32 - 360).max(0);
                    let y = pos.y.clamp(0, size.height as i32 - 460).max(0);
                    let _ = window.set_position(PhysicalPosition::new(x, y));
                }
            } else {
                let _ = window.center();
            }
        }
    }

    // 置顶仅用于迷你窗，状态记忆
    let on_top = if enabled {
        match app.state::<AppState>().db.lock() {
            Ok(conn) => get_setting(&conn, "compact_on_top").as_deref() == Some("true"),
            Err(_) => false,
        }
    } else {
        false
    };
    let _ = window.set_always_on_top(on_top);

    {
        let state = app.state::<AppState>();
        let conn = state
            .db
            .lock()
            .map_err(|_| "数据库被占用".to_string())?;
        services::settings::set(&conn, "window_mode", new_mode).map_err(|e| e.message)?;
    }
    save_geometry(app, new_mode);
    let _ = window.show();
    let _ = window.set_focus();
    let _ = app.emit("mode-changed", serde_json::json!({ "compact": enabled }));
    Ok(())
}

/// 注册全局快捷键；被占用时降级为托盘呼出并通知前端
pub fn register_shortcut(app: &AppHandle, accel: &str) -> Result<(), String> {
    let parsed: Shortcut = accel
        .to_lowercase()
        .parse()
        .map_err(|_| format!("快捷键格式不合法：{accel}"))?;
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();
    gs.register(parsed)
        .map_err(|e| format!("快捷键 {accel} 注册失败（可能被其他程序占用）：{e}"))?;
    // 记录当前生效快捷键，供 handler 比对
    if let Some(state) = app.try_state::<ShortcutStateMirror>() {
        *state.current.lock().unwrap() = Some(parsed);
    }
    Ok(())
}

/// 插件 handler 是全局的，用此镜像判断是哪个快捷键触发
pub struct ShortcutStateMirror {
    pub current: std::sync::Mutex<Option<Shortcut>>,
}

pub fn handle_global_shortcut(
    app: &AppHandle,
    shortcut: &Shortcut,
    event: tauri_plugin_global_shortcut::ShortcutEvent,
) {
    if event.state() != ShortcutState::Pressed {
        return;
    }
    let matches = app
        .try_state::<ShortcutStateMirror>()
        .and_then(|s| s.current.lock().ok().map(|g| g.as_ref() == Some(shortcut)))
        .unwrap_or(false);
    if matches {
        toggle_main_window(app);
    }
}

pub fn setup(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let handle = app.handle().clone();

    // 快捷键镜像状态
    app.manage(ShortcutStateMirror {
        current: std::sync::Mutex::new(None),
    });

    // 恢复正常模式窗口几何
    let window = app
        .get_webview_window("main")
        .ok_or("主窗口不存在")?;
    {
        let state = handle.state::<AppState>();
        // 不能直接 `?`：PoisonError 内含 MutexGuard，转 Box<dyn Error> 会把对局部值的引用提前返回
        let conn = state.db.lock().map_err(|_| "数据库连接被占用")?;
        services::settings::set(&conn, "window_mode", "normal")?;
        if let Some(g) = get_setting(&conn, "normal_geometry") {
            apply_geometry(&window, &g, NORMAL_SIZE);
        }
    }

    // 托盘
    let open_item = MenuItem::with_id(&handle, "open", "打开吾道笔记", true, None::<&str>)?;
    let compact_item = MenuItem::with_id(&handle, "compact", "切换迷你模式", true, None::<&str>)?;
    let new_todo_item = MenuItem::with_id(&handle, "new_todo", "新建待办", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(&handle, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(&handle, &[&open_item, &compact_item, &new_todo_item, &quit_item])?;

    TrayIconBuilder::with_id("wudao-tray")
        .icon(app.default_window_icon().expect("缺少应用图标").clone())
        .tooltip("吾道笔记")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => activate_main_window(app),
            "compact" => {
                let enabled = {
                    match app.state::<AppState>().db.lock() {
                        Ok(conn) => current_mode(&conn) != "compact",
                        Err(_) => true,
                    }
                };
                if let Err(e) = set_compact_mode(app, enabled) {
                    log_err(app, &e);
                }
            }
            "new_todo" => {
                activate_main_window(app);
                let _ = app.emit("create-todo", ());
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if matches!(event, TrayIconEvent::DoubleClick { .. }) {
                activate_main_window(tray.app_handle());
            }
        })
        .build(app)?;

    // 窗口事件：× 隐藏到托盘；拖动结束记忆几何
    let w = window.clone();
    window.on_window_event(move |event| match event {
        WindowEvent::CloseRequested { api, .. } => {
            api.prevent_close();
            hide_main_window(w.app_handle());
        }
        WindowEvent::Destroyed => {}
        _ => {}
    });

    // 全局快捷键
    let accel = {
        let state = handle.state::<AppState>();
        let conn = state.db.lock().map_err(|_| "数据库连接被占用")?;
        get_setting(&conn, "global_shortcut").unwrap_or_else(|| DEFAULT_SHORTCUT.into())
    };
    if let Err(e) = register_shortcut(&handle, &accel) {
        eprintln!("{e}");
        // 降级不崩溃：托盘仍可呼出；前端就绪后会查询状态展示提示
        let _ = handle.emit(
            "shortcut-status",
            serde_json::json!({ "ok": false, "shortcut": accel, "message": e }),
        );
    }

    Ok(())
}

fn log_err(app: &AppHandle, msg: &str) {
    eprintln!("[wudao-notes] {msg}");
    let _ = app.emit("app-error", serde_json::json!({ "message": msg }));
}
