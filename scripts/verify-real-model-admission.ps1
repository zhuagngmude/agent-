$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot

function Write-Step {
  param([string]$Message)
  Write-Host "[real-model-admission] $Message"
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

function Assert-NoSideEffects {
  param(
    [Parameter(Mandatory = $true)][object]$Result,
    [string]$Prefix = "Project plan model request"
  )

  Assert-Equal $Result.realProviderRequestAttempted $false "$Prefix should not attempt provider requests."
  Assert-Equal $Result.providerResponseStored $false "$Prefix should not store provider responses."
  Assert-Equal $Result.sideEffects.writesSqlite $false "$Prefix should not write SQLite."
  Assert-Equal $Result.sideEffects.writesRuntimeState $false "$Prefix should not write runtime state."
  Assert-Equal $Result.sideEffects.createsTasks $false "$Prefix should not create tasks."
  Assert-Equal $Result.sideEffects.createsApprovals $false "$Prefix should not create approvals."
  Assert-Equal $Result.sideEffects.createsRunnerJobs $false "$Prefix should not create Runner jobs."
  Assert-Equal $Result.sideEffects.triggersAgents $false "$Prefix should not trigger Agents."
  Assert-Equal $Result.sideEffects.callsRealModel $false "$Prefix should not call real models."
  Assert-Equal $Result.sideEffects.executesRunner $false "$Prefix should not execute Runner."
  Assert-Equal $Result.sideEffects.logsPromptOrResult $false "$Prefix should not log prompts or results."
  Assert-Equal $Result.sideEffects.storesProviderResponse $false "$Prefix should not store provider responses."
  Assert-Equal $Result.sideEffects.readsRawSecrets $false "$Prefix should not read raw secrets."
  Assert-Equal $Result.sideEffects.modifiesGit $false "$Prefix should not modify Git."
  Assert-Equal $Result.sideEffects.writesProjectFiles $false "$Prefix should not write project files."
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

Push-Location $root
try {
  Write-Step "Load helper validation cases."
  $casesJson = node -e @"
const { buildProjectPlanGenerationModelRequest } = require('./services/api/model-gateway-project-plan');

const base = {
  projectId: 'project_agent_swarm',
  purpose: 'project_plan_generation',
  idea: 'Build a local customer lead tracker',
  constraints: 'Mock/SQLite first; no Runner execution',
  requestedBy: 'verify_real_model_admission'
};

const previousFlag = process.env.AGENT_SWARM_ENABLE_REAL_MODEL_PROJECT_PLAN;
const cases = {
  featureDisabled: buildProjectPlanGenerationModelRequest(base),
  missingIdea: buildProjectPlanGenerationModelRequest({ ...base, idea: '' }),
  invalidPurpose: buildProjectPlanGenerationModelRequest({ ...base, purpose: 'chat_completion' }),
  forbiddenApiKey: buildProjectPlanGenerationModelRequest({ ...base, apiKey: 'sk-local-test' }),
  forbiddenBaseUrl: buildProjectPlanGenerationModelRequest({ ...base, baseUrl: 'https://relay.example.test/v1' }),
  forbiddenHeaders: buildProjectPlanGenerationModelRequest({ ...base, headers: { Authorization: 'Bearer test' } }),
  forbiddenPrompt: buildProjectPlanGenerationModelRequest({ ...base, prompt: 'use this free-form prompt' }),
  forbiddenProviderBody: buildProjectPlanGenerationModelRequest({ ...base, providerRequestBody: { messages: [{ role: 'user', content: 'hello' }] } }),
  forbiddenRunnerJob: buildProjectPlanGenerationModelRequest({ ...base, runnerJobId: 'runner_job_1' }),
  forbiddenSecretValue: buildProjectPlanGenerationModelRequest({ ...base, constraints: 'api_key=abc123' })
};
process.env.AGENT_SWARM_ENABLE_REAL_MODEL_PROJECT_PLAN = 'true';
cases.flagRequested = buildProjectPlanGenerationModelRequest(base);
if (previousFlag === undefined) {
  delete process.env.AGENT_SWARM_ENABLE_REAL_MODEL_PROJECT_PLAN;
} else {
  process.env.AGENT_SWARM_ENABLE_REAL_MODEL_PROJECT_PLAN = previousFlag;
}

process.stdout.write(JSON.stringify(cases));
"@
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

  Write-Step "Verify feature flag request cannot activate real provider calls."
  Assert-Equal $cases.flagRequested.featureFlags.realModelProjectPlanRequested $true "Feature flag request should be reported."
  Assert-Equal $cases.flagRequested.featureFlags.realModelProjectPlanActive $false "Feature flag request should not activate model calls."
  Assert-Equal $cases.flagRequested.featureFlags.realProviderRequestsAllowed $false "Feature flag request should not allow provider calls."
  Assert-Equal $cases.flagRequested.errorCategory "feature_disabled" "Feature flag request should remain feature-disabled."
  Assert-NoSideEffects -Result $cases.flagRequested -Prefix "Feature flag requested case"

  Write-Step "Real model admission helper checks passed."
} finally {
  Pop-Location
}
