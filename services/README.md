# services

后端和本地服务目录。

- `api/`: 云端或本地 API 服务。
- `runner/`: 本地 Runner，负责文件读写、Git checkpoint、测试执行。
- `worker/`: Agent 调度、异步任务和模型调用队列。

关键原则：Runner 不能绕过 Approval Service 直接执行。

