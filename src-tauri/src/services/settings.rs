use rusqlite::Connection;

use super::{BizError, BizResult};

/// settings 允许写入的键（白名单，避免前端写任意键）
pub const ALLOWED_KEYS: &[&str] = &[
    "window_mode",
    "normal_geometry",
    "compact_geometry",
    "capsule_geometry",
    "capsule_docked",
    "compact_on_top",
    "global_shortcut",
    "launch_at_login",
];

pub fn get(conn: &Connection, key: &str) -> Option<String> {
    conn.query_row(
        "SELECT value FROM settings WHERE key = ?1",
        [key],
        |r| r.get::<_, String>(0),
    )
    .ok()
}

pub fn get_all(conn: &Connection) -> Result<std::collections::HashMap<String, String>, String> {
    let mut stmt = conn
        .prepare("SELECT key, value FROM settings")
        .map_err(|e| format!("读取设置失败: {e}"))?;
    let rows = stmt
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
        .map_err(|e| format!("读取设置失败: {e}"))?;
    let mut map = std::collections::HashMap::new();
    for r in rows {
        let (k, v) = r.map_err(|e| format!("读取设置失败: {e}"))?;
        map.insert(k, v);
    }
    Ok(map)
}

pub fn set(conn: &Connection, key: &str, value: &str) -> BizResult<()> {
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        rusqlite::params![key, value],
    )
    .map_err(|e| BizError::new("db", format!("写入设置失败: {e}")))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_upserts() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(crate::db::migrations::V1_SCHEMA).unwrap();
        set(&conn, "k", "v1").unwrap();
        set(&conn, "k", "v2").unwrap();
        assert_eq!(get(&conn, "k"), Some("v2".into()));
    }
}
