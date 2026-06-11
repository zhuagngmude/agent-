# Model Gateway 正式入口设计

日期：2026-06-11

本文是阶段 2 的入口设计草案。它不启用真实模型调用，不导入 provider SDK，不读取真实 API Key，不发起 provider 网络请求，不建表，不写 Mock / SQLite，不开放 Runner 执行。

## 编码前五问

1. 真正要解决的问题是什么？

   固定未来真实模型调用只能走哪一个后端入口、怎样读取 provider 配置和密钥、怎样记录和脱敏，避免 UI、Agent、Runner 或任务状态机绕过 Model Gateway。

2. 谁创建、谁调用、谁消费，调用链是否闭合？

   用户在 Web UI 发起项目计划意图；API route 只接收业务字段；Model Gateway 构造固定请求；provider adapter 只由 Model Gateway 调用；redaction / response limiter 处理结果；`model_calls` 和 runtime event 记录脱敏状态；`project_plan` 审批草案消费结构化摘要。调用链闭合为：

   ```text
   Web UI intent
   -> project plan model route
   -> Model Gateway request builder
   -> provider config resolver
   -> provider adapter
   -> redaction / response limiter
   -> model_calls
   -> runtime_events
   -> project_plan approval draft
   ```

3. 现有系统里有没有已经承担同一职责的资源？

   已有 `services/api/model-gateway.js`、`services/api/model-gateway-adapters.js`、`services/api/model-gateway-project-plan.js`、`docs/data-model-draft.md` 中的 `model_calls` 草案、`scripts/verify-model-gateway.ps1` 和 `scripts/verify-real-model-admission.ps1`。后续只能扩展这些资源，不能另开自由模型调用入口。

4. 更简单或更保守的替代方案是什么？

   继续保持本地确定性项目计划模板，不引入真实模型。当前仍采用该保守方案；本文只补入口和密钥边界，让后续实现前有可审查契约。

5. 最大回归风险是什么，用哪些验证挡住？

   最大风险是前端传入 key、base URL、headers、prompt 或 provider body，或者真实模型结果直接创建任务 / Runner request。用 helper 字段拒绝、Model Gateway disabled 验证、真实模型准入验证、`model_calls` 脱敏约束和项目计划审批前不创建任务的回归验证挡住。

## 当前状态

- 已有 `model_calls` 结构草案。
- 已有 `project_plan_generation` helper-only 准入构造器。
- Model Gateway 仍是 disabled。
- `AGENT_SWARM_ENABLE_REAL_MODEL_PROJECT_PLAN` 当前只能被报告，不能激活真实调用。
- 本文只补正式入口设计和 provider 配置 / Key 存储方案。

## 第一条正式入口草案

候选本地 API route：

```text
POST /api/projects/:projectId/project-plan-model-requests
```

当前不实现该 route。进入实现时，它必须保持以下规则：

- 只允许 `purpose=project_plan_generation`。
- 只允许为 `projectId` 所属本地项目生成 `project_plan` 审批草案。
- feature flag 默认关闭时返回 `blocked / feature_disabled`。
- 即使 feature flag 开启，也不得在审批前创建任务或 Runner request。
- route 不得接受 API key、base URL、headers、provider body、prompt template、system prompt、stream、tools、files 或 Runner job id。
- route 不得由 Agent、Runner、后台任务或页面加载自动触发；第一版必须是用户明确动作触发。

允许的客户端请求字段：

```json
{
  "purpose": "project_plan_generation",
  "idea": "Build a local customer lead tracker",
  "constraints": "Mock/SQLite first; no Runner execution",
  "requestedBy": "local_user",
  "secondConfirm": true,
  "confirmText": "I understand this may make one backend model request."
}
```

说明：

- `projectId` 只来自 URL path。
- `provider`、`model`、timeout、response body limit、base URL 和 API key 都只能来自后端配置。
- `secondConfirm` 和 `confirmText` 是第一版真实业务调用的人工确认闸口；后续如要放宽，必须单独更新验收规格。

## 后端内部入口

未来 route 只能调用一个内部 Model Gateway 方法，例如：

```text
ModelGateway.createProjectPlanModelCall(input, context)
```

内部方法职责：

1. 验证固定业务请求形态。
2. 拒绝所有客户端控制的 provider/key/prompt/network 字段。
3. 解析后端 provider 配置。
4. 检查 feature flag、key、base URL、model 白名单、timeout 和 response body limit。
5. 创建或更新脱敏 `model_calls` 记录。
6. 只通过准入 provider adapter 发起请求。
7. 对 provider 响应做限长、结构化解析和脱敏。
8. 写入 runtime event。
9. 只生成 `project_plan` 审批草案。

禁止：

- UI 直接调用 provider adapter。
- Agent 直接调用 provider adapter。
- Runner 直接调用 provider adapter。
- 任务状态机直接调用 provider adapter。
- 任意模块绕过 Model Gateway 写 `model_calls`。

## Provider 选择

第一版只允许一个 provider 进入实现批次：

```text
provider=openai_compat
model=gpt-5.4-mini
```

原因：

- 已有 cheng.pink OpenAI-compatible relay 的非秘密连接事实。
- 已有固定 minimal ping 文档和 base URL 规范化草案。
- 已有 disabled adapter registry 元数据。

