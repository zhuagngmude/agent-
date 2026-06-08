# data/mock

本地 mock 数据预留目录。

当前 Mock API 会在这里生成 `runtime-state.json`，用于保存审批状态等运行时数据。这个文件会被 `.gitignore` 忽略：

- 重启 Mock API 后，审批状态会从 `runtime-state.json` 恢复。
- 删除 `runtime-state.json` 后，系统会回到 `services/api/mock-data.js` 中定义的初始示例状态。
- 后续接入真实数据库时，可以把这里的数据迁移到 SQLite 或 PostgreSQL。
