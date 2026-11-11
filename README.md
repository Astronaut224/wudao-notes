# 吾道笔记（wudao-notes）

随时能打开的「短期待办清单 + 关联资料 + 轻量笔记本」。Windows 桌面常驻工具：
高/中/低优先级安排一天的工作，点开条目就能看到下一步动作、备注和关联的文件/图片/链接；
需要时切到迷你悬浮窗速记，用完随手收进托盘。完全离线、本地保存、无账号。

![技术栈](https://img.shields.io/badge/Tauri%202%20%2B%20Svelte%205%20%2B%20SQLite-吾道笔记-4466E8)

## 核心功能

| 模块 | 能力 |
|---|---|
| 待办 | 高/中/低优先级分组、拖拽排序（跨组拖动改优先级）、一键完成/恢复、下一步动作、备注自动保存、截止日期提示、复制条目、搜索 |
| 关联资料 | 文件/图片（默认复制进应用，原文件移走仍可打开）、仅引用模式（失效提示 + 重新定位）、网页链接（默认浏览器打开）、图片缩略与放大 |
| 笔记 | Markdown 纯文本编辑 + 安全预览（禁原始 HTML）、单层分类、置顶/归档、搜索、可选关联待办 |
| 桌面集成 | 单窗口正常/迷你形态切换、迷你窗置顶、托盘常驻（打开/切换迷你/新建待办/退出）、全局快捷键呼出、单实例、窗口几何记忆、开机启动（默认关） |
| 数据 | SQLite（WAL + 外键 + 参数化 SQL）、导出备份（zip：数据库快照 + 附件）、校验后恢复 |

## 开发

```bash
npm install          # 前端依赖
npm run dev          # 前端热更（浏览器模式，无 Tauri 外壳）
npx tauri dev        # 桌面应用开发模式（首次会编译 Rust，耗时较长）
```

测试与检查：

```bash
npm run check        # svelte-check 类型检查
npm test             # Vitest 单元测试（Markdown 渲染器等）
npm run build        # 前端生产构建
cd src-tauri && cargo test    # Rust 单元测试（迁移/排序/级联删除等）
```

打包 Windows 安装包（NSIS）：

```bash
npx tauri build      # 产物在 src-tauri/target/release/bundle/nsis/
```

## 数据与备份位置

- 数据库：`%LOCALAPPDATA%\cn.wudao.notes\data\wudao.db`
- 托管附件：`%LOCALAPPDATA%\cn.wudao.notes\attachments\`
- 备份：设置 → 导出备份（zip）；恢复前会校验归档并二次确认覆盖风险

## 文档

- [docs/IMPLEMENTATION_STATUS.md](docs/IMPLEMENTATION_STATUS.md) —— 功能状态与测试证据
- [docs/DATA_MODEL.md](docs/DATA_MODEL.md) —— 数据模型
- [docs/TEST_REPORT.md](docs/TEST_REPORT.md) —— 测试记录
- [docs/PERFORMANCE.md](docs/PERFORMANCE.md) —— 性能实测
- [AGENTS.md](AGENTS.md) —— Agent 协作约定（产品红线）

## 已知边界（首版按需求不做）

不做项目管理中心、统计、日历、协作、AI、云同步；笔记不提供富文本；截止日期仅文本提示，无系统通知；回收站未内置（删除为二次确认）。
