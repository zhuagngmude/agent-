# services/api

API 服务预留目录。

第一版接口契约见：

```text
../../docs/api-draft.md
```

后续可以先实现 mock API，再接 SQLite / PostgreSQL。

SQLite 初始化和 seed 方案见：

```text
../../docs/sqlite-seed-plan.md
```

## 本地 mock API

当前已提供纯 Node.js mock API：

```text
server.js
mock-data.js
```

启动：

```powershell
powershell -ExecutionPolicy Bypass -File scripts/start-mock-api.ps1
```

默认地址：

```text
http://127.0.0.1:8787
```

健康检查：

```text
GET /api/health
```

## SQLite Dashboard 读取

当前默认仍从 Mock 内存数据读取 Dashboard。可通过环境变量只读试用 SQLite Dashboard：

```powershell
$env:AGENT_SWARM_DASHBOARD_SOURCE="sqlite"
powershell -ExecutionPolicy Bypass -File scripts/start-mock-api.ps1
```

如果 `data/local/agent-swarm.sqlite` 不存在或查询失败，API 会回退到 Mock Dashboard。
