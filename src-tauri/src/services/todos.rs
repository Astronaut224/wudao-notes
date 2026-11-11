use rusqlite::Connection;
use serde::Deserialize;

use super::{now_string, BizError, BizResult};

pub const SORT_STEP: i64 = 1000;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Todo {
    pub id: String,
    pub title: String,
    pub next_action: String,
    pub priority: i64,
    pub status: String,
    pub due_date: Option<String>,
    pub sort_order: i64,
    pub description: String,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
}

/// 列表行的附加标记（资料/关联笔记数量，用于行内小图标）
#[derive(Debug, Clone, serde::Serialize)]
pub struct TodoSummary {
    #[serde(flatten)]
    pub todo: Todo,
    pub resource_count: i64,
    pub note_count: i64,
}

#[derive(Debug, Deserialize)]
pub struct TodoFilter {
    /// all | active | done | priority:<n>
    pub scope: Option<String>,
    /// 搜索关键词（标题/下一步/备注）
    pub query: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub struct TodoPatch {
    pub title: Option<String>,
    pub next_action: Option<String>,
    pub priority: Option<i64>,
    pub due_date: Option<String>,
    pub description: Option<String>,
}

fn row_to_todo(r: &rusqlite::Row) -> rusqlite::Result<Todo> {
    Ok(Todo {
        id: r.get("id")?,
        title: r.get("title")?,
        next_action: r.get("next_action")?,
        priority: r.get("priority")?,
        status: r.get("status")?,
        due_date: r.get("due_date")?,
        sort_order: r.get("sort_order")?,
        description: r.get("description")?,
        created_at: r.get("created_at")?,
        updated_at: r.get("updated_at")?,
        completed_at: r.get("completed_at")?,
    })
}

fn escape_like(q: &str) -> String {
    let mut out = String::with_capacity(q.len() + 2);
    for c in q.chars() {
        match c {
            '%' | '_' | '\\' => {
                out.push('\\');
                out.push(c);
            }
            _ => out.push(c),
        }
    }
    out
}

pub fn list(conn: &Connection, filter: &TodoFilter) -> BizResult<Vec<TodoSummary>> {
    let scope = filter.scope.as_deref().unwrap_or("all");
    let mut sql = String::from(
        "SELECT t.*, \
           (SELECT COUNT(*) FROM resources r WHERE r.todo_id = t.id) AS resource_count, \
           (SELECT COUNT(*) FROM notes n WHERE n.todo_id = t.id) AS note_count \
         FROM todo_items t WHERE 1=1",
    );
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    match scope {
        "done" => sql.push_str(" AND t.status = 'done'"),
        "active" => sql.push_str(" AND t.status = 'active'"),
        s if s.starts_with("priority:") => {
            let p: i64 = s
                .strip_prefix("priority:")
                .and_then(|v| v.parse().ok())
                .ok_or_else(|| BizError::new("invalid", "优先级筛选不合法"))?;
            if !(1..=3).contains(&p) {
                return Err(BizError::new("invalid", "优先级筛选不合法"));
            }
            sql.push_str(" AND t.status = 'active' AND t.priority = ?");
            params.push(Box::new(p));
        }
        _ => {}
    }

    if let Some(q) = filter.query.as_deref().map(str::trim).filter(|q| !q.is_empty()) {
        let like = format!("%{}%", escape_like(q));
        sql.push_str(
            " AND (t.title LIKE ? ESCAPE '\\' OR t.next_action LIKE ? ESCAPE '\\' OR t.description LIKE ? ESCAPE '\\')",
        );
        for _ in 0..3 {
            params.push(Box::new(like.clone()));
        }
    }

    sql.push_str(" ORDER BY t.status ASC, t.priority ASC, t.sort_order ASC, t.created_at ASC");

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| BizError::new("db", format!("查询待办失败: {e}")))?;
    let refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    let rows = stmt
        .query_map(refs.as_slice(), |r| {
            Ok(TodoSummary {
                todo: row_to_todo(r)?,
                resource_count: r.get("resource_count")?,
                note_count: r.get("note_count")?,
            })
        })
        .map_err(|e| BizError::new("db", format!("查询待办失败: {e}")))?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| BizError::new("db", format!("读取待办失败: {e}")))?);
    }
    Ok(out)
}

pub fn get(conn: &Connection, id: &str) -> BizResult<Todo> {
    conn.query_row("SELECT * FROM todo_items WHERE id = ?1", [id], row_to_todo)
        .map_err(|_| BizError::new("not_found", "待办不存在或已被删除"))
}

