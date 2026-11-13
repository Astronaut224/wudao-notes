pub mod backup;
pub mod notes;
pub mod resources;
pub mod settings;
pub mod todos;

use rusqlite::Connection;

use chrono::Local;

/// 统一的时间戳格式（本地时区，字符串排序即时间排序）
pub fn now_string() -> String {
    Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

/// 手动事务：BEGIN IMMEDIATE → 业务 → COMMIT，出错回滚。
/// 单连接（Mutex）模式下无法使用 Connection::transaction(&mut self)。
pub fn with_tx<T>(
    conn: &Connection,
    f: impl FnOnce(&Connection) -> BizResult<T>,
) -> BizResult<T> {
    conn.execute_batch("BEGIN IMMEDIATE")
        .map_err(|e| BizError::new("db", format!("开启事务失败: {e}")))?;
    match f(conn) {
        Ok(v) => {
            conn.execute_batch("COMMIT")
                .map_err(|e| BizError::new("db", format!("提交事务失败: {e}")))?;
            Ok(v)
        }
        Err(e) => {
            let _ = conn.execute_batch("ROLLBACK");
            Err(e)
        }
    }
}

/// io / tauri 等各类 Display 错误 → 业务错误
pub fn map_io<E: std::fmt::Display>(e: E) -> BizError {
    BizError::new("io", format!("文件操作失败: {e}"))
}

/// 业务错误：code 用于前端区分处理方式（如引用文件失效 → 提示重新定位）
#[derive(Debug, serde::Serialize)]
pub struct BizError {
    pub code: String,
    pub message: String,
}

impl BizError {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.to_string(),
            message: message.into(),
        }
    }
}

impl std::fmt::Display for BizError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for BizError {}

pub type BizResult<T> = Result<T, BizError>;

pub fn validate_title(title: &str, max: usize) -> BizResult<String> {
    let t = title.trim();
    if t.is_empty() {
        return Err(BizError::new("invalid", "标题不能为空"));
    }
    if t.chars().count() > max {
        return Err(BizError::new("invalid", format!("标题不能超过 {max} 字")));
    }
    Ok(t.to_string())
}

pub fn validate_due_date(due: &Option<String>) -> BizResult<()> {
    if let Some(d) = due {
        if !d.is_empty()
            && chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d").is_err()
        {
            return Err(BizError::new("invalid", "截止日期格式应为 YYYY-MM-DD"));
        }
    }
    Ok(())
}
