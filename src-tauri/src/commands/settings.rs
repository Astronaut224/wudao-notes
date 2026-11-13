use std::collections::HashMap;

use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::DialogExt;

use crate::{
    desktop,
    services::{self, backup, BizError, BizResult},
    AppState,
};
#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> BizResult<HashMap<String, String>> {
    let conn = state.db.lock().unwrap_or_else(|e| e.into_inner());
    services::settings::get_all(&conn).map_err(|e| BizError::new("db", e))
}

/// 更新设置项（白名单键）。全局快捷键会尝试重新注册，失败时返回错误但不落库。
#[tauri::command]
pub fn update_setting(app: AppHandle, state: State<'_, AppState>, key: String, value: String) -> BizResult<()> {
    if !services::settings::ALLOWED_KEYS.contains(&key.as_str()) {
        return Err(BizError::new("invalid", "不支持的设置项"));
    }
    match key.as_str() {
        "global_shortcut" => {
            let accel = if value.trim().is_empty() {
                desktop::DEFAULT_SHORTCUT.to_string()
            } else {
                value.trim().to_string()
            };
            desktop::register_shortcut(&app, &accel)
                .map_err(|e| BizError::new("shortcut_conflict", e))?;
            let conn = state.db.lock().unwrap_or_else(|e| e.into_inner());
            services::settings::set(&conn, "global_shortcut", &accel)
        }
        "compact_on_top" => {
            let conn = state.db.lock().unwrap_or_else(|e| e.into_inner());
            services::settings::set(&conn, "compact_on_top", &value)?;
            drop(conn);
            crate::commands::desktop::apply_saved_always_on_top(app);
            Ok(())
        }
        _ => {
            let conn = state.db.lock().unwrap_or_else(|e| e.into_inner());
            services::settings::set(&conn, &key, &value)
        }
    }
}

/// 导出备份：选择保存位置 → 生成 zip（数据库快照 + 托管附件）
#[tauri::command]
pub async fn export_backup(app: AppHandle, state: State<'_, AppState>) -> BizResult<String> {
    let default_name = format!(
        "wudao-backup-{}.zip",
        chrono::Local::now().format("%Y%m%d-%H%M%S")
    );
    let destination: std::path::PathBuf = {
        let dialog_app = app.clone();
        tauri::async_runtime::spawn_blocking(move || {
            let path = dialog_app
                .dialog()
                .file()
                .set_title("选择备份保存位置")
                .set_file_name(&default_name)
                .add_filter("吾道笔记备份", &["zip"])
                .blocking_save_file();
            let picked: Result<Option<std::path::PathBuf>, BizError> = Ok(match path {
                Some(tauri_plugin_dialog::FilePath::Path(p)) => Some(p),
                _ => None,
            });
            picked
        })
        .await
        .map_err(|e| BizError::new("io", format!("保存对话框异常: {e}")))?
        ?
        .ok_or_else(|| BizError::new("cancelled", "已取消导出"))?
    };

    let attachments_root = app
        .path()
        .app_local_data_dir()
        .map_err(services::map_io)?
        .join("attachments");

    // 附件复制与快照均较重，放阻塞线程执行
    let db = state.db.clone();
    let dest = destination.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        backup::export(&conn, &attachments_root, &dest)
    })
    .await
    .map_err(|e| BizError::new("io", format!("备份任务异常: {e}")))??;

    Ok(destination.to_string_lossy().to_string())
}

/// 恢复第一步：选择备份并校验（不写入）。返回校验结果供前端确认覆盖。
#[tauri::command]
pub async fn prepare_restore(app: AppHandle) -> BizResult<serde_json::Value> {
    let picked: std::path::PathBuf = tauri::async_runtime::spawn_blocking(move || {
        let path = app
            .dialog()
            .file()
            .set_title("选择备份文件")
            .add_filter("吾道笔记备份", &["zip"])
            .blocking_pick_file();
        let r: Result<Option<std::path::PathBuf>, BizError> = Ok(match path {
            Some(tauri_plugin_dialog::FilePath::Path(p)) => Some(p),
            _ => None,
        });
        r
    })
    .await
    .map_err(|e| BizError::new("io", format!("文件对话框异常: {e}")))?
    ?
    .ok_or_else(|| BizError::new("cancelled", "已取消恢复"))?;

    backup::validate_backup(&picked)?;
    Ok(serde_json::json!({
        "path": picked.to_string_lossy(),
    }))
}

/// 恢复第二步：用户确认覆盖后执行。完成前曾提示“将覆盖当前全部数据”。
#[tauri::command]
pub async fn execute_restore(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> BizResult<serde_json::Value> {
    let attachments_root = app
        .path()
        .app_local_data_dir()
        .map_err(services::map_io)?
        .join("attachments");
    let db = state.db.clone();
    let archive = std::path::PathBuf::from(path);
    let (todos, notes) = tauri::async_runtime::spawn_blocking(move || {
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        backup::restore(&conn, &attachments_root, &archive)
    })
    .await
    .map_err(|e| BizError::new("io", format!("恢复任务异常: {e}")))??;

    Ok(serde_json::json!({ "todos": todos, "notes": notes }))
}