pub fn create(
    conn: &Connection,
    title: &str,
    priority: i64,
    next_action: &str,
    description: &str,
    due_date: Option<String>,
) -> BizResult<Todo> {
    if !(1..=3).contains(&priority) {
        return Err(BizError::new("invalid", "优先级不合法"));
    }
    let title = super::validate_title(title, 160)?;
    if next_action.chars().count() > 500 {
        return Err(BizError::new("invalid", "下一步最多 500 字"));
    }
    super::validate_due_date(&due_date)?;
    if description.chars().count() > 20000 {
        return Err(BizError::new("invalid", "备注过长（上限 20000 字）"));
    }

    let id = uuid::Uuid::new_v4().to_string();
    let now = now_string();
    let max_sort: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(sort_order), 0) FROM todo_items WHERE status='active' AND priority=?1",
            [priority],
            |r| r.get(0),
        )
        .map_err(|e| BizError::new("db", format!("查询排序失败: {e}")))?;

    conn.execute(
        "INSERT INTO todo_items (id, title, next_action, priority, status, due_date, sort_order, description, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, 'active', ?5, ?6, ?7, ?8, ?8)",
        rusqlite::params![
            id,
            title,
            next_action.trim(),
            priority,
            due_date.filter(|d| !d.is_empty()),
            max_sort + SORT_STEP,
            description.trim(),
            now
        ],
    )
    .map_err(|e| BizError::new("db", format!("写入待办失败: {e}")))?;
    get(conn, &id)
}

pub fn update(conn: &Connection, id: &str, patch: &TodoPatch) -> BizResult<Todo> {
    {
        // 确认存在
        let _ = get(conn, id)?;
    }
    if let Some(p) = patch.priority {
        if !(1..=3).contains(&p) {
            return Err(BizError::new("invalid", "优先级不合法"));
        }
    }
    if let Some(t) = &patch.title {
        super::validate_title(t, 160)?;
    }
    if let Some(na) = &patch.next_action {
        if na.chars().count() > 500 {
            return Err(BizError::new("invalid", "下一步最多 500 字"));
        }
    }
    super::validate_due_date(&patch.due_date)?;
    if let Some(d) = &patch.description {
        if d.chars().count() > 20000 {
            return Err(BizError::new("invalid", "备注过长（上限 20000 字）"));
        }
    }

    let now = now_string();
    conn.execute(
        "UPDATE todo_items SET
           title        = COALESCE(?2, title),
           next_action  = COALESCE(?3, next_action),
           priority     = COALESCE(?4, priority),
           due_date     = CASE WHEN ?5 IS NULL THEN due_date ELSE NULLIF(?5, '') END,
           description  = COALESCE(?6, description),
           updated_at   = ?7
         WHERE id = ?1",
        rusqlite::params![
            id,
            patch.title.as_deref().map(str::trim),
            patch.next_action.as_deref().map(str::trim),
            patch.priority,
            patch.due_date.as_deref(),
            patch.description.as_deref(),
            now
        ],
    )
    .map_err(|e| BizError::new("db", format!("更新待办失败: {e}")))?;

    get(conn, id)
}

pub fn set_done(conn: &Connection, id: &str, done: bool) -> BizResult<Todo> {
    let _ = get(conn, id)?;
    let now = now_string();
    if done {
        conn.execute(
            "UPDATE todo_items SET status='done', completed_at=?2, updated_at=?2 WHERE id=?1",
            rusqlite::params![id, now],
        )
        .map_err(|e| BizError::new("db", format!("更新状态失败: {e}")))?;
    } else {
        // 恢复为未完成：排到原优先级组末尾，避免打乱既有顺序
        let todo = get(conn, id)?;
        let max_sort: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(sort_order),0) FROM todo_items WHERE status='active' AND priority=?1",
                [todo.priority],
                |r| r.get(0),
            )
            .map_err(|e| BizError::new("db", format!("查询排序失败: {e}")))?;
        conn.execute(
            "UPDATE todo_items SET status='active', completed_at=NULL, sort_order=?2, updated_at=?3 WHERE id=?1",
            rusqlite::params![id, max_sort + SORT_STEP, now],
        )
        .map_err(|e| BizError::new("db", format!("更新状态失败: {e}")))?;
    }
    get(conn, id)
}

