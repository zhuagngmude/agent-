$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
$apiScript = Join-Path $root "services\api\server.js"
$port = 8789
$baseUrl = "http://127.0.0.1:$port"
$projectId = "project_agent_swarm"

function Write-Step {
  param([string]$Message)
  Write-Host "[mock-flow] $Message"
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

$verifyLogDir = Join-Path ([System.IO.Path]::GetTempPath()) "agent-swarm-verify"
New-Item -ItemType Directory -Force -Path $verifyLogDir | Out-Null
$outLog = Join-Path $verifyLogDir "mock-flow-api.out.log"
$errLog = Join-Path $verifyLogDir "mock-flow-api.err.log"
$tempRuntimeStateFile = Join-Path $verifyLogDir "mock-runtime-state.json"

if (Test-ApiReady) {
  throw "Port $port already has an API responding before verification started. This script will not attach to an existing service; stop that process or use a different isolated verification port."
}

Write-Step "Start isolated Mock API on port $port."
$previousPort = $env:AGENT_SWARM_API_PORT
$previousSource = $env:AGENT_SWARM_DASHBOARD_SOURCE
$previousRuntimeStateFile = $env:AGENT_SWARM_RUNTIME_STATE_FILE
$env:AGENT_SWARM_API_PORT = "$port"
$env:AGENT_SWARM_DASHBOARD_SOURCE = "mock"
$env:AGENT_SWARM_RUNTIME_STATE_FILE = $tempRuntimeStateFile
$process = Start-Process `
  -WindowStyle Hidden `
  -FilePath "node" `
  -ArgumentList @($apiScript) `
  -RedirectStandardOutput $outLog `
  -RedirectStandardError $errLog `
  -PassThru

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
  if ($process -and -not $process.HasExited) {
    Stop-Process -Id $process.Id -Force
    $process.WaitForExit()
  }
  $env:AGENT_SWARM_API_PORT = $previousPort
  $env:AGENT_SWARM_DASHBOARD_SOURCE = $previousSource
  $env:AGENT_SWARM_RUNTIME_STATE_FILE = $previousRuntimeStateFile
  throw "Mock API did not start on port $port. Check $outLog and $errLog"
}

