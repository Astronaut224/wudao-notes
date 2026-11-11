use rusqlite::Connection;
use serde::Deserialize;

use super::{now_string, BizError, BizResult};

pub const DEFAULT_TITLE: &str = "未命名笔记";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Note {
    pub id: String,
    pub title: String,
    pub body: String,
    pub category: Option<String>,
    pub is_pinned: bool,
    pub is_archived: bool,
    pub todo_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct NoteSummary {
    #[serde(flatten)]
    pub note: Note,
    pub resource_count: i64,
    /// 关联待办标题（便于列表直接辨认），未关联为 null
    pub todo_title: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub struct NotePatch {
    pub title: Option<String>,
    pub body: Option<String>,
    pub category: Option<String>,
    pub is_pinned: Option<bool>,
    pub is_archived: Option<bool>,
    /// Some("") 或 None 表示解除关联；Some(id) 表示关联到该待办
    pub todo_id: Option<String>,
}

fn row_to_note(r: &rusqlite::Row) -> rusqlite::Result<Note> {
    Ok(Note {
        id: r.get("id")?,
        title: r.get("title")?,
        body: r.get("body")?,
        category: r.get("category")?,
        is_pinned: r.get::<_, i64>("is_pinned")? != 0,
        is_archived: r.get::<_, i64>("is_archived")? != 0,
        todo_id: r.get("todo_id")?,
        created_at: r.get("created_at")?,
        updated_at: r.get("updated_at")?,
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

pub fn list(
    conn: &Connection,
    query: Option<String>,
    category: Option<String>,
    include_archived: bool,
) -> BizResult<Vec<NoteSummary>> {
    let mut sql = String::from(
        "SELECT n.*, \
           (SELECT COUNT(*) FROM resources r WHERE r.note_id = n.id) AS resource_count, \
           t.title AS todo_title \
         FROM notes n LEFT JOIN todo_items t ON t.id = n.todo_id WHERE 1=1",
    );
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if !include_archived {
        sql.push_str(" AND n.is_archived = 0");
    }
    if let Some(cat) = category.as_deref().map(str::trim).filter(|c| !c.is_empty()) {
        sql.push_str(" AND n.category = ?");
        params.push(Box::new(cat.to_string()));
    }
    if let Some(q) = query.as_deref().map(str::trim).filter(|q| !q.is_empty()) {
        let like = format!("%{}%", escape_like(q));
        sql.push_str(" AND (n.title LIKE ? ESCAPE '\\' OR n.body LIKE ? ESCAPE '\\')");
        for _ in 0..2 {
            params.push(Box::new(like.clone()));
        }
    }

    sql.push_str(" ORDER BY n.is_pinned DESC, n.updated_at DESC, n.created_at DESC");

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| BizError::new("db", format!("查询笔记失败: {e}")))?;
    let refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    let rows = stmt
        .query_map(refs.as_slice(), |r| {
            Ok(NoteSummary {
                note: row_to_note(r)?,
                resource_count: r.get("resource_count")?,
                todo_title: r.get("todo_title")?,
            })
        })
        .map_err(|e| BizError::new("db", format!("查询笔记失败: {e}")))?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| BizError::new("db", format!("读取笔记失败: {e}")))?);
    }
    Ok(out)
}

pub fn get(conn: &Connection, id: &str) -> BizResult<Note> {
    conn.query_row("SELECT * FROM notes WHERE id = ?1", [id], row_to_note)
        .map_err(|_| BizError::new("not_found", "笔记不存在或已被删除"))
}

pub fn create(
    conn: &Connection,
    title: Option<String>,
    body: &str,
    category: Option<String>,
    todo_id: Option<String>,
) -> BizResult<Note> {
    if let Some(tid) = &todo_id {
        ensure_todo_exists(conn, tid)?;
    }
    let title = match title.as_deref().map(str::trim).filter(|t| !t.is_empty()) {
        Some(t) => super::validate_title(t, 160)?,
        None => DEFAULT_TITLE.to_string(),
    };
    if body.chars().count() > 200000 {
        return Err(BizError::new("invalid", "笔记正文过长"));
    }
    if let Some(c) = &category {
        let c = c.trim();
        if c.chars().count() > 40 {
            return Err(BizError::new("invalid", "分类最多 40 字"));
        }
    }

    let id = uuid::Uuid::new_v4().to_string();
    let now = now_string();
    conn.execute(
        "INSERT INTO notes (id, title, body, category, is_pinned, is_archived, todo_id, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, 0, 0, ?5, ?6, ?6)",
        rusqlite::params![
            id,
            title,
            body,
            category.as_deref().map(str::trim).filter(|c| !c.is_empty()),
            todo_id,
            now
        ],
    )
    .map_err(|e| BizError::new("db", format!("写入笔记失败: {e}")))?;
    get(conn, &id)
}

pub fn update(conn: &Connection, id: &str, patch: &NotePatch) -> BizResult<Note> {
    let _ = get(conn, id)?;
    if let Some(t) = patch.title.as_deref().map(str::trim).filter(|t| !t.is_empty()) {
        super::validate_title(t, 160)?;
    }
    if let Some(b) = &patch.body {
        if b.chars().count() > 200000 {
            return Err(BizError::new("invalid", "笔记正文过长"));
        }
    }
    if let Some(c) = &patch.category {
        if c.trim().chars().count() > 40 {
            return Err(BizError::new("invalid", "分类最多 40 字"));
        }
    }
    if let Some(tid) = &patch.todo_id {
        if !tid.is_empty() {
            ensure_todo_exists(conn, tid)?;
        }
    }

    let now = now_string();
    conn.execute(
        "UPDATE notes SET
           title       = CASE WHEN ?2 IS NULL OR trim(?2) = '' THEN title ELSE trim(?2) END,
           body        = COALESCE(?3, body),
           category    = CASE WHEN ?4 IS NULL THEN category ELSE NULLIF(trim(?4), '') END,
           is_pinned   = COALESCE(?5, is_pinned),
           is_archived = COALESCE(?6, is_archived),
           todo_id     = CASE
                           WHEN ?7 IS NULL THEN todo_id
                           WHEN trim(?7) = '' THEN NULL
                           ELSE ?7
                         END,
           updated_at  = ?8
         WHERE id = ?1",
        rusqlite::params![
            id,
            patch.title.as_deref(),
            patch.body.as_deref(),
            patch.category.as_deref(),
            patch.is_pinned.map(|b| b as i64),
            patch.is_archived.map(|b| b as i64),
            patch.todo_id.as_deref(),
            now
        ],
    )
    .map_err(|e| BizError::new("db", format!("更新笔记失败: {e}")))?;
    get(conn, id)
}

pub fn delete(conn: &Connection, id: &str) -> BizResult<Vec<String>> {
    let _ = get(conn, id)?;
    let mut managed: Vec<String> = Vec::new();
    {
        let mut stmt = conn
            .prepare(
                "SELECT stored_rel_path FROM resources WHERE note_id=?1 AND file_mode='copy' AND stored_rel_path IS NOT NULL",
            )
            .map_err(|e| BizError::new("db", e.to_string()))?;
        let rows = stmt
            .query_map([id], |r| r.get::<_, String>(0))
            .map_err(|e| BizError::new("db", e.to_string()))?;
        for r in rows {
            managed.push(r.map_err(|e| BizError::new("db", e.to_string()))?);
        }
    }
    conn.execute("DELETE FROM resources WHERE note_id=?1", [id])
        .map_err(|e| BizError::new("db", format!("删除资料失败: {e}")))?;
    conn.execute("DELETE FROM notes WHERE id=?1", [id])
        .map_err(|e| BizError::new("db", format!("删除笔记失败: {e}")))?;
    Ok(managed)
}

pub fn list_categories(conn: &Connection) -> BizResult<Vec<String>> {
    let mut stmt = conn
        .prepare(
            "SELECT DISTINCT category FROM notes WHERE category IS NOT NULL AND category != '' ORDER BY category",
        )
        .map_err(|e| BizError::new("db", format!("查询分类失败: {e}")))?;
    let rows = stmt
        .query_map([], |r| r.get::<_, String>(0))
        .map_err(|e| BizError::new("db", format!("查询分类失败: {e}")))?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r.map_err(|e| BizError::new("db", e.to_string()))?);
    }
    Ok(out)
}

fn ensure_todo_exists(conn: &Connection, todo_id: &str) -> BizResult<()> {
    let exists: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM todo_items WHERE id=?1)",
            [todo_id],
            |r| r.get(0),
        )
        .map_err(|e| BizError::new("db", e.to_string()))?;
    if !exists {
        return Err(BizError::new("not_found", "关联的待办不存在"));
    }
    Ok(())
}