/// 组内/跨组移动：new_priority 可变；before_id 为目标位置参照条目（空则移到末尾）。
/// 实现为对目标组整组重排：先得到不含被移动项的组内顺序，插入后按序重写 sort_order，
/// 避免中点插值耗尽间距的重编号边界问题。仅移动项会更新 updated_at。
pub fn reorder(
    conn: &Connection,
    id: &str,
    new_priority: i64,
    before_id: Option<String>,
) -> BizResult<Todo> {
    if !(1..=3).contains(&new_priority) {
        return Err(BizError::new("invalid", "优先级不合法"));
    }
    let todo = get(conn, id)?;

    // 目标组（同状态同优先级）现有顺序，不含被移动项
    let mut order: Vec<String> = {
        let mut stmt = conn
            .prepare(
                "SELECT id FROM todo_items WHERE status=?1 AND priority=?2 AND id != ?3 ORDER BY sort_order ASC, created_at ASC",
            )
            .map_err(|e| BizError::new("db", format!("查询分组失败: {e}")))?;
        let rows = stmt
            .query_map(rusqlite::params![todo.status, new_priority, id], |r| {
                r.get::<_, String>(0)
            })
            .map_err(|e| BizError::new("db", format!("查询分组失败: {e}")))?;
        let mut v = Vec::new();
        for r in rows {
            v.push(r.map_err(|e| BizError::new("db", format!("读取分组失败: {e}")))?);
        }
        v
    };

    match before_id.as_deref().filter(|s| !s.is_empty()) {
        Some(before) => {
            let idx = order
                .iter()
                .position(|x| x == before)
                .ok_or_else(|| BizError::new("invalid", "排序参照条目不存在"))?;
            order.insert(idx, id.to_string());
        }
        None => order.push(id.to_string()),
    }

    for (i, gid) in order.iter().enumerate() {
        if gid == id {
            conn.execute(
                "UPDATE todo_items SET priority=?2, sort_order=?3, updated_at=?4 WHERE id=?1",
                rusqlite::params![id, new_priority, (i as i64 + 1) * SORT_STEP, now_string()],
            )
            .map_err(|e| BizError::new("db", format!("更新排序失败: {e}")))?;
        } else {
            conn.execute(
                "UPDATE todo_items SET sort_order=?2 WHERE id=?1",
                rusqlite::params![gid, (i as i64 + 1) * SORT_STEP],
            )
            .map_err(|e| BizError::new("db", format!("更新排序失败: {e}")))?;
        }
    }
    get(conn, id)
}

/// 复制条目：复制内容字段，状态重置为未完成并排到同组末尾
pub fn duplicate(conn: &Connection, id: &str) -> BizResult<Todo> {
    let t = get(conn, id)?;
    create(
        conn,
        &format!("{}（副本）", t.title),
        t.priority,
        &t.next_action,
        &t.description,
        t.due_date.clone(),
    )
}

