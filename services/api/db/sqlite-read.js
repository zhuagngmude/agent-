const { execFileSync } = require("child_process");
const fs = require("fs");
const path = require("path");

const defaultDbFile = path.resolve(__dirname, "..", "..", "..", "data", "local", "agent-swarm.sqlite");

function readDashboardFromSqlite(projectId, options = {}) {
  return readProjectSnapshotFromSqlite(projectId, options).dashboard;
}

function readProjectSnapshotFromSqlite(projectId, options = {}) {
  const dbFile = options.dbFile || process.env.AGENT_SWARM_SQLITE_DB || defaultDbFile;
  if (!fs.existsSync(dbFile)) {
    throw new Error(`SQLite database not found: ${dbFile}`);
  }

  const payload = runPythonSnapshotQuery(dbFile, projectId);
  return JSON.parse(payload);
}

function runPythonSnapshotQuery(dbFile, projectId) {
  const script = String.raw`
import json
import sqlite3
import sys

db_file = sys.argv[1]
project_id = sys.argv[2]


def from_json(value, fallback):
    if value in (None, ""):
        return fallback
    return json.loads(value)


def bool_from_int(value):
    return bool(value)


def pick(row, key, fallback=""):
    return row[key] if row[key] is not None else fallback


with sqlite3.connect(db_file) as connection:
    connection.row_factory = sqlite3.Row
    connection.execute("PRAGMA foreign_keys = ON")

    project = connection.execute(
        """
        SELECT id, name, status, phase, description
        FROM projects
        WHERE id = ?
        """,
        (project_id,),
    ).fetchone()
    if project is None:
        raise SystemExit(f"Project not found: {project_id}")

    agents = [
        {
            "id": row["id"],
            "name": row["name"],
            "role": row["role"],
            "status": row["status"],
            "version": pick(row, "version"),
            "model": pick(row, "model"),
            "canSpawnSubAgents": bool_from_int(row["can_spawn_sub_agents"]),
            "maxSubAgents": row["max_sub_agents"],
            "parentAgentId": "",
            "childAgentIds": [],
            "reportsToAgentId": "",
            "spawnDepth": 0,
            "permissions": from_json(row["permissions"], []),
        }
        for row in connection.execute(
            """
            SELECT id, name, role, status, version, model, can_spawn_sub_agents,
                   max_sub_agents, permissions
            FROM agents
            WHERE project_id = ?
            ORDER BY id
            """,
            (project_id,),
        ).fetchall()
    ]

    agent_by_id = {agent["id"]: agent for agent in agents}
    for row in connection.execute(
        """
        SELECT parent_agent_id, child_agent_id, reports_to_agent_id, spawn_depth
        FROM agent_relationships
        WHERE project_id = ?
        ORDER BY child_agent_id
        """,
        (project_id,),
    ).fetchall():
        child = agent_by_id.get(row["child_agent_id"])
        parent = agent_by_id.get(row["parent_agent_id"])
        if child:
            child["parentAgentId"] = row["parent_agent_id"] or ""
            child["reportsToAgentId"] = row["reports_to_agent_id"] or ""
            child["spawnDepth"] = row["spawn_depth"]
        if parent:
            parent["childAgentIds"].append(row["child_agent_id"])

    tasks = [
        {
            "id": row["id"],
            "title": row["title"],
            "description": pick(row, "description"),
            "status": row["status"],
            "priority": pick(row, "priority"),
            "assignedAgentId": pick(row, "assigned_agent_id"),
            "riskLevel": pick(row, "risk_level"),
            "relatedFiles": from_json(row["related_files"], []),
            "requiresApproval": bool_from_int(row["requires_approval"]),
            "dependsOn": from_json(row["depends_on"], []),
            "startedAt": pick(row, "started_at"),
            "completedAt": pick(row, "completed_at"),
            "failedAt": pick(row, "failed_at"),
            "cancelledAt": pick(row, "cancelled_at"),
            "failureReason": pick(row, "failure_reason"),
            "updatedAt": pick(row, "updated_at"),
        }
        for row in connection.execute(
            """
            SELECT *
            FROM tasks
            WHERE project_id = ?
            ORDER BY id
            """,
            (project_id,),
        ).fetchall()
    ]

    approvals = [
        {
            "id": row["id"],
            "status": row["status"],
            "riskLevel": row["risk_level"],
            "riskTone": pick(row, "risk_tone"),
            "requestAgentId": pick(row, "request_agent_id"),
            "requestAgentName": pick(row, "request_agent_name"),
            "operationTypes": from_json(row["operation_types"], []),
            "reason": pick(row, "reason"),
            "checkpoint": {
                "required": bool_from_int(row["checkpoint_required"]),
                "created": bool_from_int(row["checkpoint_created"]),
                "commit": pick(row, "checkpoint_commit"),
            },
            "affectedFiles": from_json(row["affected_files"], []),
            "diffSummary": pick(row, "diff_summary"),
            "diffPreview": from_json(row["diff_preview"], []),
            "requiresSecondConfirm": bool_from_int(row["requires_second_confirm"]),
            "targetService": pick(row, "target_service"),
            "changeRequest": from_json(row["change_request"], None) if row["change_request"] else None,
            "rejectReason": pick(row, "reject_reason"),
            "runnerJobId": pick(row, "runner_job_id"),
            "patchArtifactId": pick(row, "patch_artifact_id"),
            "approvedAt": pick(row, "approved_at"),
            "rejectedAt": pick(row, "rejected_at"),
            "patchOnlyAt": pick(row, "patch_only_at"),
            "createdAt": pick(row, "created_at"),
            "updatedAt": pick(row, "updated_at"),
        }
        for row in connection.execute(
            """
            SELECT *
            FROM approvals
            WHERE project_id = ?
            ORDER BY created_at, id
            """,
            (project_id,),
        ).fetchall()
    ]

    runner_jobs = [
        {
            "id": row["id"],
            "approvalId": row["approval_id"],
            "taskId": pick(row, "task_id"),
            "status": row["status"],
            "operationTypes": from_json(row["operation_types"], []),
            "affectedFiles": from_json(row["affected_files"], []),
            "checkpoint": pick(row, "checkpoint_commit"),
            "safetyNote": pick(row, "safety_note"),
            "createdAt": pick(row, "created_at"),
            "updatedAt": pick(row, "updated_at"),
        }
        for row in connection.execute(
            """
            SELECT *
            FROM runner_jobs
            WHERE project_id = ?
            ORDER BY created_at, id
            """,
            (project_id,),
        ).fetchall()
    ]

    applications = [
        {
            "id": row["id"],
            "approvalId": row["approval_id"],
            "agentId": row["agent_id"],
            "agentName": pick(row, "agent_name"),
            "changeType": row["change_type"],
            "changes": from_json(row["changes"], []),
            "status": row["status"],
            "appliedAt": pick(row, "applied_at"),
            "appliedBy": pick(row, "applied_by"),
            "applyConfirmText": pick(row, "apply_confirm_text"),
            "cancelledAt": pick(row, "cancelled_at"),
            "cancelledBy": pick(row, "cancelled_by"),
            "cancelReason": pick(row, "cancel_reason"),
            "createdAt": pick(row, "created_at"),
            "updatedAt": pick(row, "updated_at"),
        }
        for row in connection.execute(
            """
            SELECT *
            FROM agent_config_applications
            WHERE project_id = ?
            ORDER BY created_at, id
            """,
            (project_id,),
        ).fetchall()
    ]

    workflows = [
        {
            "id": row["id"],
            "name": row["name"],
            "status": row["status"],
            "description": pick(row, "description"),
            "steps": from_json(row["steps"], []),
            "stats": from_json(row["stats"], []),
            "nodes": from_json(row["nodes"], []),
            "edges": from_json(row["edges"], []),
            "updatedAt": pick(row, "updated_at"),
        }
        for row in connection.execute(
            """
            SELECT *
            FROM workflows
            WHERE project_id = ?
            ORDER BY id
            """,
            (project_id,),
        ).fetchall()
    ]

    runner_status_row = connection.execute(
        """
        SELECT *
        FROM runner_status
        WHERE project_id = ?
        ORDER BY updated_at DESC
        LIMIT 1
        """,
        (project_id,),
    ).fetchone()
    runner_status = None
    if runner_status_row:
        runner_status = {
            "connected": bool_from_int(runner_status_row["connected"]),
            "runnerId": runner_status_row["runner_id"],
            "version": runner_status_row["version"],
            "workspacePath": pick(runner_status_row, "workspace_path"),
            "permissions": from_json(runner_status_row["permissions"], {}),
            "lastHeartbeatAt": pick(runner_status_row, "last_heartbeat_at"),
        }

    git_checkpoints = [
        {
            "commit": row["commit_hash"],
            "message": row["message"],
            "type": pick(row, "type"),
            "relatedTaskId": pick(row, "related_task_id"),
            "createdAt": pick(row, "created_at"),
        }
        for row in connection.execute(
            """
            SELECT *
            FROM git_checkpoints
            WHERE project_id = ?
            ORDER BY created_at, commit_hash
            """,
            (project_id,),
        ).fetchall()
    ]

    knowledge_updates = [
        {
            "id": row["id"],
            "document": row["document"],
            "section": pick(row, "section"),
            "status": row["status"],
            "relatedFeature": pick(row, "related_feature"),
            "updatedAt": pick(row, "updated_at"),
        }
        for row in connection.execute(
            """
            SELECT *
            FROM knowledge_updates
            WHERE project_id = ?
            ORDER BY updated_at, id
            """,
            (project_id,),
        ).fetchall()
    ]

    active_agents = sum(1 for agent in agents if agent["status"] == "running")
    pending_approvals = sum(1 for approval in approvals if approval["status"] == "pending")
    active_tasks = sum(1 for task in tasks if task["status"] in ("running", "queued"))
    completed_tasks = sum(1 for task in tasks if task["status"] == "completed")

    dashboard = {
        "project": {
            "id": project["id"],
            "name": project["name"],
            "status": project["status"],
            "phase": pick(project, "phase"),
            "description": pick(project, "description"),
        },
        "metrics": {
            "activeAgents": active_agents,
            "pendingApprovals": pending_approvals,
            "activeTasks": active_tasks,
            "gitCheckpoints": len(git_checkpoints),
            "tokenUsage": "1.23M",
            "modelCount": 3,
        },
        "workflowSummary": {
            "totalAgents": len(agents),
            "totalTasks": len(tasks),
            "completedTasks": completed_tasks,
            "successRate": 0.923,
            "averageResponseMs": 1200,
        },
        "workflows": workflows,
        "runnerStatus": runner_status,
        "runnerJobs": runner_jobs,
        "agentConfigApplications": applications,
        "pendingApprovals": approvals,
        "taskQueue": tasks,
        "agentStatus": agents,
        "gitCheckpoints": git_checkpoints,
        "knowledgeUpdates": knowledge_updates,
        "usageSummary": {
            "tokenUsage": {"total": 1230000, "today": 82000},
            "estimatedCost": {"currency": "CNY", "today": 128.4, "month": 245.6},
            "byModel": [
                {"provider": "openai", "model": "gpt", "tokens": 500000},
                {"provider": "anthropic", "model": "claude", "tokens": 400000},
                {"provider": "google", "model": "gemini", "tokens": 330000},
            ],
        },
        "integrationHealth": [
            {"provider": "local_runner", "status": "connected", "display": "本地 Runner 已连接"},
            {"provider": "git", "status": "connected", "display": "Git 可用"},
            {"provider": "github", "status": "planned", "display": "GitHub 待接入"},
        ],
    }

    snapshot = {
        "dashboard": dashboard,
        "agents": agents,
        "tasks": tasks,
        "approvals": approvals,
        "workflows": workflows,
        "runnerJobs": runner_jobs,
        "agentConfigApplications": applications,
    }

    print(json.dumps(snapshot, ensure_ascii=False))
`;

  return execFileSync("python", ["-X", "utf8", "-c", script, dbFile, projectId], {
    encoding: "utf8",
    windowsHide: true,
    maxBuffer: 2 * 1024 * 1024,
  });
}

module.exports = {
  readDashboardFromSqlite,
  readProjectSnapshotFromSqlite,
};
