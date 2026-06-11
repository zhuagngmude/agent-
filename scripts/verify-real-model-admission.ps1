$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
$apiScript = Join-Path $root "services\api\server.js"
$port = 8791
$baseUrl = "http://127.0.0.1:$port"
$projectId = "project_agent_swarm"

function Write-Step {
  param([string]$Message)
  Write-Host "[real-model-admission] $Message"
}

function Invoke-Json {
  param(
    [Parameter(Mandatory = $true)][string]$Method,
    [Parameter(Mandatory = $true)][string]$Path,
    [object]$Body = $null
  )

  $uri = "$baseUrl$Path"
  if ($null -eq $Body) {
    return Invoke-RestMethod -Method $Method -Uri $uri -TimeoutSec 5
  }

  return Invoke-RestMethod `
    -Method $Method `
    -Uri $uri `
    -TimeoutSec 5 `
    -ContentType "application/json" `
    -Body ($Body | ConvertTo-Json -Depth 20)
}

function Invoke-JsonExpectStatus {
  param(
    [Parameter(Mandatory = $true)][string]$Method,
    [Parameter(Mandatory = $true)][string]$Path,
    [Parameter(Mandatory = $true)][int]$ExpectedStatus,
    [object]$Body = $null
  )

  try {
    $result = Invoke-Json -Method $Method -Path $Path -Body $Body
    throw "Expected HTTP $ExpectedStatus but request succeeded: $($result | ConvertTo-Json -Depth 20)"
  } catch {
    $response = $_.Exception.Response
    if ($null -eq $response) {
      throw
    }

    $actualStatus = [int]$response.StatusCode
    if ($actualStatus -ne $ExpectedStatus) {
      throw "Expected HTTP $ExpectedStatus, got HTTP $actualStatus."
    }

    $raw = $_.ErrorDetails.Message
    if (-not $raw) {
      $reader = [System.IO.StreamReader]::new($response.GetResponseStream())
      $raw = $reader.ReadToEnd()
    }

    return $raw | ConvertFrom-Json
  }
}

function Assert-Equal {
  param(
    [object]$Actual,
    [object]$Expected,
    [string]$Message
  )

  if ($Actual -ne $Expected) {
    throw "$Message Expected '$Expected', got '$Actual'."
  }
}

function Assert-True {
  param(
    [bool]$Condition,
    [string]$Message
  )

  if (-not $Condition) {
    throw $Message
  }
}

function Assert-TextContains {
  param(
    [string]$Text,
    [string]$Needle,
    [string]$Message
  )

  if (-not $Text.Contains($Needle)) {
    throw "$Message Missing '$Needle'."
  }
}

function Assert-TextNotContains {
  param(
    [string]$Text,
    [string]$Needle,
    [string]$Message
  )

  if ($Text.Contains($Needle)) {
    throw "$Message Unexpected '$Needle'."
  }
}

function Assert-SideEffectsFalse {
  param(
    [Parameter(Mandatory = $true)][object]$SideEffects,
    [string]$Prefix
  )

  Assert-Equal $SideEffects.writesSqlite $false "$Prefix should not write SQLite."
  Assert-Equal $SideEffects.writesRuntimeState $false "$Prefix should not write runtime state."
  Assert-Equal $SideEffects.createsTasks $false "$Prefix should not create tasks."
  Assert-Equal $SideEffects.createsApprovals $false "$Prefix should not create approvals."
  Assert-Equal $SideEffects.createsRunnerJobs $false "$Prefix should not create Runner jobs."
  Assert-Equal $SideEffects.triggersAgents $false "$Prefix should not trigger Agents."
  Assert-Equal $SideEffects.callsRealModel $false "$Prefix should not call real models."
  Assert-Equal $SideEffects.executesRunner $false "$Prefix should not execute Runner."
  Assert-Equal $SideEffects.logsPromptOrResult $false "$Prefix should not log prompts or results."
  Assert-Equal $SideEffects.storesProviderResponse $false "$Prefix should not store provider responses."
  Assert-Equal $SideEffects.readsRawSecrets $false "$Prefix should not read raw secrets."
  Assert-Equal $SideEffects.returnsRawSecrets $false "$Prefix should not return raw secrets."
  Assert-Equal $SideEffects.makesNetworkRequests $false "$Prefix should not make network requests."
  Assert-Equal $SideEffects.modifiesGit $false "$Prefix should not modify Git."
  Assert-Equal $SideEffects.writesProjectFiles $false "$Prefix should not write project files."
}

function Assert-NoSideEffects {
  param(
    [Parameter(Mandatory = $true)][object]$Result,
    [string]$Prefix = "Project plan model request"
  )

  Assert-Equal $Result.realProviderRequestAttempted $false "$Prefix should not attempt provider requests."
  Assert-Equal $Result.providerResponseStored $false "$Prefix should not store provider responses."
  Assert-SideEffectsFalse -SideEffects $Result.sideEffects -Prefix $Prefix
}

function Assert-ProviderConfigSafe {
  param(
    [Parameter(Mandatory = $true)][object]$Result,
    [string]$Prefix = "Provider config resolver"
  )

  Assert-Equal $Result.ok $false "$Prefix should not be ok."
  Assert-Equal $Result.result "blocked" "$Prefix should remain blocked."
  Assert-Equal $Result.featureFlags.realModelProjectPlanActive $false "$Prefix should not activate real model calls."
  Assert-Equal $Result.featureFlags.realProviderRequestsAllowed $false "$Prefix should not allow provider requests."
  Assert-Equal $Result.configSource.providerSource "server_config" "$Prefix should use server provider config."
  Assert-Equal $Result.configSource.modelSource "server_config" "$Prefix should use server model config."
  Assert-Equal $Result.configSource.acceptsClientProvider $false "$Prefix should not accept client provider."
  Assert-Equal $Result.configSource.acceptsClientModel $false "$Prefix should not accept client model."
  Assert-Equal $Result.configSource.acceptsClientApiKey $false "$Prefix should not accept client API keys."
  Assert-Equal $Result.configSource.acceptsClientBaseUrl $false "$Prefix should not accept client base URLs."
  Assert-Equal $Result.configSource.acceptsClientHeaders $false "$Prefix should not accept client headers."
  Assert-Equal $Result.configSource.acceptsClientProviderBody $false "$Prefix should not accept client provider bodies."
  Assert-Equal $Result.keyStatus.valueReturned $false "$Prefix should not return raw key values."
  Assert-Equal $Result.keyStatus.suffixReturned $false "$Prefix should not return key suffixes."
  Assert-Equal $Result.keyStatus.maskedFragmentReturned $false "$Prefix should not return masked key fragments."
  Assert-Equal $Result.baseUrlStatus.valueReturned $false "$Prefix should not return raw base URLs."
  Assert-Equal $Result.baseUrlStatus.normalizedValueReturned $false "$Prefix should not return normalized base URLs."
  Assert-Equal $Result.baseUrlStatus.endpointUrlReturned $false "$Prefix should not return endpoint URLs."
  Assert-Equal $Result.adapterPolicy.mode "disabled" "$Prefix adapter should remain disabled."
  Assert-SideEffectsFalse -SideEffects $Result.sideEffects -Prefix $Prefix
}

function Assert-RequestPolicyLocked {
  param(
    [Parameter(Mandatory = $true)][object]$Result,
    [string]$Prefix = "Project plan model request"
  )

  Assert-Equal $Result.requestPolicy.providerSource "server_config" "$Prefix provider should be server-configured."
  Assert-Equal $Result.requestPolicy.modelSource "server_config" "$Prefix model should be server-configured."
  Assert-Equal $Result.requestPolicy.acceptsClientApiKey $false "$Prefix should not accept client API keys."
  Assert-Equal $Result.requestPolicy.acceptsClientBaseUrl $false "$Prefix should not accept client base URLs."
  Assert-Equal $Result.requestPolicy.acceptsClientHeaders $false "$Prefix should not accept client headers."
  Assert-Equal $Result.requestPolicy.acceptsClientProviderBody $false "$Prefix should not accept client provider bodies."
  Assert-Equal $Result.requestPolicy.acceptsFreeFormPrompt $false "$Prefix should not accept free-form prompts."
  Assert-Equal $Result.requestPolicy.acceptsSystemPrompt $false "$Prefix should not accept system prompts."
  Assert-Equal $Result.requestPolicy.acceptsStreamSetting $false "$Prefix should not accept stream settings."
  Assert-Equal $Result.requestPolicy.acceptsFiles $false "$Prefix should not accept files."
  Assert-Equal $Result.requestPolicy.acceptsToolCalls $false "$Prefix should not accept tool calls."
  Assert-Equal $Result.requestPolicy.acceptsRunnerJob $false "$Prefix should not accept Runner jobs."
  Assert-Equal $Result.outputTarget.createsTasksBeforeApproval $false "$Prefix should not create tasks before approval."
  Assert-Equal $Result.outputTarget.createsRunnerRequestsBeforeApproval $false "$Prefix should not create Runner requests before approval."
  Assert-Equal $Result.outputTarget.storesRawProviderResponse $false "$Prefix should not store raw provider responses."
  Assert-Equal $Result.outputTarget.logsPromptOrResult $false "$Prefix should not log prompts or results."
}

function Assert-RedactionSideEffects {
  param(
    [Parameter(Mandatory = $true)][object]$Result,
    [string]$Prefix = "Model Gateway redaction"
  )

  Assert-SideEffectsFalse -SideEffects $Result.sideEffects -Prefix $Prefix
}

function Assert-NoRawSecretsInJson {
  param(
    [Parameter(Mandatory = $true)][object]$Value,
    [string]$Prefix = "Response"
  )

  $json = $Value | ConvertTo-Json -Depth 30
  Assert-TextNotContains $json "sk-provider-secret" "$Prefix should not expose provider secret."
  Assert-TextNotContains $json "sk-route-secret" "$Prefix should not expose route secret."
  Assert-TextNotContains $json "sk-redaction-secret" "$Prefix should not expose redaction secret."
  Assert-TextNotContains $json "sk-record-secret" "$Prefix should not expose record secret."
  Assert-TextNotContains $json "https://api.cheng.pink" "$Prefix should not expose raw base URL."
  Assert-TextNotContains $json "Bearer route-token" "$Prefix should not expose bearer tokens."
}

function Test-ApiReady {
  try {
    $health = Invoke-Json -Method "GET" -Path "/api/health"
    return $health.ok -eq $true
  } catch {
    return $false
  }
}

function Get-StateCounts {
  $tasks = Invoke-Json -Method "GET" -Path "/api/projects/$projectId/tasks"
  $approvals = Invoke-Json -Method "GET" -Path "/api/projects/$projectId/approvals"
  $runnerJobs = Invoke-Json -Method "GET" -Path "/api/projects/$projectId/runner/jobs"
  $runtimeEvents = Invoke-Json -Method "GET" -Path "/api/projects/$projectId/runtime-events"

  return [pscustomobject]@{
    Tasks = @($tasks.tasks).Count
    Approvals = @($approvals.approvals).Count
    RunnerJobs = @($runnerJobs.jobs).Count
    RuntimeEvents = @($runtimeEvents.events).Count
  }
}

Push-Location $root
try {
  Write-Step "Load helper validation cases."
  $casesJson = node -e @'
const { buildProjectPlanGenerationModelRequest } = require('./services/api/model-gateway-project-plan');
const { resolveModelGatewayProviderConfig } = require('./services/api/model-gateway-provider-config');
const { buildSafeModelCallRecordDraft, redactModelGatewayText } = require('./services/api/model-gateway-redaction');

const base = {
  projectId: 'project_agent_swarm',
  purpose: 'project_plan_generation',
  idea: 'Build a local customer lead tracker',
  constraints: 'Mock/SQLite first; no Runner execution',
  requestedBy: 'verify_real_model_admission'
};

const keyEnv = 'AGENT_SWARM_OPENAI_COMPAT_API_KEY';
const baseUrlEnv = 'AGENT_SWARM_OPENAI_COMPAT_BASE_URL';
const flagEnv = 'AGENT_SWARM_ENABLE_REAL_MODEL_PROJECT_PLAN';
const providerEnv = {
  [keyEnv]: 'sk-provider-secret',
  [baseUrlEnv]: 'https://api.cheng.pink/v1',
  [flagEnv]: 'true'
};
const missingKeyEnv = {
  [baseUrlEnv]: 'https://api.cheng.pink/v1'
};
const missingBaseUrlEnv = {
  [keyEnv]: 'sk-provider-secret'
};
const invalidBaseUrlEnv = {
  [keyEnv]: 'sk-provider-secret',
  [baseUrlEnv]: 'http://127.0.0.1:8787/v1'
};

const previousFlag = process.env[flagEnv];
delete process.env[flagEnv];
const cases = {
  featureDisabled: buildProjectPlanGenerationModelRequest(base, { providerConfigOptions: { env: providerEnv } }),
  missingIdea: buildProjectPlanGenerationModelRequest({ ...base, idea: '' }, { providerConfigOptions: { env: providerEnv } }),
  invalidPurpose: buildProjectPlanGenerationModelRequest({ ...base, purpose: 'chat_completion' }, { providerConfigOptions: { env: providerEnv } }),
  forbiddenApiKey: buildProjectPlanGenerationModelRequest({ ...base, apiKey: 'sk-local-test' }, { providerConfigOptions: { env: providerEnv } }),
  forbiddenBaseUrl: buildProjectPlanGenerationModelRequest({ ...base, baseUrl: 'https://relay.example.test/v1' }, { providerConfigOptions: { env: providerEnv } }),
  forbiddenHeaders: buildProjectPlanGenerationModelRequest({ ...base, headers: { Authorization: 'Bearer test' } }, { providerConfigOptions: { env: providerEnv } }),
  forbiddenPrompt: buildProjectPlanGenerationModelRequest({ ...base, prompt: 'use this free-form prompt' }, { providerConfigOptions: { env: providerEnv } }),
  forbiddenProviderBody: buildProjectPlanGenerationModelRequest({ ...base, providerRequestBody: { messages: [{ role: 'user', content: 'hello' }] } }, { providerConfigOptions: { env: providerEnv } }),
  forbiddenRunnerJob: buildProjectPlanGenerationModelRequest({ ...base, runnerJobId: 'runner_job_1' }, { providerConfigOptions: { env: providerEnv } }),
  forbiddenSecretValue: buildProjectPlanGenerationModelRequest({ ...base, constraints: 'api_key=abc123' }, { providerConfigOptions: { env: providerEnv } }),
  providerConfigs: {
    validSafeDisabled: resolveModelGatewayProviderConfig({ provider: 'openai_compat', model: 'gpt-5.4-mini', purpose: 'project_plan_generation' }, { env: providerEnv }),
    missingKey: resolveModelGatewayProviderConfig({ provider: 'openai_compat', model: 'gpt-5.4-mini', purpose: 'project_plan_generation' }, { env: missingKeyEnv }),
    missingBaseUrl: resolveModelGatewayProviderConfig({ provider: 'openai_compat', model: 'gpt-5.4-mini', purpose: 'project_plan_generation' }, { env: missingBaseUrlEnv }),
    invalidBaseUrl: resolveModelGatewayProviderConfig({ provider: 'openai_compat', model: 'gpt-5.4-mini', purpose: 'project_plan_generation' }, { env: invalidBaseUrlEnv }),
    unsupportedProvider: resolveModelGatewayProviderConfig({ provider: 'deepseek', model: 'gpt-5.4-mini', purpose: 'project_plan_generation' }, { env: providerEnv }),
    unsupportedModel: resolveModelGatewayProviderConfig({ provider: 'openai_compat', model: 'not-supported', purpose: 'project_plan_generation' }, { env: providerEnv }),
    invalidPurpose: resolveModelGatewayProviderConfig({ provider: 'openai_compat', model: 'gpt-5.4-mini', purpose: 'chat_completion' }, { env: providerEnv })
  },
  redaction: {
    mixed: redactModelGatewayText('Authorization: Bearer route-token\n{"api_key":"sk-redaction-secret","url":"https://api.cheng.pink/v1","password":"hunter2","token":"abc"}', { limitBytes: 4096 }),
    objectValue: redactModelGatewayText({ api_key: 'sk-redaction-secret', nested: { authorization: 'Bearer route-token', url: 'https://api.cheng.pink/v1' } }, { limitBytes: 4096 }),
    truncated: redactModelGatewayText(`safe prefix ${'x'.repeat(200)}`, { limitBytes: 32 }),
    recordDraft: buildSafeModelCallRecordDraft({
      provider: 'openai_compat',
      model: 'gpt-5.4-mini',
      purpose: 'project_plan_generation',
      status: 'blocked',
      errorCategory: 'feature_disabled',
      durationMs: 12,
      tokenUsage: { prompt_tokens: 1, completion_tokens: 2, total_tokens: 3 },
      costEstimate: { amount: 0.01, currency: 'usd' },
      structuredSummary: 'Plan summary sk-record-secret https://api.cheng.pink/v1 Authorization: Bearer route-token',
      summaryLimitBytes: 128
    })
  }
};
process.env[flagEnv] = 'true';
cases.flagRequested = buildProjectPlanGenerationModelRequest(base, { providerConfigOptions: { env: providerEnv } });
if (previousFlag === undefined) {
  delete process.env[flagEnv];
} else {
  process.env[flagEnv] = previousFlag;
}

process.stdout.write(JSON.stringify(cases));
'@
  $cases = $casesJson | ConvertFrom-Json

  Write-Step "Verify valid future request shape remains feature-disabled."
  Assert-Equal $cases.featureDisabled.ok $false "Project plan model helper should not be ok."
  Assert-Equal $cases.featureDisabled.result "blocked" "Project plan model helper should be blocked."
  Assert-Equal $cases.featureDisabled.errorCategory "feature_disabled" "Project plan model helper should report feature disabled."
  Assert-Equal $cases.featureDisabled.requestShape "project_plan_generation_v1" "Project plan model helper should expose the fixed request shape."
  Assert-Equal $cases.featureDisabled.requestValid $true "Base project plan model request should be valid."
  Assert-Equal $cases.featureDisabled.featureFlags.realModelProjectPlanActive $false "Project plan model flag should not be active."
  Assert-Equal $cases.featureDisabled.featureFlags.realProviderRequestsAllowed $false "Project plan model helper should not allow provider requests."
  Assert-TextContains (@($cases.featureDisabled.blockingCategories) -join "`n") "feature_disabled" "Base case should report feature disabled."
  Assert-NoSideEffects -Result $cases.featureDisabled -Prefix "Base project plan model helper"
  Assert-RequestPolicyLocked -Result $cases.featureDisabled -Prefix "Base project plan model helper"
  Assert-ProviderConfigSafe -Result $cases.featureDisabled.providerConfig -Prefix "Base project plan provider config"
  Assert-NoRawSecretsInJson -Value $cases.featureDisabled -Prefix "Base project plan helper"

  Write-Step "Verify validation failures."
  Assert-Equal $cases.missingIdea.requestValid $false "Missing idea should be invalid."
  Assert-Equal $cases.missingIdea.errorCategory "invalid_request" "Missing idea should be invalid_request."
  Assert-TextContains (@($cases.missingIdea.validationErrors) -join "`n") "idea is required." "Missing idea should report validation error."
  Assert-NoSideEffects -Result $cases.missingIdea -Prefix "Missing idea case"

  Assert-Equal $cases.invalidPurpose.requestValid $false "Invalid purpose should be invalid."
  Assert-TextContains (@($cases.invalidPurpose.validationErrors) -join "`n") "purpose must be project_plan_generation." "Invalid purpose should report validation error."
  Assert-NoSideEffects -Result $cases.invalidPurpose -Prefix "Invalid purpose case"

  Write-Step "Verify forbidden client-controlled fields."
  Assert-Equal $cases.forbiddenApiKey.requestValid $false "Client API key field should be invalid."
  Assert-TextContains (@($cases.forbiddenApiKey.validationErrors) -join "`n") "apiKey" "Client API key field should be reported."
  Assert-Equal $cases.forbiddenBaseUrl.requestValid $false "Client base URL field should be invalid."
  Assert-TextContains (@($cases.forbiddenBaseUrl.validationErrors) -join "`n") "baseUrl" "Client base URL field should be reported."
  Assert-Equal $cases.forbiddenHeaders.requestValid $false "Client headers should be invalid."
  Assert-TextContains (@($cases.forbiddenHeaders.validationErrors) -join "`n") "headers" "Client headers should be reported."
  Assert-Equal $cases.forbiddenPrompt.requestValid $false "Client prompt should be invalid."
  Assert-TextContains (@($cases.forbiddenPrompt.validationErrors) -join "`n") "prompt" "Client prompt should be reported."
  Assert-Equal $cases.forbiddenProviderBody.requestValid $false "Client provider body should be invalid."
  Assert-TextContains (@($cases.forbiddenProviderBody.validationErrors) -join "`n") "providerRequestBody" "Client provider body should be reported."
  Assert-Equal $cases.forbiddenRunnerJob.requestValid $false "Runner job input should be invalid."
  Assert-TextContains (@($cases.forbiddenRunnerJob.validationErrors) -join "`n") "runnerJobId" "Runner job field should be reported."
  Assert-Equal $cases.forbiddenSecretValue.requestValid $false "Secret-like values should be invalid."
  Assert-TextContains (@($cases.forbiddenSecretValue.validationErrors) -join "`n") "API key" "Secret-like value should be reported."

  Assert-NoSideEffects -Result $cases.forbiddenApiKey -Prefix "Forbidden API key case"
  Assert-NoSideEffects -Result $cases.forbiddenHeaders -Prefix "Forbidden headers case"
  Assert-NoSideEffects -Result $cases.forbiddenProviderBody -Prefix "Forbidden provider body case"
  Assert-NoRawSecretsInJson -Value $cases.forbiddenApiKey -Prefix "Forbidden API key helper"

  Write-Step "Verify provider config resolver cases."
  Assert-ProviderConfigSafe -Result $cases.providerConfigs.validSafeDisabled -Prefix "Valid provider config"
  Assert-Equal $cases.providerConfigs.validSafeDisabled.providerSupported $true "Valid provider should be supported."
  Assert-Equal $cases.providerConfigs.validSafeDisabled.keyStatus.configured $true "Valid provider config should report configured key."
  Assert-Equal $cases.providerConfigs.validSafeDisabled.baseUrlStatus.configured $true "Valid provider config should report configured base URL."
  Assert-Equal $cases.providerConfigs.validSafeDisabled.baseUrlStatus.valid $true "Valid provider config should report valid base URL."
  Assert-Equal $cases.providerConfigs.validSafeDisabled.errorCategory "feature_disabled" "Valid provider config should still be feature-disabled."
  Assert-TextContains (@($cases.providerConfigs.validSafeDisabled.blockingCategories) -join "`n") "feature_disabled" "Valid provider config should report feature disabled."
  Assert-NoRawSecretsInJson -Value $cases.providerConfigs.validSafeDisabled -Prefix "Valid provider config"

  Assert-ProviderConfigSafe -Result $cases.providerConfigs.missingKey -Prefix "Missing-key provider config"
  Assert-Equal $cases.providerConfigs.missingKey.errorCategory "missing_key" "Missing-key provider config should report missing_key."
  Assert-Equal $cases.providerConfigs.missingKey.keyStatus.configured $false "Missing-key provider config should report key missing."
  Assert-TextContains (@($cases.providerConfigs.missingKey.blockingCategories) -join "`n") "missing_key" "Missing-key provider config should block on missing key."

  Assert-ProviderConfigSafe -Result $cases.providerConfigs.missingBaseUrl -Prefix "Missing-base-url provider config"
  Assert-Equal $cases.providerConfigs.missingBaseUrl.errorCategory "missing_base_url" "Missing-base-url provider config should report missing_base_url."
  Assert-Equal $cases.providerConfigs.missingBaseUrl.baseUrlStatus.configured $false "Missing-base-url provider config should report base URL missing."
  Assert-TextContains (@($cases.providerConfigs.missingBaseUrl.blockingCategories) -join "`n") "missing_base_url" "Missing-base-url provider config should block on missing base URL."

  Assert-ProviderConfigSafe -Result $cases.providerConfigs.invalidBaseUrl -Prefix "Invalid-base-url provider config"
  Assert-Equal $cases.providerConfigs.invalidBaseUrl.errorCategory "invalid_base_url" "Invalid-base-url provider config should report invalid_base_url."
  Assert-Equal $cases.providerConfigs.invalidBaseUrl.baseUrlStatus.valid $false "Invalid-base-url provider config should reject unsafe base URL."
  Assert-TextContains (@($cases.providerConfigs.invalidBaseUrl.blockingCategories) -join "`n") "invalid_base_url" "Invalid-base-url provider config should block on invalid base URL."

  Assert-ProviderConfigSafe -Result $cases.providerConfigs.unsupportedProvider -Prefix "Unsupported-provider config"
  Assert-Equal $cases.providerConfigs.unsupportedProvider.errorCategory "unsupported_provider" "Unsupported-provider config should report unsupported_provider."
  Assert-Equal $cases.providerConfigs.unsupportedProvider.providerSupported $false "Unsupported provider should not be supported."

  Assert-ProviderConfigSafe -Result $cases.providerConfigs.unsupportedModel -Prefix "Unsupported-model config"
  Assert-Equal $cases.providerConfigs.unsupportedModel.errorCategory "unsupported_model" "Unsupported-model config should report unsupported_model."
  Assert-Equal $cases.providerConfigs.unsupportedModel.modelSupported $false "Unsupported model should not be supported."

  Assert-ProviderConfigSafe -Result $cases.providerConfigs.invalidPurpose -Prefix "Invalid-purpose provider config"
  Assert-Equal $cases.providerConfigs.invalidPurpose.errorCategory "invalid_request" "Invalid-purpose provider config should report invalid_request."
  Assert-TextContains (@($cases.providerConfigs.invalidPurpose.validationErrors) -join "`n") "purpose is not supported" "Invalid-purpose provider config should report purpose error."

  Write-Step "Verify redaction and safe model-call record draft."
  Assert-Equal $cases.redaction.mixed.redactionApplied $true "Mixed redaction should report redaction."
  Assert-TextContains $cases.redaction.mixed.text "[REDACTED_SECRET]" "Mixed redaction should redact secrets."
  Assert-TextContains $cases.redaction.mixed.text "[REDACTED_URL]" "Mixed redaction should redact URLs."
  Assert-TextNotContains $cases.redaction.mixed.text "sk-redaction-secret" "Mixed redaction should not expose API key."
  Assert-TextNotContains $cases.redaction.mixed.text "route-token" "Mixed redaction should not expose bearer token."
  Assert-TextNotContains $cases.redaction.mixed.text "https://api.cheng.pink" "Mixed redaction should not expose URL."
  Assert-RedactionSideEffects -Result $cases.redaction.mixed -Prefix "Mixed redaction"

  Assert-Equal $cases.redaction.objectValue.redactionApplied $true "Object redaction should report redaction."
  Assert-TextNotContains $cases.redaction.objectValue.text "sk-redaction-secret" "Object redaction should not expose API key."
  Assert-TextNotContains $cases.redaction.objectValue.text "route-token" "Object redaction should not expose bearer token."
  Assert-TextNotContains $cases.redaction.objectValue.text "https://api.cheng.pink" "Object redaction should not expose URL."
  Assert-RedactionSideEffects -Result $cases.redaction.objectValue -Prefix "Object redaction"

  Assert-Equal $cases.redaction.truncated.truncated $true "Long response should be truncated."
  Assert-Equal $cases.redaction.truncated.responseBodyLimitExceeded $true "Long response should report body limit exceeded."
  Assert-Equal $cases.redaction.truncated.limitBytes 32 "Long response should respect the configured byte limit."
  Assert-RedactionSideEffects -Result $cases.redaction.truncated -Prefix "Truncated redaction"

  Assert-Equal $cases.redaction.recordDraft.modelCallRecordReady $false "Safe record draft should not be write-ready."
  Assert-Equal $cases.redaction.recordDraft.canWrite $false "Safe record draft should not write."
  Assert-Equal $cases.redaction.recordDraft.storesRawPrompt $false "Safe record draft should not store raw prompt."
  Assert-Equal $cases.redaction.recordDraft.storesRawProviderRequest $false "Safe record draft should not store raw provider request."
  Assert-Equal $cases.redaction.recordDraft.storesRawProviderResponse $false "Safe record draft should not store raw provider response."
  Assert-Equal $cases.redaction.recordDraft.storesRawProviderError $false "Safe record draft should not store raw provider error."
  Assert-Equal $cases.redaction.recordDraft.storesRequestHeaders $false "Safe record draft should not store request headers."
  Assert-Equal $cases.redaction.recordDraft.storesResponseHeaders $false "Safe record draft should not store response headers."
  Assert-Equal $cases.redaction.recordDraft.storesModelReasoning $false "Safe record draft should not store model reasoning."
  Assert-Equal $cases.redaction.recordDraft.storesKeyMaterial $false "Safe record draft should not store key material."
  Assert-Equal $cases.redaction.recordDraft.providerResponseStored $false "Safe record draft should not store provider response."
  Assert-TextNotContains $cases.redaction.recordDraft.structuredSummary "sk-record-secret" "Safe record draft should redact API key."
  Assert-TextNotContains $cases.redaction.recordDraft.structuredSummary "route-token" "Safe record draft should redact bearer token."
  Assert-TextNotContains $cases.redaction.recordDraft.structuredSummary "https://api.cheng.pink" "Safe record draft should redact URL."
  Assert-RedactionSideEffects -Result $cases.redaction.recordDraft -Prefix "Safe record draft"
  Assert-NoRawSecretsInJson -Value $cases.redaction -Prefix "Redaction helpers"

  Write-Step "Verify feature flag request cannot activate real provider calls."
  Assert-Equal $cases.flagRequested.featureFlags.realModelProjectPlanRequested $true "Feature flag request should be reported."
  Assert-Equal $cases.flagRequested.featureFlags.realModelProjectPlanActive $false "Feature flag request should not activate model calls."
  Assert-Equal $cases.flagRequested.featureFlags.realProviderRequestsAllowed $false "Feature flag request should not allow provider calls."
  Assert-Equal $cases.flagRequested.errorCategory "feature_disabled" "Feature flag request should remain feature-disabled."
  Assert-NoSideEffects -Result $cases.flagRequested -Prefix "Feature flag requested case"

  Write-Step "Start isolated API and verify disabled route draft."
  if (Test-ApiReady) {
    throw "Port $port already has an API responding before verification started. This script will not attach to an existing service; stop that process or use a different isolated verification port."
  }

  $verifyLogDir = Join-Path ([System.IO.Path]::GetTempPath()) "agent-swarm-verify-real-model"
  New-Item -ItemType Directory -Force -Path $verifyLogDir | Out-Null
  $outLog = Join-Path $verifyLogDir "real-model-admission-api.out.log"
  $errLog = Join-Path $verifyLogDir "real-model-admission-api.err.log"
  $tempRuntimeStateFile = Join-Path $verifyLogDir "runtime-state.json"

  $previousPort = $env:AGENT_SWARM_API_PORT
  $previousSource = $env:AGENT_SWARM_DASHBOARD_SOURCE
  $previousRuntimeStateFile = $env:AGENT_SWARM_RUNTIME_STATE_FILE
  $previousProjectPlanFlag = $env:AGENT_SWARM_ENABLE_REAL_MODEL_PROJECT_PLAN
  $previousCompatKey = $env:AGENT_SWARM_OPENAI_COMPAT_API_KEY
  $previousCompatBaseUrl = $env:AGENT_SWARM_OPENAI_COMPAT_BASE_URL

  $env:AGENT_SWARM_API_PORT = "$port"
  $env:AGENT_SWARM_DASHBOARD_SOURCE = "mock"
  $env:AGENT_SWARM_RUNTIME_STATE_FILE = $tempRuntimeStateFile
  $env:AGENT_SWARM_ENABLE_REAL_MODEL_PROJECT_PLAN = "true"
  $env:AGENT_SWARM_OPENAI_COMPAT_API_KEY = "sk-route-secret"
  $env:AGENT_SWARM_OPENAI_COMPAT_BASE_URL = "https://api.cheng.pink/v1"

  $process = $null
  try {
    $process = Start-Process `
      -WindowStyle Hidden `
      -FilePath "node" `
      -ArgumentList @($apiScript) `
      -RedirectStandardOutput $outLog `
      -RedirectStandardError $errLog `
      -PassThru

    $ready = $false
    for ($i = 0; $i -lt 20; $i++) {
      Start-Sleep -Milliseconds 250
      if ($process.HasExited) {
        break
      }
      if (Test-ApiReady) {
        $ready = $true
        break
      }
    }

    if (-not $ready) {
      throw "Real model admission API did not start on port $port. Check $outLog and $errLog"
    }

    $beforeCounts = Get-StateCounts
    $route = Invoke-Json -Method "POST" -Path "/api/projects/$projectId/project-plan-model-requests" -Body @{
      projectId = "other_project_ignored"
      purpose = "project_plan_generation"
      idea = "Build a local customer lead tracker"
      constraints = "Mock/SQLite first; no Runner execution"
      requestedBy = "verify_real_model_admission_route"
      secondConfirm = $true
      confirmText = "I understand this may make one backend model request."
    }
    $afterCounts = Get-StateCounts

    Assert-Equal $route.ok $false "Disabled route should not be ok."
    Assert-Equal $route.result "blocked" "Disabled route should be blocked."
    Assert-Equal $route.errorCategory "feature_disabled" "Disabled route should report feature_disabled."
    Assert-Equal $route.route "project_plan_model_requests_disabled" "Disabled route should expose disabled route id."
    Assert-Equal $route.routeImplemented $true "Disabled route draft should be implemented."
    Assert-Equal $route.routeEnabled $false "Disabled route draft should not be enabled."
    Assert-Equal $route.routeMode "feature_disabled" "Disabled route draft should be feature-disabled."
    Assert-Equal $route.projectIdSource "url_path" "Disabled route should take projectId from the URL path."
    Assert-Equal $route.bodyProjectIdIgnored $true "Disabled route should ignore body projectId."
    Assert-Equal $route.requestValid $true "Disabled route should accept the fixed business shape."
    Assert-Equal $route.featureFlags.realModelProjectPlanRequested $true "Disabled route should report requested feature flag."
    Assert-Equal $route.featureFlags.realModelProjectPlanActive $false "Disabled route should not activate model calls."
    Assert-Equal $route.featureFlags.realProviderRequestsAllowed $false "Disabled route should not allow provider requests."
    Assert-NoSideEffects -Result $route -Prefix "Disabled route"
    Assert-RequestPolicyLocked -Result $route -Prefix "Disabled route"
    Assert-ProviderConfigSafe -Result $route.providerConfig -Prefix "Disabled route provider config"
    Assert-NoRawSecretsInJson -Value $route -Prefix "Disabled route response"

    Assert-Equal $afterCounts.Tasks $beforeCounts.Tasks "Disabled route should not create tasks."
    Assert-Equal $afterCounts.Approvals $beforeCounts.Approvals "Disabled route should not create approvals."
    Assert-Equal $afterCounts.RunnerJobs $beforeCounts.RunnerJobs "Disabled route should not create Runner jobs."
    Assert-Equal $afterCounts.RuntimeEvents $beforeCounts.RuntimeEvents "Disabled route should not create runtime events."

    $forbiddenRoute = Invoke-Json -Method "POST" -Path "/api/projects/$projectId/project-plan-model-requests" -Body @{
      purpose = "project_plan_generation"
      idea = "Build a local customer lead tracker"
      constraints = "Mock/SQLite first; no Runner execution"
      requestedBy = "verify_real_model_admission_route"
      apiKey = "sk-route-secret"
      headers = @{
        Authorization = "Bearer route-token"
      }
    }
    Assert-Equal $forbiddenRoute.ok $false "Forbidden route request should not be ok."
    Assert-Equal $forbiddenRoute.requestValid $false "Forbidden route request should be invalid."
    Assert-Equal $forbiddenRoute.errorCategory "invalid_request" "Forbidden route request should report invalid_request."
    Assert-TextContains (@($forbiddenRoute.validationErrors) -join "`n") "apiKey" "Forbidden route request should reject API key field."
    Assert-TextContains (@($forbiddenRoute.validationErrors) -join "`n") "headers" "Forbidden route request should reject headers field."
    Assert-NoSideEffects -Result $forbiddenRoute -Prefix "Forbidden disabled route"
    Assert-NoRawSecretsInJson -Value $forbiddenRoute -Prefix "Forbidden disabled route response"

    $wrongProject = Invoke-JsonExpectStatus -Method "POST" -Path "/api/projects/wrong_project/project-plan-model-requests" -ExpectedStatus 404 -Body @{
      purpose = "project_plan_generation"
      idea = "Build a local customer lead tracker"
      requestedBy = "verify_real_model_admission_route"
    }
    Assert-Equal $wrongProject.ok $false "Wrong project route should not be ok."
    Assert-Equal $wrongProject.error "project_not_found" "Wrong project route should report project_not_found."
  } finally {
    if ($process -and -not $process.HasExited) {
      Stop-Process -Id $process.Id -Force
      $process.WaitForExit()
    }
    $env:AGENT_SWARM_API_PORT = $previousPort
    $env:AGENT_SWARM_DASHBOARD_SOURCE = $previousSource
    $env:AGENT_SWARM_RUNTIME_STATE_FILE = $previousRuntimeStateFile
    $env:AGENT_SWARM_ENABLE_REAL_MODEL_PROJECT_PLAN = $previousProjectPlanFlag
    $env:AGENT_SWARM_OPENAI_COMPAT_API_KEY = $previousCompatKey
    $env:AGENT_SWARM_OPENAI_COMPAT_BASE_URL = $previousCompatBaseUrl
  }

  Write-Step "Real model admission checks passed."
} finally {
  Pop-Location
}
