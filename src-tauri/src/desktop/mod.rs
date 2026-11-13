use std::sync::Mutex;

use tauri::{
    menu::{Menu, MenuItem},
    tray::{TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, LogicalSize, Manager, PhysicalPosition, PhysicalSize, WebviewWindow,
    WindowEvent,
};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

use crate::{services, AppState};

pub const DEFAULT_SHORTCUT: &str = "Ctrl+Shift+Space";

/// 三种窗口形态（单窗口复用同一 WebView）
pub const MODE_NORMAL: &str = "normal";
pub const MODE_COMPACT: &str = "compact";
pub const MODE_CAPSULE: &str = "capsule";

/// 各形态默认/目标尺寸（逻辑像素）
pub const NORMAL_SIZE: (f64, f64) = (950.0, 640.0);
pub const COMPACT_SIZE: (f64, f64) = (340.0, 440.0);
/// 胶囊展开态（图标 + 计数 + 收起钮）
pub const CAPSULE_SIZE: (f64, f64) = (208.0, 52.0);
/// 胶囊收起后的贴边细条
pub const CAPSULE_DOCK_SIZE: (f64, f64) = (12.0, 56.0);

/// 窗口几何记忆键（按形态）
fn geometry_key(mode: &str) -> &'static str {
    match mode {
        MODE_COMPACT => "compact_geometry",
        MODE_CAPSULE => "capsule_geometry",
        _ => "normal_geometry",
    }
}

fn default_size(mode: &str) -> (f64, f64) {
    match mode {
        MODE_COMPACT => COMPACT_SIZE,
        MODE_CAPSULE => CAPSULE_SIZE,
        _ => NORMAL_SIZE,
    }
}

fn min_size(mode: &str) -> Option<tauri::LogicalSize<f64>> {
    match mode {
        MODE_COMPACT => Some(tauri::LogicalSize::new(300.0, 360.0)),
        // 胶囊/细条尺寸很小，不设下限
        MODE_CAPSULE => None,
        _ => Some(tauri::LogicalSize::new(760.0, 520.0)),
    }
}

/// 内存中的形态镜像（Moved 事件高频读取，避免查库）
pub struct ModeMirror {
    pub mode: Mutex<String>,
    pub docked: Mutex<String>, // "" | "left" | "right"
    pub busy: Mutex<bool>,     // dock/undock 变形中，抑制 Moved 重入
}

impl ModeMirror {
    pub fn mode(&self) -> String {
        self.mode.lock().map(|g| g.clone()).unwrap_or_default()
    }
    pub fn docked(&self) -> String {
        self.docked.lock().map(|g| g.clone()).unwrap_or_default()
    }
}

fn get_setting(conn: &rusqlite::Connection, key: &str) -> Option<String> {
    services::settings::get(conn, key)
}

/// 从几何串 "x,y,w,h" 中取 y（恢复收起条位置用）
fn get_geometry_y(app: &AppHandle, key: &str) -> Option<i32> {
    let state = app.state::<AppState>();
    let conn = state.db.lock().ok()?;
    let g = get_setting(&conn, key)?;
    g.split(',').nth(1)?.parse::<i32>().ok()
}

