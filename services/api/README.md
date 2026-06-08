# services/api

API 服务预留目录。

第一版接口契约见：

```text
../../docs/api-draft.md
```

后续可以先实现 mock API，再接 SQLite / PostgreSQL。

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
