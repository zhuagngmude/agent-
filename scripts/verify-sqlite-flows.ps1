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

function Assert-AgentConfigVersionHistoryNoSideEffects {
  param([object]$VersionHistory)

  Assert-Equal $VersionHistory.sideEffects.writesAgents $false "Agent config version history should not write Agents."
  Assert-Equal $VersionHistory.sideEffects.writesAgentConfigVersions $false "Agent config version history should not write versions."
  Assert-Equal $VersionHistory.sideEffects.writesAgentConfigApplications $false "Agent config version history should not write applications."
  Assert-Equal $VersionHistory.sideEffects.writesRuntimeEvents $false "Agent config version history should not write runtime events."
  Assert-Equal $VersionHistory.sideEffects.writesSqlite $false "Agent config version history should not write SQLite."
  Assert-Equal $VersionHistory.sideEffects.writesRuntimeState $false "Agent config version history should not write runtime state."
  Assert-Equal $VersionHistory.sideEffects.createsApprovals $false "Agent config version history should not create approvals."
  Assert-Equal $VersionHistory.sideEffects.createsRunnerJobs $false "Agent config version history should not create Runner jobs."
  Assert-Equal $VersionHistory.sideEffects.executesRunner $false "Agent config version history should not execute Runner."
  Assert-Equal $VersionHistory.sideEffects.callsRealModel $false "Agent config version history should not call models."
  Assert-Equal $VersionHistory.sideEffects.readsRawSecrets $false "Agent config version history should not read raw secrets."
}

