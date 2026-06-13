# 技术栈说明

日期：2026-06-12

本文记录 `agent蜂群` 的当前实际技术栈、老师反馈后的调整方向，以及重新立项后已确认的正式技术栈。当前只做已确认架构内的最小闭环和只读链路，不写完整真实业务功能。

## 当前实际技术栈

当前仓库已经落地的是 MVP 验证原型：

- 前端：`apps/web/` 中的原生 HTML、CSS、JavaScript。
- 后端：`services/api/server.js` 中的 Node.js 原生 HTTP server。
- Mock 数据和状态：JavaScript mock 数据加 `data/mock/runtime-state.json`。
- 本地数据库验证：SQLite，文件位于 `data/local/agent-swarm.sqlite`，通过 PowerShell 和 Python 标准库 `sqlite3` 初始化、seed 和验证。
- 脚本：PowerShell。
- 文档：Markdown。

这套技术栈的价值是快速验证产品流程、API 形态、状态流转、审批边界和 Runner 安全规则。它不是正式工程架构，也不适合继续作为长期交付方案扩展。

## 重新立项判断

老师反馈后，当前进入重新立项讨论阶段。结论是：

- 不继续手搓前端。
- 不继续把原生 Node.js HTTP server 扩展为正式后端。
- 不直接在旧目录上叠功能。
- 已确认的新工程按最小闭环推进，不在未确认范围内扩展。
- 旧 MVP 作为原型和验证资产保留，后续按新架构选择性迁移。

## 已确认正式技术栈

当前确认方案：

```text
桌面主入口：
Tauri 2 + Rust

前端 UI：
React + TypeScript + Vite + Ant Design

本地数据库：
SQLite + rusqlite（运行库放 Tauri app data 目录）

后续云端数据库：
PostgreSQL / Supabase（单机本地版稳定后再讨论）

共享 UI：
packages/ui 作为唯一 UI 源码
apps/web 和 apps/desktop 作为运行入口
```

## 前端方案

旧前端位于 `apps/web/`，是原生 HTML / CSS / JavaScript。

后续不再继续手搓 UI。建议使用：

- React：组件化 UI。
- TypeScript：约束数据结构和组件 props。
- Vite：本地开发和构建。
- Ant Design：表格、表单、弹窗、标签、菜单、布局、提示等成熟组件。

目标是把 Web 和桌面端共用 UI 源码放到：

```text
packages/ui
```

`apps/web` 和 `apps/desktop` 不再各自维护一套 UI，它们只作为运行入口消费 `packages/ui`。当前 `packages/ui` 已完成 React + TypeScript + Vite + Ant Design 最小骨架。

## 桌面端方案

桌面端建议使用：

```text
Tauri + Rust
```

Rust 的职责：

- 本地文件能力。
- Git 状态和 checkpoint。
- Runner 连接。
- 权限边界。
- 本地安全能力。

Rust 不负责重新绘制一套 UI。桌面端应加载和复用 `packages/ui` 中的 React UI，避免 Web 和桌面两套界面分裂。

当前 `apps/desktop` 已初始化 Tauri 2 + Rust 最小宿主，并接通 `get_project` -> SQLite 只读链路。

## 后端方案

当前项目不再单独起网络后端服务。

Rust 宿主负责：

- 本地文件能力
- Git 状态和 checkpoint
- SQLite 访问
- 本地权限边界
- 本地安全能力

后续如果确实需要服务化能力，再单独讨论是否增加 API 层，但当前重立项阶段不做。

## 数据库方案

当前 SQLite 是本地验证层，不是云端生产数据库。

本地版建议继续保留 SQLite，因为它适合单机桌面应用：

- 无需云服务。
- 易于打包。
- 适合本地项目状态和审计记录。

但后续不建议继续手写大量 SQL 映射。可以讨论：

- Prisma：生态成熟、类型生成友好。
- Drizzle：更轻量、更贴近 SQL。

后续团队和云端版再讨论 PostgreSQL / Supabase。

## 暂不做

- 不直接开始完整 React 业务重写。
- 不直接删除旧前端目录。
- 不继续扩展原生 HTML / CSS / JavaScript UI。
- 不继续扩展 Node.js 原生 HTTP server 作为正式后端。
- 不接真实模型。
- 不启用真实 Runner。
- 不做云同步或完整权限系统。

## 下一步清单

1. 确认旧原型归档和迁移方案。
2. 补齐 `agents / tasks / approvals` 只读 Tauri commands。
3. 让 `packages/ui` 从 SQLite 读取真实只读数据。
4. 确认 `packages/agent-core`、`packages/shared` 和桌面宿主层的长期拆分。
