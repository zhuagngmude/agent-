# 后端状态切片文档

## 1. 本地命令定义

### 1.1 命令列表

| 命令 | 描述 | 参数 |
|------|------|------|
| `init` | 初始化项目 | `project_name` |
| `start` | 启动开发服务器 | `port` (默认3000) |
| `build` | 构建生产版本 | `output_dir` (默认dist) |
| `deploy` | 部署到服务器 | `target`, `env` |
| `test` | 运行测试 | `suite` (可选) |
| `status` | 查看项目状态 | 无 |

### 1.2 命令执行流程

```
用户输入命令 → 命令解析器 → 参数校验 → 执行逻辑 → 状态更新 → 返回结果
```

## 2. 状态流转

### 2.1 项目状态机

```
[未初始化] → init → [已初始化]
[已初始化] → start → [运行中]
[运行中] → stop → [已停止]
[已初始化] → build → [构建中]
[构建中] → complete → [已构建]
[已构建] → deploy → [部署中]
[部署中] → success → [已部署]
[部署中] → fail → [部署失败]
```

### 2.2 状态转换规则

- 每个状态转换必须经过命令触发
- 转换前校验前置条件
- 转换后更新持久化存储
- 记录转换时间戳

## 3. SQLite 持久化边界

### 3.1 数据库表结构

```sql
-- 项目表
CREATE TABLE projects (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    status TEXT NOT NULL DEFAULT 'uninitialized',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 命令历史表
CREATE TABLE command_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id INTEGER NOT NULL,
    command TEXT NOT NULL,
    params TEXT,
    status TEXT NOT NULL,
    executed_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (project_id) REFERENCES projects(id)
);

-- 状态变更日志表
CREATE TABLE status_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id INTEGER NOT NULL,
    from_status TEXT,
    to_status TEXT NOT NULL,
    changed_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (project_id) REFERENCES projects(id)
);
```

### 3.2 数据访问层接口

```python
class ProjectRepository:
    def create_project(name: str) -> Project
    def get_project(id: int) -> Project
    def update_status(id: int, status: str) -> None
    def list_projects() -> List[Project]

class CommandHistoryRepository:
    def log_command(project_id: int, command: str, params: dict, status: str) -> None
    def get_history(project_id: int) -> List[CommandHistory]

class StatusLogRepository:
    def log_status_change(project_id: int, from_status: str, to_status: str) -> None
    def get_status_log(project_id: int) -> List[StatusLog]
```

## 4. 边界定义

### 4.1 当前实现范围（最小执行记录）

- 命令解析与参数校验
- 状态机核心逻辑
- SQLite 内存数据库操作
- 日志记录

### 4.2 后续开放范围

- 真实文件写入
- 系统命令执行
- Git 集成
- 网络部署

### 4.3 接口契约

```
输入: JSON格式的命令请求
输出: JSON格式的执行结果
错误: 标准错误码 + 错误消息
```

## 5. 示例执行记录

```json
{
  "command": "init",
  "params": {"project_name": "my-ui-app"},
  "status_flow": [
    {"from": null, "to": "uninitialized"},
    {"from": "uninitialized", "to": "initialized"}
  ],
  "result": {
    "success": true,
    "project_id": 1,
    "message": "Project my-ui-app initialized"
  }
}
```