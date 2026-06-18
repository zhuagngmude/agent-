# 后端架构设计（索引页）

日期：2026-06-13

本文是 `docs/backend-architecture.md` 的索引页。本项目为 Tauri 桌面应用，架构内容分散在以下文档中，本文只保留仍在用的入口。

## 指向

| 内容 | 所在文档 |
|------|----------|
| 后端场景、状态枚举、审批边界、Rust 分层、commands 清单、数据流转 | [docs/backend-design.md](backend-design.md) |
| 数据库表结构、字段规范、迁移顺序 | [docs/data-model-draft.md](data-model-draft.md) |
| SQLite 初始化、seed、验证方案 | [docs/sqlite-seed-plan.md](sqlite-seed-plan.md) |
| 前端技术栈、目录、组件规范 | [packages/ui/README.md](../packages/ui/README.md) |
| 当前项目导航 | [dev-docs/当前项目导航.md](../dev-docs/当前项目导航.md) |
| 下一步开发路线 | [dev-docs/下一步开发路线.md](../dev-docs/下一步开发路线.md) |
| 新窗口交接说明 | [dev-docs/新窗口交接说明.md](../dev-docs/新窗口交接说明.md) |

## 技术路线

- 语言：Rust（Tauri 2 宿主）
- 数据库：SQLite（rusqlite）
- 前端：React + TypeScript + Vite + Ant Design（packages/ui）
- 前后端通信：Tauri invoke（不暴露 HTTP）

## 关键约束

- 不新增依赖必须先说明理由
- 后端开发遵守 backend-design.md 的分层和边界
- 数据库变更必须从 Migration 文件发起
