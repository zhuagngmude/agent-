$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
$dbFile = Join-Path $root "data\local\agent-swarm.sqlite"
$seedFile = Join-Path $root "data\seed\project_agent_swarm.seed.json"
$initScript = Join-Path $PSScriptRoot "init-sqlite.ps1"
$pythonScript = Join-Path $PSScriptRoot "sqlite\seed_sqlite.py"

if (-not (Test-Path $dbFile)) {
  powershell -ExecutionPolicy Bypass -File $initScript
}

if (-not (Test-Path $seedFile)) {
  throw "Seed file not found: $seedFile"
}

if (-not (Test-Path $pythonScript)) {
  throw "SQLite seed bridge not found: $pythonScript"
}

$env:AGENT_SWARM_SQLITE_DB = $dbFile
$env:AGENT_SWARM_SQLITE_SEED = $seedFile

python -X utf8 $pythonScript
