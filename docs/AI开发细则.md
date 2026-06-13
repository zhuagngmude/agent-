# AI 开发细则

> 本文件承接《Agent 宪法》的执行细则。宪法管原则，细则管日常开发怎么落地；如果两者冲突，以《Agent 宪法》为准。

## 一、当前阶段边界

- 桌面端是主入口，Web 只作为辅助预览或后续扩展入口。
- 当前允许的本地写入只限已确认的工程文件和 Tauri app data 下的 SQLite 运行库。
- 不写入受保护路径，不绕过审批写用户项目文件。
- 不开放真实 Runner 执行、不调用真实模型、不做云同步、不做完整权限系统。
- Runner 不得自动执行命令、写文件、删文件、发网络请求或修改 Git。
- 真实模型调用默认关闭；禁止导入 provider SDK、读取 raw key、返回 key fragment、记录 raw prompt 或 raw provider response。

## 二、前端开发细则

- 技术栈固定为 React + TypeScript + Vite + Ant Design。
- `packages/ui/` 是唯一共享 UI 源码，桌面端和 Web 预览必须复用它。
- UI 组件优先使用 Ant Design 和已确认的图标/组件库，禁止重新手搓已有成熟组件。
- 同类 UI 结构出现超过 2 次，必须封装为通用组件放入 `packages/ui/`。
- 颜色、字号、间距、圆角、阴影必须从设计 Token 中引用，避免在组件内分散硬编码。
- 多处重复出现的按钮文字、标题、提示信息等文案，应提取到统一文案文件。

## 三、数据库开发细则

- 先理清业务对象和关系，再设计表结构，禁止跳过讨论直接建表。
- Migration 文件是数据库结构真相源，所有结构变更必须纳入 Git。
- 表名、字段名统一 `snake_case`。
- 每张业务表包含 `id`、`created_at`、`updated_at`。
- 当前本地数据库使用 SQLite + rusqlite；运行库放 Tauri app data 目录，不放 `data/local/`。
- SQL 查询必须使用参数化查询，禁止拼接用户输入。

## 四、Tauri/Rust 后端细则

- Rust 侧分层遵守 `commands/`、`services/`、`db/`：
  - `commands/` 只作为 Tauri invoke 入口，负责参数接收和响应封装。
  - `services/` 放业务规则、状态流转和审批判断。
  - `db/` 只做 SQLite 连接、迁移、seed 和数据读写。
- 第一阶段优先补齐只读 commands，再讨论写入 commands。
- 写入、变更、Runner、Git checkpoint、模型相关能力必须进入审批链，不得直接开放。

## 五、版本管理细则

- 每完成一个可验证的代码变更批次，必须执行 Git 提交，提交信息格式为 `<type>: <简述>`。
- 大范围重构、删文件或数据库结构变更前，必须确认当前工作区干净且已有最近提交。
- 暂存文件时优先指定路径，避免使用 `git add .` 误纳入保护路径、构建产物或本地运行数据。
- 推送到远程前，必须列出新增/修改文件清单，确认无敏感内容。

## 六、文档同步细则

- 改 API 同步 `docs/api-draft.md`。
- 改数据结构同步 `docs/data-model-draft.md`。
- 改演示或验收流程同步 `docs/demo-checklist.md` 和 `scripts/README.md`。
- 改阶段状态同步 `dev-docs/下一步开发路线.md`。
- 改 AI 操作边界同步 `docs/Agent宪法.md`、本文件和 `dev-docs/AI开发维护手册.md`。
- 公开计划、交接说明和阶段记录统一放到 `dev-docs/`。
