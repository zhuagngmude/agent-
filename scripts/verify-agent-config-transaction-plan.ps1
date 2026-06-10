$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot

function Write-Step {
  param([string]$Message)
  Write-Host "[agent-config-transaction-plan] $Message"
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
  Write-Step "Load Agent config transaction plan cases."
  $casesJson = node -e @"
const {
  buildAgentConfigApplyDryRun,
  buildAgentConfigRealApplyGate,
  buildAgentConfigApplyTransactionPlan
} = require('./services/api/server');

const application = {
  id: 'agent_config_application_transaction',
  approvalId: 'approval_agent_transaction',
  agentId: 'agent_reviewer',
  changeType: 'permission',
  status: 'pending_apply',
  changes: [
    { field: 'permissions', before: 'read_project', after: 'reviewer_agent' },
    { field: 'model', before: 'gpt-low', after: 'gpt-high' }
  ]
};
const approval = {
  id: 'approval_agent_transaction',
  status: 'approved',
  targetService: 'agent_config',
  runnerJobId: ''
};
const agent = {
  id: 'agent_reviewer',
  status: 'idle',
  model: 'gpt-low',
  permissions: ['read_project'],
  canSpawnSubAgents: false,
  maxSubAgents: 1,
  versionNumber: 3
};
const dryRunBody = {
  secondConfirm: true,
  confirmText: 'transaction dry-run',
  requestedBy: 'verify_agent_config_transaction_plan'
};
const gateBody = {
  secondConfirm: true,
  confirmText: 'transaction gate',
  requestedBy: 'verify_agent_config_transaction_plan',
  gitCheckpoint: { created: true, commit: 'checkpoint_transaction' },
  rollbackPlanAccepted: true
};
const transactionBody = {
  secondConfirm: true,
  confirmText: 'transaction plan only',
  appliedBy: 'verify_agent_config_transaction_plan'
};
const dryRun = buildAgentConfigApplyDryRun({ application, approval, agent, body: dryRunBody });
const gate = buildAgentConfigRealApplyGate({ application, approval, agent, dryRun, body: gateBody });
const invalidVersionDryRun = {
  ...dryRun,
  writePlan: { ...dryRun.writePlan, targetVersion: 9 }
};

const cases = {
  gateWithPlan: gate,
  validPlan: buildAgentConfigApplyTransactionPlan({ application, approval, agent, dryRun, gate, body: transactionBody }),
  missingGate: buildAgentConfigApplyTransactionPlan({ application, approval, agent, dryRun, gate: null, body: transactionBody }),
  invalidVersion: buildAgentConfigApplyTransactionPlan({ application, approval, agent, dryRun: invalidVersionDryRun, gate, body: transactionBody }),
  nonPendingApplication: buildAgentConfigApplyTransactionPlan({ application: { ...application, status: 'applied' }, approval, agent, dryRun, gate, body: transactionBody }),
  sourceApprovalWithRunnerJob: buildAgentConfigApplyTransactionPlan({ application, approval: { ...approval, runnerJobId: 'runner_job_invalid' }, agent, dryRun, gate, body: transactionBody }),
  missingAppliedBy: buildAgentConfigApplyTransactionPlan({ application, approval, agent, dryRun, gate, body: { secondConfirm: true, confirmText: 'missing actor' } }),
  notReadyGate: buildAgentConfigApplyTransactionPlan({ application, approval, agent, dryRun, gate: { ...gate, preconditionsReady: false }, body: transactionBody })
};

process.stdout.write(JSON.stringify(cases));
"@
  $cases = $casesJson | ConvertFrom-Json

  Write-Step "Verify real apply gate includes a no-write transaction plan."
  Assert-Equal $cases.gateWithPlan.transactionPlan.transactionPlan $true "Gate should include transaction plan object."
  Assert-Equal $cases.gateWithPlan.transactionPlan.canWrite $false "Gate transaction plan should not allow write."
  Assert-NoSideEffects -Result $cases.gateWithPlan.transactionPlan -Prefix "Gate transaction plan"

  Write-Step "Verify valid transaction plan shape."
  Assert-Equal $cases.validPlan.ok $false "Valid transaction plan should stay disabled."
  Assert-Equal $cases.validPlan.transactionPlan $true "Valid transaction plan should identify itself."
  Assert-Equal $cases.validPlan.planReady $true "Valid transaction plan should satisfy preconditions."
  Assert-Equal $cases.validPlan.canWrite $false "Valid transaction plan should not allow writes in MVP."
  Assert-TextContains (@($cases.validPlan.blockedReasons) -join "`n") "feature_disabled" "Valid transaction plan should remain feature-disabled."
  Assert-Equal $cases.validPlan.transaction.required $true "Transaction should be required."
  Assert-Equal $cases.validPlan.transaction.rollbackOnAnyFailure $true "Transaction should roll back on failure."
  Assert-Equal $cases.validPlan.versionPlan.currentVersion 3 "Current version should be read from Agent."
  Assert-Equal $cases.validPlan.versionPlan.targetVersion 4 "Target version should increment by one."
  Assert-Equal $cases.validPlan.versionPlan.insertRequired $true "Version insert should be required in future write."
  Assert-Equal $cases.validPlan.versionPlan.deleteHistory $false "Version history should not be deleted."
  Assert-Equal $cases.validPlan.versionPlan.overwriteHistory $false "Version history should not be overwritten."
  Assert-Equal $cases.validPlan.writeSet.updateAgentsCurrentState $true "Future write set should update Agents."
  Assert-Equal $cases.validPlan.writeSet.insertAgentConfigVersion $true "Future write set should insert version."
  Assert-Equal $cases.validPlan.writeSet.updateAgentConfigApplicationStatus $true "Future write set should mark application applied."
  Assert-Equal $cases.validPlan.writeSet.insertRuntimeEvent $true "Future write set should insert runtime event."
  Assert-Equal $cases.validPlan.writeSet.createRunnerJob $false "Transaction plan should not create Runner jobs."
  Assert-Equal $cases.validPlan.writeSet.executeRunner $false "Transaction plan should not execute Runner."
  Assert-Equal $cases.validPlan.writeSet.callRealModel $false "Transaction plan should not call models."
  Assert-Equal $cases.validPlan.writeSet.readRawSecrets $false "Transaction plan should not read secrets."
  Assert-TextContains (@($cases.validPlan.sqliteOperations.table) -join "`n") "agents" "SQLite operations should include agents."
  Assert-TextContains (@($cases.validPlan.sqliteOperations.table) -join "`n") "agent_config_versions" "SQLite operations should include versions."
  Assert-TextContains (@($cases.validPlan.sqliteOperations.table) -join "`n") "agent_config_applications" "SQLite operations should include application status."
  Assert-TextContains (@($cases.validPlan.sqliteOperations.table) -join "`n") "runtime_events" "SQLite operations should include runtime event."
  Assert-NoSideEffects -Result $cases.validPlan -Prefix "Valid transaction plan"

  Write-Step "Verify transaction precondition failures."
  Assert-Equal $cases.missingGate.planReady $false "Missing gate should not be plan ready."
  Assert-TextContains (@($cases.missingGate.validationErrors) -join "`n") "real apply gate result is required." "Missing gate should be reported."
  Assert-NoSideEffects -Result $cases.missingGate -Prefix "Missing gate transaction plan"

  Assert-Equal $cases.invalidVersion.planReady $false "Invalid target version should not be plan ready."
  Assert-TextContains (@($cases.invalidVersion.validationErrors) -join "`n") "targetVersion must increment current Agent config version by 1." "Invalid target version should be reported."
  Assert-NoSideEffects -Result $cases.invalidVersion -Prefix "Invalid version transaction plan"

  Assert-Equal $cases.nonPendingApplication.planReady $false "Non-pending application should not be plan ready."
  Assert-TextContains (@($cases.nonPendingApplication.validationErrors) -join "`n") "application must be pending_apply" "Non-pending application should be reported."
  Assert-NoSideEffects -Result $cases.nonPendingApplication -Prefix "Non-pending transaction plan"

  Assert-Equal $cases.sourceApprovalWithRunnerJob.planReady $false "Runner-job source approval should not be plan ready."
  Assert-TextContains (@($cases.sourceApprovalWithRunnerJob.validationErrors) -join "`n") "source approval must not have a Runner job." "Runner job source approval should be reported."
  Assert-NoSideEffects -Result $cases.sourceApprovalWithRunnerJob -Prefix "Runner-job transaction plan"

  Assert-Equal $cases.missingAppliedBy.planReady $false "Missing appliedBy should not be plan ready."
  Assert-TextContains (@($cases.missingAppliedBy.validationErrors) -join "`n") "appliedBy is required." "Missing appliedBy should be reported."
  Assert-NoSideEffects -Result $cases.missingAppliedBy -Prefix "Missing appliedBy transaction plan"

  Assert-Equal $cases.notReadyGate.planReady $false "Not-ready gate should not be plan ready."
  Assert-TextContains (@($cases.notReadyGate.validationErrors) -join "`n") "real apply gate preconditions must be ready." "Not-ready gate should be reported."
  Assert-NoSideEffects -Result $cases.notReadyGate -Prefix "Not-ready gate transaction plan"

  Write-Step "Agent config transaction plan checks passed."
} finally {
  Pop-Location
}
