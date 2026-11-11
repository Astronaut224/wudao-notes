use tauri::State;

use crate::AppState;
use crate::services::{self, notes::NotePatch, BizResult};
use crate::services::notes::{Note, NoteSummary};

#[tauri::command]
pub fn list_notes(
    state: State<'_, AppState>,
    query: Option<String>,
    category: Option<String>,
    include_archived: Option<bool>,
) -> BizResult<Vec<NoteSummary>> {
    let conn = state.db.lock().map_err(services::lock_error)?;
    services::notes::list(
        &conn,
        query,
        category,
        include_archived.unwrap_or(false),
    )
}

#[tauri::command]
pub fn get_note(state: State<'_, AppState>, id: String) -> BizResult<Note> {
    let conn = state.db.lock().map_err(services::lock_error)?;
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
    let conn = state.db.lock().map_err(services::lock_error)?;
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
    let conn = state.db.lock().map_err(services::lock_error)?;
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
        let conn = state.db.lock().map_err(services::lock_error)?;
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
    let conn = state.db.lock().map_err(services::lock_error)?;
    services::notes::list_categories(&conn)
}
