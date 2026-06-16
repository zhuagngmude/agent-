param(
  [string]$ApiKey = $env:AGENT_SWARM_OPENAI_COMPAT_API_KEY,
  [string]$BaseUrl = $env:AGENT_SWARM_OPENAI_COMPAT_BASE_URL,
  [switch]$KeepPortProcess
)

$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
$tauriDir = Join-Path $root "apps\desktop\src-tauri"
$uiPort = 5173

function Stop-PortListeners {
  param([int]$Port)

  $connections = Get-NetTCPConnection -LocalPort $Port -State Listen -ErrorAction SilentlyContinue
  $pids = $connections | Select-Object -ExpandProperty OwningProcess -Unique

  foreach ($processId in $pids) {
    if ($processId -and $processId -ne $PID) {
      $process = Get-Process -Id $processId -ErrorAction SilentlyContinue
      if ($null -ne $process) {
        Write-Host "[desktop] Stopping process on port ${Port}: $($process.ProcessName) ($processId)"
        Stop-Process -Id $processId -Force -ErrorAction SilentlyContinue
      }
    }
  }
}

if (-not (Test-Path $tauriDir)) {
  throw "Tauri desktop directory not found: $tauriDir"
}

if (-not $KeepPortProcess) {
  Stop-PortListeners -Port $uiPort
}

if ([string]::IsNullOrWhiteSpace($ApiKey)) {
  $secureKey = Read-Host "Enter AGENT_SWARM_OPENAI_COMPAT_API_KEY" -AsSecureString
  if ($secureKey.Length -eq 0) {
    throw "API key is required."
  }

  $bstr = [Runtime.InteropServices.Marshal]::SecureStringToBSTR($secureKey)
  try {
    $ApiKey = [Runtime.InteropServices.Marshal]::PtrToStringBSTR($bstr)
  } finally {
    [Runtime.InteropServices.Marshal]::ZeroFreeBSTR($bstr)
  }
}

if ([string]::IsNullOrWhiteSpace($BaseUrl)) {
  $BaseUrl = "https://api.cheng.pink/v1"
}

$env:AGENT_SWARM_ENABLE_REAL_MODEL_PROJECT_PLAN = "true"
$env:AGENT_SWARM_OPENAI_COMPAT_API_KEY = $ApiKey
$env:AGENT_SWARM_OPENAI_COMPAT_BASE_URL = $BaseUrl

Write-Host ""
Write-Host "[desktop] Starting agent-swarm desktop with real model preview enabled."
Write-Host "[desktop] Base URL: $BaseUrl"
Write-Host "[desktop] API key: set for this process only, not written to disk."
Write-Host "[desktop] UI port 5173 will be freed unless -KeepPortProcess is passed."
Write-Host "[desktop] Press Ctrl+C in this window to stop."
Write-Host ""

Set-Location $tauriDir
cargo tauri dev
