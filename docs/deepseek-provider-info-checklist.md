# DeepSeek Provider Information Checklist

Date: 2026-06-10

Purpose: record non-secret DeepSeek public documentation facts before any real DeepSeek manual connectivity request is implemented.

This checklist is documentation only. It does not enable provider SDKs, DeepSeek HTTP requests, feature flags, real model calls, Runner execution, cloud sync, or permission changes.

## Current Status

- Provider candidate: `deepseek`.
- Provider type: official DeepSeek API with OpenAI-compatible format.
- Real provider request: not implemented.
- Provider SDK: not imported.
- Feature flag activation: still inactive.
- Proposed future server env var name: `AGENT_SWARM_DEEPSEEK_API_KEY`.

## Public Documentation Facts

Source documents:

- `https://api-docs.deepseek.com/`
- `https://api-docs.deepseek.com/api/create-chat-completion`
- `https://api-docs.deepseek.com/api/list-models`

Non-secret facts gathered from official docs:

```text
Provider platform name: DeepSeek
Base URL shape: https://api.deepseek.com
Endpoint family: Chat Completions
Example endpoint path: POST /chat/completions
Authentication scheme: Authorization: Bearer <DEEPSEEK_API_KEY>
OpenAI-compatible format: yes
Streaming: optional; docs show stream=false and stream=true support
Example current model ids: deepseek-v4-flash, deepseek-v4-pro
Model list endpoint: GET /models
```

DeepSeek docs also mention `deepseek-chat` and `deepseek-reasoner` as compatibility names scheduled for deprecation on 2026-07-24 15:59 UTC. Do not use these as the first fixed connectivity-test model.

## Candidate Provider Boundary

DeepSeek must be treated as a distinct provider candidate from:

- Official OpenAI.
- Anthropic.
- Google Gemini.
- Unknown OpenAI-compatible relays.
- The earlier `openai_compat` relay candidate.

The future DeepSeek adapter should use a distinct provider id such as `deepseek` and should read only `AGENT_SWARM_DEEPSEEK_API_KEY` from server env.

The frontend must not send, store, or display the key.

## Minimal Ping Draft

The future manual connectivity test must stay backend-only, fixed, and manually triggered.

Allowed local API request shape:

```json
{
  "provider": "deepseek",
  "model": "deepseek-v4-flash",
  "purpose": "manual_connectivity_test",
  "secondConfirm": true,
  "confirmText": "I understand this will make one real provider connectivity request.",
  "requestedBy": "local_user"
}
```

Future fixed provider request should be a minimal non-stream Chat Completions request with:

- `stream=false`.
- No tools.
- No files.
- No Agent context.
- No Runner job id.
- No arbitrary headers.
- No arbitrary URL override.
- No client-provided API key.

The exact request body must be drafted and reviewed before implementation.

## Coarse Result Only

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
- Reasoning text.
- Token usage.
- Cost.
- Request or response headers.
- Raw API key.
- API key suffix.
- Masked key fragment.
- Raw provider error.
- Prompt or result text.

## Required Failure Paths Before Real Request

These paths must pass without a real key and without network requests:

- Feature disabled.
- Missing key.
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

Do not implement a real DeepSeek request until all of these are documented:

- Fixed model id.
- Exact minimal ping request body.
- Timeout limit.
- Response body limit.
- Coarse error mapping.
- Secret redaction verification.
- Provider body redaction verification.
- No-side-effect regression checks.

The real implementation must be a separate commit after this checklist and acceptance tests are updated.
