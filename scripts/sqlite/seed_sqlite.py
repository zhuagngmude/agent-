import json
import os
import sqlite3
from pathlib import Path


def as_json(value):
    return json.dumps(value if value is not None else [], ensure_ascii=False, separators=(",", ":"))


def as_bool(value):
    return 1 if value else 0


def clear_tables(connection):
    for table in [
        "runtime_events",
        "agent_config_versions",
        "agent_config_applications",
        "runner_jobs",
        "git_checkpoints",
        "knowledge_updates",
        "workflows",
        "approvals",
        "tasks",
        "agent_relationships",
        "agents",
        "runner_status",
        "projects",
    ]:
        connection.execute(f"DELETE FROM {table}")


def seed_project(connection, seed, project_id, seeded_at):
    project = seed["project"]
    connection.execute(
        """
        INSERT INTO projects (
          id, name, status, phase, description, workspace_path, created_at, updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        """,
        (
            project["id"],
            project["name"],
            project["status"],
            project.get("phase", ""),
            project.get("description", ""),
            project.get("workspacePath", ""),
            seeded_at,
            seeded_at,
        ),
    )

    for agent in seed.get("agents", []):
        connection.execute(
            """
            INSERT INTO agents (
              id, project_id, name, role, status, version, model,
              can_spawn_sub_agents, max_sub_agents, permissions, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            """,
            (
                agent["id"],
                project_id,
                agent["name"],
                agent["role"],
                agent["status"],
                agent.get("version", ""),
                agent.get("model", ""),
                as_bool(agent.get("canSpawnSubAgents")),
                int(agent.get("maxSubAgents", 0)),
                as_json(agent.get("permissions", [])),
                seeded_at,
                seeded_at,
            ),
        )

    for agent in seed.get("agents", []):
        parent = agent.get("parentAgentId") or None
        if not parent:
            continue

        connection.execute(
            """
            INSERT INTO agent_relationships (
              id, project_id, parent_agent_id, child_agent_id, reports_to_agent_id,
              spawn_depth, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            """,
            (
                f"agent_relationship_{parent}_{agent['id']}",
                project_id,
                parent,
                agent["id"],
                agent.get("reportsToAgentId") or None,
                int(agent.get("spawnDepth", 0)),
                seeded_at,
                seeded_at,
            ),
        )


def seed_tasks(connection, seed, project_id, seeded_at):
    for task in seed.get("tasks", []):
        connection.execute(
            """
            INSERT INTO tasks (
              id, project_id, title, description, status, priority, assigned_agent_id,
              risk_level, related_files, requires_approval, depends_on,
              started_at, completed_at, failed_at, cancelled_at, failure_reason,
              created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            """,
            (
                task["id"],
                project_id,
                task["title"],
                task.get("description", ""),
                task["status"],
                task.get("priority", ""),
                task.get("assignedAgentId") or None,
                task.get("riskLevel", ""),
                as_json(task.get("relatedFiles", [])),
                as_bool(task.get("requiresApproval")),
                as_json(task.get("dependsOn", [])),
                task.get("startedAt"),
                task.get("completedAt"),
                task.get("failedAt"),
                task.get("cancelledAt"),
                task.get("failureReason"),
                task.get("createdAt", seeded_at),
                task.get("updatedAt", seeded_at),
            ),
        )


def seed_approvals(connection, seed, project_id, seeded_at):
    for approval in seed.get("approvals", []):
        checkpoint = approval.get("checkpoint", {})
        connection.execute(
            """
            INSERT INTO approvals (
              id, project_id, status, risk_level, risk_tone, request_agent_id,
              request_agent_name, target_service, operation_types, reason,
              checkpoint_required, checkpoint_created, checkpoint_commit,
              affected_files, diff_summary, diff_preview, requires_second_confirm,
              change_request, runner_job_id, patch_artifact_id, reject_reason,
              approved_at, rejected_at, patch_only_at, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            """,
            (
                approval["id"],
                project_id,
                approval["status"],
                approval["riskLevel"],
                approval.get("riskTone", ""),
                approval.get("requestAgentId") or None,
                approval.get("requestAgentName", ""),
                approval.get("targetService", "runner"),
                as_json(approval.get("operationTypes", [])),
                approval.get("reason", ""),
                as_bool(checkpoint.get("required")),
                as_bool(checkpoint.get("created")),
                checkpoint.get("commit", ""),
                as_json(approval.get("affectedFiles", [])),
                approval.get("diffSummary", ""),
                as_json(approval.get("diffPreview", [])),
                as_bool(approval.get("requiresSecondConfirm")),
                as_json(approval.get("changeRequest")) if approval.get("changeRequest") else None,
                approval.get("runnerJobId", ""),
                approval.get("patchArtifactId", ""),
                approval.get("rejectReason", ""),
                approval.get("approvedAt"),
                approval.get("rejectedAt"),
                approval.get("patchOnlyAt"),
                approval.get("createdAt", seeded_at),
                approval.get("updatedAt", approval.get("createdAt", seeded_at)),
            ),
        )


