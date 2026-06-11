$ErrorActionPreference = "Stop"

$checks = @(
  "verify-agent-permissions.ps1",
  "verify-agent-config-fields.ps1",
  "verify-agent-config-dry-run.ps1",
  "verify-agent-config-apply-gate.ps1",
  "verify-agent-config-transaction-plan.ps1",
  "verify-agent-config-version-history.ps1",
  "verify-agent-config-rollback-request.ps1",
  "verify-mock-flows.ps1",
  "verify-sqlite-flows.ps1",
  "verify-agent-config-real-apply-sqlite.ps1"
)

function Write-Step {
  param([string]$Message)
  Write-Host "[agent-config-safety-loop] $Message"
}

$startedAt = Get-Date

Write-Step "Start MVP-0.2 Agent config safety loop verification."
Write-Step "This script keeps real Runner execution, real model calls, cloud sync, and default real rollback disabled."
Write-Step "Flow checks use isolated verification ports 8788, 8789, and 8790; the human local trial port 8787 is not used."

foreach ($check in $checks) {
  $scriptPath = Join-Path $PSScriptRoot $check
  if (-not (Test-Path $scriptPath)) {
    throw "Verification script not found: $scriptPath"
  }

  Write-Step "Run $check"
  & powershell -NoProfile -ExecutionPolicy Bypass -File $scriptPath
  if ($LASTEXITCODE -ne 0) {
    throw "$check failed with exit code $LASTEXITCODE."
  }
}

$duration = (Get-Date) - $startedAt
Write-Step ("MVP-0.2 Agent config safety loop passed in {0:n1}s." -f $duration.TotalSeconds)
