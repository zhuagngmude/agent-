$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
$dbDir = Join-Path $root "data\local"
$dbFile = Join-Path $dbDir "agent-swarm.sqlite"
$migrationFile = Join-Path $root "data\migrations\001_initial_sqlite.sql"
$pythonScript = Join-Path $PSScriptRoot "sqlite\init_sqlite.py"

if (-not (Test-Path $migrationFile)) {
  throw "Migration file not found: $migrationFile"
}

if (-not (Test-Path $pythonScript)) {
  throw "SQLite init bridge not found: $pythonScript"
}

New-Item -ItemType Directory -Force -Path $dbDir | Out-Null

$env:AGENT_SWARM_SQLITE_DB = $dbFile
$env:AGENT_SWARM_SQLITE_MIGRATION = $migrationFile

python -X utf8 $pythonScript