/// 删除待办。返回随条目删除的“托管附件”相对路径列表（供调用方在事务提交后清理磁盘文件）。
/// 关联笔记仅解除关联（FK ON DELETE SET NULL），不会被删除。
pub fn delete(conn: &Connection, id: &str) -> BizResult<(Vec<String>, i64)> {
    let _ = get(conn, id)?;
    let mut managed: Vec<String> = Vec::new();
    {
        let mut stmt = conn
            .prepare(
                "SELECT stored_rel_path FROM resources WHERE todo_id=?1 AND file_mode='copy' AND stored_rel_path IS NOT NULL",
            )
            .map_err(|e| BizError::new("db", e.to_string()))?;
        let rows = stmt
            .query_map([id], |r| r.get::<_, String>(0))
            .map_err(|e| BizError::new("db", e.to_string()))?;
        for r in rows {
            managed.push(r.map_err(|e| BizError::new("db", e.to_string()))?);
        }
    }
    let detached_notes: i64 = conn
        .query_row("SELECT COUNT(*) FROM notes WHERE todo_id=?1", [id], |r| r.get(0))
        .map_err(|e| BizError::new("db", e.to_string()))?;

    conn.execute("DELETE FROM resources WHERE todo_id=?1", [id])
        .map_err(|e| BizError::new("db", format!("删除资料失败: {e}")))?;
    conn.execute("DELETE FROM todo_items WHERE id=?1", [id])
        .map_err(|e| BizError::new("db", format!("删除待办失败: {e}")))?;
    Ok((managed, detached_notes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migrations::V1_SCHEMA;

    fn test_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(V1_SCHEMA).unwrap();
        conn
    }

    #[test]
    fn create_and_list_grouped() {
        let conn = test_conn();
        let high = create(&conn, "常州大屏修改", 1, "", "", None).unwrap();
        let mid = create(&conn, "宿迁报告", 2, "", "", None).unwrap();
        let low = create(&conn, "杂项", 3, "", "", None).unwrap();
        let all = list(
            &conn,
            &TodoFilter {
                scope: Some("all".into()),
                query: None,
            },
        )
        .unwrap();
        assert_eq!(all.len(), 3);
        assert_eq!(all[0].todo.id, high.id);
        assert_eq!(all[1].todo.id, mid.id);
        assert_eq!(all[2].todo.id, low.id);
        // 优先级过滤
        let only_high = list(
            &conn,
            &TodoFilter {
                scope: Some("priority:1".into()),
                query: None,
            },
        )
        .unwrap();
        assert_eq!(only_high.len(), 1);
    }

    #[test]
    fn search_matches_title_next_and_description() {
        let conn = test_conn();
        create(&conn, "四川标签复核", 1, "核对判定规则", "统计口径", None).unwrap();
        for (q, expect) in [("四川", 1usize), ("判定", 1), ("口径", 1), ("不存在", 0)] {
            let r = list(
                &conn,
                &TodoFilter {
                    scope: Some("all".into()),
                    query: Some(q.into()),
                },
            )
            .unwrap();
            assert_eq!(r.len(), expect, "query={q}");
        }
    }

    #[test]
    fn reorder_within_group_and_cross_priority() {
        let conn = test_conn();
        let a = create(&conn, "甲", 1, "", "", None).unwrap();
        let b = create(&conn, "乙", 1, "", "", None).unwrap();
        let c = create(&conn, "丙", 1, "", "", None).unwrap();

        // 丙移到最前
        let moved = reorder(&conn, &c.id, 1, Some(a.id.clone())).unwrap();
        let group = list(
            &conn,
            &TodoFilter {
                scope: Some("priority:1".into()),
                query: None,
            },
        )
        .unwrap();
        assert_eq!(group[0].todo.id, c.id);
        assert_eq!(group[0].todo.sort_order, moved.sort_order);
        assert!(group[0].todo.sort_order < group[1].todo.sort_order);

        // 甲移到末尾
        reorder(&conn, &a.id, 1, None).unwrap();
        let group = list(
            &conn,
            &TodoFilter {
                scope: Some("priority:1".into()),
                query: None,
            },
        )
        .unwrap();
        assert_eq!(group[2].todo.id, a.id);

        // 跨组移动乙到低优先级
        reorder(&conn, &b.id, 3, None).unwrap();
        let b_row = get(&conn, &b.id).unwrap();
        assert_eq!(b_row.priority, 3);
        let _ = c;
        let _ = moved;
    }

    #[test]
    fn set_done_and_restore_goes_to_group_end() {
        let conn = test_conn();
        let a = create(&conn, "甲", 2, "", "", None).unwrap();
        let b = create(&conn, "乙", 2, "", "", None).unwrap();
        set_done(&conn, &a.id, true).unwrap();
        let done = list(
            &conn,
            &TodoFilter {
                scope: Some("done".into()),
                query: None,
            },
        )
        .unwrap();
        assert_eq!(done.len(), 1);
        // 恢复后排到组末尾
        set_done(&conn, &a.id, false).unwrap();
        let group = list(
            &conn,
            &TodoFilter {
                scope: Some("priority:2".into()),
                query: None,
            },
        )
        .unwrap();
        assert_eq!(group[1].todo.id, a.id);
        assert!(group[1].todo.sort_order > b.sort_order);
    }

    #[test]
    fn delete_detaches_notes_and_collects_managed_files() {
        let conn = test_conn();
        let t = create(&conn, "带附件", 1, "", "", None).unwrap();
        let now = "2026-01-01 00:00:00";
        conn.execute(
            "INSERT INTO notes (id, title, body, todo_id, created_at, updated_at) VALUES ('n1','纪要','', ?1, ?2, ?2)",
            rusqlite::params![t.id, now],
        )
        .unwrap();
        // 托管附件 + 引用附件
        conn.execute(
            "INSERT INTO resources (id, todo_id, kind, display_name, file_mode, stored_rel_path, created_at)
             VALUES ('r1', ?1, 'file', 'a.docx', 'copy', 'todo/t1/r1_a.docx', ?2)",
            rusqlite::params![t.id, now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO resources (id, todo_id, kind, display_name, file_mode, original_path, created_at)
             VALUES ('r2', ?1, 'file', 'b.xlsx', 'reference', 'D:/外部/b.xlsx', ?2)",
            rusqlite::params![t.id, now],
        )
        .unwrap();

        let (managed, detached) = delete(&conn, &t.id).unwrap();
        assert_eq!(managed, vec!["todo/t1/r1_a.docx".to_string()]);
        assert_eq!(detached, 1);
        // 待办与资料行已删除
        assert!(get(&conn, &t.id).is_err());
        let res: i64 = conn
            .query_row("SELECT COUNT(*) FROM resources", [], |r| r.get(0))
            .unwrap();
        assert_eq!(res, 0);
        // 笔记保留且解除关联
        let (title, todo_id): (String, Option<String>) = conn
            .query_row("SELECT title, todo_id FROM notes WHERE id='n1'", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .unwrap();
        assert_eq!(title, "纪要");
        assert!(todo_id.is_none());
    }

    #[test]
    fn validation_rejects_bad_input() {
        let conn = test_conn();
        assert!(create(&conn, "", 2, "", "", None).is_err());
        assert!(create(&conn, "x", 9, "", "", None).is_err());
        assert!(create(&conn, "x", 2, "", "", Some("2026/01/01".into())).is_err());
        let long = "字".repeat(161);
        assert!(create(&conn, &long, 2, "", "", None).is_err());
    }
}
