# apps/desktop

桌面主入口预留目录。

后续承载 Tauri 2 + Rust 的宿主和本地能力层，界面直接复用 `packages/ui`，不在这里重新画一套 UI。

Rust 代码建议放在 `src-tauri/src/` 下，并按职责再分：

- `commands/`：Tauri 命令入口，对应前端 `invoke`
- `db/`：rusqlite 数据访问
- `services/`：本地业务服务和流程编排
