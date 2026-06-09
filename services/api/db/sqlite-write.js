const { execFileSync } = require("child_process");
const fs = require("fs");
const path = require("path");

const defaultDbFile = path.resolve(__dirname, "..", "..", "..", "data", "local", "agent-swarm.sqlite");
const projectRoot = path.resolve(__dirname, "..", "..", "..");
const seedScript = path.join(projectRoot, "scripts", "seed-sqlite.ps1");
const sqliteWriteScript = path.join(projectRoot, "scripts", "sqlite", "sqlite_write.py");

function runSqliteWrite(command, args = {}, options = {}) {
  const dbFile = options.dbFile || process.env.AGENT_SWARM_SQLITE_DB || defaultDbFile;
  if (!fs.existsSync(dbFile)) {
    throw new Error(`SQLite database not found: ${dbFile}`);
  }

  const payload = execFileSync(
    "python",
    ["-X", "utf8", sqliteWriteScript, dbFile, command, JSON.stringify(args)],
    {
      encoding: "utf8",
      windowsHide: true,
      maxBuffer: 2 * 1024 * 1024,
    }
  );

  return JSON.parse(payload);
}

function resetSqliteState() {
  execFileSync("powershell", ["-ExecutionPolicy", "Bypass", "-File", seedScript], {
    cwd: projectRoot,
    encoding: "utf8",
    windowsHide: true,
    maxBuffer: 2 * 1024 * 1024,
  });
}

module.exports = {
  runSqliteWrite,
  resetSqliteState,
};

