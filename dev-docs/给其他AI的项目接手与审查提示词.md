# 给其他 AI 的项目接手与审查提示词

用途：把下面整段提示词复制给另一个 AI，让它快速理解 `agent-swarm` 项目，并重点审查最近两次后端落地步骤是否正确。

---

## 可复制提示词

你现在接手一个本地多模型 AI Agent 编排桌面项目，请先审查，不要急着改代码。

项目路径：

```text
F:\Projects\agent-swarm
```

产品目标：

```text
用户目标
-> 总控 Agent 理解项目类型、技术栈、风险和阶段
-> 总控选择固定 AI 员工和项目专家
-> 每个 Agent 绑定执行器、模型、职责边界和 Skill
-> 模型调用经过模型网关
-> 写文件、高风险动作、Runner 执行都经过后端校验、审批和运行记录
-> 产物归档到 workspace/generated
```

当前优先级：

```text
先把核心功能接到 Tauri + Rust + SQLite，
再考虑记忆库。

现在正在做 P0：
AI 员工 / 模型 / 执行器 / Skill 配置从前端临时状态迁移到真实后端数据链路。
```

请先阅读这些文件：

```text
dev-docs/当前产品目标与落地路线.md
dev-docs/P0-AI员工模型执行器配置落地计划.md
docs/data-model-draft.md
docs/api-draft.md
data/migrations/017_add_agent_config_core.sql
apps/desktop/src-tauri/src/db/mod.rs
apps/desktop/src-tauri/src/services/agent_config.rs
apps/desktop/src-tauri/src/commands/agent_config.rs
apps/desktop/src-tauri/src/lib.rs
```

最近需要审查的提交：

```text
7a9ca09 feat: add agent config core schema
29d2815 feat: add agent config commands
```

请使用 code review 的方式检查这两次提交是否正确，优先找 bug、越界风险、数据模型问题、缺失测试和后续接前端会踩的坑。不要只夸实现。

审查重点：

1. `017_add_agent_config_core.sql` 是否符合 P0 目标。
2. migration 是否幂等，是否会破坏旧库。
3. `model_catalog.executor_key` 的新增方式是否安全。
4. seed 是否可重复运行，是否会污染用户已有配置。
5. 是否有 API Key、Token、raw prompt、raw response、raw provider error 进入 SQLite、日志或文档。
6. `executor_configs`、`agent_templates`、`project_agents`、`executor_skills`、`agent_boundary_checks` 的字段和索引是否够后续使用。
7. `agent_config` service 的输入校验是否足够，是否误拦正常字段，是否漏放危险字段。
8. 删除执行器、模型、模板、skill 的依赖保护是否合理。
9. `project_agents` 是否是软移除，是否避免断掉历史运行链路。
10. Tauri commands 是否都注册到了 `lib.rs`。
11. 前端下一步接 `desktopHost.ts` 时，命令形状是否足够清晰。
12. 测试是否覆盖了默认 seed、CRUD、模型增删、内置项保护、数据库初始化。

请运行或建议运行这些验证命令：

```powershell
cd F:\Projects\agent-swarm\apps\desktop\src-tauri
cargo fmt --check
cargo check
cargo test --lib

cd F:\Projects\agent-swarm\packages\ui
npm run typecheck
npm run build
```

已知工作区状态：

```text
workspace/generated 下有一批运行产物删除记录。
这些不是本次 P0 后端配置改动，不要误提交，除非用户明确要求。
```

项目硬边界：

```text
不要把浏览器预览当成真实桌面能力。
不要让前端直接伪造真实保存。
不要把密钥写进 SQLite、localStorage、日志或文档。
不要让 Agent 自由 shell、自动 git push、自动删除文件或写保护路径。
Runner 和高风险动作必须走后端校验、审批和运行记录。
```

请输出：

```text
1. Findings：按严重程度列出问题，带文件路径和具体原因。
2. Open Questions：不确定但需要确认的点。
3. Verdict：这两步是否能算 P0 阶段 2/3 合格。
4. Next Step：如果合格，下一步应如何把前端 AI 员工页接到 Tauri commands。
```

如果没有发现严重问题，也请明确说“未发现阻塞问题”，并列出剩余风险。

---

## 给用户看的简短结论

这个提示词的目的不是让别的 AI 重新做一遍，而是让它站在审查者角度检查：

- 我刚做的数据库底座是否合理。
- 我刚做的 Rust service / Tauri commands 是否真能支撑前端接入。
- 有没有偷偷绕过我们定好的安全边界。
- 下一步接前端前，有没有必须先修的坑。

如果审查通过，下一步就是 P0 阶段 4：把 `AI 员工` 页从本地假数据改成调用这些真实 Tauri commands。
