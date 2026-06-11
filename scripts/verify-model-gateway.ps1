$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
$apiBase = "http://127.0.0.1:8787"
$projectId = "project_agent_swarm"

function Write-Step {
  param([string]$Message)
  Write-Host "[model-gateway] $Message"
}

function Invoke-Json {
  param(
    [Parameter(Mandatory = $true)][string]$Path,
    [string]$Method = "GET",
    [object]$Body = $null
  )

  $uri = "$apiBase$Path"
  if ($null -eq $Body) {
    return Invoke-RestMethod -Method $Method -Uri $uri -TimeoutSec 5
  }

  return Invoke-RestMethod `
    -Method $Method `
    -Uri $uri `
    -TimeoutSec 5 `
    -ContentType "application/json" `
    -Body ($Body | ConvertTo-Json -Depth 10)
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

function Assert-ModelGatewayFeatureFlags {
  param(
    [Parameter(Mandatory = $true)][object]$FeatureFlags,
    [string]$Prefix = "Model Gateway feature flags"
  )

  Assert-Equal $FeatureFlags.manualConnectivityTestEnvVar "AGENT_SWARM_ENABLE_MODEL_CONNECTIVITY_TEST" "$Prefix should expose the manual connectivity env var name."
  Assert-True (($FeatureFlags.manualConnectivityTestRequested -eq $true) -or ($FeatureFlags.manualConnectivityTestRequested -eq $false)) "$Prefix should report the requested flag as a boolean."
  Assert-Equal $FeatureFlags.manualConnectivityTestActive $false "$Prefix should not be active in MVP-0.6."
  Assert-Equal $FeatureFlags.realProviderRequestsAllowed $false "$Prefix should not allow provider requests in MVP-0.6."
}

function Assert-ModelGatewayContract {
  param(
    [Parameter(Mandatory = $true)][object]$Contract,
    [string]$Prefix = "Model Gateway contract"
  )

  Assert-Equal $Contract.version "mvp-0.6" "$Prefix should expose the frozen stage version."
  Assert-Equal $Contract.boundary "disabled" "$Prefix should remain disabled."
  Assert-Equal $Contract.requestShapes.dryRun.purpose "connectivity_check" "$Prefix dry-run purpose should be frozen."
  Assert-Equal $Contract.requestShapes.connectivityTest.purpose "manual_connectivity_test" "$Prefix connectivity-test purpose should be frozen."
  Assert-Equal $Contract.requestShapes.dryRun.blockedByDefault $true "$Prefix dry-run should remain blocked by default."
  Assert-Equal $Contract.requestShapes.connectivityTest.blockedByDefault $true "$Prefix connectivity-test should remain blocked by default."
  Assert-TextContains (@($Contract.requestShapes.connectivityTest.requiredFields) -join "`n") "secondConfirm" "$Prefix should require secondConfirm."
  Assert-TextContains (@($Contract.requestShapes.connectivityTest.requiredFields) -join "`n") "confirmText" "$Prefix should require confirmText."
  Assert-TextContains (@($Contract.requestShapes.connectivityTest.optionalFields) -join "`n") "timeoutMs" "$Prefix should keep timeoutMs optional."
  Assert-TextContains (@($Contract.requestShapes.connectivityTest.optionalFields) -join "`n") "responseBodyLimitBytes" "$Prefix should keep responseBodyLimitBytes optional."
  Assert-Equal @($Contract.providerCatalog).Count 4 "$Prefix should expose four provider entries."
  Assert-Equal $Contract.providerCatalog[1].futureProviderAdapterId "openai_compat_manual_connectivity_adapter" "$Prefix should expose relay future adapter metadata."
}

function Assert-DryRunNoSideEffects {
  param(
    [Parameter(Mandatory = $true)][object]$DryRun,
    [string]$Prefix = "Model Gateway dry-run"
  )

  Assert-ModelGatewayFeatureFlags -FeatureFlags $DryRun.featureFlags -Prefix "$Prefix feature flags"
  Assert-Equal $DryRun.sideEffects.writesSqlite $false "$Prefix should not write SQLite."
  Assert-Equal $DryRun.sideEffects.writesRuntimeState $false "$Prefix should not write runtime state."
  Assert-Equal $DryRun.sideEffects.createsTasks $false "$Prefix should not create tasks."
  Assert-Equal $DryRun.sideEffects.createsApprovals $false "$Prefix should not create approvals."
  Assert-Equal $DryRun.sideEffects.createsRunnerJobs $false "$Prefix should not create Runner jobs."
  Assert-Equal $DryRun.sideEffects.triggersAgents $false "$Prefix should not trigger Agents."
  Assert-Equal $DryRun.sideEffects.callsRealModel $false "$Prefix should not call real models."
  Assert-Equal $DryRun.sideEffects.logsPromptOrResult $false "$Prefix should not log prompts or results."
}

function Assert-ConnectivityTestNoSideEffects {
  param(
    [Parameter(Mandatory = $true)][object]$ConnectivityTest,
    [string]$Prefix = "Model Gateway connectivity test"
  )

  Assert-ModelGatewayFeatureFlags -FeatureFlags $ConnectivityTest.featureFlags -Prefix "$Prefix feature flags"
  Assert-Equal $ConnectivityTest.sideEffects.writesSqlite $false "$Prefix should not write SQLite."
  Assert-Equal $ConnectivityTest.sideEffects.writesRuntimeState $false "$Prefix should not write runtime state."
  Assert-Equal $ConnectivityTest.sideEffects.createsTasks $false "$Prefix should not create tasks."
  Assert-Equal $ConnectivityTest.sideEffects.createsApprovals $false "$Prefix should not create approvals."
  Assert-Equal $ConnectivityTest.sideEffects.createsRunnerJobs $false "$Prefix should not create Runner jobs."
  Assert-Equal $ConnectivityTest.sideEffects.triggersAgents $false "$Prefix should not trigger Agents."
  Assert-Equal $ConnectivityTest.sideEffects.callsRealModel $false "$Prefix should not call real models."
  Assert-Equal $ConnectivityTest.sideEffects.executesRunner $false "$Prefix should not execute Runner."
  Assert-Equal $ConnectivityTest.sideEffects.logsPromptOrResult $false "$Prefix should not log prompts or results."
  Assert-Equal $ConnectivityTest.sideEffects.storesProviderResponse $false "$Prefix should not store provider responses."
}

