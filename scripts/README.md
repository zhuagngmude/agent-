# scripts

项目脚本目录。

后续可以放：

- 本地开发启动脚本。
- mock API 启动脚本。
- 数据导出脚本。
- 日志归档脚本。

脚本文件名和路径尽量使用英文 ASCII。

## 当前脚本

```text
start-mock-api.ps1
start-dev.ps1
verify-mock-flows.ps1
```

启动 `services/api/server.js`。

`start-dev.ps1` 会启动 mock API 并打开 `apps/web/index.html`。

`verify-mock-flows.ps1` 会验证 Mock API 的关键状态流转，并在结束后重置本地 runtime state。

本地 Demo 验收步骤见：

```text
../docs/demo-checklist.md
```
