$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
$apiScript = Join-Path $root "services\api\server.js"
$webEntry = Join-Path $root "apps\web\index.html"
$port = 8787

function Test-ApiReady {
  try {
    $response = Invoke-RestMethod -Uri "http://127.0.0.1:$port/api/health" -TimeoutSec 1
    return $response.ok -eq $true
  } catch {
    return $false
  }
}

if (-not (Test-ApiReady)) {
  $apiOutLog = Join-Path $root "logs\mock-api.out.log"
  $apiErrLog = Join-Path $root "logs\mock-api.err.log"
  New-Item -ItemType Directory -Force -Path (Split-Path -Parent $apiOutLog) | Out-Null

  Start-Process -WindowStyle Hidden -FilePath "node" -ArgumentList @($apiScript) -RedirectStandardOutput $apiOutLog -RedirectStandardError $apiErrLog

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

Start-Process -FilePath $webEntry

Write-Host "agent蜂群 dev environment is ready."
Write-Host "Mock API: http://127.0.0.1:$port"
Write-Host "Web App:  $webEntry"
