$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot

function Write-Step {
  param([string]$Message)
  Write-Host "[agent-permissions] $Message"
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

function Assert-NoSideEffects {
  param(
    [Parameter(Mandatory = $true)][object]$Result,
    [string]$Prefix = "Agent permission validation"
  )

  Assert-Equal $Result.sideEffects.writesSqlite $false "$Prefix should not write SQLite."
  Assert-Equal $Result.sideEffects.writesRuntimeState $false "$Prefix should not write runtime state."
  Assert-Equal $Result.sideEffects.createsTasks $false "$Prefix should not create tasks."
  Assert-Equal $Result.sideEffects.createsApprovals $false "$Prefix should not create approvals."
  Assert-Equal $Result.sideEffects.createsRunnerJobs $false "$Prefix should not create Runner jobs."
  Assert-Equal $Result.sideEffects.triggersAgents $false "$Prefix should not trigger Agents."
  Assert-Equal $Result.sideEffects.executesRunner $false "$Prefix should not execute Runner."
  Assert-Equal $Result.sideEffects.callsRealModel $false "$Prefix should not call real models."
  Assert-Equal $Result.sideEffects.readsRawSecrets $false "$Prefix should not read raw secrets."
}

Push-Location $root
try {
  Write-Step "Load Agent permission helper cases."
  $casesJson = node -e @"
const permissions = require('./services/api/agent-permissions');
const cases = {
  profiles: permissions.agentPermissionProfiles,
  knownCapabilities: permissions.allKnownCapabilities(),
  forbidden: permissions.forbiddenAgentCapabilities,
  architect: permissions.validateAgentCapabilities({ profile: 'architect_admin' }),
  executor: permissions.validateAgentCapabilities({ profile: 'executor_agent' }),
  reviewer: permissions.validateAgentCapabilities({ profile: 'reviewer_agent' }),
  fullManagement: permissions.validateAgentCapabilities({ profile: 'all_agents_full_management' }),
  unknownProfile: permissions.validateAgentCapabilities({ profile: 'root_admin' }),
  allFlag: permissions.validateAgentCapabilities({ all: true, capabilities: ['canViewProject'] }),
  forbiddenCapability: permissions.validateAgentCapabilities({ capabilities: ['canViewProject', 'canExecuteRunnerJob'] }),
  rawSecretCapability: permissions.validateAgentCapabilities({ capabilities: ['canReferenceSecretPresence', 'canAccessRawSecrets'] }),
  unknownCapability: permissions.validateAgentCapabilities({ capabilities: ['canViewProject', 'canTeleport'] }),
  explicitSafeCapabilities: permissions.validateAgentCapabilities({ capabilities: ['canViewProject', 'canReadKnowledge', 'canRequestExecution'] }),
  expandedMissing: permissions.expandAgentPermissionProfile('missing_profile')
};
process.stdout.write(JSON.stringify(cases));
"@
  $cases = $casesJson | ConvertFrom-Json

  Write-Step "Verify profile registry."
  Assert-True (($cases.knownCapabilities | Measure-Object).Count -gt 0) "Known capabilities should not be empty."
  Assert-TextContains (@($cases.forbidden) -join "`n") "canApproveOwnRequest" "Forbidden capabilities should include self approval."
  Assert-TextContains (@($cases.forbidden) -join "`n") "canApproveHighRisk" "Forbidden capabilities should include high-risk approval."
  Assert-TextContains (@($cases.forbidden) -join "`n") "canExecuteRunnerJob" "Forbidden capabilities should include Runner execution."
  Assert-TextContains (@($cases.forbidden) -join "`n") "canAccessRawSecrets" "Forbidden capabilities should include raw secret access."

  Write-Step "Verify safe built-in profiles."
  foreach ($profileCase in @(
    @{ Name = "architect_admin"; Result = $cases.architect },
    @{ Name = "executor_agent"; Result = $cases.executor },
    @{ Name = "reviewer_agent"; Result = $cases.reviewer },
    @{ Name = "all_agents_full_management"; Result = $cases.fullManagement }
  )) {
    Assert-Equal $profileCase.Result.ok $true "$($profileCase.Name) should validate."
    Assert-Equal (($profileCase.Result.forbiddenCapabilities | Measure-Object).Count) 0 "$($profileCase.Name) should not include forbidden capabilities."
    Assert-Equal (($profileCase.Result.unknownCapabilities | Measure-Object).Count) 0 "$($profileCase.Name) should not include unknown capabilities."
    Assert-NoSideEffects -Result $profileCase.Result -Prefix "$($profileCase.Name) validation"
  }

  Assert-TextContains (@($cases.architect.capabilities) -join "`n") "canPlanArchitecture" "architect_admin should include planning authority."
  Assert-TextContains (@($cases.architect.capabilities) -join "`n") "canRequestExecution" "architect_admin should include request authority."
  Assert-TextContains (@($cases.architect.capabilities) -join "`n") "canRequestSecretUse" "architect_admin should include guarded secret-use request authority."
  Assert-True (-not ((@($cases.architect.capabilities) -join "`n").Contains("canApproveOwnRequest"))) "architect_admin should not self-approve."
  Assert-True (-not ((@($cases.architect.capabilities) -join "`n").Contains("canExecuteRunnerJob"))) "architect_admin should not execute Runner jobs."
  Assert-True (-not ((@($cases.architect.capabilities) -join "`n").Contains("canAccessRawSecrets"))) "architect_admin should not access raw secrets."

  Assert-TextContains (@($cases.fullManagement.capabilities) -join "`n") "canRequestNetwork" "all_agents_full_management should include broad request authority."
  Assert-True (-not ((@($cases.fullManagement.capabilities) -join "`n").Contains("canApproveHighRisk"))) "all_agents_full_management should not approve high risk."
  Assert-True (-not ((@($cases.fullManagement.capabilities) -join "`n").Contains("canWriteFiles"))) "all_agents_full_management should not write files directly."
  Assert-True (-not ((@($cases.fullManagement.capabilities) -join "`n").Contains("canMakeNetworkRequests"))) "all_agents_full_management should not make network requests directly."

  Write-Step "Verify invalid contracts are rejected."
  Assert-Equal $cases.unknownProfile.ok $false "Unknown profile should be rejected."
  Assert-TextContains (@($cases.unknownProfile.validationErrors) -join "`n") "permission profile is not supported." "Unknown profile should report validation error."
  Assert-NoSideEffects -Result $cases.unknownProfile -Prefix "Unknown profile validation"

  Assert-Equal $cases.allFlag.ok $false "all=true should be rejected."
  Assert-Equal $cases.allFlag.allFlagRequested $true "all=true should be reported."
  Assert-TextContains (@($cases.allFlag.validationErrors) -join "`n") "all=true is not a valid Agent permission contract." "all=true should report validation error."
  Assert-NoSideEffects -Result $cases.allFlag -Prefix "all=true validation"

  Assert-Equal $cases.forbiddenCapability.ok $false "Forbidden execution capability should be rejected."
  Assert-TextContains (@($cases.forbiddenCapability.forbiddenCapabilities) -join "`n") "canExecuteRunnerJob" "Forbidden execution capability should be identified."
  Assert-NoSideEffects -Result $cases.forbiddenCapability -Prefix "Forbidden execution validation"

  Assert-Equal $cases.rawSecretCapability.ok $false "Raw secret capability should be rejected."
  Assert-TextContains (@($cases.rawSecretCapability.forbiddenCapabilities) -join "`n") "canAccessRawSecrets" "Raw secret capability should be identified."
  Assert-NoSideEffects -Result $cases.rawSecretCapability -Prefix "Raw secret validation"

  Assert-Equal $cases.unknownCapability.ok $false "Unknown capability should be rejected."
  Assert-TextContains (@($cases.unknownCapability.unknownCapabilities) -join "`n") "canTeleport" "Unknown capability should be identified."
  Assert-NoSideEffects -Result $cases.unknownCapability -Prefix "Unknown capability validation"

  Assert-Equal $cases.explicitSafeCapabilities.ok $true "Explicit safe capabilities should validate."
  Assert-Equal (($cases.explicitSafeCapabilities.forbiddenCapabilities | Measure-Object).Count) 0 "Explicit safe capabilities should not include forbidden capabilities."
  Assert-NoSideEffects -Result $cases.explicitSafeCapabilities -Prefix "Explicit safe capabilities validation"

  Assert-Equal $cases.expandedMissing.ok $false "Missing profile expansion should fail."
  Assert-TextContains (@($cases.expandedMissing.validationErrors) -join "`n") "permission profile is not supported." "Missing profile expansion should report validation error."

  Write-Step "Agent permission profile checks passed."
} finally {
  Pop-Location
}
