use std::path::{Path, PathBuf};

use rusqlite::Connection;

use super::{now_string, BizError, BizResult};

/// 附件根目录内的相对路径，仅由本模块写入；读取时一律做目录穿越校验。
pub struct AttachmentRoot(pub PathBuf);

#[derive(Debug, Clone, serde::Serialize)]
pub struct Resource {
    pub id: String,
    pub owner_type: String,
    pub todo_id: Option<String>,
    pub note_id: Option<String>,
    pub kind: String,
    pub display_name: String,
    pub url: Option<String>,
    pub file_mode: Option<String>,
    pub stored_rel_path: Option<String>,
    pub original_path: Option<String>,
    pub file_size: Option<i64>,
    pub created_at: String,
    /// 引用模式下原文件当前是否存在（copy 模式恒为 true）
    pub exists: bool,
}

const IMAGE_EXTS: &[&str] = &["png", "jpg", "jpeg", "gif", "webp", "bmp", "ico"];
/// 内联（缩略图/放大查看）允许的图片扩展名；svg 可能含脚本，不提供内联预览
const INLINE_IMAGE_EXTS: &[&str] = &["png", "jpg", "jpeg", "gif", "webp", "bmp", "ico"];
pub const MAX_INLINE_BYTES: i64 = 64 * 1024 * 1024;

fn row_to_resource(r: &rusqlite::Row) -> rusqlite::Result<Resource> {
    let todo_id: Option<String> = r.get("todo_id")?;
    let note_id: Option<String> = r.get("note_id")?;
    Ok(Resource {
        id: r.get("id")?,
        owner_type: if todo_id.is_some() {
            "todo".to_string()
        } else {
            "note".to_string()
        },
        todo_id,
        note_id,
        kind: r.get("kind")?,
        display_name: r.get("display_name")?,
        url: r.get("url")?,
        file_mode: r.get("file_mode")?,
        stored_rel_path: r.get("stored_rel_path")?,
        original_path: r.get("original_path")?,
        file_size: r.get("file_size")?,
        created_at: r.get("created_at")?,
        exists: true,
    })
}

pub fn detect_kind(path: &Path) -> String {
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    if IMAGE_EXTS.contains(&ext.as_str()) {
        "image".to_string()
    } else {
        "file".to_string()
    }
}

/// Windows 文件名清洗：去掉路径成分与非法字符，保留中文名
pub fn sanitize_filename(name: &str) -> String {
    let trimmed = name
        .rsplit(['\\', '/'])
        .next()
        .unwrap_or("文件")
        .trim();
    let mut out: String = trimmed
        .chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '|' | '?' | '*' | '\0'..='\x1f' => '_',
            _ => c,
        })
        .collect();
    while out.ends_with('.') || out.ends_with(' ') {
        out.pop();
    }
    if out.is_empty() {
        out = "文件".to_string();
    }
    if out.chars().count() > 120 {
        out = out.chars().take(120).collect();
    }
    out
}

fn validate_owner(conn: &Connection, owner_type: &str, owner_id: &str) -> BizResult<()> {
    match owner_type {
        "todo" => {
            let ok: bool = conn
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM todo_items WHERE id=?1)",
                    [owner_id],
                    |r| r.get(0),
                )
                .map_err(|e| BizError::new("db", e.to_string()))?;
            if !ok {
                return Err(BizError::new("not_found", "待办不存在，无法添加资料"));
            }
        }
        "note" => {
            let ok: bool = conn
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM notes WHERE id=?1)",
                    [owner_id],
                    |r| r.get(0),
                )
                .map_err(|e| BizError::new("db", e.to_string()))?;
            if !ok {
                return Err(BizError::new("not_found", "笔记不存在，无法添加资料"));
            }
        }
        _ => return Err(BizError::new("invalid", "资料归属类型不合法")),
    }
    Ok(())
}

