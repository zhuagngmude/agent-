$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
$apiScript = Join-Path $root "services\api\server.js"
$port = 8787
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

function Test-ApiReady {
  try {
    $health = Invoke-Json -Method "GET" -Path "/api/health"
    return $health.ok -eq $true
  } catch {
    return $false
  }
}

if (-not (Test-ApiReady)) {
  Write-Step "Mock API not ready, starting local server."
  $outLog = Join-Path $root "logs\mock-api.out.log"
  $errLog = Join-Path $root "logs\mock-api.err.log"
  New-Item -ItemType Directory -Force -Path (Split-Path -Parent $outLog) | Out-Null
  Start-Process -WindowStyle Hidden -FilePath "node" -ArgumentList @($apiScript) -RedirectStandardOutput $outLog -RedirectStandardError $errLog

  $ready = $false
  for ($i = 0; $i -lt 20; $i++) {
    Start-Sleep -Milliseconds 250
    if (Test-ApiReady) {
      $ready = $true
      break
    }
  }

  if (-not $ready) {
    throw "Mock API did not start. Check logs/mock-api.out.log and logs/mock-api.err.log"
  }
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

  Write-Step "Verify Agent config apply flow."
  $applyRequest = Invoke-Json -Method "POST" -Path "/api/agents/agent_reviewer/change-requests" -Body @{
    changeType = "permission"
    riskLevel = "high"
    reason = "Verify mock agent config apply flow."
    changes = @(
      @{
        field = "permissions"
        before = "read_project / review_risk / review_diff"
        after = "read_project / review_risk / review_diff / request_code_execution"
      }
    )
  }
  Assert-Equal $applyRequest.approval.status "pending" "Agent config approval should start pending."
  $applyApproval = Invoke-Json -Method "POST" -Path "/api/approvals/$($applyRequest.approval.id)/approve" -Body @{
    secondConfirm = $true
    confirmText = "Verify agent config approval."
  }
  Assert-Equal $applyApproval.status "approved" "Agent config approval should be approved."
  Assert-True ($applyApproval.agentConfigApplicationId -like "agent_config_application_*") "Approval should create an application id."
  $applied = Invoke-Json -Method "POST" -Path "/api/agent-config-applications/$($applyApproval.agentConfigApplicationId)/apply" -Body @{
    secondConfirm = $true
    confirmText = "Verify mock apply status transition."
    appliedBy = "verify_mock_flows"
  }
  Assert-Equal $applied.application.status "applied" "Agent config application should be applied."

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
}
