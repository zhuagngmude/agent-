$ErrorActionPreference = "Stop"

$apiBase = "http://127.0.0.1:8787"
$webUrl = "http://127.0.0.1:5175/index.html"
$projectId = "project_agent_swarm"
$session = "agent-swarm-local-ui"
$playwrightWorkDir = Join-Path $env:TEMP "agent-swarm-playwright-cli"

function Write-Step {
  param([string]$Message)
  Write-Host "[local-ui] $Message"
}

function Invoke-Json {
  param(
    [Parameter(Mandatory = $true)][string]$Path,
    [string]$Method = "GET",
    [object]$Body = $null
  )

  $uri = "$apiBase$Path"
  if ($null -eq $Body) {
    return Invoke-RestMethod -Method $Method -Uri $uri -TimeoutSec 5
  }

  return Invoke-RestMethod `
    -Method $Method `
    -Uri $uri `
    -TimeoutSec 5 `
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

function Test-Command {
  param([string]$Name)
  return $null -ne (Get-Command $Name -ErrorAction SilentlyContinue)
}

function Get-EdgePath {
  $paths = @(
    "C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
    "C:\Program Files\Microsoft\Edge\Application\msedge.exe"
  )

  foreach ($path in $paths) {
    if (Test-Path $path) {
      return $path
    }
  }

  return ""
}

function Invoke-PlaywrightCli {
  param([Parameter(Mandatory = $true)][string[]]$Arguments)

  Push-Location $playwrightWorkDir
  try {
    $previousErrorActionPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    $output = & npx --package "@playwright/cli" playwright-cli @Arguments 2>&1
    $exitCode = $LASTEXITCODE
    $ErrorActionPreference = $previousErrorActionPreference

    $filteredOutput = @($output | ForEach-Object { "$_" } | Where-Object {
      -not $_.StartsWith("npm warn ")
    })

    if ($exitCode -ne 0) {
      throw ($filteredOutput | Out-String)
    }
    return ($filteredOutput | Out-String)
  } finally {
    if ($null -ne $previousErrorActionPreference) {
      $ErrorActionPreference = $previousErrorActionPreference
    }
    Pop-Location
  }
}

function Invoke-PageEval {
  param([Parameter(Mandatory = $true)][string]$Expression)

  $raw = Invoke-PlaywrightCli -Arguments @("-s=$session", "--json", "eval", $Expression)
  $outer = $raw | ConvertFrom-Json
  if ($outer.isError -eq $true) {
    throw $outer.error
  }

  if ($null -eq $outer.result) {
    throw "Playwright eval did not return a result. Raw output: $raw"
  }

  $result = $outer.result
  if ($result -is [string]) {
    $decoded = $result | ConvertFrom-Json
    if ($decoded -is [string]) {
      $trimmed = $decoded.TrimStart()
      if ($trimmed.StartsWith("{") -or $trimmed.StartsWith("[")) {
        return $decoded | ConvertFrom-Json
      }
      return $decoded
    }
    return $decoded
  }

  return $result
}

function Invoke-PageText {
  param([Parameter(Mandatory = $true)][string]$Selector)

  $escapedSelector = $Selector.Replace("\", "\\").Replace("'", "\'")
  return Invoke-PageEval -Expression "() => document.querySelector('$escapedSelector')?.innerText || ''"
}

function Invoke-PageClickByDataPage {
  param([Parameter(Mandatory = $true)][string]$Page)

  $escapedPage = $Page.Replace("\", "\\").Replace("'", "\'")
  Invoke-PageEval -Expression "() => { const el = Array.from(document.querySelectorAll('[data-page]')).find((item) => item.dataset.page === '$escapedPage'); if (el) el.click(); return true; }" | Out-Null
}

Write-Step "Verify local trial API is ready."
$health = Invoke-Json -Path "/api/health"
Assert-Equal $health.ok $true "API health should be ok."
Assert-Equal $health.projectId $projectId "API project id mismatch."

$runtime = Invoke-Json -Path "/api/runtime-state"
Assert-Equal $runtime.mode "sqlite" "Local trial should run in SQLite mode."
Assert-Equal $runtime.localTrial.safety.realModelCalls $false "Local trial should not allow real model calls."
Assert-Equal $runtime.localTrial.safety.runnerExecutesCommands $false "Runner command execution should be disabled."
Assert-Equal $runtime.localTrial.safety.runnerWritesFiles $false "Runner file writes should be disabled."
Assert-Equal $runtime.localTrial.safety.cloudSync $false "Cloud sync should be disabled."
Assert-Equal $runtime.localTrial.safety.agentConfigRealApplyDefaultOff $true "Agent config real apply should be documented as default-off."
Assert-Equal $runtime.localTrial.safety.agentConfigRealApplyRequiresDryRunProof $true "Agent config real apply should require dry-run proof."
Assert-Equal $runtime.localTrial.safety.agentConfigRealApplyRequiresGitCheckpoint $true "Agent config real apply should require Git checkpoint acknowledgement."
Assert-Equal $runtime.localTrial.safety.agentConfigRealApplyRequiresRollbackAcceptance $true "Agent config real apply should require rollback acceptance."

Write-Step "Verify browser prerequisites."
if (-not (Test-Command "npx")) {
  throw "npx is required for Playwright CLI. Install Node.js/npm first."
}

$edgePath = Get-EdgePath
if (-not $edgePath) {
  throw "Microsoft Edge was not found. Install Edge or update Get-EdgePath in this script."
}

New-Item -ItemType Directory -Force -Path $playwrightWorkDir | Out-Null

Write-Step "Open Web App in Microsoft Edge through Playwright CLI."
Invoke-PlaywrightCli -Arguments @("-s=$session", "open", $webUrl, "--browser", "msedge") | Out-Null

try {
  Write-Step "Verify core pages and safety copy in the DOM."
  Assert-TextContains (Invoke-PageEval -Expression "() => document.title") "agent" "Page title should identify the app."
  Assert-TextContains (Invoke-PageText -Selector "#apiStatus") "API" "Top bar should show local API status."

  Invoke-PageClickByDataPage -Page "overview"
  Assert-TextContains (Invoke-PageText -Selector "#overview") "Token" "Overview should include the model usage placeholder."

  Invoke-PageClickByDataPage -Page "tasks"
  Assert-True ((Invoke-PageText -Selector "#taskDetail").Length -gt 0) "Tasks page should render task details."
  Assert-True ((Invoke-PageEval -Expression "() => document.querySelectorAll('#startTaskAction, #completeTaskAction, #failTaskAction, #cancelTaskAction').length") -eq 4) "Task action buttons should be present."

  Invoke-PageClickByDataPage -Page "workflow"
  Assert-TextContains (Invoke-PageText -Selector "#workflowPage") "MVP-0.3" "Workflow page should render MVP-0.3 project plan request panel."
  Assert-TextContains (Invoke-PageText -Selector "#workflowPage") "Runner request queue" "Workflow page should describe read-only Runner request queue."
  Assert-True ((Invoke-PageEval -Expression "() => document.querySelector('#projectPlanIdea')?.tagName === 'TEXTAREA'") -eq $true) "Workflow page should render the project idea input."
  Assert-True ((Invoke-PageEval -Expression "() => document.querySelector('#submitProjectPlanRequest')?.disabled === false") -eq $true) "Project plan request button should be available in local UI."

  Invoke-PageClickByDataPage -Page "approval"
  Assert-TextContains (Invoke-PageText -Selector "#approval") "Runner job" "Approval page should describe Runner jobs."

  Invoke-PageClickByDataPage -Page "agents"
  Assert-TextContains (Invoke-PageText -Selector "#agentConfigApplications") "SQLite real apply gate" "Agent page should render real-apply gate status."
  Assert-TextContains (Invoke-PageText -Selector "#agentConfigApplications") "Dry-run proof" "Agent page should show real-apply dry-run requirement."
  Assert-TextContains (Invoke-PageText -Selector "#agentConfigApplications") "Rollback plan" "Agent page should show rollback acceptance requirement."
  Assert-True ((Invoke-PageEval -Expression "() => document.querySelectorAll('#agentDetail .task-files').length >= 2") -eq $true) "Agent detail should render config version history."
  Assert-True ((Invoke-PageEval -Expression "() => { const panel = document.querySelector('#agentConfigApplications'); const review = panel?.querySelector('.application-review-layout'); return !review || review.querySelectorAll('.application-checklist').length >= 4; }") -eq $true) "Agent config rollback review should include version-history readiness when application records exist."
  Assert-True ((Invoke-PageEval -Expression "() => Array.from(document.querySelectorAll('.real-apply-gate button')).every((button) => button.disabled)") -eq $true) "Real apply gate buttons should remain disabled in UI."

  Invoke-PageClickByDataPage -Page "runtime"
  Assert-TextContains (Invoke-PageText -Selector "#runtime") "Runner Job" "Runtime page should render Runner job details."

  Invoke-PageClickByDataPage -Page "settings"
  Assert-True ((Invoke-PageText -Selector "#localTrialStatus").Length -gt 0) "Settings page should render local trial status."
  Assert-TextContains (Invoke-PageText -Selector "#localTrialStatus") "Agent config real apply" "Settings page should show Agent config real apply flag state."
  Assert-TextContains (Invoke-PageText -Selector "#modelGatewaySettingsStatus") "API keys stay server-side" "Settings page should render Model Gateway safety copy."
  Assert-TextContains (Invoke-PageText -Selector "#modelGatewaySettingsStatus") "Connectivity Dry-Run" "Settings page should render Model Gateway dry-run preview."
  Assert-TextContains (Invoke-PageText -Selector "#modelGatewaySettingsStatus") "Would call provider" "Settings page should render dry-run provider call boundary."
  Assert-TextContains (Invoke-PageText -Selector "#modelGatewaySettingsStatus") "Prompt/result logging" "Settings page should render dry-run logging boundary."

  Invoke-PageClickByDataPage -Page "integrations"
  Assert-TextContains (Invoke-PageText -Selector "#integrations") "GitHub" "Integrations page should render integration cards."
  Assert-TextContains (Invoke-PageText -Selector "#modelGatewayIntegrationStatus") "Real calls" "Integrations page should render Model Gateway status."
  Assert-TextContains (Invoke-PageText -Selector "#modelGatewayIntegrationStatus") "Connectivity Dry-Run" "Integrations page should render Model Gateway dry-run preview."
  Assert-Equal (Invoke-PageText -Selector "#modelGatewayIntegrationBadge") "disabled" "Model Gateway badge should be disabled."

  $hrefHashCount = Invoke-PageEval -Expression "() => Array.from(document.querySelectorAll('a')).filter((item) => item.getAttribute('href') === '#').length"
  Assert-Equal $hrefHashCount 0 "No href=# placeholder links should remain."
  $disabledButtons = Invoke-PageEval -Expression "() => Array.from(document.querySelectorAll('button:disabled')).length"
  Assert-True ($disabledButtons -gt 0) "Unimplemented controls should remain visibly disabled."

  Write-Step "Verify browser console has no errors or warnings."
  $consoleOutput = Invoke-PlaywrightCli -Arguments @("-s=$session", "console")
  Assert-TextContains $consoleOutput "Errors: 0, Warnings: 0" "Browser console should be clean."

  Write-Step "Local UI smoke checks passed."
} finally {
  Write-Step "Close Playwright browser session."
  try {
    Invoke-PlaywrightCli -Arguments @("-s=$session", "close") | Out-Null
  } catch {
    Write-Warning "Failed to close Playwright session: $($_.Exception.Message)"
  }
}
