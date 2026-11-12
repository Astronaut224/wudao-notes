# 数据模型

## 存储位置

- 数据库：`%LOCALAPPDATA%\cn.wudao.notes\data\wudao.db`（SQLite，`journal_mode=WAL`、`foreign_keys=ON`、`busy_timeout=3000ms`）
- 托管附件：`%LOCALAPPDATA%\cn.wudao.notes\attachments\{todo|note}\{owner_id}\{resource_id}_{文件名}`

## 表结构（迁移 v1，见 `src-tauri/src/db/migrations.rs`）

### todo_items

| 字段 | 说明 |
|---|---|
| id | UUID v4 主键 |
| title | 1–160 字 |
| next_action | 下一步动作，≤500 字 |
| priority | 1=高 / 2=中 / 3=低（CHECK） |
| status | active / done（CHECK） |
| due_date | YYYY-MM-DD，可空 |
| sort_order | 组内排序，步距 1000；移动 = 目标组整组重排 |
| description | 备注，自由文本 |
| created_at / updated_at / completed_at | 本地时区 `YYYY-MM-DD HH:MM:SS` |

索引：`(status, priority, sort_order)`、`updated_at`。

### notes

单层分类 `category`（自由输入 ≤40 字）、`is_pinned`、`is_archived`；
`todo_id` 外键 → `todo_items(id) ON DELETE SET NULL`（删除待办仅解除关联）。

### resources

- 恰好属于一个待办或一篇笔记（CHECK 约束），`ON DELETE CASCADE`。
- `kind`：file / image / url。
- `file_mode = 'copy'`：`stored_rel_path` 指向附件目录内副本，随条目删除；
  `file_mode = 'reference'`：`original_path` 为用户外部原文件，应用绝不删除。
- 路径解析一律在 Rust 端 canonicalize 并校验仍在附件根目录内（防目录穿越）。

### settings

键值表；允许写入的键有白名单（`services/settings.rs::ALLOWED_KEYS`）：
`window_mode`（normal/compact/capsule 三种形态）、`normal_geometry`、`compact_geometry`、
`capsule_geometry`、`capsule_docked`（胶囊收起侧：left/right/空）、`compact_on_top`（简洁/胶囊置顶）、
`global_shortcut`、`launch_at_login`。

## 排序语义

`reorder_todo(id, new_priority, before_id)`：目标组（同 status + priority）整组按新顺序重写
`sort_order = (i+1)*1000`；仅被移动项更新 `updated_at`。恢复未完成 = 挪到原优先级组末尾。

## 删除语义

删除待办（单事务）：
1. 收集该条目 `file_mode='copy'` 的附件相对路径；
2. 级联删除 resources 行；notes.todo_id 置空（FK SET NULL）；
3. 删除 todo_items 行；
4. 事务提交后尽力清理磁盘托管附件目录（孤儿文件不阻塞业务）。

删除笔记同理。删除资料关联：copy 模式删除内部副本；reference 模式绝不触碰原文件。

## 备份格式

zip 归档：`data/wudao.db`（SQLite backup API 一致性快照，非裸拷贝）+ `attachments/**`。
恢复：校验 zip 内数据库结构与 user_version → 同事务 ATTACH 覆盖导入 → 附件目录替换（先移入
`.restore-bak` 以便失败回滚）。
