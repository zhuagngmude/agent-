$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
$apiScript = Join-Path $root "services\api\server.js"
$seedScript = Join-Path $PSScriptRoot "seed-sqlite.ps1"
$port = 8788
$baseUrl = "http://127.0.0.1:$port"
$projectId = "project_agent_swarm"

function Write-Step {
  param([string]$Message)
  Write-Host "[sqlite-flow] $Message"
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
    -Body ($Body | ConvertTo-Json -Depth 10)
}

function Invoke-JsonExpectStatus {
  param(
    [Parameter(Mandatory = $true)][string]$Method,
    [Parameter(Mandatory = $true)][string]$Path,
    [Parameter(Mandatory = $true)][int]$ExpectedStatus,
    [object]$Body = $null
  )

  try {
    $result = Invoke-Json -Method $Method -Path $Path -Body $Body
    throw "Expected HTTP $ExpectedStatus but request succeeded: $($result | ConvertTo-Json -Depth 10)"
  } catch {
    $response = $_.Exception.Response
    if ($null -eq $response) {
      throw
    }

    $actualStatus = [int]$response.StatusCode
    if ($actualStatus -ne $ExpectedStatus) {
      throw "Expected HTTP $ExpectedStatus, got HTTP $actualStatus."
    }

    $raw = $_.ErrorDetails.Message
    if (-not $raw) {
      $reader = [System.IO.StreamReader]::new($response.GetResponseStream())
      $raw = $reader.ReadToEnd()
    }

    return $raw | ConvertFrom-Json
  }
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

function Assert-AgentConfigDryRunNoSideEffects {
  param([object]$DryRun)

  Assert-Equal $DryRun.sideEffects.writesAgents $false "Agent config dry-run should not write Agents."
  Assert-Equal $DryRun.sideEffects.writesAgentConfigVersions $false "Agent config dry-run should not write versions."
  Assert-Equal $DryRun.sideEffects.writesRuntimeEvents $false "Agent config dry-run should not write runtime events."
  Assert-Equal $DryRun.sideEffects.writesSqlite $false "Agent config dry-run should not write SQLite."
  Assert-Equal $DryRun.sideEffects.writesRuntimeState $false "Agent config dry-run should not write runtime state."
  Assert-Equal $DryRun.sideEffects.createsApprovals $false "Agent config dry-run should not create approvals."
  Assert-Equal $DryRun.sideEffects.createsRunnerJobs $false "Agent config dry-run should not create Runner jobs."
  Assert-Equal $DryRun.sideEffects.executesRunner $false "Agent config dry-run should not execute Runner."
  Assert-Equal $DryRun.sideEffects.callsRealModel $false "Agent config dry-run should not call models."
  Assert-Equal $DryRun.sideEffects.readsRawSecrets $false "Agent config dry-run should not read raw secrets."
}

function Test-ApiReady {
  try {
    $health = Invoke-Json -Method "GET" -Path "/api/health"
    return $health.ok -eq $true
  } catch {
    return $false
  }
}

if (Test-ApiReady) {
  throw "Port $port already has an API responding before verification started. This script will not attach to an existing service; stop that process or use a different isolated verification port."
}

Write-Step "Rebuild SQLite database from seed."
powershell -ExecutionPolicy Bypass -File $seedScript | Out-Null

$outLog = Join-Path $root "logs\sqlite-api.out.log"
$errLog = Join-Path $root "logs\sqlite-api.err.log"
New-Item -ItemType Directory -Force -Path (Split-Path -Parent $outLog) | Out-Null

Write-Step "Start API in SQLite mode on port $port."
$previousPort = $env:AGENT_SWARM_API_PORT
$previousSource = $env:AGENT_SWARM_DASHBOARD_SOURCE
$env:AGENT_SWARM_API_PORT = "$port"
$env:AGENT_SWARM_DASHBOARD_SOURCE = "sqlite"
$process = Start-Process `
  -WindowStyle Hidden `
  -FilePath "node" `
  -ArgumentList @($apiScript) `
  -RedirectStandardOutput $outLog `
  -RedirectStandardError $errLog `
  -PassThru

try {
  $ready = $false
  for ($i = 0; $i -lt 20; $i++) {
    Start-Sleep -Milliseconds 250
    if ($process.HasExited) {
      break
    }
    if (Test-ApiReady) {
      $ready = $true
      break
    }
  }

  if (-not $ready) {
    throw "SQLite API did not start. Check logs/sqlite-api.out.log and logs/sqlite-api.err.log"
  }

  Write-Step "Reset SQLite runtime state."
  $reset = Invoke-Json -Method "POST" -Path "/api/runtime-state/reset"
  Assert-Equal $reset.mode "sqlite" "Runtime reset should run in SQLite mode."

  Write-Step "Verify dashboard and SQLite runtime state."
  $dashboard = Invoke-Json -Method "GET" -Path "/api/projects/$projectId/dashboard"
  Assert-Equal $dashboard.project.id $projectId "Dashboard project id mismatch."
  Assert-True ($null -ne $dashboard.runnerStatus) "Dashboard should include runnerStatus."
  $runtimeState = Invoke-Json -Method "GET" -Path "/api/runtime-state"
  Assert-Equal $runtimeState.mode "sqlite" "Runtime state endpoint should report SQLite mode."
  Assert-True ($runtimeState.sqliteRuntimeState -eq $true) "Runtime state should be SQLite-backed."

  Write-Step "Verify task start -> complete persists in SQLite."
  $taskStart = Invoke-Json -Method "POST" -Path "/api/tasks/task_task_state_api/start"
  Assert-Equal $taskStart.task.status "running" "Task should be running after start."
  $taskComplete = Invoke-Json -Method "POST" -Path "/api/tasks/task_task_state_api/complete"
  Assert-Equal $taskComplete.task.status "completed" "Task should be completed after complete."
  $tasks = Invoke-Json -Method "GET" -Path "/api/projects/$projectId/tasks"
  $completedTask = @($tasks.tasks | Where-Object { $_.id -eq "task_task_state_api" })[0]
  Assert-Equal $completedTask.status "completed" "Task status should be read back from SQLite."

  Write-Step "Verify Runner approval creates read-only SQLite job."
  $approval = Invoke-Json -Method "POST" -Path "/api/approvals/approval_docs_safety/approve"
  Assert-Equal $approval.status "approved" "Runner approval should be approved."
  Assert-True ($approval.runnerJobId -like "runner_job_*") "Runner approval should create a runner job id."
  $jobs = Invoke-Json -Method "GET" -Path "/api/projects/$projectId/runner/jobs"
  $matchingJobs = @($jobs.jobs | Where-Object { $_.id -eq $approval.runnerJobId })
  Assert-True ($matchingJobs.Count -eq 1) "Runner job should be read back from SQLite."

  Write-Step "Verify invalid Agent permission request is rejected before SQLite write."
  $invalidPermission = Invoke-JsonExpectStatus -Method "POST" -Path "/api/agents/agent_reviewer/change-requests" -ExpectedStatus 422 -Body @{
    changeType = "permission"
    riskLevel = "high"
    reason = "Verify invalid sqlite agent permission validation."
    capabilities = @("canViewProject", "canExecuteRunnerJob")
    changes = @(
      @{
        field = "permissions"
        before = "reviewer_agent"
        after = "canViewProject / canExecuteRunnerJob"
      }
    )
  }
  Assert-Equal $invalidPermission.error "agent_permission_validation_failed" "Invalid permission request should fail validation."
  Assert-Equal $invalidPermission.permissionValidation.ok $false "Invalid permission validation should be false."
  Assert-TextContains (@($invalidPermission.permissionValidation.forbiddenCapabilities) -join "`n") "canExecuteRunnerJob" "Invalid permission should identify Runner execution."
  $approvalsAfterInvalidPermission = Invoke-Json -Method "GET" -Path "/api/projects/$projectId/approvals"
  $invalidApproval = @($approvalsAfterInvalidPermission.approvals | Where-Object { $_.id -eq "approval_agent_agent_reviewer_permission" })
  Assert-Equal $invalidApproval.Count 0 "Invalid permission request should not create SQLite approval."

  Write-Step "Verify Agent config apply persists in SQLite."
  $agentsBeforeApplyRequest = Invoke-Json -Method "GET" -Path "/api/projects/$projectId/agents"
  $reviewerBeforeApplyRequest = @($agentsBeforeApplyRequest.agents | Where-Object { $_.id -eq "agent_reviewer" })[0]
  $reviewerPermissionsBeforeApplyRequest = @($reviewerBeforeApplyRequest.permissions) -join "`n"
  $applyRequest = Invoke-Json -Method "POST" -Path "/api/agents/agent_reviewer/change-requests" -Body @{
    changeType = "permission"
    riskLevel = "high"
    reason = "Verify sqlite agent config apply flow."
    permissionProfile = "reviewer_agent"
    changes = @(
      @{
        field = "permissions"
        before = "read_project / review_risk / review_diff"
        after = "reviewer_agent"
      }
    )
  }
  Assert-Equal $applyRequest.approval.status "pending" "Agent config approval should start pending."
  Assert-Equal $applyRequest.permissionValidation.ok $true "Safe permission profile should validate."
  Assert-Equal $applyRequest.approval.changeRequest.permissionValidation.ok $true "SQLite approval should store permission validation."
  $applyApproval = Invoke-Json -Method "POST" -Path "/api/approvals/$($applyRequest.approval.id)/approve" -Body @{
    secondConfirm = $true
    confirmText = "Verify sqlite agent config approval."
  }
  Assert-Equal $applyApproval.status "approved" "Agent config approval should be approved."
  Assert-Equal $applyApproval.runnerJobId "" "Agent config approval should not create a Runner job."
  Assert-True ($applyApproval.agentConfigApplicationId -like "agent_config_application_*") "Approval should create an application id."
  $applicationsAfterApproval = Invoke-Json -Method "GET" -Path "/api/projects/$projectId/agent-config-applications"
  $pendingApplication = @($applicationsAfterApproval.applications | Where-Object { $_.id -eq $applyApproval.agentConfigApplicationId })[0]
  Assert-True ($null -ne $pendingApplication) "Agent config approval should create a pending SQLite application record."
  Assert-Equal $pendingApplication.status "pending_apply" "SQLite agent config application should start pending_apply."
  Assert-Equal $pendingApplication.approvalId $applyRequest.approval.id "SQLite agent config application should reference the source approval."
  Assert-Equal $pendingApplication.agentId "agent_reviewer" "SQLite agent config application should target the reviewer agent."
  $jobsAfterAgentConfigApproval = Invoke-Json -Method "GET" -Path "/api/projects/$projectId/runner/jobs"
  $agentConfigJobs = @($jobsAfterAgentConfigApproval.jobs | Where-Object { $_.approvalId -eq $applyRequest.approval.id })
  Assert-Equal $agentConfigJobs.Count 0 "SQLite agent config approval should not create a Runner job queue item."
  $agentsAfterAgentConfigApproval = Invoke-Json -Method "GET" -Path "/api/projects/$projectId/agents"
  $reviewerAfterAgentConfigApproval = @($agentsAfterAgentConfigApproval.agents | Where-Object { $_.id -eq "agent_reviewer" })[0]
  Assert-Equal (@($reviewerAfterAgentConfigApproval.permissions) -join "`n") $reviewerPermissionsBeforeApplyRequest "SQLite agent config approval should not modify Agent permissions."
  $dryRun = Invoke-Json -Method "POST" -Path "/api/agent-config-applications/$($applyApproval.agentConfigApplicationId)/dry-run" -Body @{
    secondConfirm = $true
    confirmText = "Verify sqlite dry-run stays blocked."
    requestedBy = "verify_sqlite_flows"
  }
  Assert-Equal $dryRun.dryRun $true "SQLite agent config dry-run should identify itself as dry-run."
  Assert-Equal $dryRun.ok $false "SQLite agent config dry-run should remain blocked."
  Assert-Equal $dryRun.canApply $false "SQLite agent config dry-run should not allow apply."
  Assert-TextContains (@($dryRun.blockedReasons) -join "`n") "feature_disabled" "SQLite agent config dry-run should report feature disabled."
  Assert-Equal $dryRun.applicationId $applyApproval.agentConfigApplicationId "SQLite agent config dry-run should reference the application."
  Assert-Equal $dryRun.approvalId $applyRequest.approval.id "SQLite agent config dry-run should reference the approval."
  Assert-Equal $dryRun.agentId "agent_reviewer" "SQLite agent config dry-run should reference the target Agent."
  Assert-Equal @($dryRun.validationErrors).Count 0 "SQLite agent config dry-run should have no validation errors for a valid blocked preview."
  Assert-Equal $dryRun.writePlan.wouldUpdateAgent $false "SQLite agent config dry-run should not update Agent."
  Assert-Equal $dryRun.writePlan.wouldCreateVersion $false "SQLite agent config dry-run should not create version."
  Assert-Equal $dryRun.writePlan.wouldWriteRuntimeEvent $false "SQLite agent config dry-run should not write runtime event."
  Assert-TextContains (@($dryRun.writePlan.changedFields) -join "`n") "permissions" "SQLite agent config dry-run should preview changed fields."
  Assert-Equal $dryRun.rollbackPlan.rollbackRequiresNewApproval $true "SQLite agent config dry-run rollback should require approval."
  Assert-Equal $dryRun.rollbackPlan.rollbackAction "create_new_agent_config_application" "SQLite agent config dry-run rollback action mismatch."
  Assert-AgentConfigDryRunNoSideEffects -DryRun $dryRun
  $applicationsAfterDryRun = Invoke-Json -Method "GET" -Path "/api/projects/$projectId/agent-config-applications"
  $applicationAfterDryRun = @($applicationsAfterDryRun.applications | Where-Object { $_.id -eq $applyApproval.agentConfigApplicationId })[0]
  Assert-Equal $applicationAfterDryRun.status "pending_apply" "SQLite agent config dry-run should not change application status."
  $agentsAfterDryRun = Invoke-Json -Method "GET" -Path "/api/projects/$projectId/agents"
  $reviewerAfterDryRun = @($agentsAfterDryRun.agents | Where-Object { $_.id -eq "agent_reviewer" })[0]
  Assert-Equal (@($reviewerAfterDryRun.permissions) -join "`n") $reviewerPermissionsBeforeApplyRequest "SQLite agent config dry-run should not modify Agent permissions."
  $missingDryRun = Invoke-JsonExpectStatus -Method "POST" -Path "/api/agent-config-applications/missing_application/dry-run" -ExpectedStatus 404 -Body @{
    secondConfirm = $true
    confirmText = "Verify missing sqlite dry-run stays safe."
  }
  Assert-Equal $missingDryRun.error "agent_config_application_not_found" "Missing SQLite Agent config dry-run should return safe not found."
  Assert-Equal $missingDryRun.dryRun $true "Missing SQLite Agent config dry-run should identify itself as dry-run."
  Assert-Equal $missingDryRun.canApply $false "Missing SQLite Agent config dry-run should not allow apply."
  Assert-TextContains (@($missingDryRun.blockedReasons) -join "`n") "application_not_found" "Missing SQLite Agent config dry-run should report missing application."
  Assert-AgentConfigDryRunNoSideEffects -DryRun $missingDryRun
  $applied = Invoke-Json -Method "POST" -Path "/api/agent-config-applications/$($applyApproval.agentConfigApplicationId)/apply" -Body @{
    secondConfirm = $true
    confirmText = "Verify sqlite apply status transition."
    appliedBy = "verify_sqlite_flows"
  }
  Assert-Equal $applied.application.status "applied" "Agent config application should be applied."
  $appliedDryRun = Invoke-Json -Method "POST" -Path "/api/agent-config-applications/$($applyApproval.agentConfigApplicationId)/dry-run" -Body @{
    secondConfirm = $true
    confirmText = "Verify applied sqlite dry-run stays blocked."
    requestedBy = "verify_sqlite_flows"
  }
  Assert-Equal $appliedDryRun.dryRun $true "Applied SQLite Agent config dry-run should identify itself as dry-run."
  Assert-Equal $appliedDryRun.canApply $false "Applied SQLite Agent config dry-run should not allow apply."
  Assert-TextContains (@($appliedDryRun.validationErrors) -join "`n") "application must be pending_apply" "Applied SQLite Agent config dry-run should reject non-pending state."
  Assert-AgentConfigDryRunNoSideEffects -DryRun $appliedDryRun
  $agentsAfterSqliteApply = Invoke-Json -Method "GET" -Path "/api/projects/$projectId/agents"
  $reviewerAfterSqliteApply = @($agentsAfterSqliteApply.agents | Where-Object { $_.id -eq "agent_reviewer" })[0]
  Assert-Equal (@($reviewerAfterSqliteApply.permissions) -join "`n") $reviewerPermissionsBeforeApplyRequest "SQLite mock apply should not modify Agent permissions."

  Write-Step "Verify Agent config cancel persists in SQLite."
  $cancelRequest = Invoke-Json -Method "POST" -Path "/api/agents/agent_docs/change-requests" -Body @{
    changeType = "model"
    riskLevel = "medium"
    reason = "Verify sqlite agent config cancel flow."
    changes = @(
      @{
        field = "model"
        before = "gpt-docs"
        after = "gpt-docs-next"
      }
    )
  }
  $cancelApproval = Invoke-Json -Method "POST" -Path "/api/approvals/$($cancelRequest.approval.id)/approve"
  Assert-Equal $cancelApproval.status "approved" "Cancelable Agent config approval should be approved."
  $cancelled = Invoke-Json -Method "POST" -Path "/api/agent-config-applications/$($cancelApproval.agentConfigApplicationId)/cancel" -Body @{
    reason = "Verify sqlite cancel status transition."
    cancelledBy = "verify_sqlite_flows"
  }
  Assert-Equal $cancelled.application.status "cancelled" "Agent config application should be cancelled."

  Write-Step "Verify reset restores seeded SQLite state."
  Invoke-Json -Method "POST" -Path "/api/runtime-state/reset" | Out-Null
  $resetTasks = Invoke-Json -Method "GET" -Path "/api/projects/$projectId/tasks"
  $resetTask = @($resetTasks.tasks | Where-Object { $_.id -eq "task_task_state_api" })[0]
  Assert-Equal $resetTask.status "queued" "Reset should restore seeded task status."

  Write-Step "All SQLite flow checks passed."
} finally {
  Write-Step "Stop SQLite API and restore environment."
  if ($process -and -not $process.HasExited) {
    Stop-Process -Id $process.Id -Force
    $process.WaitForExit()
  }
  $env:AGENT_SWARM_API_PORT = $previousPort
  $env:AGENT_SWARM_DASHBOARD_SOURCE = $previousSource
  try {
    powershell -ExecutionPolicy Bypass -File $seedScript | Out-Null
  } catch {
    Write-Warning "Failed to reseed SQLite after verification: $($_.Exception.Message)"
  }
}
