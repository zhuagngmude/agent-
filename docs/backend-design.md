# agent蜂群 后端设计

日期：2026-06-13
阶段：新架构后端边界界定

> 2026-06-17 更新：本文是新架构早期后端边界设计，仍可参考分层原则和 Tauri/Rust 方向，但“第一版”“审批边界”等描述不再完整代表当前产品状态。当前 Runner 全自动主链路、模型配置入口和任务页行为以当前源码、`docs/Agent宪法.md`、`docs/AI开发细则.md` 和 `dev-docs/当前项目导航.md` 为准。

本文是教材 #8 的产出文档，定义新架构下后端的业务边界、规则、数据流转和 command 清单。后续所有后端开发以此为准。

---

## 一、后端场景

旧 Node.js HTTP 后端（`services/api/`）已冻结。新架构后端为 **Tauri 2 + Rust 本地宿主**，不对外暴露 HTTP 端口，UI 通过 `invoke` 调用 Tauri commands。

一句话：**项目级桌面后端，单用户本地运行，UI 不直接访问 SQLite，所有数据操作走 Rust 层。**

---

## 二、状态枚举

完整状态定义见 `docs/api-draft.md`。第一版只开放部分状态，其余保留兼容：

### AgentStatus

完整：`running` / `idle` / `waiting` / `failed` / `disabled`

第一版开放：`running` / `idle` / `failed`

### TaskStatus

完整：`queued` / `running` / `blocked` / `waiting_user` / `completed` / `failed` / `cancelled`

第一版开放：`queued` / `running` / `blocked` / `completed`

### ApprovalStatus

完整：`draft` / `pending` / `approved` / `rejected` / `patch_only` / `executed` / `rolled_back` / `expired`

第一版开放：`pending` / `approved` / `rejected` / `patch_only`

---

## 三、审批边界

### 不需要审批（只读操作）

读仪表盘、读 Agent 列表/详情、读 Task 列表/详情、读 Approval 列表/详情。

### 必须走审批（写入/变更操作）

- 创建/更新/删除 Task
- 创建/更新 Agent
- 变更 Agent 状态
- 文件写入
- Git checkpoint
- Runner 请求
- Agent 配置变更
- 模型相关准入

---

## 四、Rust 分层

```
apps/desktop/src-tauri/src/
  commands/     -- Tauri commands（前端 invoke 入口），只做参数接收和响应封装
  services/     -- 业务规则层：状态机、审批逻辑、权限判断
  db/           -- 数据访问层：CRUD、事务，通过 rusqlite 操作 SQLite
```

- `commands/` 不写业务逻辑，只调 `services/`
- `services/` 负责所有"能不能做"的判断
- `db/` 只做读写，不做判断

---

## 五、第一版 Tauri Commands

### projects
| command | 说明 |
|---------|------|
| `get_project` | 获取当前项目信息（已实现） |

### agents
| command | 说明 |
|---------|------|
| `list_agents` | Agent 列表（已实现，只读） |
| `create_agent` | 创建 Agent（需审批） |
| `update_agent_status` | 更新 Agent 状态（需审批） |

### tasks
| command | 说明 |
|---------|------|
| `list_tasks` | 任务列表（已实现，只读） |
| `create_task` | 创建任务（需审批） |
| `update_task_status` | 更新任务状态（需审批） |
| `assign_task_agent` | 分配 Agent（需审批） |

### approvals
| command | 说明 |
|---------|------|
| `list_approvals` | 审批列表（已实现，只读） |
| `create_approval` | 创建审批申请 |
| `approve_approval` | 批准 |
| `reject_approval` | 拒绝 |
| `patch_only_approval` | 仅生成补丁（不执行） |

### 暂不开放

Runner、Model Gateway、Agent Run、Agent 配置版本管理、Git checkpoint 当前不开放真实执行，只保留 helper-only / readonly。

---

## 六、数据流转

```
前端                    Rust                        SQLite
─────────────────────────────────────────────────────────
invoke("list_tasks") → commands::list_tasks()
                         → services::TaskService::list()
                           → db::TaskRepo::find_all()
                             → SELECT ...      → 返回行
                           ← Vec<Task>
                         ← 序列化为 JSON
← JSON                 ←
```

写入流转：

```
invoke("create_task",{...})
  → commands::create_task(params)
    → 参数校验
    → services::TaskService::create(params)
      → 业务规则检查（状态合法？agent 存在？）
      → 需要审批？
        → 是：创建 approval 记录，返回 pending
        → 否：db::TaskRepo::insert(task)
               → INSERT INTO tasks ...
             ← Task
    ← JSON
```

---

## 七、约束与禁区

- Runner 当前允许在受控 service 链路内自动执行并写入 `workspace/generated`；不得退化成自由命令、任意文件写入、文件删除或 Git push/commit
- 真实模型调用已通过 Model Gateway 和系统设置模型配置开放；不得绕过受控入口
- 禁止 UI/Agent/Runner 绕过 services 层直接访问 db 层
- 禁止导入 provider SDK、读取 raw key
- `AGENT_SWARM_ENABLE_MODEL_CONNECTIVITY_TEST` 不可复用为真实业务模型调用开关
- 旧 REST API 路由仅作为迁移参考，不继续扩展

---

## 八、与旧项目的关系

旧 `services/api/server.js`、`services/api/mock-data.js` 已冻结，保留在仓库作为：
- 业务规则参考（状态机流转语义）
- Mock 数据来源（seed 快照）
- 旧 REST endpoint 清单（迁移对照）

新架构不要求与新 REST API 一一对应，但核心业务语义（Task 状态流转、Approval 审批逻辑）必须保持一致。
