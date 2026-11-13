use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

use crate::AppState;
use crate::services::{
    self,
    resources::{AttachmentRoot, Resource},
    BizError, BizResult,
};

fn attachments_root(app: &AppHandle) -> BizResult<AttachmentRoot> {
    let dir = app
        .path()
        .app_local_data_dir()
        .map_err(services::map_io)?
        .join("attachments");
    Ok(AttachmentRoot(dir))
}

fn fp_to_string(fp: tauri_plugin_dialog::FilePath) -> Option<String> {
    match fp {
        tauri_plugin_dialog::FilePath::Url(u) => Some(u.to_string()),
        tauri_plugin_dialog::FilePath::Path(p) => Some(p.to_string_lossy().to_string()),
    }
}

/// 系统文件选择器（多选）。返回所选路径；用户取消返回空数组，不产生空记录。
#[tauri::command]
pub async fn pick_files(
    app: AppHandle,
    title: Option<String>,
    extensions: Option<Vec<String>>,
) -> BizResult<Vec<String>> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut d = app.dialog().file();
        if let Some(t) = title {
            d = d.set_title(t);
        }
        if let Some(exts) = &extensions {
            let refs: Vec<&str> = exts.iter().map(|s| s.as_str()).collect();
            d = d.add_filter("支持的文件", &refs);
        }
        let picked = d.blocking_pick_files().unwrap_or_default();
        Ok(picked.into_iter().filter_map(fp_to_string).collect())
    })
    .await
    .map_err(|e| BizError::new("io", format!("文件选择器异常: {e}")))?
}

/// 系统文件选择器（单选）
#[tauri::command]
pub async fn pick_file(
    app: AppHandle,
    title: Option<String>,
    extensions: Option<Vec<String>>,
) -> BizResult<Option<String>> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut d = app.dialog().file();
        if let Some(t) = title {
            d = d.set_title(t);
        }
        if let Some(exts) = &extensions {
            let refs: Vec<&str> = exts.iter().map(|s| s.as_str()).collect();
            d = d.add_filter("支持的文件", &refs);
        }
        Ok(d.blocking_pick_file().and_then(fp_to_string))
    })
    .await
    .map_err(|e| BizError::new("io", format!("文件选择器异常: {e}")))?
}

#[tauri::command]
pub fn attach_local_file(
    app: AppHandle,
    state: State<'_, AppState>,
    owner_type: String,
    owner_id: String,
    source_path: String,
    mode: String,
) -> BizResult<Resource> {
    let root = attachments_root(&app)?;
    let conn = state.db.lock().unwrap_or_else(|e| e.into_inner());
    services::resources::attach_local_file(
        &conn,
        &root,
        &owner_type,
        &owner_id,
        std::path::Path::new(&source_path),
        &mode,
    )
}

#[tauri::command]
pub fn attach_url(
    app: AppHandle,
    state: State<'_, AppState>,
    owner_type: String,
    owner_id: String,
    display_name: String,
    url: String,
) -> BizResult<Resource> {
    let root = attachments_root(&app)?;
    let conn = state.db.lock().unwrap_or_else(|e| e.into_inner());
    services::resources::attach_url(&conn, &root, &owner_type, &owner_id, &display_name, &url)
}

#[tauri::command]
pub fn list_resources(
    app: AppHandle,
    state: State<'_, AppState>,
    owner_type: String,
    owner_id: String,
) -> BizResult<Vec<Resource>> {
    let root = attachments_root(&app)?;
    let conn = state.db.lock().unwrap_or_else(|e| e.into_inner());
    services::resources::list(&conn, &root, &owner_type, &owner_id)
}

/// 打开资料：链接走默认浏览器，文件/图片用系统默认程序。
/// 引用文件失效时返回 file_missing，由前端提示重新定位。
#[tauri::command]
pub fn open_resource(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> BizResult<()> {
    let root = attachments_root(&app)?;
    let res = {
        let conn = state.db.lock().unwrap_or_else(|e| e.into_inner());
        services::resources::get(&conn, &root, &id)?
    };
    match res.kind.as_str() {
        "url" => {
            let url = res
                .url
                .clone()
                .ok_or_else(|| BizError::new("invalid", "链接内容缺失"))?;
            app.opener()
                .open_url(url, None::<&str>)
                .map_err(|e| BizError::new("io", format!("打开链接失败: {e}")))
        }
        _ => {
            let path = services::resources::resolve_file_path(&root, &res)?;
            if !path.is_file() {
                return Err(BizError::new(
                    "file_missing",
                    "文件不存在（可能已被移动或删除）",
                ));
            }
            app.opener()
                .open_path(path.to_string_lossy(), None::<&str>)
                .map_err(|e| BizError::new("io", format!("打开文件失败: {e}")))
        }
    }
}

#[tauri::command]
pub fn remove_resource(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> BizResult<()> {
    let root = attachments_root(&app)?;
    let conn = state.db.lock().unwrap_or_else(|e| e.into_inner());
    services::resources::remove(&conn, &root, &id)
}

#[tauri::command]
pub fn relocate_resource(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    new_path: String,
) -> BizResult<Resource> {
    let root = attachments_root(&app)?;
    let conn = state.db.lock().unwrap_or_else(|e| e.into_inner());
    services::resources::relocate(&conn, &root, &id, std::path::Path::new(&new_path))
}
