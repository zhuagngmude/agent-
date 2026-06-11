$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot

function Write-Step {
  param([string]$Message)
  Write-Host "[project-plan-flow] $Message"
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

function Assert-NoRealSideEffects {
  param(
    [Parameter(Mandatory = $true)][object]$SideEffects,
    [string]$Prefix = "Project plan flow"
  )

  Assert-Equal $SideEffects.writesProjectFiles $false "$Prefix should not write project files."
  Assert-Equal $SideEffects.modifiesGit $false "$Prefix should not modify Git."
  Assert-Equal $SideEffects.executesRunner $false "$Prefix should not execute Runner."
  Assert-Equal $SideEffects.callsRealModel $false "$Prefix should not call real models."
  Assert-Equal $SideEffects.readsRawSecrets $false "$Prefix should not read raw secrets."
  Assert-Equal $SideEffects.makesNetworkRequests $false "$Prefix should not make network requests."
  Assert-Equal $SideEffects.triggersAgents $false "$Prefix should not trigger Agents."
}

Push-Location $root
try {
  Write-Step "Load project-plan helper cases."
  $casesJson = node -e @"
const projectPlan = require('./services/api/project-plan');
const agents = [
  { id: 'agent_frontend', name: 'Frontend Agent' },
  { id: 'agent_backend', name: 'Backend Agent' },
  { id: 'agent_qa', name: 'QA Agent' },
  { id: 'agent_docs', name: 'Docs Agent' },
  { id: 'agent_reviewer', name: 'Reviewer Agent' }
];
const draft = projectPlan.buildProjectPlanApprovalFromRequest({
  projectId: 'project_agent_swarm',
  agents,
  now: '2026-06-11T00:00:00.000Z',
  body: {
    planId: 'verify_mvp03_helper',
    idea: 'Build a local customer lead tracker',
    constraints: 'Mock/SQLite only; no real Runner; no real model calls',
    requestedBy: 'verify_project_plan_flow'
  }
});
const tasks = [];
const runnerJobs = [];
const instantiated = projectPlan.instantiateProjectPlanApproval({
  approval: draft.approval,
  tasks,
  runnerJobs,
  now: '2026-06-11T00:01:00.000Z'
});
const duplicate = projectPlan.instantiateProjectPlanApproval({
  approval: draft.approval,
  tasks,
  runnerJobs,
  now: '2026-06-11T00:02:00.000Z'
});
const emptyIdea = projectPlan.buildProjectPlanApprovalFromRequest({
  projectId: 'project_agent_swarm',
  agents,
  body: { idea: '   ' }
});
const invalidApproval = projectPlan.instantiateProjectPlanApproval({
  approval: { id: 'approval_invalid_project_plan', targetService: 'project_plan', changeRequest: { type: 'project_plan', plan: { tasks: [], runnerRequests: [] } } },
  tasks: [],
  runnerJobs: []
});
process.stdout.write(JSON.stringify({
  assignments: projectPlan.PROJECT_PLAN_AGENT_ASSIGNMENTS,
  sideEffects: projectPlan.noProjectPlanRequestSideEffects(),
  draft,
  instantiated,
  duplicate,
  tasks,
  runnerJobs,
  emptyIdea,
  invalidApproval
}));
"@
  $cases = $casesJson | ConvertFrom-Json

  Write-Step "Verify deterministic plan draft and approval shell."
  Assert-Equal $cases.draft.plan.id "verify_mvp03_helper" "Plan id should be deterministic."
  Assert-Equal $cases.draft.plan.status "draft" "Plan should start as draft."
  Assert-Equal $cases.draft.plan.generatedBy "local_deterministic_template" "Plan must not claim a real model result."
  Assert-Equal $cases.draft.approval.status "pending" "Project plan approval should start pending."
  Assert-Equal $cases.draft.approval.targetService "project_plan" "Project plan approval target service mismatch."
  Assert-Equal $cases.draft.approval.requestAgentId "agent_architect" "Architect Agent should request plan approval."
  Assert-Equal $cases.draft.approval.runnerJobId "" "Project plan approval must not create a generic Runner job."
  Assert-Equal $cases.draft.approval.requiresSecondConfirm $true "Project plan approval should require confirmation."
  Assert-Equal $cases.draft.approval.changeRequest.type "project_plan" "Project plan change request type mismatch."

  Assert-NoRealSideEffects -SideEffects $cases.draft.plan.sideEffects -Prefix "Project plan draft"
  Assert-NoRealSideEffects -SideEffects $cases.draft.approval.changeRequest.sideEffects -Prefix "Project plan approval change request"
  Assert-Equal $cases.draft.plan.sideEffects.createsApproval $false "Draft helper must not persist approvals."
  Assert-Equal $cases.draft.plan.sideEffects.createsTasks $false "Draft helper must not persist tasks."
  Assert-Equal $cases.draft.plan.sideEffects.createsRunnerJobs $false "Draft helper must not persist Runner jobs."

  Write-Step "Verify Agent assignment and read-only Runner request shape."
  Assert-Equal (@($cases.draft.plan.tasks).Count) 5 "Project plan should create five planned tasks."
  Assert-Equal (@($cases.draft.plan.runnerRequests).Count) 5 "Project plan should create five planned Runner requests."
  foreach ($agentId in @("agent_frontend", "agent_backend", "agent_qa", "agent_docs", "agent_reviewer")) {
    $assigned = @($cases.draft.plan.tasks | Where-Object { $_.assignedAgentId -eq $agentId })
    Assert-Equal $assigned.Count 1 "Plan should assign exactly one task to $agentId."
    Assert-Equal $assigned[0].status "queued" "Planned task for $agentId should be queued."
    Assert-Equal $assigned[0].requiresApproval $false "Planned task for $agentId should not be an execution approval."
  }
  foreach ($request in @($cases.draft.plan.runnerRequests)) {
    Assert-TextContains (@($request.operationTypes) -join "`n") "runner_request_readonly" "Runner request should be read-only."
    Assert-TextContains $request.safetyNote "No command" "Runner request should state no command execution."
    Assert-Equal $request.status "queued" "Runner request should start queued."
    Assert-True ($request.taskId -like "task_verify_mvp03_helper_*") "Runner request should reference a planned task."
  }

  Write-Step "Verify approval instantiation is idempotent and still non-executing."
  Assert-Equal (@($cases.instantiated.createdTaskIds).Count) 5 "First approval should create five tasks."
  Assert-Equal (@($cases.instantiated.createdRunnerJobIds).Count) 5 "First approval should create five Runner request records."
  Assert-Equal (@($cases.tasks).Count) 5 "Task queue should contain five tasks after instantiation."
  Assert-Equal (@($cases.runnerJobs).Count) 5 "Runner queue should contain five records after instantiation."
  Assert-Equal (@($cases.duplicate.createdTaskIds).Count) 0 "Duplicate approval should not create duplicate tasks."
  Assert-Equal (@($cases.duplicate.createdRunnerJobIds).Count) 0 "Duplicate approval should not create duplicate Runner requests."
  foreach ($job in @($cases.runnerJobs)) {
    Assert-TextContains (@($job.operationTypes) -join "`n") "runner_request_readonly" "Instantiated Runner queue record should be read-only."
    Assert-Equal $job.checkpoint "" "Project plan Runner request should not have a Git checkpoint."
    Assert-TextContains $job.safetyNote "No command" "Instantiated Runner queue record should state no command execution."
  }

  Write-Step "Verify invalid helper inputs fail without side effects."
  Assert-Equal $cases.emptyIdea.error "project_idea_required" "Empty idea should be rejected."
  Assert-Equal $cases.invalidApproval.error "invalid_project_plan_approval" "Invalid approval should be rejected."
  Assert-NoRealSideEffects -SideEffects $cases.sideEffects -Prefix "Project plan helper baseline"

  Write-Step "Project plan helper checks passed."
} finally {
  Pop-Location
}
