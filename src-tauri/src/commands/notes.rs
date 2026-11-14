use tauri::State;

use crate::AppState;
use crate::services::{self, notes::NotePatch, BizResult};
use crate::services::notes::{Note, NoteSummary};

#[tauri::command]
pub fn list_notes(
    state: State<'_, AppState>,
    query: Option<String>,
    category: Option<String>,
    archived: Option<String>,
) -> BizResult<Vec<NoteSummary>> {
    let conn = state.db.lock().unwrap_or_else(|e| e.into_inner());
    services::notes::list(&conn, query, category, archived)
}

/// 永久删除全部已归档笔记（调用前前端需二次确认）
#[tauri::command]
pub fn clear_archived_notes(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> BizResult<serde_json::Value> {
    use tauri::Manager;
    let attachments_root = app
        .path()
        .app_local_data_dir()
        .map_err(services::map_io)?
        .join("attachments");
    let (deleted, ids, _managed) = {
        let conn = state.db.lock().unwrap_or_else(|e| e.into_inner());
        services::with_tx(&conn, |conn| services::notes::clear_archived(conn))?
    };
    for id in &ids {
        services::resources::cleanup_owner_dir(
            &services::resources::AttachmentRoot(attachments_root.clone()),
            "note",
            id,
        );
    }
    Ok(serde_json::json!({
        "deleted": deleted,
    }))
}

#[tauri::command]
pub fn get_note(state: State<'_, AppState>, id: String) -> BizResult<Note> {
    let conn = state.db.lock().unwrap_or_else(|e| e.into_inner());
    services::notes::get(&conn, &id)
}

#[tauri::command]
pub fn create_note(
    state: State<'_, AppState>,
    title: Option<String>,
    body: Option<String>,
    category: Option<String>,
    todo_id: Option<String>,
) -> BizResult<Note> {
    let conn = state.db.lock().unwrap_or_else(|e| e.into_inner());
    services::notes::create(
        &conn,
        title,
        body.as_deref().unwrap_or(""),
        category,
        todo_id,
    )
}

#[tauri::command]
pub fn update_note(state: State<'_, AppState>, id: String, patch: NotePatch) -> BizResult<Note> {
    let conn = state.db.lock().unwrap_or_else(|e| e.into_inner());
    services::notes::update(&conn, &id, &patch)
}

#[tauri::command]
pub fn delete_note(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> BizResult<serde_json::Value> {
    use tauri::Manager;
    let attachments_root = app
        .path()
        .app_local_data_dir()
        .map_err(services::map_io)?
        .join("attachments");
    let managed = {
        let conn = state.db.lock().unwrap_or_else(|e| e.into_inner());
        services::with_tx(&conn, |conn| services::notes::delete(conn, &id))?
    };
    services::resources::cleanup_owner_dir(
        &services::resources::AttachmentRoot(attachments_root),
        "note",
        &id,
    );
    Ok(serde_json::json!({ "managed_removed": managed.len() }))
}

#[tauri::command]
pub fn list_note_categories(state: State<'_, AppState>) -> BizResult<Vec<String>> {
    let conn = state.db.lock().unwrap_or_else(|e| e.into_inner());
    services::notes::list_categories(&conn)
}
