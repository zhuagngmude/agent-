$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
$logsDir = Join-Path $root "logs"
$pidTargets = @(
  @{ Name = "Web"; PidFile = (Join-Path $logsDir "local-web.pid"); Port = 5175 },
  @{ Name = "API"; PidFile = (Join-Path $logsDir "local-api.pid"); Port = 8787 }
)

function Stop-RecordedProcess {
  param([string]$PidFile)

  if (-not (Test-Path $pidFile)) {
    return $false
  }

  $pidText = (Get-Content -Raw $pidFile).Trim()
  if (-not $pidText) {
    Remove-Item -LiteralPath $pidFile -Force
    return $false
  }

  $process = Get-Process -Id ([int]$pidText) -ErrorAction SilentlyContinue
  if ($process) {
    Write-Host "[local] Stopping process $($process.Id) ($($process.ProcessName))"
    Stop-Process -Id $process.Id -Force
    $process.WaitForExit()
  }

  Remove-Item -LiteralPath $pidFile -Force
  return $true
}

function Stop-PortProcess {
  param([int]$Port)

  $connections = Get-NetTCPConnection -LocalPort $Port -State Listen -ErrorAction SilentlyContinue
  foreach ($connection in $connections) {
    $process = Get-Process -Id $connection.OwningProcess -ErrorAction SilentlyContinue
    if (-not $process) {
      continue
    }

    $commandLine = (Get-CimInstance Win32_Process -Filter "ProcessId = $($process.Id)" -ErrorAction SilentlyContinue).CommandLine
    if (-not $commandLine -or -not $commandLine.Contains($root)) {
      Write-Host "[local] Skip port $Port listener $($process.Id) ($($process.ProcessName)); command line is outside this project."
      continue
    }

    Write-Host "[local] Stopping port $Port listener $($process.Id) ($($process.ProcessName))"
    Stop-Process -Id $process.Id -Force
    $process.WaitForExit()
  }
}

foreach ($target in $pidTargets) {
  $hadRecordedProcess = Stop-RecordedProcess -PidFile $target.PidFile
  if ($hadRecordedProcess) {
    Stop-PortProcess -Port $target.Port
  }
}

Write-Host "[local] Local trial processes stopped."
