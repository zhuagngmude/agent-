# MVP-0.4 验收记录

日期：2026-06-11

## 验收结论

MVP-0.4 已按当前文档主线收口完成。

当前可验收能力：

- MVP-0.3 链路保持稳定：项目想法 -> `project_plan` 审批 -> 5 个 queued 任务 -> 5 条只读 Runner request 记录。
- MVP-0.4 链路已闭合：execution request 审查视图、生命周期流转、runtime events 审计。
- Mock / SQLite 两种状态来源均可重复验证。
- Agent config 安全闭环保持受控。
- Model Gateway 和真实模型准入 helper 仍保持禁用态和无真实 provider 请求。

## 编码前五问记录

1. 真正要解决的问题是什么？

   收口 MVP-0.4 的验收状态，避免后续交接误判为“0.4 仍待实现”或误入 0.5 / 0.6 路线。

2. 谁创建、谁调用、谁消费？

   当前批次只创建文档验收记录。后续人类维护者和 AI 接手者消费该记录，用来判断当前阶段、验证入口和安全边界。

3. 现有系统里有没有已经承担同一职责的资源？

   已有 `docs/README.md`、`dev-docs/下一步开发路线.md`、`dev-docs/新窗口交接说明.md`、`dev-docs/应用真正可用落地计划.md` 和 `scripts/README.md`。本记录只做验收快照，不替代正式 API / 数据模型文档。

4. 更简单或更保守的替代方案是什么？

   只口头确认 0.4 已完成。但口头确认不可追踪，也无法把本次验证结果留给后续窗口，所以采用更可审计的文档记录。

5. 最大回归风险是什么，用哪些验证挡住？

   最大风险是误开放真实 Runner、真实模型调用、云同步或完整权限系统，或把当前主线误写成 0.5 / 0.6。通过 Mock / SQLite 回归、Agent config 安全闭环、Model Gateway 禁用态验证和真实模型准入 helper 验证来挡住。

## 本次验证

基准 commit：`6778107`

已通过：

```powershell
powershell -ExecutionPolicy Bypass -File scripts\verify-project-plan-flow.ps1
powershell -ExecutionPolicy Bypass -File scripts\verify-mock-flows.ps1
powershell -ExecutionPolicy Bypass -File scripts\verify-sqlite-flows.ps1
powershell -ExecutionPolicy Bypass -File scripts\verify-agent-config-safety-loop.ps1
powershell -ExecutionPolicy Bypass -File scripts\verify-model-gateway.ps1
powershell -ExecutionPolicy Bypass -File scripts\verify-real-model-admission.ps1
```

说明：

- `verify-model-gateway.ps1` 需要 `127.0.0.1:8787` 上有 API。本次使用临时 Mock API 进程运行验证，输出放在系统临时目录，未使用 `scripts/start-local.ps1`。
- 本次没有触碰 `data/local/`、`logs/`、`design/image2/`、`_internal/`、`.playwright-cli/` 或 `data/mock/runtime-state.json`。

## 仍然关闭的能力

- 不开放真实 Runner 执行。
- 不开放真实模型调用。
- 不做云同步。
- 不做完整权限系统。
- 不允许只读 Runner request 变成可执行命令。
- 不允许 Model Gateway 发起真实 provider 请求。

## 下一步入口

下一步不是补 MVP-0.4 功能，而是进入“真实可用前规划 / 准入设计”：

- 按 `dev-docs/应用真正可用落地计划.md` 的阶段顺序推进。
- 真实模型调用先按 `dev-docs/真实模型接入准入规格.md` 补模型调用记录结构草案和数据模型文档。
- Runner 永远最后开；在模型调用链、Agent Run 记录链和执行计划审查链稳定前，不开放真实执行。