/// 几何串 "x,y,w,h"（物理像素）
fn save_geometry(app: &AppHandle, mode: &str) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    if window.is_maximized().unwrap_or(false) || window.is_minimized().unwrap_or(false) {
        return;
    }
    if let (Ok(pos), Ok(size)) = (window.outer_position(), window.outer_size()) {
        let g = format!("{},{},{},{}", pos.x, pos.y, size.width, size.height);
        if let Ok(conn) = app.state::<AppState>().db.lock() {
            let _ = services::settings::set(&conn, geometry_key(mode), &g);
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
                let _ = window.set_size(PhysicalSize::new(w.max(40), h.max(40)));
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
    let mode = current_mode(app);
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

fn current_mode(app: &AppHandle) -> String {
    app.try_state::<ModeMirror>()
        .map(|m| m.mode())
        .unwrap_or_else(|| MODE_NORMAL.into())
}

/// 切换窗口形态（normal / compact / capsule）
pub fn set_window_mode(app: &AppHandle, mode: &str) -> Result<(), String> {
    if !matches!(mode, MODE_NORMAL | MODE_COMPACT | MODE_CAPSULE) {
        return Err(format!("未知窗口形态：{mode}"));
    }
    let Some(window) = app.get_webview_window("main") else {
        return Err("主窗口不存在".into());
    };
    let old_mode = current_mode(app);
    if old_mode != mode {
        save_geometry(app, &old_mode);
    }

    let _ = window.set_min_size(min_size(mode));
    let _ = window.unmaximize();

    // 胶囊形态且保持收起态（如重启恢复）：直接按收起条几何贴边，不套用胶囊几何
    let capsule_docked_restore = mode == MODE_CAPSULE && {
        let mirror = app.state::<ModeMirror>();
        let d = mirror.docked();
        d == "left" || d == "right"
    };
    if capsule_docked_restore {
        let scale = window.scale_factor().unwrap_or(1.0);
        let dock_w = (CAPSULE_DOCK_SIZE.0 * scale) as i32;
        let dock_h = (CAPSULE_DOCK_SIZE.1 * scale) as i32;
        let side = app.state::<ModeMirror>().docked();
        let y = get_geometry_y(app, geometry_key(MODE_CAPSULE)).unwrap_or(200);
        let (mx, _my, mw, _mh) = screen_bounds(&window);
        let x = if side == "left" {
            mx
        } else {
            (mx + mw - dock_w).max(mx)
        };
        let _ = window.set_size(PhysicalSize::new(dock_w.max(20), dock_h.max(20)));
        let _ = window.set_position(PhysicalPosition::new(x, y));
    } else {
        let geom = {
            let state = app.state::<AppState>();
            let conn = state.db.lock().map_err(|_| "数据库被占用".to_string())?;
            get_setting(&conn, geometry_key(mode))
        };
        match geom {
            Some(g) => apply_geometry(&window, &g, default_size(mode)),
            None => {
                let _ = window.set_size(LogicalSize::new(default_size(mode).0, default_size(mode).1));
                match mode {
                    // 胶囊默认出现在屏幕右缘、垂直居中偏上，便于发现
                    MODE_CAPSULE => place_capsule_default(&window),
                    _ => {
                        let _ = window.center();
                    }
                }
            }
        }
    }

    // 置顶：胶囊/简洁窗可记忆置顶；正常窗不置顶
    let on_top = if mode != MODE_NORMAL {
        match app.state::<AppState>().db.lock() {
            Ok(conn) => get_setting(&conn, "compact_on_top").as_deref() == Some("true"),
            Err(_) => false,
        }
    } else {
        false
    };
    let _ = window.set_always_on_top(on_top);

    // 胶囊形态：从其他形态切入时清除收起状态（进入时为展开态）
    if mode == MODE_CAPSULE && old_mode != MODE_CAPSULE {
        set_docked(app, "");
    }

    {
        let state = app.state::<AppState>();
        let conn = state
            .db
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        services::settings::set(&conn, "window_mode", mode).map_err(|e| e.message)?;
    }
    if let Some(mirror) = app.try_state::<ModeMirror>() {
        if let Ok(mut g) = mirror.mode.lock() {
            *g = mode.to_string();
        }
    }
    save_geometry(app, mode);
    let _ = window.show();
    let _ = window.set_focus();
    let _ = app.emit(
        "mode-changed",
        serde_json::json!({ "mode": mode, "compact": mode == MODE_COMPACT }),
    );
    Ok(())
}

fn place_capsule_default(window: &WebviewWindow) {
    let scale = window.scale_factor().unwrap_or(1.0);
    let cap_w = (CAPSULE_SIZE.0 * scale) as i32;
    let cap_h = (CAPSULE_SIZE.1 * scale) as i32;
    let (mx, my, mw, mh) = screen_bounds(window);
    let x = (mx + mw - cap_w - 24).max(mx);
    let y = (my + (mh / 3) - cap_h).max(my);
    let _ = window.set_position(PhysicalPosition::new(x, y));
}

/// 窗口所在显示器的绝对边界（虚拟屏幕坐标）
fn screen_bounds(window: &WebviewWindow) -> (i32, i32, i32, i32) {
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

/// 胶囊收起/展开（贴边细条 ⇄ 完整胶囊）
pub fn set_capsule_docked(app: &AppHandle, docked: bool, side: Option<String>) -> Result<(), String> {
    let Some(window) = app.get_webview_window("main") else {
        return Err("主窗口不存在".into());
    };
    {
        let mirror = app.state::<ModeMirror>();
        let mut busy = mirror.busy.lock().map_err(|_| "内部状态异常".to_string())?;
        if *busy {
            return Ok(());
        }
        *busy = true;
    }
    let result = dock_or_expand(app, &window, docked, side);
    if let Some(mirror) = app.try_state::<ModeMirror>() {
        if let Ok(mut busy) = mirror.busy.lock() {
            *busy = false;
        }
    }
    result
}

fn dock_or_expand(
    app: &AppHandle,
    window: &WebviewWindow,
    docked: bool,
    side: Option<String>,
) -> Result<(), String> {
    let scale = window.scale_factor().unwrap_or(1.0);
    let (mx, _my, mw, _mh) = screen_bounds(window);
    let cur = window
        .outer_position()
        .map_err(|e| format!("获取窗口位置失败: {e}"))?;
    let cap_w = (CAPSULE_SIZE.0 * scale) as i32;
    let cap_h = (CAPSULE_SIZE.1 * scale) as i32;
    let dock_w = (CAPSULE_DOCK_SIZE.0 * scale) as i32;
    let dock_h = (CAPSULE_DOCK_SIZE.1 * scale) as i32;

    if docked {
        // 收起：贴到指定侧（默认右），y 尽量保持
        let side = match side.as_deref() {
            Some("left") => "left",
            _ => "right",
        };
        let x = if side == "left" { mx } else { (mx + mw - dock_w).max(mx) };
        let y = cur.y;
        let _ = window.set_size(PhysicalSize::new(
            dock_w.max(20),
            cap_h.max(dock_h).max(20),
        ));
        let _ = window.set_position(PhysicalPosition::new(x, y));
        set_docked(app, side);
    } else {
        // 展开：从贴边位置向屏幕内侧弹出完整胶囊（沿用收起侧，无记录则默认右侧）
        let side = {
            let mirror = app.state::<ModeMirror>();
            let d = mirror.docked();
            if d == "left" {
                "left".to_string()
            } else {
                "right".to_string()
            }
        };
        let _ = window.set_size(PhysicalSize::new(cap_w.max(40), cap_h.max(40)));
        let x = if side == "left" {
            mx
        } else {
            (mx + mw - cap_w - 2).max(mx)
        };
        let _ = window.set_position(PhysicalPosition::new(x, cur.y));
        set_docked(app, "");
    }
    save_geometry(app, MODE_CAPSULE);
    let _ = app.emit(
        "capsule-state",
        serde_json::json!({ "docked": docked, "side": side_of(app) }),
    );
    Ok(())
}

fn side_of(app: &AppHandle) -> String {
    app.try_state::<ModeMirror>()
        .map(|m| m.docked())
        .unwrap_or_default()
}

fn set_docked(app: &AppHandle, side: &str) {
    if let Some(mirror) = app.try_state::<ModeMirror>() {
        if let Ok(mut g) = mirror.docked.lock() {
            *g = side.to_string();
        }
    }
    if let Ok(conn) = app.state::<AppState>().db.lock() {
        let _ = services::settings::set(&conn, "capsule_docked", side);
    }
}

/// 窗口移动事件：目前无需在移动中做吸附（吸附由前端拖拽结束时判定，避免与拖动循环冲突）
fn on_moved(_app: &AppHandle) {}

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

/// 移除 Windows 对无边框窗口自动绘制的系统描边（用户反馈的“外层边框”）。
/// DWMWA_BORDER_COLOR = 34；DWMWA_COLOR_NONE = 0xFFFFFFFE 表示无边框。
#[cfg(windows)]
fn remove_system_border(window: &WebviewWindow) {
    use windows_sys::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_BORDER_COLOR};
    const DWMWA_COLOR_NONE: u32 = 0xFFFF_FFFE;
    if let Ok(hwnd) = window.hwnd() {
        let none: u32 = DWMWA_COLOR_NONE;
        unsafe {
            let _ = DwmSetWindowAttribute(
                hwnd.0 as *mut core::ffi::c_void,
                DWMWA_BORDER_COLOR as u32,
                &none as *const u32 as *const core::ffi::c_void,
                4,
            );
        }
    }
}

pub fn setup(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let handle = app.handle().clone();

    app.manage(ShortcutStateMirror {
        current: std::sync::Mutex::new(None),
    });

    let window = app.get_webview_window("main").ok_or("主窗口不存在")?;
    remove_system_border(&window);

    // 读取持久化形态并恢复几何
    let (saved_mode, saved_docked) = {
        let state = handle.state::<AppState>();
        let conn = state.db.lock().unwrap_or_else(|e| e.into_inner());
        (
            get_setting(&conn, "window_mode").unwrap_or_else(|| MODE_NORMAL.into()),
            get_setting(&conn, "capsule_docked").unwrap_or_default(),
        )
    };
    let start_mode = match saved_mode.as_str() {
        MODE_COMPACT | MODE_CAPSULE => saved_mode.as_str(),
        _ => MODE_NORMAL,
    };
    app.manage(ModeMirror {
        mode: Mutex::new(start_mode.to_string()),
        docked: Mutex::new(saved_docked.clone()),
        busy: Mutex::new(false),
    });

    {
        let state = handle.state::<AppState>();
        // 不能直接 `?`：PoisonError 内含 MutexGuard，转 Box<dyn Error> 会把对局部值的引用提前返回
        let conn = state.db.lock().unwrap_or_else(|e| e.into_inner());
        if start_mode == MODE_NORMAL {
            if let Some(g) = get_setting(&conn, "normal_geometry") {
                apply_geometry(&window, &g, NORMAL_SIZE);
            }
        }
    }

    // 托盘
    let open_item = MenuItem::with_id(&handle, "open", "打开吾道笔记", true, None::<&str>)?;
    let compact_item = MenuItem::with_id(&handle, "compact", "切换简洁模式", true, None::<&str>)?;
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
                let cur = app
                    .try_state::<ModeMirror>()
                    .map(|m| m.mode())
                    .unwrap_or_else(|| MODE_NORMAL.into());
                let target = if cur == MODE_COMPACT { MODE_NORMAL } else { MODE_COMPACT };
                if let Err(e) = set_window_mode(app, target) {
                    eprintln!("[wudao-notes] {e}");
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

    // 窗口事件：× 隐藏到托盘；移动时处理胶囊吸附
    let w = window.clone();
    let app_for_event = handle.clone();
    window.on_window_event(move |event| match event {
        WindowEvent::CloseRequested { api, .. } => {
            api.prevent_close();
            hide_main_window(w.app_handle());
        }
        WindowEvent::Moved(_) => {
            // Moved 触发频繁，处理需极轻量（内存判断 + 少量几何比较）
            on_moved(&app_for_event);
        }
        _ => {}
    });

    // 全局快捷键
    let accel = {
        let state = handle.state::<AppState>();
        let conn = state.db.lock().unwrap_or_else(|e| e.into_inner());
        get_setting(&conn, "global_shortcut").unwrap_or_else(|| DEFAULT_SHORTCUT.into())
    };
    if let Err(e) = register_shortcut(&handle, &accel) {
        eprintln!("{e}");
        let _ = handle.emit(
            "shortcut-status",
            serde_json::json!({ "ok": false, "shortcut": accel, "message": e }),
        );
    }

    // 非正常形态启动时，同步前端
    if start_mode != MODE_NORMAL {
        set_window_mode(&handle, start_mode)?;
    }

    Ok(())
}
