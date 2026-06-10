$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot

function Write-Step {
  param([string]$Message)
  Write-Host "[agent-config-dry-run] $Message"
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

function Assert-BlockedDryRun {
  param(
    [object]$Result,
    [string]$Prefix
  )

  Assert-Equal $Result.ok $false "$Prefix should stay blocked."
  Assert-Equal $Result.dryRun $true "$Prefix should identify dry-run."
  Assert-Equal $Result.canApply $false "$Prefix should not allow apply."
  Assert-TextContains (@($Result.blockedReasons) -join "`n") "feature_disabled" "$Prefix should report feature disabled."
  Assert-NoSideEffects -Result $Result -Prefix $Prefix
}

Push-Location $root
try {
  Write-Step "Load Agent config dry-run helper cases."
  $casesJson = node -e @"
const { buildAgentConfigApplyDryRun } = require('./services/api/server');

const application = {
  id: 'agent_config_application_test',
  approvalId: 'approval_agent_test_permission',
  agentId: 'agent_reviewer',
  changeType: 'permission',
  status: 'pending_apply',
  changes: [{ field: 'permissions', before: 'read_project', after: 'reviewer_agent' }]
};
const approvedApproval = {
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
const validBody = {
  secondConfirm: true,
  confirmText: 'dry-run only',
  requestedBy: 'verify_agent_config_dry_run'
};

const cases = {
  validBlocked: buildAgentConfigApplyDryRun({ application, approval: approvedApproval, agent, body: validBody }),
  missingSecondConfirm: buildAgentConfigApplyDryRun({ application, approval: approvedApproval, agent, body: { confirmText: 'dry-run only' } }),
  missingConfirmText: buildAgentConfigApplyDryRun({ application, approval: approvedApproval, agent, body: { secondConfirm: true } }),
  nonPendingApplication: buildAgentConfigApplyDryRun({ application: { ...application, status: 'applied' }, approval: approvedApproval, agent, body: validBody }),
  unapprovedApproval: buildAgentConfigApplyDryRun({ application, approval: { ...approvedApproval, status: 'pending' }, agent, body: validBody }),
  approvalWithRunnerJob: buildAgentConfigApplyDryRun({ application, approval: { ...approvedApproval, runnerJobId: 'runner_job_invalid' }, agent, body: validBody }),
  wrongTargetService: buildAgentConfigApplyDryRun({ application, approval: { ...approvedApproval, targetService: 'runner' }, agent, body: validBody }),
  missingAgent: buildAgentConfigApplyDryRun({ application, approval: approvedApproval, agent: null, body: validBody })
};

process.stdout.write(JSON.stringify(cases));
"@
  $cases = $casesJson | ConvertFrom-Json

  Write-Step "Verify valid pending dry-run remains feature-disabled."
  Assert-BlockedDryRun -Result $cases.validBlocked -Prefix "Valid Agent config dry-run"
  Assert-Equal (($cases.validBlocked.validationErrors | Measure-Object).Count) 0 "Valid blocked preview should have no validation errors."
  Assert-TextContains (@($cases.validBlocked.writePlan.changedFields) -join "`n") "permissions" "Valid blocked preview should list changed fields."
  Assert-Equal $cases.validBlocked.rollbackPlan.rollbackRequiresNewApproval $true "Valid blocked preview rollback should require approval."

  Write-Step "Verify request confirmation failures stay side-effect free."
  Assert-BlockedDryRun -Result $cases.missingSecondConfirm -Prefix "Missing secondConfirm dry-run"
  Assert-TextContains (@($cases.missingSecondConfirm.validationErrors) -join "`n") "secondConfirm=true is required." "Missing secondConfirm should be reported."
  Assert-BlockedDryRun -Result $cases.missingConfirmText -Prefix "Missing confirmText dry-run"
  Assert-TextContains (@($cases.missingConfirmText.validationErrors) -join "`n") "confirmText is required." "Missing confirmText should be reported."

  Write-Step "Verify application and approval precondition failures."
  Assert-BlockedDryRun -Result $cases.nonPendingApplication -Prefix "Non-pending application dry-run"
  Assert-TextContains (@($cases.nonPendingApplication.validationErrors) -join "`n") "application must be pending_apply" "Non-pending application should be reported."
  Assert-BlockedDryRun -Result $cases.unapprovedApproval -Prefix "Unapproved source approval dry-run"
  Assert-TextContains (@($cases.unapprovedApproval.validationErrors) -join "`n") "source approval must be approved" "Unapproved source approval should be reported."
  Assert-BlockedDryRun -Result $cases.approvalWithRunnerJob -Prefix "Runner-job source approval dry-run"
  Assert-TextContains (@($cases.approvalWithRunnerJob.validationErrors) -join "`n") "source approval must not have a Runner job." "Runner job source approval should be reported."
  Assert-BlockedDryRun -Result $cases.wrongTargetService -Prefix "Wrong target service dry-run"
  Assert-TextContains (@($cases.wrongTargetService.validationErrors) -join "`n") "source approval targetService must be agent_config." "Wrong target service should be reported."
  Assert-BlockedDryRun -Result $cases.missingAgent -Prefix "Missing target Agent dry-run"
  Assert-TextContains (@($cases.missingAgent.validationErrors) -join "`n") "target agent not found." "Missing target Agent should be reported."

  Write-Step "Agent config dry-run helper checks passed."
} finally {
  Pop-Location
}
