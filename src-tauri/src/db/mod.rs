use std::path::Path;

use rusqlite::Connection;

pub mod migrations;

/// 打开并初始化数据库连接（WAL + 外键 + busy_timeout + 迁移）。
/// 全局以 `Arc<Mutex<Connection>>` 持有：单窗口低频写入场景，一次锁即可传递 `&Connection`。
pub fn open_connection(data_dir: &Path) -> Result<Connection, String> {
    std::fs::create_dir_all(data_dir)
        .map_err(|e| format!("创建数据目录失败: {e}"))?;
    let db_path = data_dir.join("wudao.db");
    let conn = Connection::open(&db_path)
        .map_err(|e| format!("打开数据库失败({}): {e}", db_path.display()))?;

    // 可靠性基线：外键约束、WAL、忙等待
    conn.pragma_update(None, "foreign_keys", "ON")
        .map_err(|e| format!("开启外键失败: {e}"))?;
    conn.pragma_update(None, "journal_mode", "WAL")
        .map_err(|e| format!("设置 WAL 失败: {e}"))?;
    conn.busy_timeout(std::time::Duration::from_millis(3000))
        .map_err(|e| format!("设置 busy_timeout 失败: {e}"))?;

    migrations::run(&conn)?;
    Ok(conn)
}