function Assert-ProjectPlanNoRealSideEffects {
  param(
    [object]$SideEffects,
    [string]$Prefix
  )

  Assert-Equal $SideEffects.writesProjectFiles $false "$Prefix should not write project files."
  Assert-Equal $SideEffects.modifiesGit $false "$Prefix should not modify Git."
  Assert-Equal $SideEffects.executesRunner $false "$Prefix should not execute Runner."
  Assert-Equal $SideEffects.callsRealModel $false "$Prefix should not call real models."
  Assert-Equal $SideEffects.readsRawSecrets $false "$Prefix should not read raw secrets."
  Assert-Equal $SideEffects.makesNetworkRequests $false "$Prefix should not make network requests."
  Assert-Equal $SideEffects.triggersAgents $false "$Prefix should not trigger Agents."
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

$verifyTempDir = Join-Path ([System.IO.Path]::GetTempPath()) "agent-swarm-verify-sqlite"
New-Item -ItemType Directory -Force -Path $verifyTempDir | Out-Null
$tempDbFile = Join-Path $verifyTempDir "agent-swarm-verify.sqlite"
$outLog = Join-Path $verifyTempDir "sqlite-api.out.log"
$errLog = Join-Path $verifyTempDir "sqlite-api.err.log"
$previousSqliteDb = $env:AGENT_SWARM_SQLITE_DB
$env:AGENT_SWARM_SQLITE_DB = $tempDbFile

Write-Step "Rebuild SQLite database from seed."
powershell -ExecutionPolicy Bypass -File $seedScript | Out-Null

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
    throw "SQLite API did not start. Check $outLog and $errLog"
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

  Write-Step "Verify MVP-0.4 project plan approval assigns Agents and queues read-only SQLite Runner requests."
  $planId = "verify_mvp04_sqlite"
  $planTaskPrefix = "task_${planId}_"
  $planRequest = Invoke-Json -Method "POST" -Path "/api/projects/$projectId/project-plan-requests" -Body @{
    planId = $planId
    idea = "Build a local customer lead tracker"
    constraints = "SQLite mode only; no real Runner; no real model calls"
    requestedBy = "verify_sqlite_flows"
  }
  Assert-Equal $planRequest.approval.status "pending" "SQLite project plan approval should start pending."
  Assert-Equal $planRequest.approval.targetService "project_plan" "SQLite project plan approval target service mismatch."
  Assert-Equal $planRequest.approval.changeRequest.type "project_plan" "SQLite project plan change request type mismatch."
  Assert-Equal @($planRequest.plan.tasks).Count 5 "SQLite project plan draft should include five tasks."
  Assert-Equal @($planRequest.plan.runnerRequests).Count 5 "SQLite project plan draft should include five Runner requests."
  Assert-ProjectPlanNoRealSideEffects -SideEffects $planRequest.sideEffects -Prefix "SQLite project plan draft request"
  Assert-Equal $planRequest.sideEffects.createsTasks $false "SQLite project plan draft should not create tasks before approval."
  Assert-Equal $planRequest.sideEffects.createsRunnerJobs $false "SQLite project plan draft should not create Runner jobs before approval."

  $draftTasks = Invoke-Json -Method "GET" -Path "/api/projects/$projectId/tasks"
  $draftPlanTasks = @($draftTasks.tasks | Where-Object { $_.id -like "$planTaskPrefix*" })
  Assert-Equal $draftPlanTasks.Count 0 "SQLite project plan draft should not appear in task queue before approval."
  $draftJobs = Invoke-Json -Method "GET" -Path "/api/projects/$projectId/runner/jobs"
  $draftPlanJobs = @($draftJobs.jobs | Where-Object { $_.approvalId -eq $planRequest.approval.id })
  Assert-Equal $draftPlanJobs.Count 0 "SQLite project plan draft should not create Runner queue records before approval."

  $planApproval = Invoke-Json -Method "POST" -Path "/api/approvals/$($planRequest.approval.id)/approve" -Body @{
    secondConfirm = $true
    confirmText = "Approve SQLite MVP-0.4 project plan verification."
  }
  Assert-Equal $planApproval.status "approved" "SQLite project plan approval should approve."
  Assert-Equal $planApproval.runnerJobId "" "SQLite project plan approval should not create a single generic Runner job."
  Assert-Equal @($planApproval.createdTaskIds).Count 5 "SQLite project plan approval should create five tasks."
  Assert-Equal @($planApproval.createdRunnerJobIds).Count 5 "SQLite project plan approval should create five Runner request records."
  Assert-ProjectPlanNoRealSideEffects -SideEffects $planApproval.sideEffects -Prefix "SQLite project plan approval"

  $approvedTasks = Invoke-Json -Method "GET" -Path "/api/projects/$projectId/tasks"
  $projectPlanTasks = @($approvedTasks.tasks | Where-Object { $_.id -like "$planTaskPrefix*" })
  Assert-Equal $projectPlanTasks.Count 5 "SQLite approved project plan tasks should appear in task queue."
  foreach ($agentId in @("agent_frontend", "agent_backend", "agent_qa", "agent_docs", "agent_reviewer")) {
    $assignedTasks = @($projectPlanTasks | Where-Object { $_.assignedAgentId -eq $agentId })
    Assert-Equal $assignedTasks.Count 1 "SQLite project plan should assign exactly one task to $agentId."
    Assert-Equal $assignedTasks[0].status "queued" "SQLite project plan task for $agentId should start queued."
  }

  $approvedJobs = Invoke-Json -Method "GET" -Path "/api/projects/$projectId/runner/jobs"
  $projectPlanJobs = @($approvedJobs.jobs | Where-Object { $_.approvalId -eq $planRequest.approval.id })
  Assert-Equal $projectPlanJobs.Count 5 "SQLite approved project plan should create five Runner queue records."
  foreach ($job in $projectPlanJobs) {
    Assert-Equal $job.status "queued" "SQLite project plan Runner request should start queued."
    Assert-True ($job.taskId -like "$planTaskPrefix*") "SQLite project plan Runner request should reference a project plan task."
    Assert-TextContains (@($job.operationTypes) -join "`n") "runner_request_readonly" "SQLite project plan Runner request should be read-only."
    Assert-TextContains $job.safetyNote "No command" "SQLite project plan Runner request should document no command execution."
  }

  Write-Step "Verify SQLite execution request lifecycle and runtime events."
  $executionRequests = Invoke-Json -Method "GET" -Path "/api/projects/$projectId/execution-requests"
  $runnerRequest = @($executionRequests.requests | Where-Object { $_.id -eq $approval.runnerJobId })[0]
  Assert-True ($null -ne $runnerRequest) "SQLite execution request should be present."
  Assert-Equal $runnerRequest.requestShape "execution_request_v1" "SQLite execution request shape mismatch."
  Assert-Equal $runnerRequest.lifecycle.reviewState "pending" "SQLite execution request should start pending review."
  Assert-Equal $runnerRequest.launchGate.approved $true "SQLite execution request should inherit approved gate."
  Assert-Equal $runnerRequest.lifecycle.availableActions[0] "review" "SQLite queued execution request should allow review first."
  $reviewed = Invoke-Json -Method "POST" -Path "/api/runner/jobs/$($approval.runnerJobId)/review" -Body @{
    requestedBy = "verify_sqlite_flows"
  }
  Assert-Equal $reviewed.job.status "reviewed" "SQLite review should move the request to reviewed."
  $started = Invoke-Json -Method "POST" -Path "/api/runner/jobs/$($approval.runnerJobId)/start" -Body @{
    requestedBy = "verify_sqlite_flows"
    scopeLockAccepted = $true
    secondConfirm = $true
    gitCheckpointCommit = "b84cf43"
  }
  Assert-Equal $started.job.status "running" "SQLite start should move the request to running."
  $paused = Invoke-Json -Method "POST" -Path "/api/runner/jobs/$($approval.runnerJobId)/pause" -Body @{
    requestedBy = "verify_sqlite_flows"
    reason = "pause for verification"
  }
  Assert-Equal $paused.job.status "paused" "SQLite pause should move the request to paused."
  $resumed = Invoke-Json -Method "POST" -Path "/api/runner/jobs/$($approval.runnerJobId)/start" -Body @{
    requestedBy = "verify_sqlite_flows"
    scopeLockAccepted = $true
    secondConfirm = $true
    gitCheckpointCommit = "b84cf43"
  }
  Assert-Equal $resumed.job.status "running" "SQLite resume should move the request back to running."
  $completed = Invoke-Json -Method "POST" -Path "/api/runner/jobs/$($approval.runnerJobId)/complete" -Body @{
    requestedBy = "verify_sqlite_flows"
  }
  Assert-Equal $completed.job.status "completed" "SQLite complete should finish the request."
  $executionHistory = Invoke-Json -Method "GET" -Path "/api/projects/$projectId/runtime-events?entityType=runner_job&entityId=$($approval.runnerJobId)&limit=10"
  Assert-True (@($executionHistory.events).Count -ge 5) "SQLite runner job should emit runtime events for lifecycle changes."
  $finalEvent = @($executionHistory.events | Select-Object -First 1)[0]
  Assert-Equal $finalEvent.eventType "completed" "SQLite final runtime event should be completed."

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
  $rollbackRequest = Invoke-Json -Method "POST" -Path "/api/agent-config-applications/$($applyApproval.agentConfigApplicationId)/rollback-request" -Body @{
    secondConfirm = $true
    confirmText = "Verify sqlite rollback request preview stays blocked."
    requestedBy = "verify_sqlite_flows"
    reason = "Verify disabled sqlite rollback request preview."
  }
  Assert-Equal $rollbackRequest.rollbackRequest $true "SQLite Agent config rollback request should identify itself."
  Assert-Equal $rollbackRequest.ok $false "SQLite Agent config rollback request should remain blocked."
  Assert-Equal $rollbackRequest.requestReady $false "SQLite Agent config rollback request should not be ready without version history."
  Assert-Equal $rollbackRequest.canCreateApproval $false "SQLite Agent config rollback request should not create approval."
  Assert-Equal $rollbackRequest.versionHistory.readOnly $true "SQLite Agent config rollback request should expose read-only version history."
  Assert-Equal $rollbackRequest.versionHistory.canWrite $false "SQLite Agent config rollback request version history should not allow writes."
  Assert-Equal $rollbackRequest.versionHistory.rollbackSourceReady $false "SQLite Agent config rollback request should report rollback source not ready without versions."
  Assert-Equal @($rollbackRequest.versionHistory.versions).Count 0 "SQLite Agent config rollback request should expose empty versions before real history exists."
  Assert-Equal @($rollbackRequest.restoreDiff).Count 0 "SQLite Agent config rollback request should not produce diff without current and restore versions."
  Assert-TextContains (@($rollbackRequest.blockedReasons) -join "`n") "feature_disabled" "SQLite Agent config rollback request should report feature disabled."
  Assert-TextContains (@($rollbackRequest.validationErrors) -join "`n") "current version is required." "SQLite Agent config rollback request should require current version."
  Assert-TextContains (@($rollbackRequest.validationErrors) -join "`n") "restore version is required." "SQLite Agent config rollback request should require restore version."
  Assert-AgentConfigRollbackRequestNoSideEffects -RollbackRequest $rollbackRequest
  $versionHistory = Invoke-Json -Method "GET" -Path "/api/agents/agent_reviewer/config-version-history"
  Assert-Equal $versionHistory.versionHistory $true "SQLite Agent config version history should identify itself."
  Assert-Equal $versionHistory.readOnly $true "SQLite Agent config version history should be read-only."
  Assert-Equal $versionHistory.canWrite $false "SQLite Agent config version history should not allow writes."
  Assert-Equal $versionHistory.agentId "agent_reviewer" "SQLite Agent config version history should target the requested Agent."
  Assert-Equal @($versionHistory.versions).Count 0 "SQLite Agent config version history should be empty until real versions exist."
  Assert-Equal $versionHistory.rollbackSourceReady $false "SQLite Agent config version history should not be rollback-source ready without versions."
  Assert-AgentConfigVersionHistoryNoSideEffects -VersionHistory $versionHistory
  $missingVersionHistory = Invoke-JsonExpectStatus -Method "GET" -Path "/api/agents/missing_agent/config-version-history" -ExpectedStatus 404
  Assert-Equal $missingVersionHistory.error "agent_not_found" "Missing SQLite Agent config version history should return safe not found."
  Assert-Equal $missingVersionHistory.versionHistory $true "Missing SQLite Agent config version history should identify itself."
  Assert-Equal $missingVersionHistory.canWrite $false "Missing SQLite Agent config version history should not allow writes."
  Assert-AgentConfigVersionHistoryNoSideEffects -VersionHistory $missingVersionHistory
  $missingRollbackRequest = Invoke-JsonExpectStatus -Method "POST" -Path "/api/agent-config-applications/missing_application/rollback-request" -ExpectedStatus 404 -Body @{
    secondConfirm = $true
    confirmText = "Verify missing sqlite rollback request stays safe."
    requestedBy = "verify_sqlite_flows"
    reason = "Verify missing sqlite rollback request."
  }
  Assert-Equal $missingRollbackRequest.error "agent_config_application_not_found" "Missing SQLite Agent config rollback request should return safe not found."
  Assert-Equal $missingRollbackRequest.rollbackRequest $true "Missing SQLite Agent config rollback request should identify itself."
  Assert-Equal $missingRollbackRequest.canCreateApproval $false "Missing SQLite Agent config rollback request should not create approval."
  Assert-TextContains (@($missingRollbackRequest.blockedReasons) -join "`n") "application_not_found" "Missing SQLite Agent config rollback request should report missing application."
  Assert-AgentConfigRollbackRequestNoSideEffects -RollbackRequest $missingRollbackRequest
  $applicationsAfterRollbackRequest = Invoke-Json -Method "GET" -Path "/api/projects/$projectId/agent-config-applications"
  $applicationAfterRollbackRequest = @($applicationsAfterRollbackRequest.applications | Where-Object { $_.id -eq $applyApproval.agentConfigApplicationId })[0]
  Assert-Equal $applicationAfterRollbackRequest.status "applied" "SQLite Agent config rollback request should not change application status."
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
  $env:AGENT_SWARM_SQLITE_DB = $previousSqliteDb
  try {
    Remove-Item -LiteralPath $tempDbFile -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath "$tempDbFile-shm" -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath "$tempDbFile-wal" -Force -ErrorAction SilentlyContinue
  } catch {
    Write-Warning "Failed to clean temp SQLite verification database: $($_.Exception.Message)"
  }
}
