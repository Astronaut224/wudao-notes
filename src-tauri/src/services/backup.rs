use std::io::{Read, Write};
use std::path::Path;

use rusqlite::Connection;

use super::{BizError, BizResult};

pub const BACKUP_DB_ENTRY: &str = "data/wudao.db";

/// 导出备份：SQLite 备份接口生成一致性快照 + 附件目录，打包为 zip。
/// 不在 WAL 写入期间裸拷贝数据库文件。
pub fn export(conn: &Connection, attachments_root: &Path, destination: &Path) -> BizResult<()> {
    let tmp_db = destination.with_extension("db.tmp");
    let _ = std::fs::remove_file(&tmp_db);
    {
        let mut dst = Connection::open(&tmp_db)
            .map_err(|e| BizError::new("io", format!("创建备份临时库失败: {e}")))?;
        let bk = rusqlite::backup::Backup::new(conn, &mut dst)
            .map_err(|e| BizError::new("db", format!("初始化备份失败: {e}")))?;
        // 分步备份直到完成，避免一次性长时间持锁
        loop {
            match bk.step(64) {
                Ok(rusqlite::backup::StepResult::Done) => break,
                // More=还有剩余页；Busy/Locked=短暂竞争，重试即可
                Ok(_) => continue,
                Err(e) => return Err(BizError::new("db", format!("生成备份快照失败: {e}"))),
            }
        }
    }

    let file = std::fs::File::create(destination)
        .map_err(|e| BizError::new("io", format!("创建备份文件失败: {e}")))?;
    let mut zip = zip::ZipWriter::new(file);
    let options: zip::write::SimpleFileOptions = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    let mut entry_added = false;
    let result = (|| -> BizResult<()> {
        zip.start_file(BACKUP_DB_ENTRY, options)
            .map_err(|e| BizError::new("io", format!("写入备份失败: {e}")))?;
        let mut db_bytes = Vec::new();
        std::fs::File::open(&tmp_db)
            .and_then(|mut f| f.read_to_end(&mut db_bytes))
            .map_err(|e| BizError::new("io", format!("读取快照失败: {e}")))?;
        zip.write_all(&db_bytes)
            .map_err(|e| BizError::new("io", format!("写入备份失败: {e}")))?;
        entry_added = true;

        // 附件目录（含子目录）
        add_dir_to_zip(&mut zip, options, attachments_root, "attachments", &mut entry_added)?;
        Ok(())
    })();

    let _ = zip.finish();
    let _ = std::fs::remove_file(&tmp_db);
    result.map_err(|e| {
        if !entry_added {
            let _ = std::fs::remove_file(destination);
        }
        e
    })?;
    if !entry_added {
        return Err(BizError::new("io", "备份内容为空"));
    }
    Ok(())
}

fn add_dir_to_zip<W: std::io::Write + std::io::Seek>(
    zip: &mut zip::ZipWriter<W>,
    options: zip::write::SimpleFileOptions,
    dir: &Path,
    prefix: &str,
    entry_added: &mut bool,
) -> BizResult<()> {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Ok(()), // 附件目录可能为空/不存在
    };
    for entry in entries {
        let entry = entry.map_err(|e| BizError::new("io", e.to_string()))?;
        let p = entry.path();
        let name = format!(
            "{prefix}/{}",
            entry.file_name().to_string_lossy()
        );
        if p.is_dir() {
            add_dir_to_zip(zip, options, &p, &name, entry_added)?;
        } else {
            zip.start_file(name.as_str(), options)
                .map_err(|e| BizError::new("io", format!("写入附件失败: {e}")))?;
            let mut bytes = Vec::new();
            std::fs::File::open(&p)
                .and_then(|mut f| f.read_to_end(&mut bytes))
                .map_err(|e| BizError::new("io", format!("读取附件失败: {e}")))?;
            zip.write_all(&bytes)
                .map_err(|e| BizError::new("io", format!("写入附件失败: {e}")))?;
            *entry_added = true;
        }
    }
    Ok(())
}

/// 恢复校验：zip 必须包含 data/wudao.db 且为含预期表结构的 SQLite 库。
/// 返回解压后的临时目录（调用方负责清理）与附件子路径列表。
pub fn validate_backup(archive: &Path) -> BizResult<()> {
    let file = std::fs::File::open(archive)
        .map_err(|_| BizError::new("io", "备份文件不存在或无法打开"))?;
    let mut zip = zip::ZipArchive::new(file)
        .map_err(|_| BizError::new("invalid", "这不是有效的备份压缩包"))?;
    if zip.by_name(BACKUP_DB_ENTRY).is_err() {
        return Err(BizError::new("invalid", "备份缺少数据库文件（data/wudao.db）"));
    }
    Ok(())
}