/// 添加本地文件。copy 模式复制到附件目录（临时文件 + 原子重命名）；reference 模式仅记录原始路径。
pub fn attach_local_file(
    conn: &Connection,
    root: &AttachmentRoot,
    owner_type: &str,
    owner_id: &str,
    source: &Path,
    mode: &str,
) -> BizResult<Resource> {
    if mode != "copy" && mode != "reference" {
        return Err(BizError::new("invalid", "附件模式不合法"));
    }
    validate_owner(conn, owner_type, owner_id)?;
    let source = source
        .canonicalize()
        .map_err(|_| BizError::new("file_missing", "所选文件不存在或无法访问"))?;
    if !source.is_file() {
        return Err(BizError::new("file_missing", "所选路径不是文件"));
    }
    let file_size: i64 = std::fs::metadata(&source)
        .map_err(|_| BizError::new("file_missing", "无法读取文件信息"))?
        .len() as i64;

    let id = uuid::Uuid::new_v4().to_string();
    let kind = detect_kind(&source);
    let display_name = sanitize_filename(
        &source
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "文件".into()),
    );
    let now = now_string();

    let (file_mode, stored_rel, original_path): (&str, Option<String>, Option<String>) =
        if mode == "copy" {
            let dir = format!("{owner_type}/{owner_id}");
            let final_rel = format!("{dir}/{id}_{}", display_name);
            let dest_dir = root.0.join(&dir);
            std::fs::create_dir_all(&dest_dir)
                .map_err(|e| BizError::new("io", format!("创建附件目录失败: {e}")))?;
            let tmp_path = dest_dir.join(format!(".tmp-{id}"));
            let final_path = root.0.join(&final_rel);
            // 复制到临时文件，再原子重命名，避免半成品附件
            std::fs::copy(&source, &tmp_path)
                .map_err(|e| BizError::new("io", format!("复制附件失败: {e}")))?;
            if let Err(e) = std::fs::rename(&tmp_path, &final_path) {
                let _ = std::fs::remove_file(&tmp_path);
                return Err(BizError::new("io", format!("保存附件失败: {e}")));
            }
            ("copy", Some(final_rel), None)
        } else {
            ("reference", None, Some(source.to_string_lossy().to_string()))
        };

    conn.execute(
        "INSERT INTO resources (id, todo_id, note_id, kind, display_name, url, file_mode, stored_rel_path, original_path, file_size, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, NULL, ?6, ?7, ?8, ?9, ?10)",
        rusqlite::params![
            id,
            if owner_type == "todo" { Some(owner_id) } else { None },
            if owner_type == "note" { Some(owner_id) } else { None },
            kind,
            display_name,
            file_mode,
            stored_rel,
            original_path,
            file_size,
            now
        ],
    )
    .map_err(|e| BizError::new("db", format!("写入资料失败: {e}")))?;
    get(conn, root, &id)
}

pub fn attach_url(
    conn: &Connection,
    root: &AttachmentRoot,
    owner_type: &str,
    owner_id: &str,
    display_name: &str,
    url: &str,
) -> BizResult<Resource> {
    validate_owner(conn, owner_type, owner_id)?;
    let name = super::validate_title(display_name, 120)?;
    let url = url.trim();
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err(BizError::new("invalid", "链接必须以 http:// 或 https:// 开头"));
    }
    if url.chars().count() > 2048 {
        return Err(BizError::new("invalid", "链接过长"));
    }
    let id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO resources (id, todo_id, note_id, kind, display_name, url, file_mode, stored_rel_path, original_path, file_size, created_at)
         VALUES (?1, ?2, ?3, 'url', ?4, ?5, NULL, NULL, NULL, NULL, ?6)",
        rusqlite::params![
            id,
            if owner_type == "todo" { Some(owner_id) } else { None },
            if owner_type == "note" { Some(owner_id) } else { None },
            name,
            url,
            now_string()
        ],
    )
    .map_err(|e| BizError::new("db", format!("写入资料失败: {e}")))?;
    get(conn, root, &id)
}

pub fn list(
    conn: &Connection,
    root: &AttachmentRoot,
    owner_type: &str,
    owner_id: &str,
) -> BizResult<Vec<Resource>> {
    let (col, val) = match owner_type {
        "todo" => ("todo_id", owner_id),
        "note" => ("note_id", owner_id),
        _ => return Err(BizError::new("invalid", "资料归属类型不合法")),
    };
    let sql = format!("SELECT * FROM resources WHERE {col} = ?1 ORDER BY created_at ASC, id ASC");
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| BizError::new("db", format!("查询资料失败: {e}")))?;
    let rows = stmt
        .query_map([val], |r| row_to_resource(r))
        .map_err(|e| BizError::new("db", format!("查询资料失败: {e}")))?;
    let mut out = Vec::new();
    for r in rows {
        let mut res = r.map_err(|e| BizError::new("db", format!("读取资料失败: {e}")))?;
        res.exists = resource_exists(root, &res);
        out.push(res);
    }
    Ok(out)
}

fn resource_exists(root: &AttachmentRoot, res: &Resource) -> bool {
    match res.kind.as_str() {
        "url" => true,
        _ => resolve_file_path(root, res)
            .map(|p| p.is_file())
            .unwrap_or(false),
    }
}

