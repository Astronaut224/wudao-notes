use rusqlite::Connection;

/// 版本化迁移：按 user_version 依次执行；失败则保留旧数据并向上报错。
/// 每个迁移必须可以在已含数据的库上安全重放（幂等）。
pub fn run(conn: &Connection) -> Result<(), String> {
    let current: i64 = conn
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .map_err(|e| format!("读取数据库版本失败: {e}"))?;

    for (version, sql) in migrations() {
        let v = version;
        if v <= current {
            continue;
        }
        conn.execute_batch(sql)
            .map_err(|e| format!("执行迁移 v{v} 失败: {e}"))?;
        conn.pragma_update(None, "user_version", v)
            .map_err(|e| format!("更新数据库版本失败: {e}"))?;
    }
    Ok(())
}

pub const V1_SCHEMA: &str = r#"
PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS todo_items (
  id           TEXT PRIMARY KEY,
  title        TEXT NOT NULL,
  next_action  TEXT NOT NULL DEFAULT '',
  priority     INTEGER NOT NULL DEFAULT 2 CHECK(priority IN (1,2,3)),
  status       TEXT NOT NULL DEFAULT 'active' CHECK(status IN ('active','done')),
  due_date     TEXT,
  sort_order   INTEGER NOT NULL DEFAULT 0,
  description  TEXT NOT NULL DEFAULT '',
  created_at   TEXT NOT NULL,
  updated_at   TEXT NOT NULL,
  completed_at TEXT
);

CREATE TABLE IF NOT EXISTS notes (
  id          TEXT PRIMARY KEY,
  title       TEXT NOT NULL DEFAULT '未命名笔记',
  body        TEXT NOT NULL DEFAULT '',
  category    TEXT,
  is_pinned   INTEGER NOT NULL DEFAULT 0 CHECK(is_pinned IN (0,1)),
  is_archived INTEGER NOT NULL DEFAULT 0 CHECK(is_archived IN (0,1)),
  todo_id     TEXT REFERENCES todo_items(id) ON DELETE SET NULL,
  created_at  TEXT NOT NULL,
  updated_at  TEXT NOT NULL
);

-- 每条资料只属于一个待办或一篇笔记；
-- copy 模式：stored_rel_path 为应用附件目录内相对路径，随条目删除；
-- reference 模式：original_path 为用户外部原文件，绝不随应用删除。
CREATE TABLE IF NOT EXISTS resources (
  id              TEXT PRIMARY KEY,
  todo_id         TEXT REFERENCES todo_items(id) ON DELETE CASCADE,
  note_id         TEXT REFERENCES notes(id) ON DELETE CASCADE,
  kind            TEXT NOT NULL CHECK(kind IN ('file','image','url')),
  display_name    TEXT NOT NULL,
  url             TEXT,
  file_mode       TEXT CHECK(file_mode IN ('copy','reference') OR file_mode IS NULL),
  stored_rel_path TEXT,
  original_path   TEXT,
  file_size       INTEGER,
  created_at      TEXT NOT NULL,
  CHECK((todo_id IS NOT NULL AND note_id IS NULL) OR (todo_id IS NULL AND note_id IS NOT NULL))
);

CREATE TABLE IF NOT EXISTS settings (
  key   TEXT PRIMARY KEY,
  value TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_todo_list ON todo_items(status, priority, sort_order);
CREATE INDEX IF NOT EXISTS idx_todo_updated ON todo_items(updated_at);
CREATE INDEX IF NOT EXISTS idx_note_list ON notes(is_archived, is_pinned, updated_at);
CREATE INDEX IF NOT EXISTS idx_note_todo ON notes(todo_id);
CREATE INDEX IF NOT EXISTS idx_resource_todo ON resources(todo_id);
CREATE INDEX IF NOT EXISTS idx_resource_note ON resources(note_id);
"#;

fn migrations() -> Vec<(i64, &'static str)> {
    vec![(1, V1_SCHEMA)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_creates_all_tables() {
        let conn = Connection::open_in_memory().unwrap();
        run(&conn).unwrap();
        let n: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('todo_items','notes','resources','settings')",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(n, 4);
    }

    #[test]
    fn migration_is_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        run(&conn).unwrap();
        run(&conn).unwrap();
        assert_eq!(
            conn.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            1
        );
    }
}