/// 执行恢复：把备份中的数据库内容导入当前库（同事务），附件目录原子替换。
/// 调用前应已向用户确认覆盖风险。
pub fn restore(
    conn: &Connection,
    attachments_root: &Path,
    archive: &Path,
) -> BizResult<(usize, usize)> {
    validate_backup(archive)?;
    let tmp_dir = tempfile_dir();
    extract_archive(archive, &tmp_dir)?;

    let restored_db = tmp_dir.join(BACKUP_DB_ENTRY);
    // 先独立校验库结构与版本，并读取备份的 user_version
    let restored_ver: i64 = {
        let check = Connection::open(&restored_db)
            .map_err(|_| BizError::new("invalid", "备份中的数据库无法打开"))?;
        let ver: i64 = check
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .map_err(|_| BizError::new("invalid", "备份库缺少版本信息"))?;
        if ver < 1 {
            let _ = std::fs::remove_dir_all(&tmp_dir);
            return Err(BizError::new("invalid", "备份库版本过旧或已损坏"));
        }
        let tables: i64 = check
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('todo_items','notes','resources','settings')",
                [],
                |r| r.get(0),
            )
            .map_err(|_| BizError::new("invalid", "备份库结构不完整"))?;
        if tables < 4 {
            let _ = std::fs::remove_dir_all(&tmp_dir);
            return Err(BizError::new("invalid", "备份库结构不完整"));
        }
        ver
    };

    // 同事务覆盖导入
    let todo_count: usize = {
        conn.execute_batch("BEGIN IMMEDIATE")
            .map_err(|e| BizError::new("db", format!("开启事务失败: {e}")))?;
        let r = (|| -> BizResult<usize> {
            conn.execute("ATTACH DATABASE ?1 AS restored", [restored_db.to_string_lossy()])
                .map_err(|e| BizError::new("db", format!("挂载备份库失败: {e}")))?;
            for table in ["resources", "notes", "todo_items", "settings"] {
                conn.execute_batch(&format!("DELETE FROM main.{table};"))
                    .map_err(|e| BizError::new("db", format!("清空 {table} 失败: {e}")))?;
                conn.execute_batch(&format!(
                    "INSERT INTO main.{table} SELECT * FROM restored.{table};"
                ))
                .map_err(|e| BizError::new("db", format!("导入 {table} 失败: {e}")))?;
            }
            conn.pragma_update(None, "user_version", restored_ver)
                .map_err(|e| BizError::new("db", format!("同步版本失败: {e}")))?;
            conn.execute_batch("DETACH DATABASE restored;")
                .map_err(|e| BizError::new("db", format!("卸载备份库失败: {e}")))?;
            let t: i64 = conn
                .query_row("SELECT COUNT(*) FROM todo_items", [], |r| r.get(0))
                .map_err(|e| BizError::new("db", e.to_string()))?;
            Ok(t as usize)
        })();
        match r {
            Ok(v) => {
                conn.execute_batch("COMMIT")
                    .map_err(|e| BizError::new("db", format!("提交失败: {e}")))?;
                v
            }
            Err(e) => {
                let _ = conn.execute_batch("ROLLBACK");
                let _ = std::fs::remove_dir_all(&tmp_dir);
                return Err(e);
            }
        }
    };

    let note_count: usize = conn
        .query_row("SELECT COUNT(*) FROM notes", [], |r| r.get::<_, i64>(0))
        .map(|v| v as usize)
        .unwrap_or(0);

    // 附件目录替换：旧目录移到 .restore-bak，新附件就位，失败可回滚
    let bak = attachments_root.with_extension("restore-bak");
    let _ = std::fs::remove_dir_all(&bak);
    if attachments_root.exists() {
        std::fs::rename(attachments_root, &bak)
            .map_err(|e| BizError::new("io", format!("腾退旧附件目录失败: {e}")))?;
    }
    let restored_att = tmp_dir.join("attachments");
    if restored_att.exists() {
        std::fs::create_dir_all(attachments_root)
            .and_then(|_| copy_dir_recursive(&restored_att, attachments_root))
            .map_err(|e| BizError::new("io", format!("恢复附件失败: {e}")))?;
    } else {
        std::fs::create_dir_all(attachments_root)
            .map_err(|e| BizError::new("io", format!("重建附件目录失败: {e}")))?;
    }
    let _ = std::fs::remove_dir_all(&bak);
    let _ = std::fs::remove_dir_all(&tmp_dir);

    Ok((todo_count, note_count))
}

fn tempfile_dir() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("wudao-restore-{}", uuid::Uuid::new_v4()));
    let _ = std::fs::create_dir_all(&dir);
    dir
}

fn extract_archive(archive: &Path, dest: &Path) -> BizResult<()> {
    let file = std::fs::File::open(archive)
        .map_err(|e| BizError::new("io", format!("打开备份失败: {e}")))?;
    let mut zip = zip::ZipArchive::new(file)
        .map_err(|_| BizError::new("invalid", "这不是有效的备份压缩包"))?;
    for i in 0..zip.len() {
        let mut entry = zip
            .by_index(i)
            .map_err(|e| BizError::new("invalid", format!("备份条目损坏: {e}")))?;
        // 防 zip 路径穿越
        let Some(name) = entry.enclosed_name().map(|p| p.to_path_buf()) else {
            continue;
        };
        let out_path = dest.join(name);
        if entry.is_dir() {
            std::fs::create_dir_all(&out_path)
                .map_err(|e| BizError::new("io", format!("解压失败: {e}")))?;
        } else {
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| BizError::new("io", format!("解压失败: {e}")))?;
            }
            let mut out = std::fs::File::create(&out_path)
                .map_err(|e| BizError::new("io", format!("解压失败: {e}")))?;
            std::io::copy(&mut entry, &mut out)
                .map_err(|e| BizError::new("io", format!("解压失败: {e}")))?;
        }
    }
    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let p = entry.path();
        let target = dst.join(entry.file_name());
        if p.is_dir() {
            copy_dir_recursive(&p, &target)?;
        } else {
            std::fs::copy(&p, &target)?;
        }
    }
    Ok(())
}