pub fn get(conn: &Connection, root: &AttachmentRoot, id: &str) -> BizResult<Resource> {
    let mut res = conn
        .query_row("SELECT * FROM resources WHERE id=?1", [id], row_to_resource)
        .map_err(|_| BizError::new("not_found", "资料不存在"))?;
    res.exists = resource_exists(root, &res);
    Ok(res)
}

/// 把记录解析为实际磁盘路径。copy 模式路径必须位于附件根目录内（防目录穿越）。
pub fn resolve_file_path(root: &AttachmentRoot, res: &Resource) -> BizResult<PathBuf> {
    let path = match (res.file_mode.as_deref(), res.stored_rel_path.as_deref(), res.original_path.as_deref()) {
        (Some("copy"), Some(rel), _) => {
            let candidate = root.0.join(rel);
            // 穿越校验：canonicalize 后必须仍在附件根目录内
            let canon = candidate
                .canonicalize()
                .map_err(|_| BizError::new("file_missing", "附件文件不存在（可能已被移动或删除）"))?;
            let root_canon = root
                .0
                .canonicalize()
                .map_err(|e| BizError::new("io", format!("附件目录异常: {e}")))?;
            if !canon.starts_with(&root_canon) {
                return Err(BizError::new("invalid", "附件路径不合法"));
            }
            canon
        }
        (Some("reference"), _, Some(orig)) => PathBuf::from(orig),
        _ => return Err(BizError::new("invalid", "该资料不是本地文件")),
    };
    Ok(path)
}

/// 移除关联。copy 模式同时删除应用内部副本；reference 模式绝不碰外部原文件。
pub fn remove(conn: &Connection, root: &AttachmentRoot, id: &str) -> BizResult<()> {
    let res = get(conn, root, id)?;
    if res.file_mode.as_deref() == Some("copy") {
        if let Some(rel) = &res.stored_rel_path {
            let p = root.0.join(rel);
            let canon_ok = p
                .canonicalize()
                .ok()
                .and_then(|c| root.0.canonicalize().ok().map(|r| c.starts_with(r)))
                .unwrap_or(false);
            if canon_ok {
                let _ = std::fs::remove_file(&p); // 删除失败不阻塞，孤儿文件可在备份清理时处理
            }
        }
    }
    conn.execute("DELETE FROM resources WHERE id=?1", [id])
        .map_err(|e| BizError::new("db", format!("移除资料失败: {e}")))?;
    Ok(())
}

/// 引用模式失效后的重新定位
pub fn relocate(conn: &Connection, root: &AttachmentRoot, id: &str, new_path: &Path) -> BizResult<Resource> {
    let res = get(conn, root, id)?;
    if res.file_mode.as_deref() != Some("reference") {
        return Err(BizError::new("invalid", "仅“仅引用”模式的资料需要重新定位"));
    }
    let canon = new_path
        .canonicalize()
        .map_err(|_| BizError::new("file_missing", "新文件不存在或无法访问"))?;
    if !canon.is_file() {
        return Err(BizError::new("file_missing", "新路径不是文件"));
    }
    let size: i64 = std::fs::metadata(&canon)
        .map_err(|e| BizError::new("io", e.to_string()))?
        .len() as i64;
    conn.execute(
        "UPDATE resources SET original_path=?2, file_size=?3 WHERE id=?1",
        rusqlite::params![id, canon.to_string_lossy(), size],
    )
    .map_err(|e| BizError::new("db", format!("更新资料失败: {e}")))?;
    get(conn, root, id)
}

/// 删除托管附件目录（owner 被删除时），尽力而为
pub fn cleanup_owner_dir(root: &AttachmentRoot, owner_type: &str, owner_id: &str) {
    let _ = std::fs::remove_dir_all(root.0.join(owner_type).join(owner_id));
}

pub fn is_inline_image(res: &Resource) -> bool {
    if res.kind != "image" {
        return false;
    }
    let ext = res
        .display_name
        .rsplit('.')
        .next()
        .map(|e| e.to_lowercase())
        .unwrap_or_default();
    INLINE_IMAGE_EXTS.contains(&ext.as_str())
        && res.file_size.map(|s| s <= MAX_INLINE_BYTES).unwrap_or(true)
}

pub fn mime_for(res: &Resource) -> &'static str {
    let ext = res
        .display_name
        .rsplit('.')
        .next()
        .map(|e| e.to_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "ico" => "image/x-icon",
        _ => "application/octet-stream",
    }
}