function Assert-PreflightNoSideEffects {
  param(
    [Parameter(Mandatory = $true)][object]$Preflight,
    [string]$Prefix = "Model Gateway connectivity preflight"
  )

  Assert-ModelGatewayFeatureFlags -FeatureFlags $Preflight.featureFlags -Prefix "$Prefix feature flags"
  Assert-Equal $Preflight.realProviderRequestAttempted $false "$Prefix should not attempt provider requests."
  Assert-Equal $Preflight.sideEffects.writesSqlite $false "$Prefix should not write SQLite."
  Assert-Equal $Preflight.sideEffects.writesRuntimeState $false "$Prefix should not write runtime state."
  Assert-Equal $Preflight.sideEffects.createsTasks $false "$Prefix should not create tasks."
  Assert-Equal $Preflight.sideEffects.createsApprovals $false "$Prefix should not create approvals."
  Assert-Equal $Preflight.sideEffects.createsRunnerJobs $false "$Prefix should not create Runner jobs."
  Assert-Equal $Preflight.sideEffects.triggersAgents $false "$Prefix should not trigger Agents."
  Assert-Equal $Preflight.sideEffects.callsRealModel $false "$Prefix should not call real models."
  Assert-Equal $Preflight.sideEffects.executesRunner $false "$Prefix should not execute Runner."
  Assert-Equal $Preflight.sideEffects.logsPromptOrResult $false "$Prefix should not log prompts or results."
  Assert-Equal $Preflight.sideEffects.storesProviderResponse $false "$Prefix should not store provider responses."
}

function Assert-DisabledConnectivityAdapter {
  param(
    [Parameter(Mandatory = $true)][object]$ConnectivityTest,
    [Parameter(Mandatory = $true)][string]$ExpectedProviderAdapterId,
    [string]$Prefix = "Model Gateway connectivity test"
  )

  Assert-Equal $ConnectivityTest.adapter "disabled_provider_connectivity_adapter" "$Prefix should use the disabled adapter stub."
  Assert-Equal $ConnectivityTest.providerAdapterId $ExpectedProviderAdapterId "$Prefix should report the provider-specific disabled adapter id."
  Assert-Equal $ConnectivityTest.providerAdapterMode "disabled" "$Prefix provider adapter should remain disabled."
  Assert-Equal $ConnectivityTest.realProviderRequestAttempted $false "$Prefix should not attempt provider requests."
  Assert-Equal $ConnectivityTest.result "blocked" "$Prefix should stay blocked."
  Assert-Equal $ConnectivityTest.errorCategory "feature_disabled" "$Prefix should report feature disabled."
  Assert-Equal $ConnectivityTest.providerResponseStored $false "$Prefix should not store provider responses."
  Assert-Equal $ConnectivityTest.durationMs 0 "$Prefix should not spend time in provider calls."
  Assert-Equal $ConnectivityTest.redactionApplied $true "$Prefix should report redaction applied."
}

function Assert-OpenAiCompatRelayInterface {
  param(
    [Parameter(Mandatory = $true)][object]$RelayInterface,
    [Parameter(Mandatory = $true)][string]$ExpectedErrorCategory,
    [string]$Prefix = "OpenAI-compatible relay adapter interface"
  )

  Assert-Equal $RelayInterface.adapter "openai_compat_relay_connectivity_adapter_interface" "$Prefix should use the relay interface checkpoint adapter."
  Assert-Equal $RelayInterface.providerAdapterId "openai_compat_manual_connectivity_adapter" "$Prefix should expose the future relay adapter id."
  Assert-Equal $RelayInterface.providerAdapterMode "interface_disabled" "$Prefix should remain interface-disabled."
  Assert-Equal $RelayInterface.result "blocked" "$Prefix should stay blocked."
  Assert-Equal $RelayInterface.errorCategory $ExpectedErrorCategory "$Prefix should report the expected coarse error category."
  Assert-Equal $RelayInterface.realProviderRequestAttempted $false "$Prefix should not attempt provider requests."
  Assert-Equal $RelayInterface.providerResponseStored $false "$Prefix should not store provider responses."
  Assert-Equal $RelayInterface.redactionApplied $true "$Prefix should report redaction applied."
  Assert-Equal $RelayInterface.interfaceOnly $true "$Prefix should identify itself as interface-only."
  Assert-Equal $RelayInterface.requestShape.provider "openai_compat" "$Prefix should be scoped to openai_compat."
  Assert-Equal $RelayInterface.requestShape.keySource "server_env" "$Prefix should read key only from server env in future."
  Assert-Equal $RelayInterface.requestShape.baseUrlSource "server_env" "$Prefix should read base URL only from server env in future."
  Assert-Equal $RelayInterface.requestShape.acceptsRequestBaseUrl $false "$Prefix should not accept request base URL overrides."
  Assert-Equal $RelayInterface.requestShape.acceptsApiKeyFromClient $false "$Prefix should not accept client API keys."
  Assert-Equal $RelayInterface.requestShape.acceptsFreeFormPrompt $false "$Prefix should not accept free-form prompts."
  Assert-Equal $RelayInterface.requestShape.acceptsAgentContext $false "$Prefix should not accept Agent context."
  Assert-Equal $RelayInterface.requestShape.acceptsFiles $false "$Prefix should not accept files."
  Assert-Equal $RelayInterface.requestShape.acceptsToolCalls $false "$Prefix should not accept tool calls."
  Assert-Equal $RelayInterface.requestShape.acceptsRunnerJob $false "$Prefix should not accept Runner jobs."
  Assert-Equal $RelayInterface.requestShape.fixedMinimalPingOnly $true "$Prefix should allow only a future fixed minimal ping."
  Assert-Equal $RelayInterface.requestShape.endpointShapeConfirmed $true "$Prefix endpoint shape should be confirmed for cheng.pink."
  Assert-Equal $RelayInterface.requestShape.fixedModel "gpt-5.4-mini" "$Prefix should expose the fixed cheng.pink manual ping model."
  Assert-Equal $RelayInterface.requestShape.fixedEndpointPath "/v1/chat/completions" "$Prefix should expose the fixed cheng.pink manual ping endpoint path."
  Assert-Equal $RelayInterface.sideEffects.writesSqlite $false "$Prefix should not write SQLite."
  Assert-Equal $RelayInterface.sideEffects.writesRuntimeState $false "$Prefix should not write runtime state."
  Assert-Equal $RelayInterface.sideEffects.createsTasks $false "$Prefix should not create tasks."
  Assert-Equal $RelayInterface.sideEffects.createsApprovals $false "$Prefix should not create approvals."
  Assert-Equal $RelayInterface.sideEffects.createsRunnerJobs $false "$Prefix should not create Runner jobs."
  Assert-Equal $RelayInterface.sideEffects.triggersAgents $false "$Prefix should not trigger Agents."
  Assert-Equal $RelayInterface.sideEffects.callsRealModel $false "$Prefix should not call real models."
  Assert-Equal $RelayInterface.sideEffects.executesRunner $false "$Prefix should not execute Runner."
  Assert-Equal $RelayInterface.sideEffects.logsPromptOrResult $false "$Prefix should not log prompts or results."
  Assert-Equal $RelayInterface.sideEffects.storesProviderResponse $false "$Prefix should not store provider responses."
}

