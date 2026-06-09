const { execFileSync } = require("child_process");
const fs = require("fs");
const path = require("path");

const defaultDbFile = path.resolve(__dirname, "..", "..", "..", "data", "local", "agent-swarm.sqlite");
const projectRoot = path.resolve(__dirname, "..", "..", "..");
const sqliteReadScript = path.join(projectRoot, "scripts", "sqlite", "sqlite_read.py");

function readDashboardFromSqlite(projectId, options = {}) {
  return readProjectSnapshotFromSqlite(projectId, options).dashboard;
}

function readProjectSnapshotFromSqlite(projectId, options = {}) {
  const dbFile = options.dbFile || process.env.AGENT_SWARM_SQLITE_DB || defaultDbFile;
  if (!fs.existsSync(dbFile)) {
    throw new Error(`SQLite database not found: ${dbFile}`);
  }

  const payload = execFileSync("python", ["-X", "utf8", sqliteReadScript, dbFile, projectId], {
    encoding: "utf8",
    windowsHide: true,
    maxBuffer: 2 * 1024 * 1024,
  });

  return JSON.parse(payload);
}

module.exports = {
  readDashboardFromSqlite,
  readProjectSnapshotFromSqlite,
};
