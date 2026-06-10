# Runner 安全验收标准

日期：2026-06-09

阶段：MVP-0.2 到真实 Runner 执行前。

本文只定义本地 Runner 从只读 Mock 队列走向真实执行前必须满足的验收标准，不实现 Runner 执行代码，不放开任何本地写入、命令执行、网络请求或 Git 操作权限。

## 1. 当前边界

当前允许：

- 展示 Runner 状态。
- 展示只读 Runner job 队列。
- 审批通过后创建只读 `runner_job_*` 记录。
- 通过 Mock API 验证审批和状态流转。

当前禁止：

- Runner 写文件。
- Runner 删除文件。
- Runner 执行命令。
- Runner 发起网络请求。
- Runner 修改 Git。
- Runner 自己决定是否可以执行。
- Agent 通过“全权限”绕过 Approval Service、Runner job、Git checkpoint 或文件范围锁定。

只要本文任一 P0 条件未满足，真实 Runner 执行功能不得实现或开启。

## 2. 验收等级

```text
P0：上线真实 Runner 前必须满足，否则禁止执行。
P1：第一版真实 Runner 应满足，不满足必须有明确降级策略。
P2：后续增强项，不阻塞第一版真实 Runner。
```

## 3. P0 必须满足项

### P0-1 Approval Service 是唯一授权入口

要求：

- 所有本地写文件、删文件、执行命令、网络请求、Git 操作都必须先创建 Approval Request。
- Runner job 只能由已批准的 Runner 审批生成。
- Runner Service 不能自己创建审批、批准审批或绕过审批。
- `targetService=agent_config` 的审批不得生成 Runner job。
- Agent 权限 profile 不能绕过 Approval Service。即使是 `architect_admin` 或后续的 `all_agents_full_management`，也只能发起或建议审批，不能自批、自执行或直接创建可执行 Runner job。

验收方式：

- 代码中不存在 Runner 直接执行未审批 action 的入口。
- `approval.targetService !== "runner"` 时不得进入 Runner 执行队列。
- Mock 回归脚本继续验证 Agent 配置审批不会生成 Runner job。
- 权限映射必须遵守 `docs/agent-permission-contract.md`，不能把 `all=true` 当成执行授权。

### P0-2 审批必须包含完整执行计划

每个可执行审批必须包含：

- `operationTypes`：操作类型，例如 `file_write`、`file_delete`、`command_execute`、`network_request`、`git_operation`。
- `affectedFiles`：影响文件范围。
- `diffSummary` 或计划摘要。
- `diffPreview` 或补丁预览。
- `reason`：为什么需要执行。
- `requestAgentId`：发起 Agent。
- `riskLevel`：风险等级。

验收方式：

- 缺少操作类型、影响文件或原因时，审批不得进入 `approved`。
- 用户界面必须展示执行后果，而不是只显示“允许执行”按钮。

### P0-3 高风险操作必须二次确认

高风险操作包括：

- 写入或删除文件。
- 执行命令。
- 发起网络请求。
- 修改 Git。
- 修改 Runner、审批、权限、Agent 配置、API Key 相关代码。
- 修改超过 5 个文件。
- 修改 `.gitignore`、脚本、CI、启动器或安全文档。

要求：

- 高风险审批必须 `requiresSecondConfirm=true`。
- 批准请求必须包含 `secondConfirm=true`。
- 批准请求必须包含非空 `confirmText`。
- UI 文案必须说明执行后果。

验收方式：

- 缺少二次确认时 API 返回失败。
- 高风险审批不能通过普通单击按钮直接执行。

### P0-4 Git checkpoint 必须在执行前存在

要求：

- 真实 Runner 执行前必须创建 Git checkpoint。
- checkpoint 必须记录 commit hash。
- Runner job 必须关联 checkpoint。
- 当前工作区必须没有未解释的脏改动。

验收方式：

- `checkpoint.required=true` 且 `checkpoint.created=true`。
- `checkpoint.commit` 非空。
- 执行前保存 `git status --short` 快照。
- 如果存在非本次计划相关改动，必须阻止执行并要求用户处理。

### P0-5 文件范围必须锁定

要求：

- Runner 只能修改审批中列出的文件或目录范围。
- Runner 不得写入 `_internal/`、`design/image2/`、`data/mock/runtime-state.json`、日志、密钥文件或依赖目录。
- 删除文件必须单独列出，不能只写目录。
- 通配符范围必须展开并展示给用户确认。

验收方式：

- 执行前计算计划文件集合。
- 执行后 diff 文件集合必须是计划集合的子集。
- 出现计划外文件变更必须标记失败并进入人工审查。

### P0-6 命令执行必须白名单化

要求：

- 默认禁止命令执行。
- 允许执行的命令必须来自白名单。
- 命令必须有超时。
- 命令必须在项目工作区内运行。
- 命令输出必须脱敏。

第一版建议白名单：

