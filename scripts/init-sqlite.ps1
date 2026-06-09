$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
$dbDir = Join-Path $root "data\local"
$dbFile = Join-Path $dbDir "agent-swarm.sqlite"
$migrationFile = Join-Path $root "data\migrations\001_initial_sqlite.sql"

if (-not (Test-Path $migrationFile)) {
  throw "Migration file not found: $migrationFile"
}

New-Item -ItemType Directory -Force -Path $dbDir | Out-Null

$env:AGENT_SWARM_SQLITE_DB = $dbFile
$env:AGENT_SWARM_SQLITE_MIGRATION = $migrationFile

@'
import os
import sqlite3
from pathlib import Path

db_file = Path(os.environ["AGENT_SWARM_SQLITE_DB"])
migration_file = Path(os.environ["AGENT_SWARM_SQLITE_MIGRATION"])

schema = migration_file.read_text(encoding="utf-8")

with sqlite3.connect(db_file) as connection:
    connection.execute("PRAGMA foreign_keys = ON")
    connection.executescript(schema)
    connection.commit()

print(f"SQLite initialized: {db_file}")
'@ | python -X utf8 -