function Assert-ChengRelayManualPingBuilderNoSideEffects {
  param(
    [Parameter(Mandatory = $true)][object]$BuilderResult,
    [string]$Prefix = "Cheng relay manual ping builder"
  )

  Assert-Equal $BuilderResult.realProviderRequestAttempted $false "$Prefix should not attempt provider requests."
  Assert-Equal $BuilderResult.providerResponseStored $false "$Prefix should not store provider responses."
  Assert-Equal $BuilderResult.sideEffects.writesSqlite $false "$Prefix should not write SQLite."
  Assert-Equal $BuilderResult.sideEffects.writesRuntimeState $false "$Prefix should not write runtime state."
  Assert-Equal $BuilderResult.sideEffects.createsTasks $false "$Prefix should not create tasks."
  Assert-Equal $BuilderResult.sideEffects.createsApprovals $false "$Prefix should not create approvals."
  Assert-Equal $BuilderResult.sideEffects.createsRunnerJobs $false "$Prefix should not create Runner jobs."
  Assert-Equal $BuilderResult.sideEffects.triggersAgents $false "$Prefix should not trigger Agents."
  Assert-Equal $BuilderResult.sideEffects.callsRealModel $false "$Prefix should not call real models."
  Assert-Equal $BuilderResult.sideEffects.executesRunner $false "$Prefix should not execute Runner."
  Assert-Equal $BuilderResult.sideEffects.logsPromptOrResult $false "$Prefix should not log prompts or results."
  Assert-Equal $BuilderResult.sideEffects.storesProviderResponse $false "$Prefix should not store provider responses."
}

function Assert-ChengRelayManualPingReady {
  param(
    [Parameter(Mandatory = $true)][object]$BuilderResult,
    [string]$Prefix = "Cheng relay manual ping builder"
  )

  Assert-Equal $BuilderResult.ok $true "$Prefix should build a ready request shape."
  Assert-Equal $BuilderResult.provider "openai_compat" "$Prefix should target openai_compat."
  Assert-Equal $BuilderResult.model "gpt-5.4-mini" "$Prefix should use the fixed cheng relay model."
  Assert-Equal $BuilderResult.endpointUrl "https://api.cheng.pink/v1/chat/completions" "$Prefix should normalize to the canonical endpoint."
  Assert-Equal $BuilderResult.method "POST" "$Prefix should use POST."
  Assert-Equal $BuilderResult.headers.authorizationSource "server_env" "$Prefix should read authorization only from server env."
  Assert-Equal $BuilderResult.headers.contentType "application/json" "$Prefix should use JSON."
  Assert-Equal $BuilderResult.headers.acceptsClientHeaders $false "$Prefix should not accept client headers."
  Assert-Equal $BuilderResult.body.model "gpt-5.4-mini" "$Prefix body should use the fixed model."
  Assert-Equal $BuilderResult.body.stream $false "$Prefix body should be non-streaming."
  Assert-Equal $BuilderResult.body.max_tokens 1 "$Prefix body should use the minimum token limit."
  Assert-Equal $BuilderResult.body.messages.Count 1 "$Prefix body should contain exactly one fixed message."
  Assert-Equal $BuilderResult.body.messages[0].role "user" "$Prefix body message role should be fixed."
  Assert-Equal $BuilderResult.body.messages[0].content "ping" "$Prefix body message content should be fixed."
  Assert-Equal $BuilderResult.acceptsClientApiKey $false "$Prefix should not accept client API keys."
  Assert-Equal $BuilderResult.acceptsClientBaseUrl $false "$Prefix should not accept client base URLs."
  Assert-Equal $BuilderResult.acceptsClientPrompt $false "$Prefix should not accept client prompts."
  Assert-Equal $BuilderResult.acceptsClientHeaders $false "$Prefix should not accept client headers."
  Assert-Equal $BuilderResult.acceptsClientStreamSetting $false "$Prefix should not accept client stream settings."
  Assert-TextContains $BuilderResult.endpointUrl "/v1/chat/completions" "$Prefix endpoint should include /v1/chat/completions."
  Assert-True (-not $BuilderResult.endpointUrl.Contains("/v1/v1/chat/completions")) "$Prefix endpoint should not duplicate /v1."
  Assert-ChengRelayManualPingBuilderNoSideEffects -BuilderResult $BuilderResult -Prefix $Prefix
}

function Assert-ChengRelayManualPingBlocked {
  param(
    [Parameter(Mandatory = $true)][object]$BuilderResult,
    [Parameter(Mandatory = $true)][string]$ExpectedErrorCategory,
    [string]$Prefix = "Cheng relay manual ping builder"
  )

  Assert-Equal $BuilderResult.ok $false "$Prefix should be blocked."
  Assert-Equal $BuilderResult.result "blocked" "$Prefix should report blocked."
  Assert-Equal $BuilderResult.errorCategory $ExpectedErrorCategory "$Prefix should report the expected error category."
  Assert-Equal $BuilderResult.endpointUrl "" "$Prefix should not expose an endpoint when blocked."
  Assert-Equal $BuilderResult.body $null "$Prefix should not expose a request body when blocked."
  Assert-ChengRelayManualPingBuilderNoSideEffects -BuilderResult $BuilderResult -Prefix $Prefix
}