不在第一版做：

- DeepSeek 正式调用。
- 官方 OpenAI / Anthropic / Google 调用。
- 多 provider 自动 fallback。
- 前端 provider 选择器。
- 任意模型名输入。

## Provider 配置来源

阶段 2 第一版只允许服务端环境变量：

```text
AGENT_SWARM_OPENAI_COMPAT_API_KEY
AGENT_SWARM_OPENAI_COMPAT_BASE_URL
AGENT_SWARM_ENABLE_REAL_MODEL_PROJECT_PLAN=false
```

配置读取规则：

- API key 只在 API 进程内读取。
- base URL 只在 API 进程内读取。
- feature flag 默认 false。
- provider 和 model 来自后端白名单或后端配置，不来自客户端。
- API 状态接口只允许返回是否已配置、是否安全、env var 名称和粗粒度错误分类。

禁止返回或记录：

- API key 原文。
- key suffix。
- masked key fragment。
- base URL 原文。
- authorization header。
- provider request body。
- provider response body。

## Key 存储方案

阶段 2 第一版采用最保守方案：

```text
server env only
```

含义：

- 不建 `api_keys` 表。
- 不在 SQLite 保存 API key。
- 不在 Mock runtime state 保存 API key。
- 不在前端 localStorage / sessionStorage / IndexedDB 保存 API key。
- 不在日志、runtime events、model_calls、错误响应中保存 API key 或 key fragment。

后续如果要支持 UI 配置 key，必须先新增单独 Secret Store 设计，并满足：

- 只写后端安全存储。
- 写入后不返回原文。
- 读取接口只能返回 configured / missing / invalid。
- 本地优先使用 OS keychain 或等价方案。
- 如果使用加密文件，密钥来源不能和密文同处一个公开配置文件。
- 必须有 redaction 和泄漏扫描验证。

## Base URL 安全规则

`openai_compat` 第一版 base URL 只来自：

```text
AGENT_SWARM_OPENAI_COMPAT_BASE_URL
```

允许：

- `https://api.cheng.pink`
- `https://api.cheng.pink/v1`

拒绝：

- `http://`
- localhost、loopback、private IP。
- URL username/password。
- query token。
- fragment。
- 非 `/v1` 的任意 path。
- frontend-provided URL。

状态接口不得返回 base URL 原文，只能返回：

- `baseUrlConfigured`
- `baseUrlRequired`
- `baseUrlValid`
- `baseUrlErrorCategory`

## Model Call 记录写入边界

后续建表和写入时，`model_calls` 只能保存：

- provider id。
- model id。
- purpose。
- status。
- duration。
- token usage 粗粒度 JSON。
- cost estimate 粗粒度 JSON。
- error category。
- redaction flag。
- structured summary。
- 关联 approval / task / runtime event id。

不得保存：

- raw prompt。
- full prompt template。
- raw provider request。
- raw provider response。
- raw error。
- request / response headers。
- model reasoning text。
- key 或 key fragment。

## 返回契约草案

feature disabled / blocked 时：

```json
{
  "ok": false,
  "result": "blocked",
  "errorCategory": "feature_disabled",
  "purpose": "project_plan_generation",
  "realProviderRequestAttempted": false,
  "providerResponseStored": false,
  "redactionApplied": true
}
```

成功时只允许返回：

```json
{
  "ok": true,
  "result": "succeeded",
  "purpose": "project_plan_generation",
  "modelCallId": "model_call_...",
  "approvalId": "approval_...",
  "provider": "openai_compat",
  "model": "gpt-5.4-mini",
  "durationMs": 1200,
  "tokenUsage": {
    "promptTokens": 0,
    "completionTokens": 0,
    "totalTokens": 0
  },
  "costEstimate": {
    "amount": 0,
    "currency": "USD"
  },
  "redactionApplied": true
}
```

成功响应也不得包含 raw provider output；项目计划内容只能通过 `project_plan` 审批草案的结构化字段展示。

## 验收矩阵

进入真实 provider 请求实现前，必须至少覆盖：

- feature flag disabled。
- missing key。
- missing base URL。
- invalid base URL。
- unsupported provider。
- unsupported model。
- invalid purpose。
- forbidden client key field。
- forbidden client base URL field。
- forbidden client headers field。
- forbidden client prompt field。
- timeout simulation。
- provider error simulation。
- response body limit simulation。
- redaction applied。
- no raw key in response。
- no key suffix in response。
- no base URL value in response。
- no raw prompt stored。
- no provider body stored。
- no task created before approval。
- no Runner request created before approval。
- no Agent triggered。
- no Runner executed。
- no Git mutation。
- no project file write。

## 实现顺序

1. 保持现有 disabled Model Gateway 验证通过。
2. 保持 `model-gateway-project-plan.js` helper-only。
3. 增加 provider config resolver helper，但只返回 disabled / metadata，不读取或暴露 raw key。
4. 增加 route 层禁用态草案测试，仍返回 `feature_disabled`。
5. 增加 `model_calls` Mock / SQLite 记录写入设计和迁移草案。
6. 增加 redaction / response limiter helper。
7. 增加真实 provider adapter 实现，默认 feature flag 关闭。
8. 只在用户明确批准并提供本地 env 后，运行一次固定用途真实调用。
