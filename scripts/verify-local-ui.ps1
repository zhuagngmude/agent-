$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
$apiBase = "http://127.0.0.1:8787"
$webUrl = "http://127.0.0.1:5175/index.html"
$projectId = "project_agent_swarm"
$session = "agent-swarm-local-ui"
$playwrightWorkDir = Join-Path $env:TEMP "agent-swarm-playwright-cli"

function Write-Step {
  param([string]$Message)
  Write-Host "[local-ui] $Message"
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
  Assert-Equal $FeatureFlags.manualConnectivityTestActive $false "$Prefix should not be active in MVP-0.2."
  Assert-Equal $FeatureFlags.realProviderRequestsAllowed $false "$Prefix should not allow provider requests in MVP-0.2."
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

function Test-Command {
  param([string]$Name)
  return $null -ne (Get-Command $Name -ErrorAction SilentlyContinue)
}

function Get-EdgePath {
  $paths = @(
    "C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
    "C:\Program Files\Microsoft\Edge\Application\msedge.exe"
  )

  foreach ($path in $paths) {
    if (Test-Path $path) {
      return $path
    }
  }

  return ""
}

function Invoke-PlaywrightCli {
  param([Parameter(Mandatory = $true)][string[]]$Arguments)

  Push-Location $playwrightWorkDir
  try {
    $previousErrorActionPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    $output = & npx --package "@playwright/cli" playwright-cli @Arguments 2>&1
    $exitCode = $LASTEXITCODE
    $ErrorActionPreference = $previousErrorActionPreference

    $filteredOutput = @($output | ForEach-Object { "$_" } | Where-Object {
      -not $_.StartsWith("npm warn ")
    })

    if ($exitCode -ne 0) {
      throw ($filteredOutput | Out-String)
    }
    return ($filteredOutput | Out-String)
  } finally {
    if ($null -ne $previousErrorActionPreference) {
      $ErrorActionPreference = $previousErrorActionPreference
    }
    Pop-Location
  }
}

function Invoke-PageEval {
  param([Parameter(Mandatory = $true)][string]$Expression)

  $raw = Invoke-PlaywrightCli -Arguments @("-s=$session", "--json", "eval", $Expression)
  $outer = $raw | ConvertFrom-Json
  if ($outer.isError -eq $true) {
    throw $outer.error
  }

  if ($null -eq $outer.result) {
    throw "Playwright eval did not return a result. Raw output: $raw"
  }

  $result = $outer.result
  if ($result -is [string]) {
    $decoded = $result | ConvertFrom-Json
    if ($decoded -is [string]) {
      $trimmed = $decoded.TrimStart()
      if ($trimmed.StartsWith("{") -or $trimmed.StartsWith("[")) {
        return $decoded | ConvertFrom-Json
      }
      return $decoded
    }
    return $decoded
  }

  return $result
}

function Invoke-PageText {
  param([Parameter(Mandatory = $true)][string]$Selector)

  $escapedSelector = $Selector.Replace("\", "\\").Replace("'", "\'")
  return Invoke-PageEval -Expression "() => document.querySelector('$escapedSelector')?.innerText || ''"
}

function Invoke-PageClickByDataPage {
  param([Parameter(Mandatory = $true)][string]$Page)

  $escapedPage = $Page.Replace("\", "\\").Replace("'", "\'")
  Invoke-PageEval -Expression "() => { const el = Array.from(document.querySelectorAll('[data-page]')).find((item) => item.dataset.page === '$escapedPage'); if (el) el.click(); return true; }" | Out-Null
}

Write-Step "Verify local trial API is ready."
$health = Invoke-Json -Path "/api/health"
Assert-Equal $health.ok $true "API health should be ok."
Assert-Equal $health.projectId $projectId "API project id mismatch."

$runtime = Invoke-Json -Path "/api/runtime-state"
Assert-Equal $runtime.mode "sqlite" "Local trial should run in SQLite mode."
Assert-Equal $runtime.localTrial.safety.realModelCalls $false "Local trial should not allow real model calls."
Assert-Equal $runtime.localTrial.safety.runnerExecutesCommands $false "Runner command execution should be disabled."
Assert-Equal $runtime.localTrial.safety.runnerWritesFiles $false "Runner file writes should be disabled."
Assert-Equal $runtime.localTrial.safety.cloudSync $false "Cloud sync should be disabled."

$gateway = Invoke-Json -Path "/api/model-gateway/status"
Assert-Equal $gateway.enabled $false "Model Gateway should be disabled."
Assert-Equal $gateway.realModelCallsAllowed $false "Real model calls should be disabled."
Assert-Equal $gateway.safety.exposesApiKeysToFrontend $false "API keys should not be exposed to the frontend."
Assert-Equal $gateway.safety.writesDatabase $false "Model Gateway status should not write the database."
Assert-Equal $gateway.safety.createsRunnerJobs $false "Model Gateway status should not create Runner jobs."
Assert-Equal $gateway.safety.makesNetworkRequests $false "Model Gateway status should not make provider network requests."
Assert-ModelGatewayFeatureFlags -FeatureFlags $gateway.featureFlags -Prefix "Model Gateway status feature flags"
Assert-Equal (($gateway.providers | Where-Object { $_.id -eq "openai" }).providerAdapterId) "openai_disabled_connectivity_adapter" "OpenAI status should expose disabled adapter id."
Assert-Equal (($gateway.providers | Where-Object { $_.id -eq "openai_compat" }).providerAdapterId) "openai_compat_disabled_connectivity_adapter" "OpenAI-compatible relay status should expose disabled adapter id."
Assert-Equal (($gateway.providers | Where-Object { $_.id -eq "openai_compat" }).keyEnvVar) "AGENT_SWARM_OPENAI_COMPAT_API_KEY" "OpenAI-compatible relay should use a dedicated key env var."
Assert-Equal (($gateway.providers | Where-Object { $_.id -eq "openai_compat" }).baseUrlEnvVar) "AGENT_SWARM_OPENAI_COMPAT_BASE_URL" "OpenAI-compatible relay should use a dedicated base URL env var."
Assert-Equal (($gateway.providers | Where-Object { $_.id -eq "openai_compat" }).baseUrlRequired) $true "OpenAI-compatible relay should require a server-side base URL."
Assert-Equal (($gateway.providers | Where-Object { $_.id -eq "anthropic" }).providerAdapterId) "anthropic_disabled_connectivity_adapter" "Anthropic status should expose disabled adapter id."
Assert-Equal (($gateway.providers | Where-Object { $_.id -eq "google" }).providerAdapterId) "google_disabled_connectivity_adapter" "Google status should expose disabled adapter id."
foreach ($provider in $gateway.providers) {
  Assert-Equal $provider.providerAdapterMode "disabled" "Provider $($provider.id) adapter mode should remain disabled."
}

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

$preflightJson = node -e @"
const gateway = require('./services/api/model-gateway');
const base = {
  provider: 'openai',
  model: 'gpt-4.1-mini',
  purpose: 'manual_connectivity_test',
  secondConfirm: true,
  confirmText: 'local preflight acceptance'
};
const relayBase = {
  provider: 'openai_compat',
  model: 'openai-compatible-relay-model',
  purpose: 'manual_connectivity_test',
  secondConfirm: true,
  confirmText: 'local relay preflight acceptance'
};
const cases = {
  featureDisabled: gateway.modelGatewayConnectivityPreflight(base, { acceptanceOnlyKeyConfigured: true }),
  missingKey: gateway.modelGatewayConnectivityPreflight(base, { acceptanceOnlyKeyConfigured: false }),
  unsupportedProvider: gateway.modelGatewayConnectivityPreflight({ ...base, provider: 'unknown' }),
  unsupportedModel: gateway.modelGatewayConnectivityPreflight({ ...base, model: 'not-a-connectivity-model' }, { acceptanceOnlyKeyConfigured: true }),
  invalidPurpose: gateway.modelGatewayConnectivityPreflight({ ...base, purpose: 'chat_completion' }, { acceptanceOnlyKeyConfigured: true }),
  timeout: gateway.modelGatewayConnectivityPreflight(base, { acceptanceOnlyKeyConfigured: true, acceptanceSimulation: 'timeout' }),
  providerError: gateway.modelGatewayConnectivityPreflight(base, { acceptanceOnlyKeyConfigured: true, acceptanceSimulation: 'provider_error' }),
  relayMissingBaseUrl: gateway.modelGatewayConnectivityPreflight(relayBase, { acceptanceOnlyKeyConfigured: true, acceptanceOnlyBaseUrl: '' }),
  relayInvalidBaseUrl: gateway.modelGatewayConnectivityPreflight(relayBase, { acceptanceOnlyKeyConfigured: true, acceptanceOnlyBaseUrl: 'http://127.0.0.1:8787/v1' }),
  relayValidBaseUrlBlocked: gateway.modelGatewayConnectivityPreflight(relayBase, { acceptanceOnlyKeyConfigured: true, acceptanceOnlyBaseUrl: 'https://relay.example.test/v1' })
};
process.stdout.write(JSON.stringify(cases));
"@
$preflightCases = $preflightJson | ConvertFrom-Json

Assert-Equal $preflightCases.featureDisabled.result "blocked" "Preflight should remain blocked when feature is disabled."
Assert-TextContains (@($preflightCases.featureDisabled.blockingCategories) -join "`n") "feature_disabled" "Preflight should report feature disabled."
Assert-PreflightNoSideEffects -Preflight $preflightCases.featureDisabled -Prefix "Feature-disabled preflight"

Assert-Equal $preflightCases.missingKey.result "blocked" "Missing-key preflight should remain blocked."
Assert-TextContains (@($preflightCases.missingKey.blockingCategories) -join "`n") "missing_key" "Preflight should report missing key."
Assert-PreflightNoSideEffects -Preflight $preflightCases.missingKey -Prefix "Missing-key preflight"

Assert-Equal $preflightCases.unsupportedProvider.requestValid $false "Unsupported-provider preflight should be invalid."
Assert-Equal $preflightCases.unsupportedProvider.providerSupported $false "Unsupported-provider preflight should not support provider."
Assert-TextContains (@($preflightCases.unsupportedProvider.blockingCategories) -join "`n") "unsupported_provider" "Preflight should report unsupported provider."
Assert-PreflightNoSideEffects -Preflight $preflightCases.unsupportedProvider -Prefix "Unsupported-provider preflight"

Assert-Equal $preflightCases.unsupportedModel.requestValid $false "Unsupported-model preflight should be invalid."
Assert-Equal $preflightCases.unsupportedModel.modelSupported $false "Unsupported-model preflight should not support model."
Assert-TextContains (@($preflightCases.unsupportedModel.blockingCategories) -join "`n") "unsupported_model" "Preflight should report unsupported model."
Assert-PreflightNoSideEffects -Preflight $preflightCases.unsupportedModel -Prefix "Unsupported-model preflight"

Assert-Equal $preflightCases.invalidPurpose.requestValid $false "Invalid-purpose preflight should be invalid."
Assert-TextContains (@($preflightCases.invalidPurpose.validationErrors) -join "`n") "purpose must be manual_connectivity_test." "Preflight should report invalid purpose."
Assert-PreflightNoSideEffects -Preflight $preflightCases.invalidPurpose -Prefix "Invalid-purpose preflight"

Assert-Equal $preflightCases.timeout.result "blocked" "Timeout preflight should remain blocked."
Assert-TextContains (@($preflightCases.timeout.blockingCategories) -join "`n") "timeout" "Preflight should report timeout."
Assert-PreflightNoSideEffects -Preflight $preflightCases.timeout -Prefix "Timeout preflight"

Assert-Equal $preflightCases.providerError.result "blocked" "Provider-error preflight should remain blocked."
Assert-TextContains (@($preflightCases.providerError.blockingCategories) -join "`n") "provider_error" "Preflight should report provider error."
Assert-PreflightNoSideEffects -Preflight $preflightCases.providerError -Prefix "Provider-error preflight"

Assert-Equal $preflightCases.relayMissingBaseUrl.result "blocked" "Relay missing-base-url preflight should remain blocked."
Assert-Equal $preflightCases.relayMissingBaseUrl.baseUrlRequired $true "Relay missing-base-url preflight should require base URL."
Assert-Equal $preflightCases.relayMissingBaseUrl.baseUrlConfigured $false "Relay missing-base-url preflight should report missing base URL."
Assert-TextContains (@($preflightCases.relayMissingBaseUrl.blockingCategories) -join "`n") "missing_base_url" "Relay preflight should report missing base URL."
Assert-PreflightNoSideEffects -Preflight $preflightCases.relayMissingBaseUrl -Prefix "Relay missing-base-url preflight"

Assert-Equal $preflightCases.relayInvalidBaseUrl.result "blocked" "Relay invalid-base-url preflight should remain blocked."
Assert-Equal $preflightCases.relayInvalidBaseUrl.baseUrlConfigured $true "Relay invalid-base-url preflight should report configured base URL."
Assert-Equal $preflightCases.relayInvalidBaseUrl.baseUrlValid $false "Relay invalid-base-url preflight should reject unsafe base URL."
Assert-TextContains (@($preflightCases.relayInvalidBaseUrl.blockingCategories) -join "`n") "invalid_base_url" "Relay preflight should report invalid base URL."
Assert-PreflightNoSideEffects -Preflight $preflightCases.relayInvalidBaseUrl -Prefix "Relay invalid-base-url preflight"

Assert-Equal $preflightCases.relayValidBaseUrlBlocked.result "blocked" "Relay valid-base-url preflight should still remain blocked."
Assert-Equal $preflightCases.relayValidBaseUrlBlocked.baseUrlConfigured $true "Relay valid-base-url preflight should report configured base URL."
Assert-Equal $preflightCases.relayValidBaseUrlBlocked.baseUrlValid $true "Relay valid-base-url preflight should accept safe URL shape."
Assert-TextContains (@($preflightCases.relayValidBaseUrlBlocked.blockingCategories) -join "`n") "feature_disabled" "Relay valid-base-url preflight should remain feature-disabled."
Assert-PreflightNoSideEffects -Preflight $preflightCases.relayValidBaseUrlBlocked -Prefix "Relay valid-base-url preflight"

$providerAdapterCases = @(
  @{ Provider = "openai_compat"; Model = "openai-compatible-relay-model"; AdapterId = "openai_compat_disabled_connectivity_adapter" },
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

$previousManualConnectivityEnv = $env:AGENT_SWARM_ENABLE_MODEL_CONNECTIVITY_TEST
try {
  $env:AGENT_SWARM_ENABLE_MODEL_CONNECTIVITY_TEST = "true"
  $flagBoundaryJson = node -e "const gateway = require('./services/api/model-gateway'); process.stdout.write(JSON.stringify(gateway.modelGatewayConnectivityTest({provider:'openai',model:'gpt-4.1-mini',purpose:'manual_connectivity_test',secondConfirm:true,confirmText:'local feature flag boundary'})));"
  $flagBoundary = $flagBoundaryJson | ConvertFrom-Json
  Assert-Equal $flagBoundary.featureFlags.manualConnectivityTestRequested $true "Manual connectivity env var should be reported as requested when set."
  Assert-Equal $flagBoundary.featureFlags.manualConnectivityTestActive $false "Manual connectivity env var should not activate connectivity tests in MVP-0.2."
  Assert-Equal $flagBoundary.featureFlags.realProviderRequestsAllowed $false "Manual connectivity env var should not allow provider requests in MVP-0.2."
  Assert-DisabledConnectivityAdapter -ConnectivityTest $flagBoundary -ExpectedProviderAdapterId "openai_disabled_connectivity_adapter" -Prefix "Manual connectivity env var boundary"
} finally {
  $env:AGENT_SWARM_ENABLE_MODEL_CONNECTIVITY_TEST = $previousManualConnectivityEnv
}

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

$runner = Invoke-Json -Path "/api/projects/$projectId/runner/status"
Assert-Equal $runner.permissions.writeFiles "approval_required" "Runner writes should remain approval-gated."
Assert-Equal $runner.permissions.executeCommands "approval_required" "Runner command execution should remain approval-gated."
Assert-Equal $runner.permissions.networkRequests "approval_required" "Runner network requests should remain approval-gated."

Write-Step "Verify browser prerequisites."
if (-not (Test-Command "npx")) {
  throw "npx is required for Playwright CLI. Install Node.js/npm first."
}

$edgePath = Get-EdgePath
if (-not $edgePath) {
  throw "Microsoft Edge was not found. Install Edge or update Get-EdgePath in this script."
}

New-Item -ItemType Directory -Force -Path $playwrightWorkDir | Out-Null

Write-Step "Open Web App in Microsoft Edge through Playwright CLI."
Invoke-PlaywrightCli -Arguments @("-s=$session", "open", $webUrl, "--browser", "msedge") | Out-Null

try {
  Write-Step "Verify core pages and safety copy in the DOM."
  Assert-TextContains (Invoke-PageEval -Expression "() => document.title") "agent" "Page title should identify the app."
  Assert-TextContains (Invoke-PageText -Selector "#apiStatus") "API" "Top bar should show local API status."

  Invoke-PageClickByDataPage -Page "overview"
  Assert-TextContains (Invoke-PageText -Selector "#overview") "Token" "Overview should include the model usage placeholder."

  Invoke-PageClickByDataPage -Page "tasks"
  Assert-True ((Invoke-PageText -Selector "#taskDetail").Length -gt 0) "Tasks page should render task details."
  Assert-True ((Invoke-PageEval -Expression "() => document.querySelectorAll('#startTaskAction, #completeTaskAction, #failTaskAction, #cancelTaskAction').length") -eq 4) "Task action buttons should be present."

  Invoke-PageClickByDataPage -Page "approval"
  Assert-TextContains (Invoke-PageText -Selector "#approval") "Runner job" "Approval page should describe Runner jobs."

  Invoke-PageClickByDataPage -Page "runtime"
  Assert-TextContains (Invoke-PageText -Selector "#runtime") "Runner Job" "Runtime page should render Runner job details."

  Invoke-PageClickByDataPage -Page "settings"
  Assert-True ((Invoke-PageText -Selector "#localTrialStatus").Length -gt 0) "Settings page should render local trial status."
  Assert-TextContains (Invoke-PageText -Selector "#modelGatewaySettingsStatus") "API keys stay server-side" "Settings page should render Model Gateway safety copy."
  Assert-TextContains (Invoke-PageText -Selector "#modelGatewaySettingsStatus") "Connectivity Dry-Run" "Settings page should render Model Gateway dry-run preview."
  Assert-TextContains (Invoke-PageText -Selector "#modelGatewaySettingsStatus") "Would call provider" "Settings page should render dry-run provider call boundary."
  Assert-TextContains (Invoke-PageText -Selector "#modelGatewaySettingsStatus") "Prompt/result logging" "Settings page should render dry-run logging boundary."

  Invoke-PageClickByDataPage -Page "integrations"
  Assert-TextContains (Invoke-PageText -Selector "#integrations") "GitHub" "Integrations page should render integration cards."
  Assert-TextContains (Invoke-PageText -Selector "#modelGatewayIntegrationStatus") "Real calls" "Integrations page should render Model Gateway status."
  Assert-TextContains (Invoke-PageText -Selector "#modelGatewayIntegrationStatus") "Connectivity Dry-Run" "Integrations page should render Model Gateway dry-run preview."
  Assert-Equal (Invoke-PageText -Selector "#modelGatewayIntegrationBadge") "disabled" "Model Gateway badge should be disabled."

  $hrefHashCount = Invoke-PageEval -Expression "() => Array.from(document.querySelectorAll('a')).filter((item) => item.getAttribute('href') === '#').length"
  Assert-Equal $hrefHashCount 0 "No href=# placeholder links should remain."
  $disabledButtons = Invoke-PageEval -Expression "() => Array.from(document.querySelectorAll('button:disabled')).length"
  Assert-True ($disabledButtons -gt 0) "Unimplemented controls should remain visibly disabled."

  Write-Step "Verify browser console has no errors or warnings."
  $consoleOutput = Invoke-PlaywrightCli -Arguments @("-s=$session", "console")
  Assert-TextContains $consoleOutput "Errors: 0, Warnings: 0" "Browser console should be clean."

  Write-Step "Local UI smoke checks passed."
} finally {
  Write-Step "Close Playwright browser session."
  try {
    Invoke-PlaywrightCli -Arguments @("-s=$session", "close") | Out-Null
  } catch {
    Write-Warning "Failed to close Playwright session: $($_.Exception.Message)"
  }
}
