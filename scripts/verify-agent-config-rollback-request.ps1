$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot

function Write-Step {
  param([string]$Message)
  Write-Host "[agent-config-rollback-request] $Message"
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
  Assert-Equal $Result.sideEffects.writesAgentConfigApplications $false "$Prefix should not write applications."
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
  Write-Step "Load Agent config rollback request cases."
  $casesJson = node -e @"
const { buildAgentConfigRollbackRequest } = require('./services/api/agent-config-rollback-request');

const originalApplication = {
  id: 'agent_config_application_original',
  approvalId: 'approval_agent_original',
  agentId: 'agent_reviewer',
  status: 'applied'
};
const sourceApproval = {
  id: 'approval_agent_original',
  status: 'approved',
  targetService: 'agent_config',
  runnerJobId: ''
};
const agent = {
  id: 'agent_reviewer'
};
const currentVersion = {
  id: 'agent_config_version_agent_reviewer_4',
  agentId: 'agent_reviewer',
  version: 4,
  configSnapshot: {
    permissions: ['read_project', 'reviewer_agent'],
    model: 'gpt-high',
    status: 'idle',
    maxSubAgents: 3,
    canSpawnSubAgents: true
  }
};
const restoreVersion = {
  id: 'agent_config_version_agent_reviewer_2',
  agent_id: 'agent_reviewer',
  versionNumber: 2,
  config_snapshot: {
    permissions: ['read_project'],
    model: 'gpt-low',
    status: 'idle',
    maxSubAgents: 1,
    canSpawnSubAgents: false
  }
};
const body = {
  secondConfirm: true,
  confirmText: 'rollback request only',
  requestedBy: 'verify_agent_config_rollback_request',
  reason: 'restore last safe reviewed Agent config'
};

const cases = {
  validRequest: buildAgentConfigRollbackRequest({ originalApplication, sourceApproval, agent, currentVersion, restoreVersion, body }),
  originalNotApplied: buildAgentConfigRollbackRequest({
    originalApplication: { ...originalApplication, status: 'pending_apply' },
    sourceApproval,
    agent,
    currentVersion,
    restoreVersion,
    body
  }),
  sourceApprovalWithRunnerJob: buildAgentConfigRollbackRequest({
    originalApplication,
    sourceApproval: { ...sourceApproval, runnerJobId: 'runner_job_invalid' },
    agent,
    currentVersion,
    restoreVersion,
    body
  }),
  wrongTargetService: buildAgentConfigRollbackRequest({
    originalApplication,
    sourceApproval: { ...sourceApproval, targetService: 'runner' },
    agent,
    currentVersion,
    restoreVersion,
    body
  }),
  unapprovedSourceApproval: buildAgentConfigRollbackRequest({
    originalApplication,
    sourceApproval: { ...sourceApproval, status: 'pending' },
    agent,
    currentVersion,
    restoreVersion,
    body
  }),
  missingConfirmation: buildAgentConfigRollbackRequest({
    originalApplication,
    sourceApproval,
    agent,
    currentVersion,
    restoreVersion,
    body: { secondConfirm: false }
  }),
  restoreNotOlder: buildAgentConfigRollbackRequest({
    originalApplication,
    sourceApproval,
    agent,
    currentVersion,
    restoreVersion: { ...restoreVersion, versionNumber: 4 },
    body
  }),
  wrongAgentVersion: buildAgentConfigRollbackRequest({
    originalApplication,
    sourceApproval,
    agent,
    currentVersion,
    restoreVersion: { ...restoreVersion, agent_id: 'agent_other' },
    body
  }),
  noChangedFields: buildAgentConfigRollbackRequest({
    originalApplication,
    sourceApproval,
    agent,
    currentVersion,
    restoreVersion: { ...restoreVersion, versionNumber: 2, config_snapshot: { ...currentVersion.configSnapshot } },
    body
  })
};

process.stdout.write(JSON.stringify(cases));
"@
  $cases = $casesJson | ConvertFrom-Json

  Write-Step "Verify valid rollback request remains feature-disabled and no-write."
  Assert-Equal $cases.validRequest.ok $false "Valid rollback request should stay disabled."
  Assert-Equal $cases.validRequest.rollbackRequest $true "Valid rollback request should identify itself."
  Assert-Equal $cases.validRequest.requestReady $true "Valid rollback request should satisfy preconditions."
  Assert-Equal $cases.validRequest.canCreateApproval $false "Valid rollback request should not create approval in MVP."
  Assert-TextContains (@($cases.validRequest.blockedReasons) -join "`n") "feature_disabled" "Valid rollback request should remain feature-disabled."
  Assert-Equal $cases.validRequest.currentVersion 4 "Current version should be read from version row."
  Assert-Equal $cases.validRequest.restoreVersion 2 "Restore version should be read from version row."
  Assert-Equal $cases.validRequest.futureVersion 5 "Future version should increment from current."
  Assert-Equal $cases.validRequest.approvalDraft.status "pending" "Rollback approval draft should be pending."
  Assert-Equal $cases.validRequest.approvalDraft.targetService "agent_config" "Rollback approval draft should target Agent config."
  Assert-Equal $cases.validRequest.approvalDraft.runnerJobId "" "Rollback approval draft should not have Runner job."
  Assert-Equal $cases.validRequest.approvalDraft.changeRequest.changeType "rollback" "Rollback approval draft should use rollback change type."
  Assert-TextContains (@($cases.validRequest.approvalDraft.operationTypes) -join "`n") "agent_config_rollback" "Rollback approval draft should include rollback operation."
  Assert-Equal $cases.validRequest.applicationDraft.status "pending_apply_after_approval" "Rollback application draft should wait for approval."
  Assert-TextContains (@($cases.validRequest.applicationDraft.changes.field) -join "`n") "permissions" "Rollback changes should include permissions diff."
  Assert-TextContains (@($cases.validRequest.applicationDraft.changes.field) -join "`n") "model" "Rollback changes should include model diff."
  Assert-TextContains (@($cases.validRequest.applicationDraft.changes.field) -join "`n") "maxSubAgents" "Rollback changes should include maxSubAgents diff."
  Assert-TextContains (@($cases.validRequest.applicationDraft.changes.field) -join "`n") "canSpawnSubAgents" "Rollback changes should include canSpawnSubAgents diff."
  Assert-NoSideEffects -Result $cases.validRequest -Prefix "Valid rollback request"

  Write-Step "Verify rollback rules."
  Assert-Equal $cases.validRequest.rollbackRules.createsNewApproval $true "Rollback must create a new approval in the future flow."
  Assert-Equal $cases.validRequest.rollbackRules.createsNewApplication $true "Rollback must create a new application in the future flow."
  Assert-Equal $cases.validRequest.rollbackRules.createsNewVersionOnFutureApply $true "Rollback must create a new future version if applied."
  Assert-Equal $cases.validRequest.rollbackRules.deletesVersionHistory $false "Rollback must not delete version history."
  Assert-Equal $cases.validRequest.rollbackRules.overwritesVersionHistory $false "Rollback must not overwrite version history."
  Assert-Equal $cases.validRequest.rollbackRules.directlyUpdatesAgents $false "Rollback request must not directly update Agents."
  Assert-Equal $cases.validRequest.rollbackRules.createsRunnerJob $false "Rollback request must not create Runner jobs."
  Assert-Equal $cases.validRequest.rollbackRules.executesRunner $false "Rollback request must not execute Runner."

  Write-Step "Verify rollback precondition failures."
  Assert-Equal $cases.originalNotApplied.requestReady $false "Non-applied original application should not be ready."
  Assert-TextContains (@($cases.originalNotApplied.validationErrors) -join "`n") "original application must be applied" "Non-applied original application should be reported."
  Assert-NoSideEffects -Result $cases.originalNotApplied -Prefix "Non-applied rollback request"

  Assert-Equal $cases.sourceApprovalWithRunnerJob.requestReady $false "Runner-job source approval should not be ready."
  Assert-TextContains (@($cases.sourceApprovalWithRunnerJob.validationErrors) -join "`n") "source approval must not have a Runner job." "Runner job source approval should be reported."
  Assert-NoSideEffects -Result $cases.sourceApprovalWithRunnerJob -Prefix "Runner-job rollback request"

  Assert-Equal $cases.wrongTargetService.requestReady $false "Wrong target service should not be ready."
  Assert-TextContains (@($cases.wrongTargetService.validationErrors) -join "`n") "source approval targetService must be agent_config." "Wrong target service should be reported."
  Assert-NoSideEffects -Result $cases.wrongTargetService -Prefix "Wrong target rollback request"

  Assert-Equal $cases.unapprovedSourceApproval.requestReady $false "Unapproved source approval should not be ready."
  Assert-TextContains (@($cases.unapprovedSourceApproval.validationErrors) -join "`n") "source approval must be approved" "Unapproved source approval should be reported."
  Assert-NoSideEffects -Result $cases.unapprovedSourceApproval -Prefix "Unapproved rollback request"

  Assert-Equal $cases.missingConfirmation.requestReady $false "Missing confirmation fields should not be ready."
  Assert-TextContains (@($cases.missingConfirmation.validationErrors) -join "`n") "secondConfirm=true is required." "Missing second confirmation should be reported."
  Assert-TextContains (@($cases.missingConfirmation.validationErrors) -join "`n") "confirmText is required." "Missing confirm text should be reported."
  Assert-TextContains (@($cases.missingConfirmation.validationErrors) -join "`n") "requestedBy is required." "Missing requester should be reported."
  Assert-TextContains (@($cases.missingConfirmation.validationErrors) -join "`n") "rollback reason is required." "Missing rollback reason should be reported."
  Assert-NoSideEffects -Result $cases.missingConfirmation -Prefix "Missing confirmation rollback request"

  Assert-Equal $cases.restoreNotOlder.requestReady $false "Restore version must be older."
  Assert-TextContains (@($cases.restoreNotOlder.validationErrors) -join "`n") "restore version must be older than current version." "Restore version ordering should be reported."
  Assert-NoSideEffects -Result $cases.restoreNotOlder -Prefix "Restore-not-older rollback request"

  Assert-Equal $cases.wrongAgentVersion.requestReady $false "Wrong Agent version should not be ready."
  Assert-TextContains (@($cases.wrongAgentVersion.validationErrors) -join "`n") "restore version must belong to target Agent." "Wrong Agent version should be reported."
  Assert-NoSideEffects -Result $cases.wrongAgentVersion -Prefix "Wrong-Agent rollback request"

  Assert-Equal $cases.noChangedFields.requestReady $false "No changed fields should not be ready."
  Assert-TextContains (@($cases.noChangedFields.validationErrors) -join "`n") "rollback must include at least one changed field." "No changed fields should be reported."
  Assert-NoSideEffects -Result $cases.noChangedFields -Prefix "No-change rollback request"

  Write-Step "Agent config rollback request checks passed."
} finally {
  Pop-Location
}
