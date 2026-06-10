# Relay Provider Information Checklist

Date: 2026-06-10

Purpose: collect only non-secret relay documentation facts before any real OpenAI-compatible relay manual connectivity request is implemented.

This checklist is documentation only. It does not enable provider SDKs, relay HTTP requests, feature flags, real model calls, Runner execution, cloud sync, or permission changes.

## Current Status

- Provider candidate: `openai_compat`.
- Real relay request: not implemented.
- Provider SDK: not imported.
- Feature flag activation: still inactive.
- Server env var names:
  - `AGENT_SWARM_OPENAI_COMPAT_API_KEY`
  - `AGENT_SWARM_OPENAI_COMPAT_BASE_URL`

## Non-Secret Facts To Collect

Fill these from public relay documentation or a provider dashboard help page. Do not paste secrets.

```text
Relay platform name:
Public documentation URL, if any:
Base URL shape shown in docs, without account tokens:
Example endpoint path:
Example model name:
Authentication scheme shown in docs:
Endpoint family:
Streaming required or optional:
Any documented rate limit:
Any documented timeout guidance:
Any documented response size guidance:
```

## Operator-Provided Relay Facts

Source: relay administrator response provided by the local operator on 2026-06-10.

These are non-secret connection facts only. No API key, account id, dashboard data, prompt, model output, provider body, usage, cost, or raw error was provided or recorded.

```text
Relay platform name: cheng.pink OpenAI-compatible relay
Base URL shape: https://api.cheng.pink/v1
Base URL note: /v1 may be included or omitted
Recommended endpoint family: Chat Completions API
Primary endpoint path: /v1/chat/completions
Optional endpoint path: /v1/responses
Available model ids: gpt-5.5, gpt-5.4, gpt-5.4-mini
Minimal test model id: gpt-5.4-mini
Authentication scheme: Authorization: Bearer <API_KEY>
Streaming: supports stream=false and stream=true
```

Implementation note: the future adapter must normalize the base URL and endpoint path so it never builds a duplicated path such as `/v1/v1/chat/completions`.

## Endpoint Family

Choose one documented endpoint family before implementation:

- Chat Completions: `POST /v1/chat/completions`
- Responses API: `POST /v1/responses`
- Unknown

If the relay documents both endpoint families, default the first manual ping plan to Chat Completions unless the relay explicitly recommends otherwise.

## Base URL Requirements

The relay base URL must come only from `AGENT_SWARM_OPENAI_COMPAT_BASE_URL`.

Allowed shape:

- `https://.../v1`

Rejected shapes:

- API keys embedded in the URL.
- Query tokens or account secrets in the URL.
- `http://` URLs.
- `localhost`, loopback, or private IP targets.
- Request-body base URL overrides.
- Frontend-provided URLs.
- Unsafe redirects to non-HTTPS or private targets.

The API may report only whether a base URL is configured and has a safe shape. It must not return the actual base URL value.

## Model Name

The fixed model id must come from relay documentation or operator configuration.

Do not assume official OpenAI model names work through the relay.

## Minimal Ping Contract

The future manual connectivity test must stay backend-only, fixed, and manually triggered.

Allowed local API request shape:

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

Rejected request inputs:

- Free-form prompt.
- System prompt.
- User content.
- Agent context.
- Files.
- Tools or function calls.
- Runner job ids.
- Arbitrary headers.
- Arbitrary provider options.
- Arbitrary URLs.
- Client-provided API keys.

## Coarse Result Only

Allowed response fields:

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
- Request or response headers.
- Raw API key.
- API key suffix.
- Masked key fragment.
- Base URL value.
- Raw provider error.
- Prompt or result text.

## Required Failure Paths Before Real Request

These paths must pass without a real key and without network requests:

- Feature disabled.
- Missing key.
- Missing base URL.
- Invalid base URL.
- Unsupported provider.
- Unsupported model.
- Timeout simulation.
- Provider error simulation.
- All side effects false.

Required no-side-effect assertions:

- No SQLite write.
- No runtime-state write.
- No task creation.
- No approval creation.
- No Runner job creation.
- No Agent trigger.
- No Runner execution.
- No prompt or result logging.
- No provider response storage.

## Implementation Gate

Do not implement a real relay request until all of these are documented:

- Endpoint family.
- Fixed model id.
- Exact minimal ping request shape.
- Timeout limit.
- Response body limit.
- Coarse error mapping.
- Secret redaction verification.
- Provider body redaction verification.

The real implementation must be a separate commit after this checklist is filled with non-secret facts.