def seed_runner_and_applications(connection, seed, project_id, seeded_at):
    for job in seed.get("runnerJobs", []):
        connection.execute(
            """
            INSERT INTO runner_jobs (
              id, project_id, approval_id, task_id, status, operation_types,
              affected_files, checkpoint_commit, safety_note, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            """,
            (
                job["id"],
                project_id,
                job["approvalId"],
                job.get("taskId") or None,
                job["status"],
                as_json(job.get("operationTypes", [])),
                as_json(job.get("affectedFiles", [])),
                job.get("checkpoint", ""),
                job.get("safetyNote", ""),
                job.get("createdAt", seeded_at),
                job.get("updatedAt", seeded_at),
            ),
        )

    for application in seed.get("agentConfigApplications", []):
        connection.execute(
            """
            INSERT INTO agent_config_applications (
              id, project_id, approval_id, agent_id, agent_name, change_type, changes,
              status, applied_at, applied_by, apply_confirm_text, cancelled_at,
              cancelled_by, cancel_reason, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            """,
            (
                application["id"],
                project_id,
                application["approvalId"],
                application["agentId"],
                application.get("agentName", ""),
                application.get("changeType", ""),
                as_json(application.get("changes", [])),
                application["status"],
                application.get("appliedAt"),
                application.get("appliedBy"),
                application.get("applyConfirmText"),
                application.get("cancelledAt"),
                application.get("cancelledBy"),
                application.get("cancelReason"),
                application.get("createdAt", seeded_at),
                application.get("updatedAt", seeded_at),
            ),
        )


def seed_workflows_and_runtime(connection, seed, project_id, seeded_at, seed_file):
    for workflow in seed.get("workflows", []):
        connection.execute(
            """
            INSERT INTO workflows (
              id, project_id, name, status, description, steps, stats,
              nodes, edges, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            """,
            (
                workflow["id"],
                project_id,
                workflow["name"],
                workflow["status"],
                workflow.get("description", ""),
                as_json(workflow.get("steps", [])),
                as_json(workflow.get("stats", [])),
                as_json(workflow.get("nodes", [])),
                as_json(workflow.get("edges", [])),
                workflow.get("createdAt", seeded_at),
                workflow.get("updatedAt", seeded_at),
            ),
        )

    runner_status = seed.get("runnerStatus", {})
    connection.execute(
        """
        INSERT INTO runner_status (
          id, project_id, connected, runner_id, version, workspace_path,
          permissions, last_heartbeat_at, created_at, updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        """,
        (
            f"runner_status_{project_id}",
            project_id,
            as_bool(runner_status.get("connected")),
            runner_status.get("runnerId", ""),
            runner_status.get("version", ""),
            runner_status.get("workspacePath", ""),
            as_json(runner_status.get("permissions", {})),
            runner_status.get("lastHeartbeatAt"),
            seeded_at,
            seeded_at,
        ),
    )

    for update in seed.get("knowledgeUpdates", []):
        updated_at = update.get("updatedAt", seeded_at)
        connection.execute(
            """
            INSERT INTO knowledge_updates (
              id, project_id, document, section, status, related_feature, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            """,
            (
                update["id"],
                project_id,
                update["document"],
                update.get("section", ""),
                update["status"],
                update.get("relatedFeature", ""),
                updated_at,
                updated_at,
            ),
        )

    for checkpoint in seed.get("gitCheckpoints", []):
        connection.execute(
            """
            INSERT INTO git_checkpoints (
              id, project_id, commit_hash, message, type, related_task_id, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?)
            """,
            (
                checkpoint["commit"],
                project_id,
                checkpoint["commit"],
                checkpoint["message"],
                checkpoint.get("type", ""),
                checkpoint.get("relatedTaskId") or None,
                checkpoint.get("createdAt", seeded_at),
            ),
        )

    connection.execute(
        """
        INSERT INTO runtime_events (
          id, project_id, entity_type, entity_id, event_type,
          before_state, after_state, actor, reason, created_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        """,
        (
            f"runtime_event_seed_completed_{project_id}",
            project_id,
            "project",
            project_id,
            "seed_completed",
            None,
            as_json({"seedFile": str(seed_file), "projectId": project_id}),
            "seed-sqlite",
            "Initial SQLite seed completed.",
            seeded_at,
        ),
    )


def main():
    db_file = Path(os.environ["AGENT_SWARM_SQLITE_DB"])
    seed_file = Path(os.environ["AGENT_SWARM_SQLITE_SEED"])
    seed = json.loads(seed_file.read_text(encoding="utf-8"))
    project_id = seed["projectId"]
    seeded_at = seed.get("seededAt") or "2026-06-09T00:00:00Z"

    with sqlite3.connect(db_file) as connection:
        connection.execute("PRAGMA foreign_keys = ON")
        clear_tables(connection)
        seed_project(connection, seed, project_id, seeded_at)
        seed_tasks(connection, seed, project_id, seeded_at)
        seed_approvals(connection, seed, project_id, seeded_at)
        seed_runner_and_applications(connection, seed, project_id, seeded_at)
        seed_workflows_and_runtime(connection, seed, project_id, seeded_at, seed_file)
        connection.commit()

    print(f"SQLite seeded: {db_file}")


if __name__ == "__main__":
    main()