Push-Location $root
try {
  Write-Step "Verify local API is ready."
  $health = Invoke-Json -Path "/api/health"
  Assert-Equal $health.ok $true "API health should be ok."
  Assert-Equal $health.projectId $projectId "API project id mismatch."

  Write-Step "Verify status remains disabled."
  $gateway = Invoke-Json -Path "/api/model-gateway/status"
  Assert-Equal $gateway.enabled $false "Model Gateway should be disabled."
  Assert-ModelGatewayContract -Contract $gateway.contract -Prefix "Status contract"
  Assert-Equal $gateway.realModelCallsAllowed $false "Real model calls should be disabled."
  Assert-Equal $gateway.safety.exposesApiKeysToFrontend $false "API keys should not be exposed to the frontend."
  Assert-Equal $gateway.safety.writesDatabase $false "Model Gateway status should not write the database."
  Assert-Equal $gateway.safety.createsRunnerJobs $false "Model Gateway status should not create Runner jobs."
  Assert-Equal $gateway.safety.makesNetworkRequests $false "Model Gateway status should not make provider network requests."
  Assert-ModelGatewayFeatureFlags -FeatureFlags $gateway.featureFlags -Prefix "Model Gateway status feature flags"
  Assert-Equal (($gateway.providers | Where-Object { $_.id -eq "openai" }).providerAdapterId) "openai_disabled_connectivity_adapter" "OpenAI status should expose disabled adapter id."
  Assert-Equal (($gateway.providers | Where-Object { $_.id -eq "openai_compat" }).providerAdapterId) "openai_compat_disabled_connectivity_adapter" "OpenAI-compatible relay status should expose disabled adapter id."
  Assert-Equal (($gateway.providers | Where-Object { $_.id -eq "openai_compat" }).futureProviderAdapterId) "openai_compat_manual_connectivity_adapter" "OpenAI-compatible relay status should expose the future relay adapter id as metadata only."
  Assert-Equal (($gateway.providers | Where-Object { $_.id -eq "openai_compat" }).futureProviderAdapterMode) "interface_disabled" "OpenAI-compatible relay future adapter should remain interface-disabled."
  Assert-Equal (($gateway.providers | Where-Object { $_.id -eq "openai_compat" }).connectivityTestModel) "gpt-5.4-mini" "OpenAI-compatible relay should use the fixed cheng relay model."
  Assert-Equal (($gateway.providers | Where-Object { $_.id -eq "openai_compat" }).keyEnvVar) "AGENT_SWARM_OPENAI_COMPAT_API_KEY" "OpenAI-compatible relay should use a dedicated key env var."
  Assert-Equal (($gateway.providers | Where-Object { $_.id -eq "openai_compat" }).baseUrlEnvVar) "AGENT_SWARM_OPENAI_COMPAT_BASE_URL" "OpenAI-compatible relay should use a dedicated base URL env var."
  Assert-Equal (($gateway.providers | Where-Object { $_.id -eq "openai_compat" }).baseUrlRequired) $true "OpenAI-compatible relay should require a server-side base URL."
  Assert-Equal (($gateway.providers | Where-Object { $_.id -eq "anthropic" }).providerAdapterId) "anthropic_disabled_connectivity_adapter" "Anthropic status should expose disabled adapter id."
  Assert-Equal (($gateway.providers | Where-Object { $_.id -eq "google" }).providerAdapterId) "google_disabled_connectivity_adapter" "Google status should expose disabled adapter id."
  foreach ($provider in $gateway.providers) {
    Assert-Equal $provider.providerAdapterMode "disabled" "Provider $($provider.id) adapter mode should remain disabled."
  }

  Write-Step "Verify dry-run request and no-side-effect cases."
  $dryRun = Invoke-Json -Method "POST" -Path "/api/model-gateway/dry-run" -Body @{
    provider = "openai"
    model = "gpt-4.1-mini"
    purpose = "connectivity_check"
    promptPreview = "local smoke dry-run"
    requestedBy = "local_user"
  }
  Assert-Equal $dryRun.ok $false "Model Gateway dry-run should remain blocked."
  Assert-Equal $dryRun.dryRun $true "Model Gateway dry-run should identify itself as dry-run."
  Assert-Equal $dryRun.requestValid $true "Model Gateway dry-run request should be valid."
  Assert-Equal $dryRun.providerSupported $true "OpenAI provider should be recognized by dry-run."
  Assert-Equal $dryRun.realModelCallsAllowed $false "Model Gateway dry-run should not allow real calls."
  Assert-Equal $dryRun.wouldCallProvider $false "Model Gateway dry-run should not call providers."
  Assert-DryRunNoSideEffects -DryRun $dryRun

  $unknownProviderDryRun = Invoke-Json -Method "POST" -Path "/api/model-gateway/dry-run" -Body @{
    provider = "unknown"
    model = "gpt-4.1-mini"
    purpose = "connectivity_check"
    requestedBy = "local_user"
  }
  Assert-Equal $unknownProviderDryRun.requestValid $false "Unknown provider dry-run should be invalid."
  Assert-Equal $unknownProviderDryRun.providerSupported $false "Unknown provider should not be supported."
  Assert-Equal $unknownProviderDryRun.wouldCallProvider $false "Unknown provider dry-run should not call providers."
  Assert-DryRunNoSideEffects -DryRun $unknownProviderDryRun -Prefix "Unknown provider dry-run"

  $missingModelDryRun = Invoke-Json -Method "POST" -Path "/api/model-gateway/dry-run" -Body @{
    provider = "openai"
    purpose = "connectivity_check"
    requestedBy = "local_user"
  }
  Assert-Equal $missingModelDryRun.requestValid $false "Missing model dry-run should be invalid."
  Assert-TextContains (@($missingModelDryRun.validationErrors) -join "`n") "model is required." "Missing model dry-run should report validation error."
  Assert-DryRunNoSideEffects -DryRun $missingModelDryRun -Prefix "Missing model dry-run"

  $invalidPurposeDryRun = Invoke-Json -Method "POST" -Path "/api/model-gateway/dry-run" -Body @{
    provider = "openai"
    model = "gpt-4.1-mini"
    purpose = "chat_completion"
    requestedBy = "local_user"
  }
  Assert-Equal $invalidPurposeDryRun.requestValid $false "Invalid purpose dry-run should be invalid."
  Assert-TextContains (@($invalidPurposeDryRun.validationErrors) -join "`n") "purpose must be connectivity_check." "Invalid purpose dry-run should report validation error."
  Assert-DryRunNoSideEffects -DryRun $invalidPurposeDryRun -Prefix "Invalid purpose dry-run"

  Write-Step "Verify connectivity-test disabled stub."
  $connectivityTest = Invoke-Json -Method "POST" -Path "/api/model-gateway/connectivity-test" -Body @{
    provider = "openai"
    model = "gpt-4.1-mini"
    purpose = "manual_connectivity_test"
    secondConfirm = $true
    confirmText = "I understand this will make one real provider connectivity request."
    requestedBy = "local_user"
  }
  Assert-Equal $connectivityTest.ok $false "Model Gateway connectivity-test stub should remain blocked."
  Assert-Equal $connectivityTest.requestValid $true "Model Gateway connectivity-test stub request should be valid."
  Assert-Equal $connectivityTest.providerSupported $true "OpenAI provider should be recognized by connectivity-test stub."
  Assert-Equal $connectivityTest.realModelCallsAllowed $false "Model Gateway connectivity-test stub should not allow real calls."
  Assert-DisabledConnectivityAdapter -ConnectivityTest $connectivityTest -ExpectedProviderAdapterId "openai_disabled_connectivity_adapter"
  Assert-ConnectivityTestNoSideEffects -ConnectivityTest $connectivityTest
  Assert-PreflightNoSideEffects -Preflight $connectivityTest.preflight -Prefix "Connectivity-test response preflight"
  Assert-TextContains (@($connectivityTest.preflight.blockingCategories) -join "`n") "feature_disabled" "Connectivity-test preflight should remain feature-disabled."

  Write-Step "Verify backend helper failure paths and cheng.pink request builder."
  $helperJson = node -e @"
const gateway = require('./services/api/model-gateway');
const adapters = require('./services/api/model-gateway-adapters');
const base = {
  provider: 'openai',
  model: 'gpt-4.1-mini',
  purpose: 'manual_connectivity_test',
  secondConfirm: true,
  confirmText: 'local preflight acceptance'
};
const relayBase = {
  provider: 'openai_compat',
  model: 'gpt-5.4-mini',
  purpose: 'manual_connectivity_test',
  secondConfirm: true,
  confirmText: 'local relay preflight acceptance'
};
const chengBuilderCases = {
  readyWithoutV1: adapters.buildChengRelayManualPingRequest({ baseUrl: 'https://api.cheng.pink', model: 'gpt-5.4-mini' }),
  readyWithV1: adapters.buildChengRelayManualPingRequest({ baseUrl: 'https://api.cheng.pink/v1', model: 'gpt-5.4-mini' }),
  readyWithTrailingSlash: adapters.buildChengRelayManualPingRequest({ baseUrl: 'https://api.cheng.pink/v1/', model: 'gpt-5.4-mini' }),
  missingBaseUrl: adapters.buildChengRelayManualPingRequest({ baseUrl: '', model: 'gpt-5.4-mini' }),
  httpBaseUrl: adapters.buildChengRelayManualPingRequest({ baseUrl: 'http://api.cheng.pink/v1', model: 'gpt-5.4-mini' }),
  localhostBaseUrl: adapters.buildChengRelayManualPingRequest({ baseUrl: 'https://localhost/v1', model: 'gpt-5.4-mini' }),
  loopbackBaseUrl: adapters.buildChengRelayManualPingRequest({ baseUrl: 'https://127.0.0.1/v1', model: 'gpt-5.4-mini' }),
  privateBaseUrl: adapters.buildChengRelayManualPingRequest({ baseUrl: 'https://192.168.1.10/v1', model: 'gpt-5.4-mini' }),
  queryTokenBaseUrl: adapters.buildChengRelayManualPingRequest({ baseUrl: 'https://api.cheng.pink/v1?token=secret', model: 'gpt-5.4-mini' }),
  wrongPathBaseUrl: adapters.buildChengRelayManualPingRequest({ baseUrl: 'https://api.cheng.pink/openai/v1', model: 'gpt-5.4-mini' }),
  unsupportedModel: adapters.buildChengRelayManualPingRequest({ baseUrl: 'https://api.cheng.pink/v1', model: 'gpt-5.5' })
};
const cases = {
  registry: adapters.disabledProviderAdapterRegistry,
  featureDisabled: gateway.modelGatewayConnectivityPreflight(base, { acceptanceOnlyKeyConfigured: true }),
  missingKey: gateway.modelGatewayConnectivityPreflight(base, { acceptanceOnlyKeyConfigured: false }),
  unsupportedProvider: gateway.modelGatewayConnectivityPreflight({ ...base, provider: 'unknown' }),
  unsupportedModel: gateway.modelGatewayConnectivityPreflight({ ...base, model: 'not-a-connectivity-model' }, { acceptanceOnlyKeyConfigured: true }),
  invalidPurpose: gateway.modelGatewayConnectivityPreflight({ ...base, purpose: 'chat_completion' }, { acceptanceOnlyKeyConfigured: true }),
  timeout: gateway.modelGatewayConnectivityPreflight(base, { acceptanceOnlyKeyConfigured: true, acceptanceSimulation: 'timeout' }),
  providerError: gateway.modelGatewayConnectivityPreflight(base, { acceptanceOnlyKeyConfigured: true, acceptanceSimulation: 'provider_error' }),
  relayMissingBaseUrl: gateway.modelGatewayConnectivityPreflight(relayBase, { acceptanceOnlyKeyConfigured: true, acceptanceOnlyBaseUrl: '' }),
  relayMissingKey: gateway.modelGatewayConnectivityPreflight(relayBase, { acceptanceOnlyKeyConfigured: false, acceptanceOnlyBaseUrl: 'https://relay.example.test/v1' }),
  relayInvalidBaseUrl: gateway.modelGatewayConnectivityPreflight(relayBase, { acceptanceOnlyKeyConfigured: true, acceptanceOnlyBaseUrl: 'http://127.0.0.1:8787/v1' }),
  relayValidBaseUrlBlocked: gateway.modelGatewayConnectivityPreflight(relayBase, { acceptanceOnlyKeyConfigured: true, acceptanceOnlyBaseUrl: 'https://relay.example.test/v1' })
};
cases.relayInterfaceCases = {
  missingKey: adapters.openAiCompatRelayConnectivityAdapter({ ...relayBase, preflight: cases.relayMissingKey }),
  missingBaseUrl: adapters.openAiCompatRelayConnectivityAdapter({ ...relayBase, preflight: cases.relayMissingBaseUrl }),
  invalidBaseUrl: adapters.openAiCompatRelayConnectivityAdapter({ ...relayBase, preflight: cases.relayInvalidBaseUrl }),
  unsupportedProvider: adapters.openAiCompatRelayConnectivityAdapter({ ...relayBase, preflight: cases.unsupportedProvider }),
  unsupportedModel: adapters.openAiCompatRelayConnectivityAdapter({ ...relayBase, preflight: gateway.modelGatewayConnectivityPreflight({ ...relayBase, model: 'not-a-relay-model' }, { acceptanceOnlyKeyConfigured: true, acceptanceOnlyBaseUrl: 'https://relay.example.test/v1' }) }),
  timeout: adapters.openAiCompatRelayConnectivityAdapter({ ...relayBase, preflight: gateway.modelGatewayConnectivityPreflight(relayBase, { acceptanceOnlyKeyConfigured: true, acceptanceOnlyBaseUrl: 'https://relay.example.test/v1', acceptanceSimulation: 'timeout' }) }),
  providerError: adapters.openAiCompatRelayConnectivityAdapter({ ...relayBase, preflight: gateway.modelGatewayConnectivityPreflight(relayBase, { acceptanceOnlyKeyConfigured: true, acceptanceOnlyBaseUrl: 'https://relay.example.test/v1', acceptanceSimulation: 'provider_error' }) }),
  featureDisabled: adapters.openAiCompatRelayConnectivityAdapter({ ...relayBase, preflight: cases.relayValidBaseUrlBlocked })
};
cases.chengBuilderCases = chengBuilderCases;
process.stdout.write(JSON.stringify(cases));
"@
  $helperCases = $helperJson | ConvertFrom-Json

  Assert-Equal $helperCases.registry.openai.providerAdapterId "openai_disabled_connectivity_adapter" "Registry should expose OpenAI disabled adapter."
  Assert-Equal $helperCases.registry.openai.mode "disabled" "OpenAI registry mode should be disabled."
  Assert-Equal $helperCases.registry.openai_compat.providerAdapterId "openai_compat_disabled_connectivity_adapter" "Registry should expose relay disabled adapter."
  Assert-Equal $helperCases.registry.openai_compat.futureProviderAdapterId "openai_compat_manual_connectivity_adapter" "Registry should expose future relay adapter metadata."
  Assert-Equal $helperCases.registry.openai_compat.futureMode "interface_disabled" "Relay future adapter should remain interface-disabled."
  Assert-Equal $helperCases.registry.openai_compat.connectivityTestModel "gpt-5.4-mini" "Relay registry should use the fixed cheng relay model."
  Assert-Equal $helperCases.registry.anthropic.providerAdapterId "anthropic_disabled_connectivity_adapter" "Registry should expose Anthropic disabled adapter."
  Assert-Equal $helperCases.registry.google.providerAdapterId "google_disabled_connectivity_adapter" "Registry should expose Google disabled adapter."

  Assert-ChengRelayManualPingReady -BuilderResult $helperCases.chengBuilderCases.readyWithoutV1 -Prefix "Cheng relay builder without /v1"
  Assert-ChengRelayManualPingReady -BuilderResult $helperCases.chengBuilderCases.readyWithV1 -Prefix "Cheng relay builder with /v1"
  Assert-ChengRelayManualPingReady -BuilderResult $helperCases.chengBuilderCases.readyWithTrailingSlash -Prefix "Cheng relay builder with trailing slash"
  Assert-ChengRelayManualPingBlocked -BuilderResult $helperCases.chengBuilderCases.missingBaseUrl -ExpectedErrorCategory "missing_base_url" -Prefix "Cheng relay builder missing base URL"
  Assert-ChengRelayManualPingBlocked -BuilderResult $helperCases.chengBuilderCases.httpBaseUrl -ExpectedErrorCategory "invalid_base_url" -Prefix "Cheng relay builder http base URL"
  Assert-ChengRelayManualPingBlocked -BuilderResult $helperCases.chengBuilderCases.localhostBaseUrl -ExpectedErrorCategory "invalid_base_url" -Prefix "Cheng relay builder localhost base URL"
  Assert-ChengRelayManualPingBlocked -BuilderResult $helperCases.chengBuilderCases.loopbackBaseUrl -ExpectedErrorCategory "invalid_base_url" -Prefix "Cheng relay builder loopback base URL"
  Assert-ChengRelayManualPingBlocked -BuilderResult $helperCases.chengBuilderCases.privateBaseUrl -ExpectedErrorCategory "invalid_base_url" -Prefix "Cheng relay builder private base URL"
  Assert-ChengRelayManualPingBlocked -BuilderResult $helperCases.chengBuilderCases.queryTokenBaseUrl -ExpectedErrorCategory "invalid_base_url" -Prefix "Cheng relay builder query-token base URL"
  Assert-ChengRelayManualPingBlocked -BuilderResult $helperCases.chengBuilderCases.wrongPathBaseUrl -ExpectedErrorCategory "invalid_base_url" -Prefix "Cheng relay builder wrong-path base URL"
  Assert-ChengRelayManualPingBlocked -BuilderResult $helperCases.chengBuilderCases.unsupportedModel -ExpectedErrorCategory "unsupported_model" -Prefix "Cheng relay builder unsupported model"

  Assert-Equal $helperCases.featureDisabled.result "blocked" "Preflight should remain blocked when feature is disabled."
  Assert-TextContains (@($helperCases.featureDisabled.blockingCategories) -join "`n") "feature_disabled" "Preflight should report feature disabled."
  Assert-PreflightNoSideEffects -Preflight $helperCases.featureDisabled -Prefix "Feature-disabled preflight"

  Assert-Equal $helperCases.missingKey.result "blocked" "Missing-key preflight should remain blocked."
  Assert-TextContains (@($helperCases.missingKey.blockingCategories) -join "`n") "missing_key" "Preflight should report missing key."
  Assert-PreflightNoSideEffects -Preflight $helperCases.missingKey -Prefix "Missing-key preflight"

  Assert-Equal $helperCases.unsupportedProvider.requestValid $false "Unsupported-provider preflight should be invalid."
  Assert-Equal $helperCases.unsupportedProvider.providerSupported $false "Unsupported-provider preflight should not support provider."
  Assert-TextContains (@($helperCases.unsupportedProvider.blockingCategories) -join "`n") "unsupported_provider" "Preflight should report unsupported provider."
  Assert-PreflightNoSideEffects -Preflight $helperCases.unsupportedProvider -Prefix "Unsupported-provider preflight"

  Assert-Equal $helperCases.unsupportedModel.requestValid $false "Unsupported-model preflight should be invalid."
  Assert-Equal $helperCases.unsupportedModel.modelSupported $false "Unsupported-model preflight should not support model."
  Assert-TextContains (@($helperCases.unsupportedModel.blockingCategories) -join "`n") "unsupported_model" "Preflight should report unsupported model."
  Assert-PreflightNoSideEffects -Preflight $helperCases.unsupportedModel -Prefix "Unsupported-model preflight"

  Assert-Equal $helperCases.invalidPurpose.requestValid $false "Invalid-purpose preflight should be invalid."
  Assert-TextContains (@($helperCases.invalidPurpose.validationErrors) -join "`n") "purpose must be manual_connectivity_test." "Preflight should report invalid purpose."
  Assert-PreflightNoSideEffects -Preflight $helperCases.invalidPurpose -Prefix "Invalid-purpose preflight"

  Assert-Equal $helperCases.timeout.result "blocked" "Timeout preflight should remain blocked."
  Assert-TextContains (@($helperCases.timeout.blockingCategories) -join "`n") "timeout" "Preflight should report timeout."
  Assert-PreflightNoSideEffects -Preflight $helperCases.timeout -Prefix "Timeout preflight"

  Assert-Equal $helperCases.providerError.result "blocked" "Provider-error preflight should remain blocked."
  Assert-TextContains (@($helperCases.providerError.blockingCategories) -join "`n") "provider_error" "Preflight should report provider error."
  Assert-PreflightNoSideEffects -Preflight $helperCases.providerError -Prefix "Provider-error preflight"

  Assert-Equal $helperCases.relayMissingBaseUrl.result "blocked" "Relay missing-base-url preflight should remain blocked."
  Assert-Equal $helperCases.relayMissingBaseUrl.baseUrlRequired $true "Relay missing-base-url preflight should require base URL."
  Assert-Equal $helperCases.relayMissingBaseUrl.baseUrlConfigured $false "Relay missing-base-url preflight should report missing base URL."
  Assert-TextContains (@($helperCases.relayMissingBaseUrl.blockingCategories) -join "`n") "missing_base_url" "Relay preflight should report missing base URL."
  Assert-PreflightNoSideEffects -Preflight $helperCases.relayMissingBaseUrl -Prefix "Relay missing-base-url preflight"

  Assert-Equal $helperCases.relayMissingKey.result "blocked" "Relay missing-key preflight should remain blocked."
  Assert-Equal $helperCases.relayMissingKey.keyConfigured $false "Relay missing-key preflight should report missing key."
  Assert-TextContains (@($helperCases.relayMissingKey.blockingCategories) -join "`n") "missing_key" "Relay preflight should report missing key."
  Assert-PreflightNoSideEffects -Preflight $helperCases.relayMissingKey -Prefix "Relay missing-key preflight"

  Assert-Equal $helperCases.relayInvalidBaseUrl.result "blocked" "Relay invalid-base-url preflight should remain blocked."
  Assert-Equal $helperCases.relayInvalidBaseUrl.baseUrlConfigured $true "Relay invalid-base-url preflight should report configured base URL."
  Assert-Equal $helperCases.relayInvalidBaseUrl.baseUrlValid $false "Relay invalid-base-url preflight should reject unsafe base URL."
  Assert-TextContains (@($helperCases.relayInvalidBaseUrl.blockingCategories) -join "`n") "invalid_base_url" "Relay preflight should report invalid base URL."
  Assert-PreflightNoSideEffects -Preflight $helperCases.relayInvalidBaseUrl -Prefix "Relay invalid-base-url preflight"

  Assert-Equal $helperCases.relayValidBaseUrlBlocked.result "blocked" "Relay valid-base-url preflight should still remain blocked."
  Assert-Equal $helperCases.relayValidBaseUrlBlocked.baseUrlConfigured $true "Relay valid-base-url preflight should report configured base URL."
  Assert-Equal $helperCases.relayValidBaseUrlBlocked.baseUrlValid $true "Relay valid-base-url preflight should accept safe URL shape."
  Assert-TextContains (@($helperCases.relayValidBaseUrlBlocked.blockingCategories) -join "`n") "feature_disabled" "Relay valid-base-url preflight should remain feature-disabled."
  Assert-PreflightNoSideEffects -Preflight $helperCases.relayValidBaseUrlBlocked -Prefix "Relay valid-base-url preflight"

  Assert-OpenAiCompatRelayInterface -RelayInterface $helperCases.relayInterfaceCases.missingKey -ExpectedErrorCategory "missing_key" -Prefix "Relay interface missing-key case"
  Assert-OpenAiCompatRelayInterface -RelayInterface $helperCases.relayInterfaceCases.missingBaseUrl -ExpectedErrorCategory "invalid_request" -Prefix "Relay interface missing-base-url case"
  Assert-OpenAiCompatRelayInterface -RelayInterface $helperCases.relayInterfaceCases.invalidBaseUrl -ExpectedErrorCategory "invalid_request" -Prefix "Relay interface invalid-base-url case"
  Assert-OpenAiCompatRelayInterface -RelayInterface $helperCases.relayInterfaceCases.unsupportedProvider -ExpectedErrorCategory "unsupported_provider" -Prefix "Relay interface unsupported-provider case"
  Assert-OpenAiCompatRelayInterface -RelayInterface $helperCases.relayInterfaceCases.unsupportedModel -ExpectedErrorCategory "unsupported_model" -Prefix "Relay interface unsupported-model case"
  Assert-OpenAiCompatRelayInterface -RelayInterface $helperCases.relayInterfaceCases.timeout -ExpectedErrorCategory "timeout" -Prefix "Relay interface timeout case"
  Assert-OpenAiCompatRelayInterface -RelayInterface $helperCases.relayInterfaceCases.providerError -ExpectedErrorCategory "provider_unavailable" -Prefix "Relay interface provider-error case"
  Assert-OpenAiCompatRelayInterface -RelayInterface $helperCases.relayInterfaceCases.featureDisabled -ExpectedErrorCategory "feature_disabled" -Prefix "Relay interface feature-disabled case"

  Write-Step "Verify provider-specific disabled adapter HTTP responses."
  $providerAdapterCases = @(
    @{ Provider = "openai_compat"; Model = "gpt-5.4-mini"; AdapterId = "openai_compat_disabled_connectivity_adapter" },
    @{ Provider = "anthropic"; Model = "claude-3-5-haiku-latest"; AdapterId = "anthropic_disabled_connectivity_adapter" },
    @{ Provider = "google"; Model = "gemini-1.5-flash"; AdapterId = "google_disabled_connectivity_adapter" }
  )

  foreach ($case in $providerAdapterCases) {
    $providerConnectivityTest = Invoke-Json -Method "POST" -Path "/api/model-gateway/connectivity-test" -Body @{
      provider = $case.Provider
      model = $case.Model
      purpose = "manual_connectivity_test"
      secondConfirm = $true
      confirmText = "I understand this will make one real provider connectivity request."
      requestedBy = "local_user"
    }
    Assert-Equal $providerConnectivityTest.ok $false "$($case.Provider) connectivity-test stub should remain blocked."
    Assert-Equal $providerConnectivityTest.requestValid $true "$($case.Provider) connectivity-test request should be valid."
    Assert-Equal $providerConnectivityTest.providerSupported $true "$($case.Provider) should be recognized by connectivity-test stub."
    Assert-DisabledConnectivityAdapter -ConnectivityTest $providerConnectivityTest -ExpectedProviderAdapterId $case.AdapterId -Prefix "$($case.Provider) connectivity-test"
    Assert-ConnectivityTestNoSideEffects -ConnectivityTest $providerConnectivityTest -Prefix "$($case.Provider) connectivity-test"
  }

  Write-Step "Verify connectivity-test negative request cases."
  $unknownProviderConnectivityTest = Invoke-Json -Method "POST" -Path "/api/model-gateway/connectivity-test" -Body @{
    provider = "unknown"
    model = "gpt-4.1-mini"
    purpose = "manual_connectivity_test"
    secondConfirm = $true
    confirmText = "I understand this will make one real provider connectivity request."
    requestedBy = "local_user"
  }
  Assert-Equal $unknownProviderConnectivityTest.requestValid $false "Unknown provider connectivity-test should be invalid."
  Assert-Equal $unknownProviderConnectivityTest.providerSupported $false "Unknown provider should not be supported by connectivity-test."
  Assert-Equal $unknownProviderConnectivityTest.realProviderRequestAttempted $false "Unknown provider connectivity-test should not attempt provider requests."
  Assert-ConnectivityTestNoSideEffects -ConnectivityTest $unknownProviderConnectivityTest -Prefix "Unknown provider connectivity-test"

  $missingModelConnectivityTest = Invoke-Json -Method "POST" -Path "/api/model-gateway/connectivity-test" -Body @{
    provider = "openai"
    purpose = "manual_connectivity_test"
    secondConfirm = $true
    confirmText = "I understand this will make one real provider connectivity request."
    requestedBy = "local_user"
  }
  Assert-Equal $missingModelConnectivityTest.requestValid $false "Missing model connectivity-test should be invalid."
  Assert-TextContains (@($missingModelConnectivityTest.validationErrors) -join "`n") "model is required." "Missing model connectivity-test should report validation error."
  Assert-ConnectivityTestNoSideEffects -ConnectivityTest $missingModelConnectivityTest -Prefix "Missing model connectivity-test"

  $invalidPurposeConnectivityTest = Invoke-Json -Method "POST" -Path "/api/model-gateway/connectivity-test" -Body @{
    provider = "openai"
    model = "gpt-4.1-mini"
    purpose = "connectivity_check"
    secondConfirm = $true
    confirmText = "I understand this will make one real provider connectivity request."
    requestedBy = "local_user"
  }
  Assert-Equal $invalidPurposeConnectivityTest.requestValid $false "Invalid purpose connectivity-test should be invalid."
  Assert-TextContains (@($invalidPurposeConnectivityTest.validationErrors) -join "`n") "purpose must be manual_connectivity_test." "Invalid purpose connectivity-test should report validation error."
  Assert-ConnectivityTestNoSideEffects -ConnectivityTest $invalidPurposeConnectivityTest -Prefix "Invalid purpose connectivity-test"

  $missingConfirmConnectivityTest = Invoke-Json -Method "POST" -Path "/api/model-gateway/connectivity-test" -Body @{
    provider = "openai"
    model = "gpt-4.1-mini"
    purpose = "manual_connectivity_test"
    requestedBy = "local_user"
  }
  Assert-Equal $missingConfirmConnectivityTest.requestValid $false "Missing confirmation connectivity-test should be invalid."
  Assert-TextContains (@($missingConfirmConnectivityTest.validationErrors) -join "`n") "secondConfirm must be true." "Missing confirmation connectivity-test should require secondConfirm."
  Assert-TextContains (@($missingConfirmConnectivityTest.validationErrors) -join "`n") "confirmText is required." "Missing confirmation connectivity-test should require confirmText."
  Assert-ConnectivityTestNoSideEffects -ConnectivityTest $missingConfirmConnectivityTest -Prefix "Missing confirmation connectivity-test"

  Write-Step "Verify feature flag cannot activate real requests."
  $previousManualConnectivityEnv = $env:AGENT_SWARM_ENABLE_MODEL_CONNECTIVITY_TEST
  try {
    $env:AGENT_SWARM_ENABLE_MODEL_CONNECTIVITY_TEST = "true"
    $flagBoundaryJson = node -e "const gateway = require('./services/api/model-gateway'); process.stdout.write(JSON.stringify(gateway.modelGatewayConnectivityTest({provider:'openai',model:'gpt-4.1-mini',purpose:'manual_connectivity_test',secondConfirm:true,confirmText:'local feature flag boundary'})));"
    $flagBoundary = $flagBoundaryJson | ConvertFrom-Json
    Assert-Equal $flagBoundary.featureFlags.manualConnectivityTestRequested $true "Manual connectivity env var should be reported as requested when set."
    Assert-Equal $flagBoundary.featureFlags.manualConnectivityTestActive $false "Manual connectivity env var should not activate connectivity tests in MVP-0.6."
    Assert-Equal $flagBoundary.featureFlags.realProviderRequestsAllowed $false "Manual connectivity env var should not allow provider requests in MVP-0.6."
    Assert-DisabledConnectivityAdapter -ConnectivityTest $flagBoundary -ExpectedProviderAdapterId "openai_disabled_connectivity_adapter" -Prefix "Manual connectivity env var boundary"
    Assert-ConnectivityTestNoSideEffects -ConnectivityTest $flagBoundary -Prefix "Manual connectivity env var boundary"
  } finally {
    $env:AGENT_SWARM_ENABLE_MODEL_CONNECTIVITY_TEST = $previousManualConnectivityEnv
  }

  Write-Step "Model Gateway verification checks passed."
} finally {
  Pop-Location
}
