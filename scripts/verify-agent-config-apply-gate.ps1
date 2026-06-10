$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot

function Write-Step {
  param([string]$Message)
  Write-Host "[agent-config-apply-gate] $Message"
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

function Assert-BlockedGate {
  param(
    [object]$Result,
    [string]$Prefix
  )

  Assert-Equal $Result.ok $false "$Prefix should stay blocked."
  Assert-Equal $Result.realApplyGate $true "$Prefix should identify real apply gate."
  Assert-Equal $Result.canApply $false "$Prefix should not allow real apply."
  Assert-TextContains (@($Result.blockedReasons) -join "`n") "feature_disabled" "$Prefix should report feature disabled."
  Assert-NoSideEffects -Result $Result -Prefix $Prefix
}

Push-Location $root
try {
  Write-Step "Load Agent config real apply gate helper cases."
  $casesJson = node -e @"
const { buildAgentConfigApplyDryRun, buildAgentConfigRealApplyGate } = require('./services/api/server');

const application = {
  id: 'agent_config_application_test',
  approvalId: 'approval_agent_test_permission',
  agentId: 'agent_reviewer',
  changeType: 'permission',
  status: 'pending_apply',
  changes: [{ field: 'permissions', before: 'read_project', after: 'reviewer_agent' }]
};
const approval = {
  id: 'approval_agent_test_permission',
  status: 'approved',
  targetService: 'agent_config',
  runnerJobId: ''
};
const agent = {
  id: 'agent_reviewer',
  permissions: ['read_project', 'review_risk', 'review_diff'],
  versionNumber: 1
};
const dryRunBody = {
  secondConfirm: true,
  confirmText: 'dry-run only',
  requestedBy: 'verify_agent_config_apply_gate'
};
const gateBody = {
  secondConfirm: true,
  confirmText: 'prepare real apply gate only',
  requestedBy: 'verify_agent_config_apply_gate',
  gitCheckpoint: { created: true, commit: 'checkpoint_test_commit' },
  rollbackPlanAccepted: true
};
const dryRun = buildAgentConfigApplyDryRun({ application, approval, agent, body: dryRunBody });

const cases = {
  validBlocked: buildAgentConfigRealApplyGate({ application, approval, agent, dryRun, body: gateBody }),
  missingRequestedBy: buildAgentConfigRealApplyGate({ application, approval, agent, dryRun, body: { ...gateBody, requestedBy: '' } }),
  missingGitCheckpoint: buildAgentConfigRealApplyGate({ application, approval, agent, dryRun, body: { ...gateBody, gitCheckpoint: { created: false, commit: '' } } }),
  missingRollbackAcceptance: buildAgentConfigRealApplyGate({ application, approval, agent, dryRun, body: { ...gateBody, rollbackPlanAccepted: false } }),
  missingDryRun: buildAgentConfigRealApplyGate({ application, approval, agent, dryRun: null, body: gateBody }),
  mismatchedDryRun: buildAgentConfigRealApplyGate({ application, approval, agent, dryRun: { ...dryRun, applicationId: 'other_application' }, body: gateBody }),
  dryRunValidationErrors: buildAgentConfigRealApplyGate({ application, approval, agent, dryRun: { ...dryRun, validationErrors: ['forced validation failure'] }, body: gateBody }),
  dryRunNotDisabled: buildAgentConfigRealApplyGate({ application, approval, agent, dryRun: { ...dryRun, ok: true, canApply: true, blockedReasons: [] }, body: gateBody }),
  dryRunSideEffects: buildAgentConfigRealApplyGate({ application, approval, agent, dryRun: { ...dryRun, sideEffects: { ...dryRun.sideEffects, writesAgents: true } }, body: gateBody }),
  sourceApprovalWithRunnerJob: buildAgentConfigRealApplyGate({ application, approval: { ...approval, runnerJobId: 'runner_job_invalid' }, agent, dryRun, body: gateBody })
};

process.stdout.write(JSON.stringify(cases));
"@
  $cases = $casesJson | ConvertFrom-Json

  Write-Step "Verify valid gate input is ready but still feature-disabled."
  Assert-BlockedGate -Result $cases.validBlocked -Prefix "Valid Agent config real apply gate"
  Assert-Equal $cases.validBlocked.preconditionsReady $true "Valid gate input should satisfy future real-apply preconditions."
  Assert-Equal $cases.validBlocked.gateReady $false "Valid gate input should still keep the feature gate closed."
  Assert-Equal (($cases.validBlocked.validationErrors | Measure-Object).Count) 0 "Valid gate input should have no validation errors."
  Assert-Equal $cases.validBlocked.writePlan.wouldUpdateAgent $false "Valid gate should not update Agent in MVP."
  Assert-Equal $cases.validBlocked.writePlan.wouldCreateVersion $false "Valid gate should not create versions in MVP."
  Assert-Equal $cases.validBlocked.writePlan.transactionRequired $true "Valid gate should require a future transaction."
  Assert-Equal $cases.validBlocked.auditPlan.requiresGitCheckpoint $true "Valid gate should require a Git checkpoint."
  Assert-Equal $cases.validBlocked.auditPlan.createsRunnerJob $false "Valid gate should not create Runner jobs."

  Write-Step "Verify missing human and rollback gates stay blocked."
  Assert-BlockedGate -Result $cases.missingRequestedBy -Prefix "Missing requestedBy gate"
  Assert-Equal $cases.missingRequestedBy.preconditionsReady $false "Missing requestedBy should not satisfy preconditions."
  Assert-TextContains (@($cases.missingRequestedBy.validationErrors) -join "`n") "requestedBy is required." "Missing requestedBy should be reported."
  Assert-BlockedGate -Result $cases.missingGitCheckpoint -Prefix "Missing Git checkpoint gate"
  Assert-Equal $cases.missingGitCheckpoint.preconditionsReady $false "Missing Git checkpoint should not satisfy preconditions."
  Assert-TextContains (@($cases.missingGitCheckpoint.validationErrors) -join "`n") "gitCheckpoint.created=true and gitCheckpoint.commit are required." "Missing Git checkpoint should be reported."
  Assert-BlockedGate -Result $cases.missingRollbackAcceptance -Prefix "Missing rollback acceptance gate"
  Assert-Equal $cases.missingRollbackAcceptance.preconditionsReady $false "Missing rollback acceptance should not satisfy preconditions."
  Assert-TextContains (@($cases.missingRollbackAcceptance.validationErrors) -join "`n") "rollbackPlanAccepted=true is required." "Missing rollback acceptance should be reported."

  Write-Step "Verify dry-run proof failures stay blocked."
  Assert-BlockedGate -Result $cases.missingDryRun -Prefix "Missing dry-run gate"
  Assert-TextContains (@($cases.missingDryRun.validationErrors) -join "`n") "dryRun result is required before real apply." "Missing dry-run should be reported."
  Assert-BlockedGate -Result $cases.mismatchedDryRun -Prefix "Mismatched dry-run gate"
  Assert-TextContains (@($cases.mismatchedDryRun.validationErrors) -join "`n") "dryRun applicationId must match application." "Mismatched dry-run should be reported."
  Assert-BlockedGate -Result $cases.dryRunValidationErrors -Prefix "Dry-run validation error gate"
  Assert-TextContains (@($cases.dryRunValidationErrors.validationErrors) -join "`n") "dryRun must have no validation errors." "Dry-run validation errors should be reported."
  Assert-BlockedGate -Result $cases.dryRunNotDisabled -Prefix "Dry-run not disabled gate"
  Assert-TextContains (@($cases.dryRunNotDisabled.validationErrors) -join "`n") "dryRun must be the current feature-disabled preview." "Non-disabled dry-run should be reported."
  Assert-BlockedGate -Result $cases.dryRunSideEffects -Prefix "Dry-run side effect gate"
  Assert-TextContains (@($cases.dryRunSideEffects.validationErrors) -join "`n") "dryRun side effects must all be false." "Dry-run side effects should be reported."

  Write-Step "Verify Runner job contamination stays blocked."
  Assert-BlockedGate -Result $cases.sourceApprovalWithRunnerJob -Prefix "Runner-job source approval gate"
  Assert-TextContains (@($cases.sourceApprovalWithRunnerJob.validationErrors) -join "`n") "source approval must not have a Runner job." "Runner job source approval should be reported."

  Write-Step "Agent config real apply gate checks passed."
} finally {
  Pop-Location
}
