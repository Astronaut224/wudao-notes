use tauri::State;

use crate::AppState;
use crate::services::{self, todos::TodoPatch, BizResult};
use crate::services::todos::{Todo, TodoFilter, TodoSummary};

#[tauri::command]
pub fn list_todos(state: State<'_, AppState>, filter: TodoFilter) -> BizResult<Vec<TodoSummary>> {
    let conn = state.db.lock().unwrap_or_else(|e| e.into_inner());
    services::todos::list(&conn, &filter)
}

#[tauri::command]
pub fn get_todo(state: State<'_, AppState>, id: String) -> BizResult<Todo> {
    let conn = state.db.lock().unwrap_or_else(|e| e.into_inner());
    services::todos::get(&conn, &id)
}

#[tauri::command]
pub fn create_todo(
    state: State<'_, AppState>,
    title: String,
    priority: i64,
    next_action: Option<String>,
    description: Option<String>,
    due_date: Option<String>,
) -> BizResult<Todo> {
    let conn = state.db.lock().unwrap_or_else(|e| e.into_inner());
    services::todos::create(
        &conn,
        &title,
        priority,
        next_action.as_deref().unwrap_or(""),
        description.as_deref().unwrap_or(""),
        due_date,
    )
}

#[tauri::command]
pub fn update_todo(state: State<'_, AppState>, id: String, patch: TodoPatch) -> BizResult<Todo> {
    let conn = state.db.lock().unwrap_or_else(|e| e.into_inner());
    services::todos::update(&conn, &id, &patch)
}

#[tauri::command]
pub fn set_todo_done(state: State<'_, AppState>, id: String, done: bool) -> BizResult<Todo> {
    let conn = state.db.lock().unwrap_or_else(|e| e.into_inner());
    services::todos::set_done(&conn, &id, done)
}

#[tauri::command]
pub fn reorder_todo(
    state: State<'_, AppState>,
    id: String,
    new_priority: i64,
    before_id: Option<String>,
) -> BizResult<Todo> {
    let conn = state.db.lock().unwrap_or_else(|e| e.into_inner());
    services::todos::reorder(&conn, &id, new_priority, before_id)
}

#[tauri::command]
pub fn duplicate_todo(state: State<'_, AppState>, id: String) -> BizResult<Todo> {
    let conn = state.db.lock().unwrap_or_else(|e| e.into_inner());
    services::todos::duplicate(&conn, &id)
}

/// 删除待办：事务删除记录，提交后清理托管附件文件。
/// 返回（已删除的托管附件数，解除关联并保留的笔记数），用于向用户反馈。
/// 永久删除全部已完成待办（调用前前端需二次确认）。
/// 关联笔记解除关联并保留；托管附件随删。
#[tauri::command]
pub fn clear_completed_todos(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> BizResult<serde_json::Value> {
    use tauri::Manager;
    let attachments_root = app
        .path()
        .app_local_data_dir()
        .map_err(services::map_io)?
        .join("attachments");

    let (deleted, detached, managed_count, ids) = {
        let conn = state.db.lock().unwrap_or_else(|e| e.into_inner());
        let ids: Vec<String> = {
            let mut stmt = conn
                .prepare("SELECT id FROM todo_items WHERE status = 'done'")
                .map_err(|e| services::BizError::new("db", e.to_string()))?;
            let rows = stmt
                .query_map([], |r| r.get(0))
                .map_err(|e| services::BizError::new("db", e.to_string()))?;
            let mut v = Vec::new();
            for r in rows {
                v.push(r.map_err(|e| services::BizError::new("db", e.to_string()))?);
            }
            v
        };
        let (deleted, detached, managed_count) =
            services::with_tx(&conn, |conn| services::todos::clear_completed(conn))?;
        (deleted, detached, managed_count, ids)
    };
    for id in &ids {
        services::resources::cleanup_owner_dir(
            &services::resources::AttachmentRoot(attachments_root.clone()),
            "todo",
            id,
        );
    }
    Ok(serde_json::json!({
        "deleted": deleted,
        "detached_notes": detached,
        "managed_removed": managed_count,
    }))
}

#[tauri::command]
pub fn delete_todo(
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

    let (managed, detached) = {
        let conn = state.db.lock().unwrap_or_else(|e| e.into_inner());
        services::with_tx(&conn, |conn| services::todos::delete(conn, &id))?
    };
    // 事务已提交，尽力清理托管文件
    services::resources::cleanup_owner_dir(
        &services::resources::AttachmentRoot(attachments_root),
        "todo",
        &id,
    );
    Ok(serde_json::json!({
        "managed_removed": managed.len(),
        "detached_notes": detached,
    }))
}
