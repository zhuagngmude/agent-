$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
$apiScript = Join-Path $root "services\api\server.js"
$seedScript = Join-Path $PSScriptRoot "seed-sqlite.ps1"
$port = 8790
$baseUrl = "http://127.0.0.1:$port"
$projectId = "project_agent_swarm"

function Write-Step {
  param([string]$Message)
  Write-Host "[agent-config-real-apply-sqlite] $Message"
}

function Invoke-Json {
  param(
    [Parameter(Mandatory = $true)][string]$Method,
    [Parameter(Mandatory = $true)][string]$Path,
    [object]$Body = $null
  )

  $uri = "$baseUrl$Path"
  if ($null -eq $Body) {
    return Invoke-RestMethod -Method $Method -Uri $uri
  }

  return Invoke-RestMethod `
    -Method $Method `
    -Uri $uri `
    -ContentType "application/json" `
    -Body ($Body | ConvertTo-Json -Depth 20)
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

function Assert-AgentConfigRollbackRequestNoSideEffects {
  param([object]$RollbackRequest)

  Assert-Equal $RollbackRequest.sideEffects.writesAgents $false "Agent config rollback request should not write Agents."
  Assert-Equal $RollbackRequest.sideEffects.writesAgentConfigVersions $false "Agent config rollback request should not write versions."
  Assert-Equal $RollbackRequest.sideEffects.writesAgentConfigApplications $false "Agent config rollback request should not write applications."
  Assert-Equal $RollbackRequest.sideEffects.writesRuntimeEvents $false "Agent config rollback request should not write runtime events."
  Assert-Equal $RollbackRequest.sideEffects.writesSqlite $false "Agent config rollback request should not write SQLite."
  Assert-Equal $RollbackRequest.sideEffects.writesRuntimeState $false "Agent config rollback request should not write runtime state."
  Assert-Equal $RollbackRequest.sideEffects.createsApprovals $false "Agent config rollback request should not create approvals."
  Assert-Equal $RollbackRequest.sideEffects.createsRunnerJobs $false "Agent config rollback request should not create Runner jobs."
  Assert-Equal $RollbackRequest.sideEffects.executesRunner $false "Agent config rollback request should not execute Runner."
  Assert-Equal $RollbackRequest.sideEffects.callsRealModel $false "Agent config rollback request should not call models."
  Assert-Equal $RollbackRequest.sideEffects.readsRawSecrets $false "Agent config rollback request should not read raw secrets."
}

function Test-ApiReady {
  try {
    $health = Invoke-Json -Method "GET" -Path "/api/health"
    return $health.ok -eq $true
  } catch {
    return $false
  }
}

function Start-SqliteApi {
  param([bool]$EnableRealApply)

  $outLog = Join-Path $root "logs\agent-config-real-apply-sqlite-api.out.log"
  $errLog = Join-Path $root "logs\agent-config-real-apply-sqlite-api.err.log"
  New-Item -ItemType Directory -Force -Path (Split-Path -Parent $outLog) | Out-Null

  $env:AGENT_SWARM_API_PORT = "$port"
  $env:AGENT_SWARM_DASHBOARD_SOURCE = "sqlite"
  if ($EnableRealApply) {
    $env:AGENT_SWARM_ENABLE_AGENT_CONFIG_REAL_APPLY = "true"
  } else {
    Remove-Item Env:\AGENT_SWARM_ENABLE_AGENT_CONFIG_REAL_APPLY -ErrorAction SilentlyContinue
  }

  $process = Start-Process `
    -WindowStyle Hidden `
    -FilePath "node" `
    -ArgumentList @($apiScript) `
    -RedirectStandardOutput $outLog `
    -RedirectStandardError $errLog `
    -PassThru

  for ($i = 0; $i -lt 20; $i++) {
    Start-Sleep -Milliseconds 250
    if ($process.HasExited) {
      break
    }
    if (Test-ApiReady) {
      return $process
    }
  }

  throw "SQLite API did not start. Check logs/agent-config-real-apply-sqlite-api.out.log and logs/agent-config-real-apply-sqlite-api.err.log"
}

function Stop-SqliteApi {
  param([object]$Process)

  if ($Process -and -not $Process.HasExited) {
    Stop-Process -Id $Process.Id -Force
    $Process.WaitForExit()
  }
}

function New-ApprovedAgentConfigApplication {
  $request = Invoke-Json -Method "POST" -Path "/api/agents/agent_reviewer/change-requests" -Body @{
    changeType = "permission"
    riskLevel = "high"
    reason = "Verify feature-gated SQLite Agent config real apply."
    permissionProfile = "reviewer_agent"
    changes = @(
      @{
        field = "permissions"
        before = "read_project / review_risk / review_diff"
        after = "reviewer_agent"
      }
    )
  }
  Assert-Equal $request.permissionValidation.ok $true "Safe reviewer_agent profile should validate."

  $approval = Invoke-Json -Method "POST" -Path "/api/approvals/$($request.approval.id)/approve" -Body @{
    secondConfirm = $true
    confirmText = "Approve Agent config application for real apply verification."
  }
  Assert-Equal $approval.status "approved" "Agent config approval should be approved."
  Assert-Equal $approval.runnerJobId "" "Agent config approval should not create Runner job."
  Assert-True ($approval.agentConfigApplicationId -like "agent_config_application_*") "Approval should create Agent config application."

  return @{
    approvalId = $request.approval.id
    applicationId = $approval.agentConfigApplicationId
  }
}

function New-ApprovedAgentConfigModelApplication {
  $request = Invoke-Json -Method "POST" -Path "/api/agents/agent_reviewer/change-requests" -Body @{
    changeType = "model"
    riskLevel = "medium"
    reason = "Verify rollback preview can compare two real Agent config versions."
    changes = @(
      @{
        field = "model"
        before = "gemini-long-context"
        after = "gpt-rollback-preview"
      }
    )
  }

  $approval = Invoke-Json -Method "POST" -Path "/api/approvals/$($request.approval.id)/approve" -Body @{
    secondConfirm = $true
    confirmText = "Approve second Agent config application for rollback preview verification."
  }
  Assert-Equal $approval.status "approved" "Second Agent config approval should be approved."
  Assert-Equal $approval.runnerJobId "" "Second Agent config approval should not create Runner job."
  Assert-True ($approval.agentConfigApplicationId -like "agent_config_application_*") "Second approval should create Agent config application."

  return @{
    approvalId = $request.approval.id
    applicationId = $approval.agentConfigApplicationId
  }
}

function New-ApplyProof {
  param([string]$ApplicationId)

  $dryRun = Invoke-Json -Method "POST" -Path "/api/agent-config-applications/$ApplicationId/dry-run" -Body @{
    secondConfirm = $true
    confirmText = "Build Agent config real apply dry-run proof."
    requestedBy = "verify_agent_config_real_apply_sqlite"
  }
  Assert-Equal $dryRun.dryRun $true "Dry-run should identify itself."
  Assert-Equal $dryRun.ok $false "Dry-run should remain feature-disabled."
  Assert-Equal $dryRun.canApply $false "Dry-run should not apply directly."
  Assert-TextContains (@($dryRun.blockedReasons) -join "`n") "feature_disabled" "Dry-run should report feature disabled."
  Assert-Equal @($dryRun.validationErrors).Count 0 "Dry-run proof should have no validation errors."
  Assert-Equal $dryRun.changePlanValidation.ok $true "Dry-run change plan should be valid."

  return @{
    secondConfirm = $true
    confirmText = "I confirm this feature-gated SQLite Agent config real apply."
    requestedBy = "verify_agent_config_real_apply_sqlite"
    gitCheckpoint = @{
      created = $true
      commit = "verification-only-checkpoint"
    }
    rollbackPlanAccepted = $true
    dryRun = $dryRun
  }
}

function Get-Reviewer {
  $agents = Invoke-Json -Method "GET" -Path "/api/projects/$projectId/agents"
  return @($agents.agents | Where-Object { $_.id -eq "agent_reviewer" })[0]
}

if (Test-ApiReady) {
  throw "Port $port already has an API responding before verification started. Stop that process or use a different isolated verification port."
}

$previousPort = $env:AGENT_SWARM_API_PORT
$previousSource = $env:AGENT_SWARM_DASHBOARD_SOURCE
$previousRealApply = $env:AGENT_SWARM_ENABLE_AGENT_CONFIG_REAL_APPLY

try {
  Write-Step "Verify feature flag off keeps SQLite apply as status-only."
  powershell -ExecutionPolicy Bypass -File $seedScript | Out-Null
  $process = Start-SqliteApi -EnableRealApply $false
  try {
    Invoke-Json -Method "POST" -Path "/api/runtime-state/reset" | Out-Null
    $before = Get-Reviewer
    $beforePermissions = @($before.permissions) -join "`n"
    $ids = New-ApprovedAgentConfigApplication
    $body = New-ApplyProof -ApplicationId $ids.applicationId
    $applied = Invoke-Json -Method "POST" -Path "/api/agent-config-applications/$($ids.applicationId)/apply" -Body $body
    Assert-Equal $applied.application.status "applied" "Flag-off apply should still mark application applied."
    Assert-Equal $applied.message "Mock application status changed to applied. Agent config was not modified." "Flag-off apply should keep status-only message."

    $after = Get-Reviewer
    Assert-Equal (@($after.permissions) -join "`n") $beforePermissions "Flag-off apply should not modify Agent permissions."
    $history = Invoke-Json -Method "GET" -Path "/api/agents/agent_reviewer/config-version-history"
    Assert-Equal @($history.versions).Count 0 "Flag-off apply should not create Agent config versions."
  } finally {
    Stop-SqliteApi -Process $process
  }

  Write-Step "Verify feature flag on performs one SQLite real-apply transaction."
  powershell -ExecutionPolicy Bypass -File $seedScript | Out-Null
  $process = Start-SqliteApi -EnableRealApply $true
  try {
    Invoke-Json -Method "POST" -Path "/api/runtime-state/reset" | Out-Null
    $ids = New-ApprovedAgentConfigApplication
    $body = New-ApplyProof -ApplicationId $ids.applicationId
    $applied = Invoke-Json -Method "POST" -Path "/api/agent-config-applications/$($ids.applicationId)/apply" -Body $body

    Assert-Equal $applied.application.status "applied" "Real apply should mark application applied."
    Assert-Equal $applied.version.agentId "agent_reviewer" "Real apply version should target reviewer Agent."
    Assert-Equal $applied.version.version 1 "First real apply version should be 1."
    Assert-Equal $applied.sideEffects.writesAgents $true "Real apply should write Agents."
    Assert-Equal $applied.sideEffects.writesAgentConfigVersions $true "Real apply should write Agent config versions."
    Assert-Equal $applied.sideEffects.writesAgentConfigApplications $true "Real apply should update application status."
    Assert-Equal $applied.sideEffects.writesRuntimeEvents $true "Real apply should write runtime event."
    Assert-Equal $applied.sideEffects.createsApprovals $false "Real apply should not create approvals."
    Assert-Equal $applied.sideEffects.createsRunnerJobs $false "Real apply should not create Runner jobs."
    Assert-Equal $applied.sideEffects.executesRunner $false "Real apply should not execute Runner."
    Assert-Equal $applied.sideEffects.callsRealModel $false "Real apply should not call models."
    Assert-Equal $applied.sideEffects.readsRawSecrets $false "Real apply should not read raw secrets."

    $after = Get-Reviewer
    Assert-TextContains (@($after.permissions) -join "`n") "canReviewApproval" "Real apply should store expanded reviewer_agent permissions."
    Assert-TextContains (@($after.permissions) -join "`n") "canRecommendApproval" "Real apply should store safe reviewer_agent capabilities."

    $history = Invoke-Json -Method "GET" -Path "/api/agents/agent_reviewer/config-version-history"
    Assert-Equal $history.versionHistory $true "Version history should identify itself."
    Assert-Equal @($history.versions).Count 1 "Real apply should create one version."
    Assert-Equal $history.currentVersion.version 1 "Current version should be version 1."
    Assert-TextContains (@($history.currentVersion.configSnapshot.permissions) -join "`n") "canReviewApproval" "Version snapshot should store expanded permissions."
    Assert-Equal $history.rollbackSourceReady $false "Single-version history should not be rollback-source ready."

    $secondIds = New-ApprovedAgentConfigModelApplication
    $secondBody = New-ApplyProof -ApplicationId $secondIds.applicationId
    $secondApplied = Invoke-Json -Method "POST" -Path "/api/agent-config-applications/$($secondIds.applicationId)/apply" -Body $secondBody
    Assert-Equal $secondApplied.application.status "applied" "Second real apply should mark application applied."
    Assert-Equal $secondApplied.version.version 2 "Second real apply should create version 2."
    Assert-Equal $secondApplied.version.configSnapshot.model "gpt-rollback-preview" "Second real apply should update model in snapshot."

    $historyAfterSecondApply = Invoke-Json -Method "GET" -Path "/api/agents/agent_reviewer/config-version-history"
    Assert-Equal @($historyAfterSecondApply.versions).Count 2 "Two real applies should create two versions."
    Assert-Equal $historyAfterSecondApply.currentVersion.version 2 "Version 2 should be current."
    Assert-Equal $historyAfterSecondApply.restoreVersion.version 1 "Default restore version should be version 1."
    Assert-Equal $historyAfterSecondApply.rollbackSourceReady $true "Two-version history should be rollback-source ready."

    $rollbackPreview = Invoke-Json -Method "POST" -Path "/api/agent-config-applications/$($secondIds.applicationId)/rollback-request" -Body @{
      secondConfirm = $true
      confirmText = "Verify rollback request preview only."
      requestedBy = "verify_agent_config_real_apply_sqlite"
      reason = "Verify read-only rollback preview diff."
    }
    Assert-Equal $rollbackPreview.rollbackRequest $true "Rollback preview should identify itself."
    Assert-Equal $rollbackPreview.ok $false "Rollback preview should remain feature-disabled."
    Assert-Equal $rollbackPreview.requestReady $true "Rollback preview should be ready when version history has current and restore."
    Assert-Equal $rollbackPreview.canCreateApproval $false "Rollback preview should not create approval."
    Assert-Equal $rollbackPreview.currentVersion 2 "Rollback preview should use current version 2."
    Assert-Equal $rollbackPreview.restoreVersion 1 "Rollback preview should default restore to version 1."
    Assert-Equal $rollbackPreview.versionHistory.rollbackSourceReady $true "Rollback preview should expose rollback source readiness."
    Assert-Equal $rollbackPreview.rollbackPreview.fieldCount 1 "Rollback preview should produce one model diff."
    Assert-TextContains (@($rollbackPreview.restoreDiff.field) -join "`n") "model" "Rollback diff should include model field."
    Assert-TextContains (@($rollbackPreview.restoreDiff.current) -join "`n") "gpt-rollback-preview" "Rollback diff should include current model."
    Assert-TextContains (@($rollbackPreview.restoreDiff.restore) -join "`n") "gemini-long-context" "Rollback diff should include restore model."
    Assert-TextContains (@($rollbackPreview.restoreDiff.action) -join "`n") "restore" "Rollback diff should include restore action."
    Assert-TextContains (@($rollbackPreview.blockedReasons) -join "`n") "feature_disabled" "Rollback preview should remain feature-disabled."
    Assert-Equal @($rollbackPreview.validationErrors).Count 0 "Rollback preview should have no validation errors with two versions."
    Assert-AgentConfigRollbackRequestNoSideEffects -RollbackRequest $rollbackPreview

    $jobs = Invoke-Json -Method "GET" -Path "/api/projects/$projectId/runner/jobs"
    $agentConfigJobs = @($jobs.jobs | Where-Object { $_.approvalId -eq $ids.approvalId })
    Assert-Equal $agentConfigJobs.Count 0 "Real apply should not create Runner jobs."
  } finally {
    Stop-SqliteApi -Process $process
  }

  Write-Step "Agent config real apply SQLite checks passed."
} finally {
  Stop-SqliteApi -Process $process
  $env:AGENT_SWARM_API_PORT = $previousPort
  $env:AGENT_SWARM_DASHBOARD_SOURCE = $previousSource
  if ($null -eq $previousRealApply) {
    Remove-Item Env:\AGENT_SWARM_ENABLE_AGENT_CONFIG_REAL_APPLY -ErrorAction SilentlyContinue
  } else {
    $env:AGENT_SWARM_ENABLE_AGENT_CONFIG_REAL_APPLY = $previousRealApply
  }
  try {
    powershell -ExecutionPolicy Bypass -File $seedScript | Out-Null
  } catch {
    Write-Warning "Failed to reseed SQLite after verification: $($_.Exception.Message)"
  }
}
