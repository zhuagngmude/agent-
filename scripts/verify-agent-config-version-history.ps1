$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot

function Write-Step {
  param([string]$Message)
  Write-Host "[agent-config-version-history] $Message"
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
  Write-Step "Load Agent config version history cases."
  $casesJson = node -e @"
const { buildAgentConfigVersionHistory } = require('./services/api/agent-config-version-history');

const agent = { id: 'agent_reviewer' };
const versions = [
  {
    id: 'version_other_agent_9',
    agentId: 'agent_other',
    version: 9,
    configSnapshot: { permissions: ['other'], model: 'gpt-other' },
    changes: []
  },
  {
    id: 'version_agent_reviewer_2',
    project_id: 'project_agent_swarm',
    agent_id: 'agent_reviewer',
    version: 2,
    approval_id: 'approval_v2',
    application_id: 'application_v2',
    config_snapshot: JSON.stringify({
      permissions: ['read_project', 'reviewer_agent'],
      model: 'gpt-mid',
      status: 'idle',
      maxSubAgents: 2,
      canSpawnSubAgents: false,
      extraDisplayName: 'ignored'
    }),
    changes: JSON.stringify([{ field: 'permissions', before: ['read_project'], after: ['read_project', 'reviewer_agent'] }]),
    applied_by: 'tester',
    applied_at: '2026-06-10T10:00:00Z',
    created_at: '2026-06-10T10:00:00Z'
  },
  {
    id: 'version_agent_reviewer_4',
    projectId: 'project_agent_swarm',
    agentId: 'agent_reviewer',
    versionNumber: 4,
    approvalId: 'approval_v4',
    applicationId: 'application_v4',
    configSnapshot: {
      permissions: ['read_project', 'reviewer_agent'],
      model: 'gpt-high',
      status: 'idle',
      maxSubAgents: 4,
      canSpawnSubAgents: true
    },
    changes: [{ field: 'model', before: 'gpt-mid', after: 'gpt-high' }],
    appliedBy: 'tester',
    appliedAt: '2026-06-10T12:00:00Z',
    createdAt: '2026-06-10T12:00:00Z'
  },
  {
    id: 'version_agent_reviewer_1',
    agentId: 'agent_reviewer',
    version: 1,
    configSnapshot: {
      permissions: ['read_project'],
      model: 'gpt-low',
      status: 'idle',
      maxSubAgents: 1,
      canSpawnSubAgents: false
    },
    changes: []
  }
];

const cases = {
  validDefaultRestore: buildAgentConfigVersionHistory({ agent, versions }),
  validRequestedRestore: buildAgentConfigVersionHistory({ agent, versions, restoreVersion: 1 }),
  requestedCurrentVersion: buildAgentConfigVersionHistory({ agent, versions, restoreVersion: 4 }),
  wrongAgentOnly: buildAgentConfigVersionHistory({ agent, versions: [versions[0]] }),
  missingAgent: buildAgentConfigVersionHistory({ agent: null, versions }),
  invalidVersionsType: buildAgentConfigVersionHistory({ agent, versions: 'not-array' }),
  duplicateVersion: buildAgentConfigVersionHistory({ agent, versions: [...versions, { ...versions[2], id: 'duplicate_v4' }] }),
  missingVersionNumber: buildAgentConfigVersionHistory({ agent, versions: [{ ...versions[1], version: 0 }] }),
  forbiddenSnapshotField: buildAgentConfigVersionHistory({
    agent,
    versions: [{
      id: 'version_forbidden_field',
      agentId: 'agent_reviewer',
      version: 1,
      configSnapshot: { permissions: ['read_project'], apiKey: 'hidden' },
      changes: []
    }]
  }),
  forbiddenSnapshotValue: buildAgentConfigVersionHistory({
    agent,
    versions: [{
      id: 'version_forbidden_value',
      agentId: 'agent_reviewer',
      version: 1,
      configSnapshot: { permissions: ['read_project'], model: 'prompt override' },
      changes: []
    }]
  }),
  forbiddenChangeField: buildAgentConfigVersionHistory({
    agent,
    versions: [{
      id: 'version_forbidden_change_field',
      agentId: 'agent_reviewer',
      version: 1,
      configSnapshot: { permissions: ['read_project'], model: 'gpt-low' },
      changes: [{ field: 'apiKey', before: '', after: 'hidden' }]
    }]
  }),
  forbiddenChangeValue: buildAgentConfigVersionHistory({
    agent,
    versions: [{
      id: 'version_forbidden_change_value',
      agentId: 'agent_reviewer',
      version: 1,
      configSnapshot: { permissions: ['read_project'], model: 'gpt-low' },
      changes: [{ field: 'model', before: 'gpt-low', after: 'prompt override' }]
    }]
  })
};

process.stdout.write(JSON.stringify(cases));
"@
  $cases = $casesJson | ConvertFrom-Json

  Write-Step "Verify valid version history normalization."
  Assert-Equal $cases.validDefaultRestore.ok $true "Valid version history should pass."
  Assert-Equal $cases.validDefaultRestore.versionHistory $true "Valid version history should identify itself."
  Assert-Equal $cases.validDefaultRestore.readOnly $true "Version history should be read-only."
  Assert-Equal $cases.validDefaultRestore.canWrite $false "Version history should not allow writes."
  Assert-Equal $cases.validDefaultRestore.currentVersion.version 4 "Current version should be highest version."
  Assert-Equal $cases.validDefaultRestore.restoreVersion.version 2 "Default restore version should be latest older version."
  Assert-Equal @($cases.validDefaultRestore.restoreCandidates).Count 2 "Restore candidates should include versions older than current."
  Assert-Equal @($cases.validDefaultRestore.versions).Count 3 "Version history should filter out other Agents."
  Assert-Equal $cases.validDefaultRestore.rollbackSourceReady $true "Valid history should be rollback-source ready."
  Assert-Equal $cases.validDefaultRestore.versions[1].configSnapshot.model "gpt-mid" "JSON snapshot should be parsed."
  Assert-Equal ($cases.validDefaultRestore.versions[1].configSnapshot.PSObject.Properties.Name -contains "extraDisplayName") $false "Snapshot should only expose allowed fields."
  Assert-NoSideEffects -Result $cases.validDefaultRestore -Prefix "Valid version history"

  Write-Step "Verify requested restore selection."
  Assert-Equal $cases.validRequestedRestore.ok $true "Requested restore version should pass."
  Assert-Equal $cases.validRequestedRestore.restoreVersion.version 1 "Requested restore version should be selected."
  Assert-NoSideEffects -Result $cases.validRequestedRestore -Prefix "Requested restore version history"

  Write-Step "Verify invalid version history inputs."
  Assert-Equal $cases.requestedCurrentVersion.ok $false "Current version cannot be restore target."
  Assert-TextContains (@($cases.requestedCurrentVersion.validationErrors) -join "`n") "restore version must exist and be older than current version." "Current restore target should be reported."
  Assert-NoSideEffects -Result $cases.requestedCurrentVersion -Prefix "Current restore target"

  Assert-Equal $cases.wrongAgentOnly.ok $false "Wrong-Agent-only versions should fail."
  Assert-TextContains (@($cases.wrongAgentOnly.validationErrors) -join "`n") "no versions belong to target Agent." "Wrong-Agent-only versions should be reported."
  Assert-NoSideEffects -Result $cases.wrongAgentOnly -Prefix "Wrong Agent history"

  Assert-Equal $cases.missingAgent.ok $false "Missing Agent should fail."
  Assert-TextContains (@($cases.missingAgent.validationErrors) -join "`n") "target agent is required." "Missing Agent should be reported."
  Assert-NoSideEffects -Result $cases.missingAgent -Prefix "Missing Agent history"

  Assert-Equal $cases.invalidVersionsType.ok $false "Invalid versions type should fail."
  Assert-TextContains (@($cases.invalidVersionsType.validationErrors) -join "`n") "versions must be an array." "Invalid versions type should be reported."
  Assert-NoSideEffects -Result $cases.invalidVersionsType -Prefix "Invalid versions type"

  Assert-Equal $cases.duplicateVersion.ok $false "Duplicate versions should fail."
  Assert-TextContains (@($cases.duplicateVersion.validationErrors) -join "`n") "duplicate Agent config version: 4" "Duplicate version should be reported."
  Assert-NoSideEffects -Result $cases.duplicateVersion -Prefix "Duplicate version history"

  Assert-Equal $cases.missingVersionNumber.ok $false "Missing version number should fail."
  Assert-TextContains (@($cases.missingVersionNumber.validationErrors) -join "`n") "version number is required" "Missing version number should be reported."
  Assert-NoSideEffects -Result $cases.missingVersionNumber -Prefix "Missing version number history"

  Assert-Equal $cases.forbiddenSnapshotField.ok $false "Forbidden snapshot field should fail."
  Assert-TextContains (@($cases.forbiddenSnapshotField.validationErrors) -join "`n") "forbidden Agent config snapshot field: apiKey" "Forbidden snapshot field should be reported."
  Assert-NoSideEffects -Result $cases.forbiddenSnapshotField -Prefix "Forbidden snapshot field history"

  Assert-Equal $cases.forbiddenSnapshotValue.ok $false "Forbidden snapshot value should fail."
  Assert-TextContains (@($cases.forbiddenSnapshotValue.validationErrors) -join "`n") "forbidden Agent config snapshot value in field: model" "Forbidden snapshot value should be reported."
  Assert-Equal $cases.forbiddenSnapshotValue.currentVersion.configSnapshot.model "[redacted_forbidden_value]" "Forbidden snapshot value should be redacted from returned history."
  Assert-NoSideEffects -Result $cases.forbiddenSnapshotValue -Prefix "Forbidden snapshot value history"

  Assert-Equal $cases.forbiddenChangeField.ok $false "Forbidden change field should fail."
  Assert-TextContains (@($cases.forbiddenChangeField.validationErrors) -join "`n") "forbidden Agent config change field: apiKey" "Forbidden change field should be reported."
  Assert-Equal @($cases.forbiddenChangeField.currentVersion.changes).Count 0 "Forbidden change field should not be returned."
  Assert-NoSideEffects -Result $cases.forbiddenChangeField -Prefix "Forbidden change field history"

  Assert-Equal $cases.forbiddenChangeValue.ok $false "Forbidden change value should fail."
  Assert-TextContains (@($cases.forbiddenChangeValue.validationErrors) -join "`n") "forbidden Agent config change value in field: model" "Forbidden change value should be reported."
  Assert-Equal $cases.forbiddenChangeValue.currentVersion.changes[0].after "[redacted_forbidden_value]" "Forbidden change value should be redacted."
  Assert-NoSideEffects -Result $cases.forbiddenChangeValue -Prefix "Forbidden change value history"

  Write-Step "Agent config version history checks passed."
} finally {
  Pop-Location
}
