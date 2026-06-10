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

function Test-ApiReady {
  try {
    $health = Invoke-Json -Method "GET" -Path "/api/health"
    return $health.ok -eq $true
  } catch {
    return $false
  }
}

$outLog = Join-Path $root "logs\mock-flow-api.out.log"
$errLog = Join-Path $root "logs\mock-flow-api.err.log"
New-Item -ItemType Directory -Force -Path (Split-Path -Parent $outLog) | Out-Null

if (Test-ApiReady) {
  throw "Port $port already has an API responding before verification started. This script will not attach to an existing service; stop that process or use a different isolated verification port."
}

Write-Step "Start isolated Mock API on port $port."
$previousPort = $env:AGENT_SWARM_API_PORT
$previousSource = $env:AGENT_SWARM_DASHBOARD_SOURCE
$env:AGENT_SWARM_API_PORT = "$port"
$env:AGENT_SWARM_DASHBOARD_SOURCE = "mock"
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
  throw "Mock API did not start on port $port. Check logs/mock-flow-api.out.log and logs/mock-flow-api.err.log"
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
  $applied = Invoke-Json -Method "POST" -Path "/api/agent-config-applications/$($applyApproval.agentConfigApplicationId)/apply" -Body @{
    secondConfirm = $true
    confirmText = "Verify mock apply status transition."
    appliedBy = "verify_mock_flows"
  }
  Assert-Equal $applied.application.status "applied" "Agent config application should be applied."
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
}
