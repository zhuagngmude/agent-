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


def project_row_to_api(row):
    return {
        "id": row["id"],
        "name": row["name"],
        "status": row["status"],
        "phase": pick(row, "phase"),
        "description": pick(row, "description"),
    }


def agent_row_to_api(row):
    return {
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


def task_row_to_api(row):
    return {
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


def approval_row_to_api(row):
    return {
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


def runner_job_row_to_api(row):
    return {
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


def agent_config_application_row_to_api(row):
    return {
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


def workflow_row_to_api(row):
    return {
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


def runner_status_row_to_api(row):
    if row is None:
        return None

    return {
        "connected": bool_from_int(row["connected"]),
        "runnerId": row["runner_id"],
        "version": row["version"],
        "workspacePath": pick(row, "workspace_path"),
        "permissions": from_json(row["permissions"], {}),
        "lastHeartbeatAt": pick(row, "last_heartbeat_at"),
    }


def git_checkpoint_row_to_api(row):
    return {
        "commit": row["commit_hash"],
        "message": row["message"],
        "type": pick(row, "type"),
        "relatedTaskId": pick(row, "related_task_id"),
        "createdAt": pick(row, "created_at"),
    }


def knowledge_update_row_to_api(row):
    return {
        "id": row["id"],
        "document": row["document"],
        "section": pick(row, "section"),
        "status": row["status"],
        "relatedFeature": pick(row, "related_feature"),
        "updatedAt": pick(row, "updated_at"),
    }


def fetch_project(connection):
    row = connection.execute(
        """
        SELECT id, name, status, phase, description
        FROM projects
        WHERE id = ?
        """,
        (project_id,),
    ).fetchone()
    if row is None:
        raise SystemExit(f"Project not found: {project_id}")
    return row


def fetch_mapped_list(connection, sql, mapper):
    return [mapper(row) for row in connection.execute(sql, (project_id,)).fetchall()]


def attach_agent_relationships(connection, agents):
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


def build_dashboard(project, agents, tasks, approvals, workflows, runner_status, runner_jobs, applications, git_checkpoints, knowledge_updates):
    active_agents = sum(1 for agent in agents if agent["status"] == "running")
    pending_approvals = sum(1 for approval in approvals if approval["status"] == "pending")
    active_tasks = sum(1 for task in tasks if task["status"] in ("running", "queued"))
    completed_tasks = sum(1 for task in tasks if task["status"] == "completed")

    return {
        "project": project,
        "metrics": {
            "activeAgents": active_agents,
            "pendingApprovals": pending_approvals,
            "activeTasks": active_tasks,
            "gitCheckpoints": len(git_checkpoints),
            "tokenUsage": "-",
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
            "tokenUsage": {"total": 0, "today": 0},
            "estimatedCost": {"currency": "CNY", "today": 0, "month": 0},
            "byModel": [],
        },
        "integrationHealth": [
            {"provider": "local_runner", "status": "connected", "display": "本地 Runner 已连接"},
            {"provider": "git", "status": "connected", "display": "Git 可用"},
            {"provider": "github", "status": "planned", "display": "GitHub 待接入"},
        ],
    }


with sqlite3.connect(db_file) as connection:
    connection.row_factory = sqlite3.Row
    connection.execute("PRAGMA foreign_keys = ON")

    project = project_row_to_api(fetch_project(connection))
    agents = fetch_mapped_list(
        connection,
        """
        SELECT id, name, role, status, version, model, can_spawn_sub_agents,
               max_sub_agents, permissions
        FROM agents
        WHERE project_id = ?
        ORDER BY id
        """,
        agent_row_to_api,
    )
    attach_agent_relationships(connection, agents)

    tasks = fetch_mapped_list(
        connection,
        """
        SELECT *
        FROM tasks
        WHERE project_id = ?
        ORDER BY id
        """,
        task_row_to_api,
    )
    approvals = fetch_mapped_list(
        connection,
        """
        SELECT *
        FROM approvals
        WHERE project_id = ?
        ORDER BY created_at, id
        """,
        approval_row_to_api,
    )
    runner_jobs = fetch_mapped_list(
        connection,
        """
        SELECT *
        FROM runner_jobs
        WHERE project_id = ?
        ORDER BY created_at, id
        """,
        runner_job_row_to_api,
    )
    applications = fetch_mapped_list(
        connection,
        """
        SELECT *
        FROM agent_config_applications
        WHERE project_id = ?
        ORDER BY created_at, id
        """,
        agent_config_application_row_to_api,
    )
    workflows = fetch_mapped_list(
        connection,
        """
        SELECT *
        FROM workflows
        WHERE project_id = ?
        ORDER BY id
        """,
        workflow_row_to_api,
    )

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
    runner_status = runner_status_row_to_api(runner_status_row)

    git_checkpoints = fetch_mapped_list(
        connection,
        """
        SELECT *
        FROM git_checkpoints
        WHERE project_id = ?
        ORDER BY created_at, commit_hash
        """,
        git_checkpoint_row_to_api,
    )
    knowledge_updates = fetch_mapped_list(
        connection,
        """
        SELECT *
        FROM knowledge_updates
        WHERE project_id = ?
        ORDER BY updated_at, id
        """,
        knowledge_update_row_to_api,
    )

    dashboard = build_dashboard(
        project,
        agents,
        tasks,
        approvals,
        workflows,
        runner_status,
        runner_jobs,
        applications,
        git_checkpoints,
        knowledge_updates,
    )

    snapshot = {
        "dashboard": dashboard,
        "agents": agents,
        "tasks": tasks,
        "approvals": approvals,
        "workflows": workflows,
        "runnerStatus": runner_status,
        "runnerJobs": runner_jobs,
        "agentConfigApplications": applications,
        "gitCheckpoints": git_checkpoints,
        "knowledgeUpdates": knowledge_updates,
    }

    print(json.dumps(snapshot, ensure_ascii=False))