try {
  Write-Step "Reset runtime state."
  Invoke-Json -Method "POST" -Path "/api/runtime-state/reset" | Out-Null

  Write-Step "Verify dashboard aggregate."
  $dashboard = Invoke-Json -Method "GET" -Path "/api/projects/$projectId/dashboard"
  Assert-Equal $dashboard.project.id $projectId "Dashboard project id mismatch."
  Assert-True `
    ($null -ne $dashboard.runnerStatus) `
    "Dashboard should include runnerStatus. If an older Mock API is already running, stop it and rerun this script."

  Write-Step "Verify Agent Run chain recording and read-only view."
  $agentRunRequest = Invoke-Json -Method "POST" -Path "/api/projects/$projectId/agent-run-requests" -Body @{
    idea = "Build a local customer lead tracker"
    constraints = "Mock mode only; no real Runner; no real model calls"
    requestedBy = "verify_mock_flows"
    chainLabel = "Mock verification Agent Run"
    simulateFailureRole = "qa"
  }
  Assert-Equal $agentRunRequest.chain.status "failed" "Agent Run chain should report the injected failure."
  Assert-Equal @($agentRunRequest.agentRuns).Count 7 "Agent Run chain should include seven runs."
  Assert-Equal @($agentRunRequest.agentRuns | Where-Object { $_.status -eq "failed" }).Count 1 "Agent Run chain should fail exactly one run."
  Assert-True ($agentRunRequest.sideEffects.callsRealModel -eq $false) "Agent Run chain should not call a real model."
  Assert-True ($agentRunRequest.sideEffects.createsRunnerJobs -eq $false) "Agent Run chain should not create Runner jobs."
  Assert-True ($agentRunRequest.sideEffects.triggersAgents -eq $false) "Agent Run chain should not trigger Agents."

  $agentRuns = Invoke-Json -Method "GET" -Path "/api/projects/$projectId/agent-runs"
  Assert-True (@($agentRuns.agentRunChains).Count -ge 2) "Agent Run read-only view should include the seed chain and the new chain."
  Assert-Equal $agentRuns.selectedChain.chainId $agentRunRequest.chain.chainId "Agent Run view should select the newest chain."
  Assert-Equal @($agentRuns.selectedChain.agentRuns).Count 7 "Selected Agent Run chain should include seven runs."

  Write-Step "Verify Runner status connection, heartbeat, and disconnect stay no-exec."
  $runnerDisconnected = Invoke-Json -Method "POST" -Path "/api/projects/$projectId/runner/status/disconnect" -Body @{
    requestedBy = "verify_mock_flows"
    reason = "prepare status connection check"
  }
  Assert-Equal $runnerDisconnected.runnerStatus.connected $false "Runner status should disconnect."
  Assert-True ($runnerDisconnected.sideEffects.executesRunner -eq $false) "Runner status disconnect should not execute Runner."
  $runnerConnected = Invoke-Json -Method "POST" -Path "/api/projects/$projectId/runner/status/connect" -Body @{
    runnerId = "verify_mock_runner"
    version = "0.1.0"
    workspaceAlias = "agent-swarm"
    workspacePath = "F:/projects/agent-swarm"
    permissions = @{
      readFiles = $true
      writeFiles = "approval_required"
      executeCommands = "approval_required"
      networkRequests = "approval_required"
    }
    capabilities = @{
      reportsStatus = $true
      reportsGitStatus = $true
      reportsDirtyWorkspace = $true
      supportsValidationCommands = $true
      executesCommands = $false
      writesFiles = $false
      modifiesGit = $false
      networkRequests = $false
    }
    gitStatus = @{
      branch = "verify-mock"
      dirty = $false
      ahead = 0
      behind = 0
      checkpointRequired = $true
    }
    validationCommands = @(
      @{
        id = "verify_mock_flows"
        label = "Mock flow verification"
        command = "powershell -ExecutionPolicy Bypass -File scripts\verify-mock-flows.ps1"
        allowed = $false
        mode = "preview_only"
      }
    )
    requestedBy = "verify_mock_flows"
  }
  Assert-Equal $runnerConnected.runnerStatus.connected $true "Runner status should connect."
  Assert-Equal $runnerConnected.runnerStatus.workspaceAlias "agent-swarm" "Runner status should keep workspace alias."
  Assert-Equal $runnerConnected.runnerStatus.capabilities.executesCommands $false "Runner status should report command execution disabled."
  Assert-Equal $runnerConnected.runnerStatus.gitStatus.dirty $false "Runner status should report clean workspace."
  Assert-True ($runnerConnected.sideEffects.executesRunner -eq $false) "Runner status connect should not execute Runner."
  $runnerHeartbeat = Invoke-Json -Method "POST" -Path "/api/projects/$projectId/runner/status/heartbeat" -Body @{
    runnerId = "verify_mock_runner"
    workspaceAlias = "agent-swarm"
    gitStatus = @{
      branch = "verify-mock"
      dirty = $true
      ahead = 0
      behind = 0
      checkpointRequired = $true
    }
    requestedBy = "verify_mock_flows"
  }
  Assert-Equal $runnerHeartbeat.runnerStatus.connected $true "Runner heartbeat should keep Runner connected."
  Assert-Equal $runnerHeartbeat.runnerStatus.gitStatus.dirty $true "Runner heartbeat should update dirty workspace status."
  Assert-True ($runnerHeartbeat.sideEffects.writesProjectFiles -eq $false) "Runner heartbeat should not write project files."

  Write-Step "Verify task start -> complete flow."
  $taskStart = Invoke-Json -Method "POST" -Path "/api/tasks/task_task_state_api/start"
  Assert-Equal $taskStart.task.status "running" "Task should be running after start."
  $taskComplete = Invoke-Json -Method "POST" -Path "/api/tasks/task_task_state_api/complete"
  Assert-Equal $taskComplete.task.status "completed" "Task should be completed after complete."

  Write-Step "Verify Runner approval creates read-only job."
  $approval = Invoke-Json -Method "POST" -Path "/api/approvals/approval_docs_safety/approve"
  Assert-Equal $approval.status "approved" "Runner approval should be approved."
  Assert-True ($approval.runnerJobId -like "runner_job_*") "Runner approval should create a runner job id."
  $jobs = Invoke-Json -Method "GET" -Path "/api/projects/$projectId/runner/jobs"
  $matchingJobs = @($jobs.jobs | Where-Object { $_.id -eq $approval.runnerJobId })
  Assert-True ($matchingJobs.Count -eq 1) "Runner job should appear in queue."

  Write-Step "Verify MVP-0.4 project plan approval assigns Agents and queues read-only Runner requests."
  $planId = "verify_mvp04_mock"
  $planTaskPrefix = "task_${planId}_"
  $planRequest = Invoke-Json -Method "POST" -Path "/api/projects/$projectId/project-plan-requests" -Body @{
    planId = $planId
    idea = "Build a local customer lead tracker"
    constraints = "Mock mode only; no real Runner; no real model calls"
    requestedBy = "verify_mock_flows"
  }
  Assert-Equal $planRequest.approval.status "pending" "Project plan approval should start pending."
  Assert-Equal $planRequest.approval.targetService "project_plan" "Project plan approval target service mismatch."
  Assert-Equal $planRequest.approval.changeRequest.type "project_plan" "Project plan change request type mismatch."
  Assert-Equal @($planRequest.plan.tasks).Count 5 "Project plan draft should include five tasks."
  Assert-Equal @($planRequest.plan.runnerRequests).Count 5 "Project plan draft should include five Runner requests."
  Assert-ProjectPlanNoRealSideEffects -SideEffects $planRequest.sideEffects -Prefix "Project plan draft request"
  Assert-Equal $planRequest.sideEffects.createsTasks $false "Project plan draft should not create tasks before approval."
  Assert-Equal $planRequest.sideEffects.createsRunnerJobs $false "Project plan draft should not create Runner jobs before approval."

  $draftTasks = Invoke-Json -Method "GET" -Path "/api/projects/$projectId/tasks"
  $draftPlanTasks = @($draftTasks.tasks | Where-Object { $_.id -like "$planTaskPrefix*" })
  Assert-Equal $draftPlanTasks.Count 0 "Project plan draft should not appear in task queue before approval."
  $draftJobs = Invoke-Json -Method "GET" -Path "/api/projects/$projectId/runner/jobs"
  $draftPlanJobs = @($draftJobs.jobs | Where-Object { $_.approvalId -eq $planRequest.approval.id })
  Assert-Equal $draftPlanJobs.Count 0 "Project plan draft should not create Runner queue records before approval."

  $planApproval = Invoke-Json -Method "POST" -Path "/api/approvals/$($planRequest.approval.id)/approve" -Body @{
    secondConfirm = $true
    confirmText = "Approve MVP-0.4 project plan verification."
  }
  Assert-Equal $planApproval.status "approved" "Project plan approval should approve."
  Assert-Equal $planApproval.runnerJobId "" "Project plan approval should not create a single generic Runner job."
  Assert-Equal @($planApproval.createdTaskIds).Count 5 "Project plan approval should create five tasks."
  Assert-Equal @($planApproval.createdRunnerJobIds).Count 5 "Project plan approval should create five Runner request records."
  Assert-ProjectPlanNoRealSideEffects -SideEffects $planApproval.sideEffects -Prefix "Project plan approval"

  $approvedTasks = Invoke-Json -Method "GET" -Path "/api/projects/$projectId/tasks"
  $projectPlanTasks = @($approvedTasks.tasks | Where-Object { $_.id -like "$planTaskPrefix*" })
  Assert-Equal $projectPlanTasks.Count 5 "Approved project plan tasks should appear in task queue."
  foreach ($agentId in @("agent_frontend", "agent_backend", "agent_qa", "agent_docs", "agent_reviewer")) {
    $assignedTasks = @($projectPlanTasks | Where-Object { $_.assignedAgentId -eq $agentId })
    Assert-Equal $assignedTasks.Count 1 "Project plan should assign exactly one task to $agentId."
    Assert-Equal $assignedTasks[0].status "queued" "Project plan task for $agentId should start queued."
  }

  $approvedJobs = Invoke-Json -Method "GET" -Path "/api/projects/$projectId/runner/jobs"
  $projectPlanJobs = @($approvedJobs.jobs | Where-Object { $_.approvalId -eq $planRequest.approval.id })
  Assert-Equal $projectPlanJobs.Count 5 "Approved project plan should create five Runner queue records."
  foreach ($job in $projectPlanJobs) {
    Assert-Equal $job.status "queued" "Project plan Runner request should start queued."
    Assert-True ($job.taskId -like "$planTaskPrefix*") "Project plan Runner request should reference a project plan task."
    Assert-TextContains (@($job.operationTypes) -join "`n") "runner_request_readonly" "Project plan Runner request should be read-only."
    Assert-TextContains $job.safetyNote "No command" "Project plan Runner request should document no command execution."
  }

  Write-Step "Verify execution request lifecycle and runtime events."
  $executionRequests = Invoke-Json -Method "GET" -Path "/api/projects/$projectId/execution-requests"
  $runnerRequest = @($executionRequests.requests | Where-Object { $_.id -eq $approval.runnerJobId })[0]
  Assert-True ($null -ne $runnerRequest) "Execution request should be present."
  Assert-Equal $runnerRequest.requestShape "execution_request_v1" "Execution request shape mismatch."
  Assert-Equal $runnerRequest.lifecycle.reviewState "pending" "New execution request should start pending review."
  Assert-Equal $runnerRequest.launchGate.approved $true "Execution request should inherit approved gate."
  Assert-Equal $runnerRequest.lifecycle.availableActions[0] "review" "Queued execution request should allow review first."
  $reviewed = Invoke-Json -Method "POST" -Path "/api/runner/jobs/$($approval.runnerJobId)/review" -Body @{
    requestedBy = "verify_mock_flows"
  }
  Assert-Equal $reviewed.job.status "reviewed" "Review should move the request to reviewed."
  $started = Invoke-Json -Method "POST" -Path "/api/runner/jobs/$($approval.runnerJobId)/start" -Body @{
    requestedBy = "verify_mock_flows"
    scopeLockAccepted = $true
    secondConfirm = $true
    gitCheckpointCommit = "b84cf43"
  }
  Assert-Equal $started.job.status "running" "Start should move the request to running."
  $paused = Invoke-Json -Method "POST" -Path "/api/runner/jobs/$($approval.runnerJobId)/pause" -Body @{
    requestedBy = "verify_mock_flows"
    reason = "pause for verification"
  }
  Assert-Equal $paused.job.status "paused" "Pause should move the request to paused."
  $resumed = Invoke-Json -Method "POST" -Path "/api/runner/jobs/$($approval.runnerJobId)/start" -Body @{
    requestedBy = "verify_mock_flows"
    scopeLockAccepted = $true
    secondConfirm = $true
    gitCheckpointCommit = "b84cf43"
  }
  Assert-Equal $resumed.job.status "running" "Resume should move the request back to running."
  $completed = Invoke-Json -Method "POST" -Path "/api/runner/jobs/$($approval.runnerJobId)/complete" -Body @{
    requestedBy = "verify_mock_flows"
  }
  Assert-Equal $completed.job.status "completed" "Complete should finish the request."
  $executionHistory = Invoke-Json -Method "GET" -Path "/api/projects/$projectId/runtime-events?entityType=runner_job&entityId=$($approval.runnerJobId)&limit=10"
  Assert-True (@($executionHistory.events).Count -ge 5) "Runner job should emit runtime events for lifecycle changes."
  $finalEvent = @($executionHistory.events | Select-Object -First 1)[0]
  Assert-Equal $finalEvent.eventType "completed" "Final runtime event should be completed."

  Write-Step "Verify invalid Agent permission request is rejected."
  $invalidPermission = Invoke-JsonExpectStatus -Method "POST" -Path "/api/agents/agent_reviewer/change-requests" -ExpectedStatus 422 -Body @{
    changeType = "permission"
    riskLevel = "high"
    reason = "Verify invalid mock agent permission validation."
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
  Assert-Equal $invalidApproval.Count 0 "Invalid permission request should not create approval."

  Write-Step "Verify Agent config apply flow."
  $agentsBeforeApplyRequest = Invoke-Json -Method "GET" -Path "/api/projects/$projectId/agents"
  $reviewerBeforeApplyRequest = @($agentsBeforeApplyRequest.agents | Where-Object { $_.id -eq "agent_reviewer" })[0]
  $reviewerPermissionsBeforeApplyRequest = @($reviewerBeforeApplyRequest.permissions) -join "`n"
  $applyRequest = Invoke-Json -Method "POST" -Path "/api/agents/agent_reviewer/change-requests" -Body @{
    changeType = "permission"
    riskLevel = "high"
    reason = "Verify mock agent config apply flow."
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
  Assert-Equal $applyRequest.approval.changeRequest.permissionValidation.ok $true "Approval should store permission validation."
  $applyApproval = Invoke-Json -Method "POST" -Path "/api/approvals/$($applyRequest.approval.id)/approve" -Body @{
    secondConfirm = $true
    confirmText = "Verify agent config approval."
  }
  Assert-Equal $applyApproval.status "approved" "Agent config approval should be approved."
  Assert-Equal $applyApproval.runnerJobId "" "Agent config approval should not create a Runner job."
  Assert-True ($applyApproval.agentConfigApplicationId -like "agent_config_application_*") "Approval should create an application id."
  $applicationsAfterApproval = Invoke-Json -Method "GET" -Path "/api/projects/$projectId/agent-config-applications"
  $pendingApplication = @($applicationsAfterApproval.applications | Where-Object { $_.id -eq $applyApproval.agentConfigApplicationId })[0]
  Assert-True ($null -ne $pendingApplication) "Agent config approval should create a pending application record."
  Assert-Equal $pendingApplication.status "pending_apply" "Agent config application should start pending_apply."
  Assert-Equal $pendingApplication.approvalId $applyRequest.approval.id "Agent config application should reference the source approval."
  Assert-Equal $pendingApplication.agentId "agent_reviewer" "Agent config application should target the reviewer agent."
  $jobsAfterAgentConfigApproval = Invoke-Json -Method "GET" -Path "/api/projects/$projectId/runner/jobs"
  $agentConfigJobs = @($jobsAfterAgentConfigApproval.jobs | Where-Object { $_.approvalId -eq $applyRequest.approval.id })
  Assert-Equal $agentConfigJobs.Count 0 "Agent config approval should not create a Runner job queue item."
  $agentsAfterAgentConfigApproval = Invoke-Json -Method "GET" -Path "/api/projects/$projectId/agents"
  $reviewerAfterAgentConfigApproval = @($agentsAfterAgentConfigApproval.agents | Where-Object { $_.id -eq "agent_reviewer" })[0]
  Assert-Equal (@($reviewerAfterAgentConfigApproval.permissions) -join "`n") $reviewerPermissionsBeforeApplyRequest "Agent config approval should not modify Agent permissions."
  $dryRun = Invoke-Json -Method "POST" -Path "/api/agent-config-applications/$($applyApproval.agentConfigApplicationId)/dry-run" -Body @{
    secondConfirm = $true
    confirmText = "Verify mock dry-run stays blocked."
    requestedBy = "verify_mock_flows"
  }
  Assert-Equal $dryRun.dryRun $true "Agent config dry-run should identify itself as dry-run."
  Assert-Equal $dryRun.ok $false "Agent config dry-run should remain blocked."
  Assert-Equal $dryRun.canApply $false "Agent config dry-run should not allow apply."
  Assert-TextContains (@($dryRun.blockedReasons) -join "`n") "feature_disabled" "Agent config dry-run should report feature disabled."
  Assert-Equal $dryRun.applicationId $applyApproval.agentConfigApplicationId "Agent config dry-run should reference the application."
  Assert-Equal $dryRun.approvalId $applyRequest.approval.id "Agent config dry-run should reference the approval."
  Assert-Equal $dryRun.agentId "agent_reviewer" "Agent config dry-run should reference the target Agent."
  Assert-Equal @($dryRun.validationErrors).Count 0 "Agent config dry-run should have no validation errors for a valid blocked preview."
  Assert-Equal $dryRun.writePlan.wouldUpdateAgent $false "Agent config dry-run should not update Agent."
  Assert-Equal $dryRun.writePlan.wouldCreateVersion $false "Agent config dry-run should not create version."
  Assert-Equal $dryRun.writePlan.wouldWriteRuntimeEvent $false "Agent config dry-run should not write runtime event."
  Assert-TextContains (@($dryRun.writePlan.changedFields) -join "`n") "permissions" "Agent config dry-run should preview changed fields."
  Assert-Equal $dryRun.rollbackPlan.rollbackRequiresNewApproval $true "Agent config dry-run rollback should require approval."
  Assert-Equal $dryRun.rollbackPlan.rollbackAction "create_new_agent_config_application" "Agent config dry-run rollback action mismatch."
  Assert-AgentConfigDryRunNoSideEffects -DryRun $dryRun
  $applicationsAfterDryRun = Invoke-Json -Method "GET" -Path "/api/projects/$projectId/agent-config-applications"
  $applicationAfterDryRun = @($applicationsAfterDryRun.applications | Where-Object { $_.id -eq $applyApproval.agentConfigApplicationId })[0]
  Assert-Equal $applicationAfterDryRun.status "pending_apply" "Agent config dry-run should not change application status."
  $agentsAfterDryRun = Invoke-Json -Method "GET" -Path "/api/projects/$projectId/agents"
  $reviewerAfterDryRun = @($agentsAfterDryRun.agents | Where-Object { $_.id -eq "agent_reviewer" })[0]
  Assert-Equal (@($reviewerAfterDryRun.permissions) -join "`n") $reviewerPermissionsBeforeApplyRequest "Agent config dry-run should not modify Agent permissions."
  $missingDryRun = Invoke-JsonExpectStatus -Method "POST" -Path "/api/agent-config-applications/missing_application/dry-run" -ExpectedStatus 404 -Body @{
    secondConfirm = $true
    confirmText = "Verify missing dry-run stays safe."
  }
  Assert-Equal $missingDryRun.error "agent_config_application_not_found" "Missing Agent config dry-run should return safe not found."
  Assert-Equal $missingDryRun.dryRun $true "Missing Agent config dry-run should identify itself as dry-run."
  Assert-Equal $missingDryRun.canApply $false "Missing Agent config dry-run should not allow apply."
  Assert-TextContains (@($missingDryRun.blockedReasons) -join "`n") "application_not_found" "Missing Agent config dry-run should report missing application."
  Assert-AgentConfigDryRunNoSideEffects -DryRun $missingDryRun
  $applied = Invoke-Json -Method "POST" -Path "/api/agent-config-applications/$($applyApproval.agentConfigApplicationId)/apply" -Body @{
    secondConfirm = $true
    confirmText = "Verify mock apply status transition."
    appliedBy = "verify_mock_flows"
  }
  Assert-Equal $applied.application.status "applied" "Agent config application should be applied."
  $appliedDryRun = Invoke-Json -Method "POST" -Path "/api/agent-config-applications/$($applyApproval.agentConfigApplicationId)/dry-run" -Body @{
    secondConfirm = $true
    confirmText = "Verify applied dry-run stays blocked."
    requestedBy = "verify_mock_flows"
  }
  Assert-Equal $appliedDryRun.dryRun $true "Applied Agent config dry-run should identify itself as dry-run."
  Assert-Equal $appliedDryRun.canApply $false "Applied Agent config dry-run should not allow apply."
  Assert-TextContains (@($appliedDryRun.validationErrors) -join "`n") "application must be pending_apply" "Applied Agent config dry-run should reject non-pending state."
  Assert-AgentConfigDryRunNoSideEffects -DryRun $appliedDryRun
  $rollbackRequest = Invoke-Json -Method "POST" -Path "/api/agent-config-applications/$($applyApproval.agentConfigApplicationId)/rollback-request" -Body @{
    secondConfirm = $true
    confirmText = "Verify rollback request preview stays blocked."
    requestedBy = "verify_mock_flows"
    reason = "Verify disabled rollback request preview."
  }
  Assert-Equal $rollbackRequest.rollbackRequest $true "Agent config rollback request should identify itself."
  Assert-Equal $rollbackRequest.ok $false "Agent config rollback request should remain blocked."
  Assert-Equal $rollbackRequest.requestReady $false "Agent config rollback request should not be ready without version history."
  Assert-Equal $rollbackRequest.canCreateApproval $false "Agent config rollback request should not create approval."
  Assert-Equal $rollbackRequest.versionHistory.readOnly $true "Agent config rollback request should expose read-only version history."
  Assert-Equal $rollbackRequest.versionHistory.canWrite $false "Agent config rollback request version history should not allow writes."
  Assert-Equal $rollbackRequest.versionHistory.rollbackSourceReady $false "Agent config rollback request should report rollback source not ready without versions."
  Assert-Equal @($rollbackRequest.versionHistory.versions).Count 0 "Agent config rollback request should expose empty versions before real history exists."
  Assert-Equal @($rollbackRequest.restoreDiff).Count 0 "Agent config rollback request should not produce diff without current and restore versions."
  Assert-TextContains (@($rollbackRequest.blockedReasons) -join "`n") "feature_disabled" "Agent config rollback request should report feature disabled."
  Assert-TextContains (@($rollbackRequest.validationErrors) -join "`n") "current version is required." "Agent config rollback request should require current version."
  Assert-TextContains (@($rollbackRequest.validationErrors) -join "`n") "restore version is required." "Agent config rollback request should require restore version."
  Assert-AgentConfigRollbackRequestNoSideEffects -RollbackRequest $rollbackRequest
  $versionHistory = Invoke-Json -Method "GET" -Path "/api/agents/agent_reviewer/config-version-history"
  Assert-Equal $versionHistory.versionHistory $true "Agent config version history should identify itself."
  Assert-Equal $versionHistory.readOnly $true "Agent config version history should be read-only."
  Assert-Equal $versionHistory.canWrite $false "Agent config version history should not allow writes."
  Assert-Equal $versionHistory.agentId "agent_reviewer" "Agent config version history should target the requested Agent."
  Assert-Equal @($versionHistory.versions).Count 0 "Agent config version history should be empty until real versions exist."
  Assert-Equal $versionHistory.rollbackSourceReady $false "Agent config version history should not be rollback-source ready without versions."
  Assert-AgentConfigVersionHistoryNoSideEffects -VersionHistory $versionHistory
  $missingVersionHistory = Invoke-JsonExpectStatus -Method "GET" -Path "/api/agents/missing_agent/config-version-history" -ExpectedStatus 404
  Assert-Equal $missingVersionHistory.error "agent_not_found" "Missing Agent config version history should return safe not found."
  Assert-Equal $missingVersionHistory.versionHistory $true "Missing Agent config version history should identify itself."
  Assert-Equal $missingVersionHistory.canWrite $false "Missing Agent config version history should not allow writes."
  Assert-AgentConfigVersionHistoryNoSideEffects -VersionHistory $missingVersionHistory
  $missingRollbackRequest = Invoke-JsonExpectStatus -Method "POST" -Path "/api/agent-config-applications/missing_application/rollback-request" -ExpectedStatus 404 -Body @{
    secondConfirm = $true
    confirmText = "Verify missing rollback request stays safe."
    requestedBy = "verify_mock_flows"
    reason = "Verify missing rollback request."
  }
  Assert-Equal $missingRollbackRequest.error "agent_config_application_not_found" "Missing Agent config rollback request should return safe not found."
  Assert-Equal $missingRollbackRequest.rollbackRequest $true "Missing Agent config rollback request should identify itself."
  Assert-Equal $missingRollbackRequest.canCreateApproval $false "Missing Agent config rollback request should not create approval."
  Assert-TextContains (@($missingRollbackRequest.blockedReasons) -join "`n") "application_not_found" "Missing Agent config rollback request should report missing application."
  Assert-AgentConfigRollbackRequestNoSideEffects -RollbackRequest $missingRollbackRequest
  $applicationsAfterRollbackRequest = Invoke-Json -Method "GET" -Path "/api/projects/$projectId/agent-config-applications"
  $applicationAfterRollbackRequest = @($applicationsAfterRollbackRequest.applications | Where-Object { $_.id -eq $applyApproval.agentConfigApplicationId })[0]
  Assert-Equal $applicationAfterRollbackRequest.status "applied" "Agent config rollback request should not change application status."
  $agentsAfterMockApply = Invoke-Json -Method "GET" -Path "/api/projects/$projectId/agents"
  $reviewerAfterMockApply = @($agentsAfterMockApply.agents | Where-Object { $_.id -eq "agent_reviewer" })[0]
  Assert-Equal (@($reviewerAfterMockApply.permissions) -join "`n") $reviewerPermissionsBeforeApplyRequest "Mock apply should not modify Agent permissions."

  Write-Step "Verify Agent config cancel flow."
  $cancelRequest = Invoke-Json -Method "POST" -Path "/api/agents/agent_docs/change-requests" -Body @{
    changeType = "model"
    riskLevel = "medium"
    reason = "Verify mock agent config cancel flow."
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
    reason = "Verify mock cancel status transition."
    cancelledBy = "verify_mock_flows"
  }
  Assert-Equal $cancelled.application.status "cancelled" "Agent config application should be cancelled."

  Write-Step "All mock flow checks passed."
} finally {
  Write-Step "Reset runtime state after verification."
  try {
    Invoke-Json -Method "POST" -Path "/api/runtime-state/reset" | Out-Null
  } catch {
    Write-Warning "Failed to reset runtime state: $($_.Exception.Message)"
  }
  Write-Step "Stop isolated Mock API and restore environment."
  if ($process -and -not $process.HasExited) {
    Stop-Process -Id $process.Id -Force
    $process.WaitForExit()
  }
  $env:AGENT_SWARM_API_PORT = $previousPort
  $env:AGENT_SWARM_DASHBOARD_SOURCE = $previousSource
  $env:AGENT_SWARM_RUNTIME_STATE_FILE = $previousRuntimeStateFile
  try {
    Remove-Item -LiteralPath $tempRuntimeStateFile -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath "$tempRuntimeStateFile.tmp" -Force -ErrorAction SilentlyContinue
  } catch {
    Write-Warning "Failed to clean temp Mock runtime state: $($_.Exception.Message)"
  }
}