```text
node --check <known-file>
powershell -ExecutionPolicy Bypass -File scripts/verify-mock-flows.ps1
git status --short
git diff --stat
```

明确禁止：

```text
rm -rf
del /s
format
curl | sh
Invoke-Expression
iex
git reset --hard
git clean -fd
git push --force
任何读取 _internal/ 的命令
任何上传密钥、日志、源码压缩包的命令
```

### P0-7 执行结果必须可审计

每次真实执行必须记录：

- Runner job ID。
- 来源审批 ID。
- 执行 Agent。
- 执行开始/结束时间。
- 执行命令或文件写入摘要。
- 执行前 checkpoint。
- 执行后 diff 摘要。
- 测试结果或未跑测试原因。
- 成功/失败状态。
- 错误原因。

验收方式：

- 执行结束后必须生成 runtime event 或执行日志。
- UI 可以查看来源审批、影响文件和结果摘要。

### P0-8 失败必须停止，不得自动扩大权限

要求：

- 执行失败后不得自动重试高风险操作。
- 不得自动扩大文件范围。
- 不得自动跳过测试。
- 不得自动创建新的高风险审批。
- 失败状态必须保留给用户审查。

验收方式：

- Runner job 失败后进入 `failed`。
- 失败原因可见。
- 需要继续执行时必须重新审批或用户明确确认。

## 4. 回滚前验收标准

真实回滚不是简单执行 `git reset`。

Agent 配置回滚也不是直接改数据库。Agent 配置真实写入前的 dry-run 和回滚准入见 `docs/agent-config-apply-dry-run-spec.md`；在该规格通过前，Agent 配置只能停留在 Mock 应用/取消状态流转。

回滚前必须满足：

- 有原始审批 ID。
- 有原始 Runner job ID。
- 有执行前 checkpoint。
- 有执行后 diff 摘要。
- 当前工作区状态已检查。
- 回滚影响文件范围已展示。
- 用户明确确认回滚。

禁止：

- 自动 `git reset --hard`。
- 自动 `git clean -fd`。
- 在存在用户未提交改动时回滚。
- 回滚 `_internal/`、密钥、日志或运行态文件。

推荐第一版回滚策略：

1. 先生成反向补丁或回滚计划。
2. 用户审查影响文件。
3. 创建新的回滚审批。
4. 用户二次确认。
5. Runner 只执行已批准的回滚 job。

Agent 配置回滚的差异：

1. 回滚必须创建新的 `agent_config` 审批。
2. 回滚不得创建 Runner job。
3. 回滚不得删除 `agent_config_versions` 历史。
4. 回滚应用后应写入新的版本记录，而不是覆盖旧版本。

## 5. P1 应满足项

### P1-1 任务锁

- 同一项目同一时间只允许一个真实 Runner 写操作。
- 读操作可以并发，但不得和写操作冲突。
- 锁必须有超时和人工释放方式。

### P1-2 文件所有权

- Runner 执行前声明计划修改文件。
- 执行期间其它 Agent 不得修改同一文件。
- 冲突时进入人工审查。

### P1-3 测试策略

- 每次执行后应运行相关检查。
- 如果无法运行测试，必须记录原因。
- 测试失败不得标记执行成功。

### P1-4 日志脱敏

日志中必须过滤：

```text
sk-
AIza
token=
password=
Authorization: Bearer
api_key
secret
```

## 6. P2 后续增强项

- 沙箱执行。
- 文件系统访问策略。
- 网络域名白名单。
- Runner 执行录屏或完整命令回放。
- 基于数据库的 runtime event 全量审计。
- 多 Runner 调度。
- 远程 Runner 心跳和能力协商。

## 7. Mock 阶段到真实 Runner 的迁移顺序

建议顺序：

1. 保持当前只读 Runner job 队列。
2. 增加 Runner job 安全检查字段，不执行。
3. 增加执行前 dry-run 校验。
4. 增加受限白名单命令执行，仅允许 `node --check` 和验证脚本。
5. 增加受限文件写入，仅允许审批列出的文件。
6. 增加执行日志和 runtime event。
7. 增加回滚审批流程。

不得跳过：

- Approval Service。
- 二次确认。
- Git checkpoint。
- 文件范围校验。
- 执行结果审计。

## 8. 第一版放行清单

真实 Runner 第一版可以放行前，必须全部回答“是”：

- 是否所有 Runner job 都来自 `approved` 审批？
- 是否高风险操作都需要二次确认？
- 是否有 Git checkpoint？
- 是否展示了影响文件？
- 是否锁定了文件范围？
- 是否禁止计划外文件变更？
- 是否有命令白名单和超时？
- 是否有执行日志和 diff 摘要？
- 是否失败即停止？
- 是否回滚也走审批？
- 是否不读取、不提交、不修改 `_internal/`？
- 是否不触碰 `design/image2/` 和 `data/mock/runtime-state.json`？

只要有一个答案是否定的，就继续停留在 Mock / dry-run 阶段。
