$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot

function Write-Step {
  param([string]$Message)
  Write-Host "[agent-config-fields] $Message"
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
    [object]$Result,
    [string]$Prefix
  )

  Assert-Equal $Result.sideEffects.writesAgents $false "$Prefix should not write Agents."
  Assert-Equal $Result.sideEffects.writesAgentConfigVersions $false "$Prefix should not write versions."
  Assert-Equal $Result.sideEffects.writesRuntimeEvents $false "$Prefix should not write runtime events."
  Assert-Equal $Result.sideEffects.writesSqlite $false "$Prefix should not write SQLite."
  Assert-Equal $Result.sideEffects.writesRuntimeState $false "$Prefix should not write runtime state."
  Assert-Equal $Result.sideEffects.createsApprovals $false "$Prefix should not create approvals."
  Assert-Equal $Result.sideEffects.createsRunnerJobs $false "$Prefix should not create Runner jobs."
  Assert-Equal $Result.sideEffects.executesRunner $false "$Prefix should not execute Runner."
  Assert-Equal $Result.sideEffects.callsRealModel $false "$Prefix should not call real models."
  Assert-Equal $Result.sideEffects.readsRawSecrets $false "$Prefix should not read raw secrets."
}

Push-Location $root
try {
  Write-Step "Load Agent config field validation cases."
  $casesJson = node -e @"
const { validateAgentConfigChangePlan } = require('./services/api/agent-config-fields');
const { buildAgentConfigApplyDryRun, buildAgentConfigRealApplyGate } = require('./services/api/server');

const baseApplication = {
  id: 'agent_config_application_fields',
  approvalId: 'approval_agent_fields',
  agentId: 'agent_reviewer',
  changeType: 'permission',
  status: 'pending_apply',
  changes: [{ field: 'permissions', before: 'read_project', after: 'reviewer_agent' }]
};
const approval = {
  id: 'approval_agent_fields',
  status: 'approved',
  targetService: 'agent_config',
  runnerJobId: ''
};
const agent = {
  id: 'agent_reviewer',
  versionNumber: 1
};
const dryRunBody = {
  secondConfirm: true,
  confirmText: 'field validation dry-run',
  requestedBy: 'verify_agent_config_fields'
};
const gateBody = {
  secondConfirm: true,
  confirmText: 'field validation gate',
  requestedBy: 'verify_agent_config_fields',
  gitCheckpoint: { created: true, commit: 'checkpoint_fields' },
  rollbackPlanAccepted: true
};

const validDryRun = buildAgentConfigApplyDryRun({ application: baseApplication, approval, agent, body: dryRunBody });
const invalidApplication = {
  ...baseApplication,
  changes: [{ field: 'apiKey', before: '', after: 'secret-value' }]
};
const invalidDryRun = buildAgentConfigApplyDryRun({ application: invalidApplication, approval, agent, body: dryRunBody });

const cases = {
  validPlan: validateAgentConfigChangePlan({
    changes: [
      { field: 'permissions', before: 'read_project', after: 'reviewer_agent' },
      { field: 'model', before: 'gpt-low', after: 'gpt-high' },
      { field: 'status', before: 'idle', after: 'disabled' },
      { field: 'maxSubAgents', before: 1, after: 3 },
      { field: 'canSpawnSubAgents', before: false, after: true }
    ]
  }),
  unsupportedField: validateAgentConfigChangePlan({ changes: [{ field: 'temperature', before: 0.2, after: 0.8 }] }),
  forbiddenApiKeyField: validateAgentConfigChangePlan({ changes: [{ field: 'apiKey', before: '', after: 'secret-value' }] }),
  forbiddenRunnerField: validateAgentConfigChangePlan({ changes: [{ field: 'runnerCommand', before: '', after: 'npm test' }] }),
  forbiddenParentField: validateAgentConfigChangePlan({ changes: [{ field: 'parentAgentId', before: '', after: 'agent_architect' }] }),
  forbiddenSecretValue: validateAgentConfigChangePlan({ changes: [{ field: 'model', before: 'gpt-low', after: 'api_key=abc' }] }),
  forbiddenPromptValue: validateAgentConfigChangePlan({ changes: [{ field: 'model', before: 'gpt-low', after: 'prompt override' }] }),
  forbiddenLocalPathValue: validateAgentConfigChangePlan({ changes: [{ field: 'model', before: 'gpt-low', after: 'C:/Users/zmd/private.txt' }] }),
  invalidStatus: validateAgentConfigChangePlan({ changes: [{ field: 'status', before: 'idle', after: 'root' }] }),
  invalidMaxSubAgents: validateAgentConfigChangePlan({ changes: [{ field: 'maxSubAgents', before: 1, after: 100 }] }),
  invalidCanSpawnSubAgents: validateAgentConfigChangePlan({ changes: [{ field: 'canSpawnSubAgents', before: false, after: 'yes' }] }),
  forbiddenPermissionCapability: validateAgentConfigChangePlan({ changes: [{ field: 'permissions', before: 'read_project', after: ['canViewProject', 'canExecuteRunnerJob'] }] }),
  allFlagPermission: validateAgentConfigChangePlan({ changes: [{ field: 'permissions', before: 'read_project', after: { all: true } }] }),
  emptyPlan: validateAgentConfigChangePlan({ changes: [] }),
  dryRunInvalidPlan: invalidDryRun,
  gateInvalidPlan: buildAgentConfigRealApplyGate({ application: invalidApplication, approval, agent, dryRun: invalidDryRun, body: gateBody }),
  gateValidPlan: buildAgentConfigRealApplyGate({ application: baseApplication, approval, agent, dryRun: validDryRun, body: gateBody })
};

process.stdout.write(JSON.stringify(cases));
"@
  $cases = $casesJson | ConvertFrom-Json

  Write-Step "Verify allowed fields and value shapes."
  Assert-Equal $cases.validPlan.ok $true "Valid Agent config change plan should pass."
  Assert-TextContains (@($cases.validPlan.allowedFields) -join "`n") "permissions" "Valid plan should allow permissions."
  Assert-TextContains (@($cases.validPlan.allowedFields) -join "`n") "model" "Valid plan should allow model."
  Assert-TextContains (@($cases.validPlan.allowedFields) -join "`n") "status" "Valid plan should allow status."
  Assert-NoSideEffects -Result $cases.validPlan -Prefix "Valid field validation"

  Write-Step "Verify unsupported and forbidden field names."
  Assert-Equal $cases.unsupportedField.ok $false "Unsupported field should fail."
  Assert-TextContains (@($cases.unsupportedField.validationErrors) -join "`n") "unsupported Agent config field: temperature" "Unsupported field should be reported."
  Assert-NoSideEffects -Result $cases.unsupportedField -Prefix "Unsupported field validation"
  Assert-Equal $cases.forbiddenApiKeyField.ok $false "API key field should fail."
  Assert-TextContains (@($cases.forbiddenApiKeyField.validationErrors) -join "`n") "forbidden Agent config field: apiKey" "API key field should be reported."
  Assert-Equal $cases.forbiddenRunnerField.ok $false "Runner field should fail."
  Assert-TextContains (@($cases.forbiddenRunnerField.validationErrors) -join "`n") "forbidden Agent config field: runnerCommand" "Runner field should be reported."
  Assert-Equal $cases.forbiddenParentField.ok $false "Parent Agent field should fail."
  Assert-TextContains (@($cases.forbiddenParentField.validationErrors) -join "`n") "forbidden Agent config field: parentAgentId" "Parent Agent field should be reported."

  Write-Step "Verify forbidden values and invalid shapes."
  Assert-Equal $cases.forbiddenSecretValue.ok $false "Secret-like value should fail."
  Assert-TextContains (@($cases.forbiddenSecretValue.validationErrors) -join "`n") "API key" "Secret-like value should be reported."
  Assert-Equal $cases.forbiddenPromptValue.ok $false "Prompt value should fail."
  Assert-TextContains (@($cases.forbiddenPromptValue.validationErrors) -join "`n") "prompts" "Prompt value should be reported."
  Assert-Equal $cases.forbiddenLocalPathValue.ok $false "Local path value should fail."
  Assert-TextContains (@($cases.forbiddenLocalPathValue.validationErrors) -join "`n") "local private paths" "Local path value should be reported."
  Assert-Equal $cases.invalidStatus.ok $false "Invalid status should fail."
  Assert-TextContains (@($cases.invalidStatus.validationErrors) -join "`n") "supported Agent status" "Invalid status should be reported."
  Assert-Equal $cases.invalidMaxSubAgents.ok $false "Invalid maxSubAgents should fail."
  Assert-TextContains (@($cases.invalidMaxSubAgents.validationErrors) -join "`n") "integer between 0 and 20" "Invalid maxSubAgents should be reported."
  Assert-Equal $cases.invalidCanSpawnSubAgents.ok $false "Invalid canSpawnSubAgents should fail."
  Assert-TextContains (@($cases.invalidCanSpawnSubAgents.validationErrors) -join "`n") "boolean value" "Invalid canSpawnSubAgents should be reported."

  Write-Step "Verify permission capability safety."
  Assert-Equal $cases.forbiddenPermissionCapability.ok $false "Forbidden permission capability should fail."
  Assert-TextContains (@($cases.forbiddenPermissionCapability.validationErrors) -join "`n") "forbidden Agent capability: canExecuteRunnerJob" "Forbidden capability should be reported."
  Assert-Equal $cases.allFlagPermission.ok $false "all=true permission should fail."
  Assert-TextContains (@($cases.allFlagPermission.validationErrors) -join "`n") "all=true is not a valid Agent permission contract." "all=true should be reported."
  Assert-Equal $cases.emptyPlan.ok $false "Empty change plan should fail."
  Assert-TextContains (@($cases.emptyPlan.validationErrors) -join "`n") "changes must be a non-empty array." "Empty plan should be reported."

  Write-Step "Verify dry-run and real apply gate include field validation."
  Assert-Equal $cases.dryRunInvalidPlan.changePlanValidation.ok $false "Dry-run should include invalid field validation."
  Assert-TextContains (@($cases.dryRunInvalidPlan.validationErrors) -join "`n") "forbidden Agent config field: apiKey" "Dry-run should reject forbidden field."
  Assert-Equal $cases.dryRunInvalidPlan.canApply $false "Invalid dry-run should not allow apply."
  Assert-NoSideEffects -Result $cases.dryRunInvalidPlan -Prefix "Invalid field dry-run"
  Assert-Equal $cases.gateInvalidPlan.preconditionsReady $false "Apply gate should reject invalid field plan."
  Assert-TextContains (@($cases.gateInvalidPlan.validationErrors) -join "`n") "forbidden Agent config field: apiKey" "Apply gate should report forbidden field."
  Assert-NoSideEffects -Result $cases.gateInvalidPlan -Prefix "Invalid field apply gate"
  Assert-Equal $cases.gateValidPlan.preconditionsReady $true "Apply gate should accept valid field preconditions."
  Assert-Equal $cases.gateValidPlan.gateReady $false "Apply gate should remain feature-disabled."
  Assert-Equal $cases.gateValidPlan.canApply $false "Apply gate should not allow real apply."
  Assert-NoSideEffects -Result $cases.gateValidPlan -Prefix "Valid field apply gate"

  Write-Step "Agent config field validation checks passed."
} finally {
  Pop-Location
}
