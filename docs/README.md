# docs

这里存放 `agent蜂群` 的正式技术文档、治理文档和验收文档。

## 先读这里

- [Agent宪法.md](./Agent宪法.md)：AI、Agent、Model Gateway、Runner 和审批链的中文治理总规则。
- [AI开发细则.md](./AI开发细则.md)：前端、数据库、Tauri/Rust、Git 和文档同步的执行细则。
- [api-draft.md](./api-draft.md)：当前 API 契约和阶段边界。
- [data-model-draft.md](./data-model-draft.md)：当前实体和数据表草案。
- [demo-checklist.md](./demo-checklist.md)：本地演示和验证清单。
- [runner-safety-acceptance.md](./runner-safety-acceptance.md)：Runner 安全契约。
- [module-stability-map.md](./module-stability-map.md)：保护区、契约区和可重构区说明。

## 当前功能规格

- [agent-config-apply-dry-run-spec.md](./agent-config-apply-dry-run-spec.md)
- [mvp-0.3-project-plan-flow-spec.md](./mvp-0.3-project-plan-flow-spec.md)
- [mvp-0.4-execution-layer-plan.md](./mvp-0.4-execution-layer-plan.md)
- [mvp-0.4-execution-layer-draft.md](./mvp-0.4-execution-layer-draft.md)
- [agent-permission-contract.md](./agent-permission-contract.md)

## Model Gateway 相关说明

- [relay-provider-info-checklist.md](./relay-provider-info-checklist.md)
- [cheng-relay-manual-ping-spec.md](./cheng-relay-manual-ping-spec.md)
- [deepseek-provider-info-checklist.md](./deepseek-provider-info-checklist.md)

## 平台说明

- [sqlite-seed-plan.md](./sqlite-seed-plan.md)
- [tech-stack-notes.md](./tech-stack-notes.md)
- [tauri-readonly-skeleton-acceptance.md](./tauri-readonly-skeleton-acceptance.md)：Tauri/Rust + SQLite 只读骨架正式验收。
- [write-commands-security-design.md](./write-commands-security-design.md)：教程 #11 在单机 Tauri 写入 commands 中的安全边界裁剪。
- [write-commands-design.md](./write-commands-design.md)：写入 commands 的参数、校验、返回值和副作用边界设计。

## 验证

验证脚本说明见 [../scripts/README.md](../scripts/README.md)。

当前阶段：MVP-0.4 已验收；阶段 2 真实模型调用准入设计已收口，只保留 helper-only scaffold；阶段 3 Agent Run 记录链已收口为本地 Mock / SQLite 流程。当前不开放真实 Runner、真实模型调用或云同步。
