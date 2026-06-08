# data/mock

本地 mock 数据预留目录。

当前 Mock API 会在这里生成 `runtime-state.json`，用于保存审批状态等运行时数据。这个文件会被 `.gitignore` 忽略：

- 重启 Mock API 后，审批状态会从 `runtime-state.json` 恢复。
- 删除 `runtime-state.json` 后，系统会回到 `services/api/mock-data.js` 中定义的初始示例状态。
- 后续接入真实数据库时，可以把这里的数据迁移到 SQLite 或 PostgreSQL。

可用接口：

- `GET /api/runtime-state`: 导出当前运行时状态。
- `POST /api/runtime-state/reset`: 重置为初始 mock 数据，并重新写入状态文件。
- `DELETE /api/runtime-state`: 清理状态文件，并把当前内存状态重置为初始 mock 数据。
