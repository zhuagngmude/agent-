# 阶段 38：多 Provider Adapter 设计

> 更新提示（2026-06-16）：本文件暂时顺延为后续“多 Provider Adapter”参考方案，不再作为当前阶段 38 执行。当前阶段 38 已调整为“项目类型分流与通用想法入口”，详见 `dev-docs/当前项目导航.md`。

## 目标

在阶段 35 模型目录、阶段 36 角色模型绑定、阶段 37 Skill 目录基础上，设计多 Provider Adapter 能力。

目标是让系统逐步支持：

```text
openai_compat
anthropic
gemini
ollama
lmstudio
```

阶段 38 第一版建议只做 adapter 架构和 OpenAI-compatible 现有实现迁移，不一次性接所有真实 provider。

## 为什么不能直接“所有模型都接”

不同模型类型差异很大：

- OpenAI-compatible：`/v1/chat/completions`
- Anthropic：messages API 格式不同。
- Gemini：请求和响应结构不同。
- Ollama / LM Studio：本地端口、模型列表、超时策略不同。
- Embedding：不是聊天补全。
- 图片 / 音频 / 视频：输入输出类型完全不同。

如果直接开放自由 provider/base URL/body/header，会破坏安全边界，并导致审计不可控。

## 当前边界

阶段 38 仍然禁止：

- 前端自由输入 raw key。
- 前端自由输入 raw base URL。
- 前端自由输入 provider adapter。
- 前端自由输入 header/body。
- 直接读取或展示 raw provider response。
- 把 raw provider error 落库。
- 让模型触发 Runner。
- 写用户项目文件。
- 改 Git。
- 发起未审计的网络请求。

## Adapter 抽象

建议定义内部 trait：

```rust
trait ModelProviderAdapter {
    fn provider_key(&self) -> &'static str;
    fn send_chat(
        &self,
        request: &ModelRequest,
        config: &ResolvedProviderConfig,
        timeout_secs: u64,
        max_response_bytes: u64,
    ) -> Result<ModelResponse, ProviderError>;
}
```

第一版实现：

```text
OpenAiCompatAdapter
```

后续再加：

```text
AnthropicAdapter
GeminiAdapter
OllamaAdapter
LmStudioAdapter
```

## Provider 配置来源

第一版仍只允许环境变量配置，不做前端 key/base URL 输入。

建议命名：

```text
AGENT_SWARM_OPENAI_COMPAT_API_KEY
AGENT_SWARM_OPENAI_COMPAT_BASE_URL

AGENT_SWARM_ANTHROPIC_API_KEY
AGENT_SWARM_GEMINI_API_KEY
AGENT_SWARM_OLLAMA_BASE_URL
AGENT_SWARM_LMSTUDIO_BASE_URL
```

阶段 38 不要求全部实现，只预留命名规则和 resolver 架构。

任何 provider config resolver 都必须：

- 不返回 raw key。
- 不返回 raw base URL 给前端。
- 不写 raw key/base URL 到 SQLite。
- 错误只返回粗粒度分类。

## 数据模型调整

阶段 35 的 `model_catalog.provider` 开始发挥作用。

允许 provider 白名单：

```text
openai_compat
anthropic
gemini
ollama
lmstudio
```

但第一版 enabled provider 仍建议只有 `openai_compat`。

如果新增 provider 配置表，必须单独设计，不要在阶段 38 直接存 raw key。

## Provider 能力声明

建议后续给模型目录增加能力字段，阶段 38 可以先写文档，不一定改表：

```text
supports_chat
supports_json_mode
supports_tool_calling
supports_vision
context_window
cost_tier
latency_tier
```

第一版不要依赖这些字段做复杂调度。

## 错误分类

所有 adapter 统一返回粗粒度错误：

```text
missing_key
missing_base_url
invalid_base_url
unsupported_provider
unsupported_model
timeout
network_error
provider_error
response_too_large
invalid_response
```

前端不能看到 raw provider status text、raw body、raw headers。

## 审计

`model_calls` 必须继续记录安全字段：

- provider
- model
- purpose
- status
- request_hash
- structured_summary 脱敏截断
- error_category
- redaction_applied

不能新增：

- raw prompt
- raw response
- raw error
- raw key
- raw base URL
- raw headers

## 前端

阶段 38 第一版不要做“新增 Provider”自由表单。

可以展示：

- provider 名称
- provider 是否配置完成
- provider 是否可用于当前模型目录

不能展示：

- key
- base URL
- raw config

如果做 provider 状态页面，状态只能是：

```text
configured
missing_key
missing_base_url
invalid_base_url
unsupported
```

## 测试要求

必须补 Rust 测试：

- adapter trait 可由 OpenAI-compatible 实现。
- unsupported provider 拒绝。
- disabled provider 不调用。
- provider config 缺 key 不调用。
- provider config 缺 base URL 不调用。
- invalid base URL 不调用。
- provider error 不泄露 raw body。
- network error 不泄露 raw transport error。
- timeout 不泄露 raw URL/key。
- model_calls 不包含 raw key/base URL/prompt/response/error。
- 不写 runtime_events。
- 不创建 tasks / approvals / runner_requests。

如果引入 Anthropic/Gemini/Ollama/LmStudio 任何一个真实 adapter，必须新增对应纯函数解析测试和 fake provider 测试，不得只靠人工试。

## 验证命令

```powershell
cd F:\projects\agent-swarm\apps\desktop\src-tauri
cargo fmt --check
cargo check
cargo test

cd F:\projects\agent-swarm\packages\ui
npm run typecheck
npm run build

cd F:\projects\agent-swarm
git diff --check
rg -n "Authorization: Bearer|sk-|raw key|raw base URL|raw prompt|raw response|raw provider" apps packages docs dev-docs
```

## 完成口径

阶段 38 完成后，系统具备多 Provider Adapter 架构。第一版可以只把现有 OpenAI-compatible provider 迁入 adapter 架构；后续 provider 必须逐个准入、逐个测试、逐个审计，不开放前端自由 provider/key/base URL/header/body。
