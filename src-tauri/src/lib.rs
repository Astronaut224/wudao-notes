mod commands;
mod db;
mod desktop;
mod resources;
mod services;

use db::open_connection;
use std::sync::{Arc, Mutex};
use tauri::Manager;

/// 全局应用状态：SQLite 连接（受控访问，不向 WebView 暴露 SQL）。
/// Arc 便于异步命令把句柄移动到阻塞线程。
pub struct AppState {
    pub db: Arc<Mutex<rusqlite::Connection>>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // 第二次启动：激活已有窗口
            desktop::activate_main_window(app);
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--hidden"]),
        ))
        .plugin({
            let builder = tauri_plugin_global_shortcut::Builder::new();
            builder.with_handler(desktop::handle_global_shortcut).build()
        })
        .setup(|app| {
            // 初始化数据库（数据位于 app_local_data_dir/data/wudao.db）
            let data_dir = app.path().app_local_data_dir()?.join("data");
            let conn = open_connection(&data_dir)?;
            app.manage(AppState {
                db: Arc::new(Mutex::new(conn)),
            });

            // 桌面能力：托盘、关闭行为、全局快捷键、窗口几何恢复
            desktop::setup(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::todos::list_todos,
            commands::todos::create_todo,
            commands::todos::get_todo,
            commands::todos::update_todo,
            commands::todos::set_todo_done,
            commands::todos::reorder_todo,
            commands::todos::duplicate_todo,
            commands::todos::delete_todo,
            commands::todos::clear_completed_todos,
            commands::notes::list_notes,
            commands::notes::create_note,
            commands::notes::get_note,
            commands::notes::update_note,
            commands::notes::delete_note,
            commands::notes::list_note_categories,
            commands::notes::clear_archived_notes,
            commands::resources::pick_files,
            commands::resources::pick_file,
            commands::resources::attach_local_file,
            commands::resources::attach_url,
            commands::resources::list_resources,
            commands::resources::open_resource,
            commands::resources::remove_resource,
            commands::resources::relocate_resource,
            commands::settings::get_settings,
            commands::settings::update_setting,
            commands::settings::export_backup,
            commands::settings::prepare_restore,
            commands::settings::execute_restore,
            commands::desktop::show_window,
            commands::desktop::hide_window,
            commands::desktop::toggle_window,
            commands::desktop::minimize_window,
            commands::desktop::toggle_maximize_window,
            commands::desktop::window_maximized,
            commands::desktop::set_window_mode,
            commands::desktop::get_window_mode,
            commands::desktop::set_capsule_docked,
            commands::desktop::set_always_on_top,
            commands::desktop::set_launch_at_login,
            commands::desktop::start_window_drag,
            commands::desktop::move_window_to,
            commands::desktop::get_window_position,
            commands::desktop::get_window_rect,
            commands::desktop::get_screen_size,
            commands::desktop::shortcut_status,
            commands::desktop::apply_saved_always_on_top,
        ])
        .register_uri_scheme_protocol("wudaores", |ctx, request| {
            resources::handle_resource_protocol(ctx.app_handle(), &request)
        })
        .run(tauri::generate_context!())
        .expect("启动吾道笔记失败");
}
