# AGENTS.md —— 吾道笔记 Agent 协作约定

> 需求唯一基准：`wudao-notes_AI_Agent_开发说明.md` v2.0（极简重设计）。
> 本文件为编程 Agent 的核心约束速查，细节以开发说明为准。

## 产品红线（硬约束）

1. **只保留两个业务模块**：待办、笔记。设置仅为底部小图标 + 托盘菜单，不是业务模块。
2. **禁止扩充**：Dashboard、项目管理中心、统计图表、日历、甘特图、番茄钟、协作、AI、云同步、知识图谱、富文本编辑器等一律不做。
3. **单窗口双形态**：正常模式 ↔ 迷你悬浮模式复用同一 WebView，绝不并行开第二个窗口。
4. **完全离线**：无账号、无云端、无遥测；数据存 SQLite（`app_local_data_dir/data/wudao.db`），附件复制到 `app_local_data_dir/attachments/`。
5. **性能是硬指标**：禁止 1 秒级定时刷新、轮询、常驻全表查询；空闲 CPU <1%，内存争取 ≤150MB，实测数据记入 `docs/PERFORMANCE.md`。
6. **安全**：不向 WebView 暴露 SQL / 任意文件路径 / shell；Markdown 预览禁原始 HTML；链接仅 http(s)。
7. **UI 文案全部简体中文**；发布版首次启动默认为空数据。
8. UI 视觉基准：仓库内三张效果图 + `极简交互预览.html`；主色 `#4466E8`。

## 技术栈

- Tauri 2 + Rust（rusqlite 单连接 Mutex，WAL + foreign_keys，版本化迁移）
- Svelte 5（runes）+ TypeScript + Vite；原生 CSS；零重型依赖
- 后端命令层 `src-tauri/src/commands/`，服务层 `services/`，数据层 `db/`

## 每阶段纪律

- 真实运行 `npm run check`、`npm test`、`cargo test`、`npm run build`；失败必须修复。
- 更新 `docs/IMPLEMENTATION_STATUS.md`（功能状态 + 测试证据）。
- 未实现的能力不得暴露操作入口。
- 数据库 schema 变更 = 新增迁移（`db/migrations.rs`），禁止改历史迁移。
