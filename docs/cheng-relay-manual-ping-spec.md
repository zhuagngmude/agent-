# Cheng Relay Manual Ping Spec

Date: 2026-06-10

Purpose: freeze the fixed minimal manual connectivity ping for the cheng.pink OpenAI-compatible relay before any real network implementation.

This spec is documentation only. It does not enable provider SDKs, relay HTTP requests, feature flags, real model calls, Runner execution, cloud sync, or permission changes.

## Current Status

- Provider id candidate: `openai_compat`.
- Relay candidate: cheng.pink OpenAI-compatible relay.
- Real relay request: not implemented.
- Provider SDK: not imported.
- Feature flag activation: still inactive.
- API key: not recorded and not required for this spec.
- Base URL env var: `AGENT_SWARM_OPENAI_COMPAT_BASE_URL`.
- API key env var: `AGENT_SWARM_OPENAI_COMPAT_API_KEY`.

## Fixed Local API Request

The local manual connectivity API request must stay fixed:

```json
{
  "provider": "openai_compat",
  "model": "gpt-5.4-mini",
  "purpose": "manual_connectivity_test",
  "secondConfirm": true,
  "confirmText": "I understand this will make one real provider connectivity request.",
  "requestedBy": "local_user"
}
```

Rejected request fields:

- API key.
- Base URL.
- Endpoint path.
- Prompt.
- System prompt.
- User content.
- Agent context.
- Files.
- Tools or function calls.
- Runner job id.
- Arbitrary headers.
- Arbitrary HTTP options.
- Stream setting from the client.

## Fixed Provider Request Draft

The future adapter may build exactly one non-stream Chat Completions request:

```json
{
  "model": "gpt-5.4-mini",
  "messages": [
    {
      "role": "user",
      "content": "ping"
    }
  ],
  "stream": false,
  "max_tokens": 1
}
```

This request body is a draft for implementation review. It must not be sent until a later implementation commit intentionally changes the feature flag boundary and passes the acceptance checks below.

Request-body rules:

- `model` must be fixed to `gpt-5.4-mini`.
- `stream` must be fixed to `false`.
- `messages` must be fixed by the backend.
- `max_tokens` must be fixed to the smallest practical value.
- No frontend text, Agent text, task content, file content, or Runner context may enter the request body.

## URL Normalization

The operator reported that the base URL may be configured with or without `/v1`.

Accepted env var shapes:

```text
https://api.cheng.pink
https://api.cheng.pink/v1
```

Canonical provider endpoint:

```text
https://api.cheng.pink/v1/chat/completions
```

Normalization rules:

- Trim trailing slashes from `AGENT_SWARM_OPENAI_COMPAT_BASE_URL`.
- If the normalized base URL ends with `/v1`, append `/chat/completions`.
- If the normalized base URL does not end with `/v1`, append `/v1/chat/completions`.
- Never build `/v1/v1/chat/completions`.
- Do not accept endpoint path overrides from the frontend or request body.
- Do not follow redirects to non-HTTPS, localhost, loopback, or private network targets.

Rejected base URL shapes:

- Missing value.
- Unparseable value.
- `http://...`.
- `https://localhost...`.
- `https://127.0.0.1...`.
- Private IPv4 targets.
- URL with query token or account secret.
- URL containing an API key.

## Timeout And Body Limits

Implementation must enforce these limits before any real request can be attempted:

- `timeoutMs <= 5000`.
- `responseBodyLimitBytes <= 4096`.

The adapter must stop reading the response body after the configured response limit. It must not store or return provider body content.

## Coarse Result Contract

Allowed local API response fields:

- `ok`
- `provider`
- `model`
- `result`
- `errorCategory`
- `durationMs`
- `realProviderRequestAttempted`
- `providerResponseStored`
- `redactionApplied`

Forbidden response or storage content:

- Provider response body.
- Model text.
- Token usage.
- Cost.
- Request headers.
- Response headers.
- Raw API key.
- API key suffix.
- Masked key fragment.
- Base URL value.
- Raw provider error.
- Prompt or result text.

## Error Mapping Draft

Allowed `result` values:

```text
blocked
missing_key
unsupported_provider
unsupported_model
timeout
provider_error
network_error
success
```

Allowed `errorCategory` values:

```text
feature_disabled
missing_key
missing_base_url
invalid_base_url
invalid_request
unsupported_provider
unsupported_model
timeout
provider_unavailable
network_error
unknown
```

Mapping rules:

- Feature flag inactive: `blocked / feature_disabled`.
- Missing `AGENT_SWARM_OPENAI_COMPAT_API_KEY`: `missing_key / missing_key`.
- Missing `AGENT_SWARM_OPENAI_COMPAT_BASE_URL`: `blocked / missing_base_url`.
- Unsafe base URL: `blocked / invalid_base_url`.
- Provider id not `openai_compat`: `blocked / unsupported_provider`.
- Model not `gpt-5.4-mini`: `blocked / unsupported_model`.
- Request timeout: `timeout / timeout`.
- Provider 4xx or 5xx without exposing body: `provider_error / provider_unavailable`.
- Network failure without exposing raw error: `network_error / network_error`.

## Acceptance Checks Before Real Request

These checks must pass without a real key and without provider network calls:

- Feature disabled.
- Missing key.
- Missing base URL.
- Invalid base URL.
- Base URL with `/v1`.
- Base URL without `/v1`.
- Unsupported provider.
- Unsupported model.
- Timeout simulation.
- Provider error simulation.
- All side effects false.

Required no-side-effect assertions:

- `realProviderRequestAttempted=false` while feature disabled or preflight blocked.
- `providerResponseStored=false`.
- `writesSqlite=false`.
- `writesRuntimeState=false`.
- `createsTasks=false`.
- `createsApprovals=false`.
- `createsRunnerJobs=false`.
- `triggersAgents=false`.
- `executesRunner=false`.
- `logsPromptOrResult=false`.
- `storesProviderResponse=false`.

## Implementation Order

1. Keep the current disabled relay adapter behavior.
2. Add deterministic request-builder tests without real network calls.
3. Verify URL normalization for base URLs with and without `/v1`.
4. Verify all failure paths and no-side-effect assertions without real credentials.
5. Only in a later separate commit, consider changing the feature flag boundary for one controlled real request.

## Still Forbidden

- Do not paste or commit API keys.
- Do not read keys from the frontend.
- Do not display keys, key suffixes, base URL values, prompts, or model outputs.
- Do not call the relay from UI page load, Agent runs, background jobs, or Runner jobs.
- Do not create tasks, approvals, Runner jobs, Agent runs, model-call records, billing records, or runtime events for this manual ping.
- Do not implement streaming for the first manual ping.
- Do not connect official OpenAI, Anthropic, Google Gemini, or DeepSeek as part of this cheng.pink step.
