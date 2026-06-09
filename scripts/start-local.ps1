$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
$apiScript = Join-Path $root "services\api\server.js"
$webDir = Join-Path $root "apps\web"
$seedScript = Join-Path $PSScriptRoot "seed-sqlite.ps1"
$logsDir = Join-Path $root "logs"
$apiPort = 8787
$webPort = 5175
$apiPidFile = Join-Path $logsDir "local-api.pid"
$webPidFile = Join-Path $logsDir "local-web.pid"
$apiOutLog = Join-Path $logsDir "local-api.out.log"
$apiErrLog = Join-Path $logsDir "local-api.err.log"
$webOutLog = Join-Path $logsDir "local-web.out.log"
$webErrLog = Join-Path $logsDir "local-web.err.log"

function Test-HttpOk {
  param([string]$Uri)
  try {
    $response = Invoke-RestMethod -Uri $Uri -TimeoutSec 1
    return $null -ne $response
  } catch {
    return $false
  }
}

function Test-LocalApiReady {
  try {
    $health = Invoke-RestMethod -Uri "http://127.0.0.1:$apiPort/api/health" -TimeoutSec 1
    $state = Invoke-RestMethod -Uri "http://127.0.0.1:$apiPort/api/runtime-state" -TimeoutSec 1
    return $health.ok -eq $true -and $state.mode -eq "sqlite"
  } catch {
    return $false
  }
}

function Test-PortListening {
  param([int]$Port)
  return $null -ne (Get-NetTCPConnection -LocalPort $Port -State Listen -ErrorAction SilentlyContinue)
}

function Wait-Until {
  param(
    [scriptblock]$Condition,
    [string]$FailureMessage
  )

  for ($i = 0; $i -lt 40; $i++) {
    Start-Sleep -Milliseconds 250
    if (& $Condition) {
      return
    }
  }

  throw $FailureMessage
}

New-Item -ItemType Directory -Force -Path $logsDir | Out-Null

if (-not (Test-Path $seedScript)) {
  throw "SQLite seed script not found: $seedScript"
}

if (-not (Test-Path (Join-Path $root "data\local\agent-swarm.sqlite"))) {
  Write-Host "[local] SQLite database not found, creating from seed."
  powershell -ExecutionPolicy Bypass -File $seedScript | Out-Null
}

if (Test-PortListening $apiPort) {
  if (-not (Test-LocalApiReady)) {
    throw "Port $apiPort is already in use, but it is not the local SQLite API. Stop the other process first."
  }
  Write-Host "[local] SQLite API already running on http://127.0.0.1:$apiPort"
} else {
  Write-Host "[local] Starting SQLite API on http://127.0.0.1:$apiPort"
  $apiCommand = "`$env:AGENT_SWARM_API_PORT='$apiPort'; `$env:AGENT_SWARM_DASHBOARD_SOURCE='sqlite'; node '$apiScript'"
  $apiProcess = Start-Process `
    -WindowStyle Hidden `
    -FilePath "powershell" `
    -ArgumentList @("-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", $apiCommand) `
    -RedirectStandardOutput $apiOutLog `
    -RedirectStandardError $apiErrLog `
    -PassThru
  Set-Content -Encoding ASCII -Path $apiPidFile -Value $apiProcess.Id
  Wait-Until -Condition { Test-LocalApiReady } -FailureMessage "Local SQLite API did not start. Check logs/local-api.out.log and logs/local-api.err.log"
}

if (Test-PortListening $webPort) {
  if (-not (Test-HttpOk "http://127.0.0.1:$webPort/index.html")) {
    throw "Port $webPort is already in use, but it is not serving the local Web App. Stop the other process first."
  }
  Write-Host "[local] Web App already running on http://127.0.0.1:$webPort/index.html"
} else {
  Write-Host "[local] Starting Web App on http://127.0.0.1:$webPort/index.html"
  $webProcess = Start-Process `
    -WindowStyle Hidden `
    -FilePath "python" `
    -ArgumentList @("-m", "http.server", "$webPort", "--bind", "127.0.0.1", "--directory", $webDir) `
    -RedirectStandardOutput $webOutLog `
    -RedirectStandardError $webErrLog `
    -PassThru
  Set-Content -Encoding ASCII -Path $webPidFile -Value $webProcess.Id
  Wait-Until -Condition { Test-HttpOk "http://127.0.0.1:$webPort/index.html" } -FailureMessage "Local Web App did not start. Check logs/local-web.out.log and logs/local-web.err.log"
}

Start-Process "http://127.0.0.1:$webPort/index.html"

Write-Host ""
Write-Host "agent蜂群 local trial is ready."
Write-Host "Web App:  http://127.0.0.1:$webPort/index.html"
Write-Host "API:      http://127.0.0.1:$apiPort"
Write-Host "Database: data/local/agent-swarm.sqlite"
Write-Host ""
Write-Host "Stop with:"
Write-Host "powershell -ExecutionPolicy Bypass -File scripts\stop-local.ps1"
