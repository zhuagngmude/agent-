$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
$logsDir = Join-Path $root "logs"
$apiPort = 8787
$webPort = 5175
$dbFile = Join-Path $root "data\local\agent-swarm.sqlite"

function Test-Url {
  param([string]$Uri)
  try {
    Invoke-RestMethod -Uri $Uri -TimeoutSec 3 | Out-Null
    return $true
  } catch {
    return $false
  }
}

function Read-PidStatus {
  param([string]$Name, [string]$PidFile)

  if (-not (Test-Path $PidFile)) {
    return "$Name pid: not recorded"
  }

  $pidText = (Get-Content -Raw $PidFile).Trim()
  if (-not $pidText) {
    return "$Name pid: empty"
  }

  $process = Get-Process -Id ([int]$pidText) -ErrorAction SilentlyContinue
  if ($process) {
    return "$Name pid: $pidText running ($($process.ProcessName))"
  }

  return "$Name pid: $pidText not running"
}

$apiState = "not ready"
try {
  $state = Invoke-RestMethod -Uri "http://127.0.0.1:$apiPort/api/runtime-state" -TimeoutSec 3
  if ($state.mode -eq "sqlite") {
    $apiState = "ready sqlite"
  } else {
    $apiState = "ready mock"
  }
} catch {
  $apiState = "not ready"
}

Write-Host "agent蜂群 local trial status"
Write-Host "API:      $apiState http://127.0.0.1:$apiPort"
Write-Host "Web App:  $(if (Test-Url "http://127.0.0.1:$webPort/index.html") { "ready" } else { "not ready" }) http://127.0.0.1:$webPort/index.html"
Write-Host "Database: $(if (Test-Path $dbFile) { "exists" } else { "missing" }) data/local/agent-swarm.sqlite"
Write-Host (Read-PidStatus "API" (Join-Path $logsDir "local-api.pid"))
Write-Host (Read-PidStatus "Web" (Join-Path $logsDir "local-web.pid"))
