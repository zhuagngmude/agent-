import json
import sqlite3
import sys
import uuid
from datetime import datetime, timezone

db_file = sys.argv[1]
command = sys.argv[2]
args = json.loads(sys.argv[3])
project_id = args.get("projectId", "project_agent_swarm")


def now():
    return datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")


def as_json(value):
    return json.dumps(value if value is not None else [], ensure_ascii=False, separators=(",", ":"))


def from_json(value, fallback):
    if value in (None, ""):
        return fallback
    return json.loads(value)


def pick(row, key, fallback=""):
    return row[key] if row[key] is not None else fallback


def bool_from_int(value):
    return bool(value)


def runtime_event(connection, entity_type, entity_id, event_type, before_state, after_state, actor="api", reason=""):
    created_at = now()
    event_id = f"runtime_event_{entity_type}_{entity_id}_{event_type}_{created_at.replace(':', '').replace('-', '')}_{uuid.uuid4().hex[:8]}"
    connection.execute(
        """
        INSERT INTO runtime_events (
          id, project_id, entity_type, entity_id, event_type,
          before_state, after_state, actor, reason, created_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        """,
        (
            event_id,
            project_id,
            entity_type,
            entity_id,
            event_type,
            as_json(before_state) if before_state is not None else None,
            as_json(after_state) if after_state is not None else None,
            actor,
            reason,
            created_at,
        ),
    )


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


def application_row_to_api(row):
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


def fetch_one(connection, sql, params):
    row = connection.execute(sql, params).fetchone()
    return row


def transition_task(connection):
    task_id = args["taskId"]
    action = args["action"]
    body = args.get("body", {})
    row = fetch_one(connection, "SELECT * FROM tasks WHERE id = ? AND project_id = ?", (task_id, project_id))
    if row is None:
        return {"statusCode": 404, "body": {"error": "task_not_found"}}

    before = task_row_to_api(row)
    status = row["status"]
    timestamp = now()
    terminal = {"completed", "failed", "cancelled"}

    updates = {"updated_at": timestamp}
    if action == "start":
        if status not in {"queued", "blocked", "waiting_user", "failed", "cancelled"}:
            return {"statusCode": 409, "body": {"error": "task_cannot_start", "message": f"Task cannot start from status {status}."}}
        updates.update({
            "status": "running",
            "started_at": timestamp,
            "completed_at": None,
            "failed_at": None,
            "cancelled_at": None,
            "failure_reason": None,
        })
    elif action == "complete":
        if status != "running":
            return {"statusCode": 409, "body": {"error": "task_cannot_complete", "message": "Only running tasks can be completed."}}
        updates.update({"status": "completed", "completed_at": timestamp})
    elif action == "fail":
        if status in terminal:
            return {"statusCode": 409, "body": {"error": "task_already_terminal", "message": f"Task is already {status}."}}
        updates.update({
            "status": "failed",
            "failed_at": timestamp,
            "failure_reason": body.get("reason") or "用户在控制台标记为失败",
        })
    elif action == "cancel":
        if status in terminal:
            return {"statusCode": 409, "body": {"error": "task_already_terminal", "message": f"Task is already {status}."}}
        updates.update({"status": "cancelled", "cancelled_at": timestamp})
    else:
        return {"statusCode": 404, "body": {"error": "unknown_task_action", "message": "Unknown task action."}}

    assignments = ", ".join(f"{key} = ?" for key in updates)
    connection.execute(
        f"UPDATE tasks SET {assignments} WHERE id = ? AND project_id = ?",
        [*updates.values(), task_id, project_id],
    )
    updated = fetch_one(connection, "SELECT * FROM tasks WHERE id = ? AND project_id = ?", (task_id, project_id))
    task = task_row_to_api(updated)
    runtime_event(connection, "task", task_id, "status_changed", before, task, reason=action)
    return {"statusCode": 200, "body": {"task": task}}


def upsert_runner_job(connection, approval):
    job_id = f"runner_job_{approval['id']}"
    existing = fetch_one(connection, "SELECT * FROM runner_jobs WHERE id = ? AND project_id = ?", (job_id, project_id))
    timestamp = now()
    if existing is None:
        connection.execute(
            """
            INSERT INTO runner_jobs (
              id, project_id, approval_id, task_id, status, operation_types,
              affected_files, checkpoint_commit, safety_note, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            """,
            (
                job_id,
                project_id,
                approval["id"],
                None,
                "queued",
                as_json(approval["operationTypes"]),
                as_json(approval["affectedFiles"]),
                approval["checkpoint"]["commit"],
                "SQLite mode read-only Runner job. No local command will be executed.",
                timestamp,
                timestamp,
            ),
        )
        runtime_event(connection, "runner_job", job_id, "created", None, {"id": job_id, "status": "queued"}, reason=approval["id"])
    else:
        connection.execute(
            """
            UPDATE runner_jobs
            SET operation_types = ?, affected_files = ?, checkpoint_commit = ?, updated_at = ?
            WHERE id = ? AND project_id = ?
            """,
            (as_json(approval["operationTypes"]), as_json(approval["affectedFiles"]), approval["checkpoint"]["commit"], timestamp, job_id, project_id),
        )
    return job_id


def upsert_agent_config_application(connection, approval):
    change_request = approval.get("changeRequest") or {}
    agent_id = change_request.get("agentId") or approval.get("requestAgentId") or ""
    application_id = f"agent_config_application_{approval['id']}"
    existing = fetch_one(connection, "SELECT * FROM agent_config_applications WHERE id = ? AND project_id = ?", (application_id, project_id))
    agent = fetch_one(connection, "SELECT name FROM agents WHERE id = ? AND project_id = ?", (agent_id, project_id))
    timestamp = now()
    changes = change_request.get("changes") or []
    if existing is None:
        connection.execute(
            """
            INSERT INTO agent_config_applications (
              id, project_id, approval_id, agent_id, agent_name, change_type, changes,
              status, applied_at, applied_by, apply_confirm_text, cancelled_at,
              cancelled_by, cancel_reason, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            """,
            (
                application_id,
                project_id,
                approval["id"],
                agent_id,
                agent["name"] if agent else approval.get("requestAgentName", agent_id),
                change_request.get("changeType", ""),
                as_json(changes),
                "pending_apply",
                None,
                None,
                None,
                None,
                None,
                None,
                timestamp,
                timestamp,
            ),
        )
        runtime_event(connection, "agent_config_application", application_id, "created", None, {"id": application_id, "status": "pending_apply"}, reason=approval["id"])
    else:
        connection.execute(
            """
            UPDATE agent_config_applications
            SET agent_id = ?, agent_name = ?, change_type = ?, changes = ?, updated_at = ?
            WHERE id = ? AND project_id = ?
            """,
            (
                agent_id,
                agent["name"] if agent else approval.get("requestAgentName", agent_id),
                change_request.get("changeType", ""),
                as_json(changes),
                timestamp,
                application_id,
                project_id,
            ),
        )
    return application_id


def approval_action(connection):
    approval_id = args["approvalId"]
    action = args["action"]
    body = args.get("body", {})
    row = fetch_one(connection, "SELECT * FROM approvals WHERE id = ? AND project_id = ?", (approval_id, project_id))
    if row is None:
        return {"statusCode": 404, "body": {"error": "approval_not_found"}}

    before = approval_row_to_api(row)
    timestamp = now()

    if action == "approve":
        if before["requiresSecondConfirm"] and body.get("secondConfirm") is not True:
            return {"statusCode": 409, "body": {"error": "second_confirm_required", "message": "High risk approval requires secondConfirm=true."}}
        runner_job_id = ""
        application_id = ""
        if before["targetService"] == "agent_config":
            application_id = upsert_agent_config_application(connection, before)
        else:
            runner_job_id = upsert_runner_job(connection, before)
        connection.execute(
            """
            UPDATE approvals
            SET status = ?, runner_job_id = ?, approved_at = ?, updated_at = ?
            WHERE id = ? AND project_id = ?
            """,
            ("approved", runner_job_id, timestamp, timestamp, approval_id, project_id),
        )
        updated = approval_row_to_api(fetch_one(connection, "SELECT * FROM approvals WHERE id = ? AND project_id = ?", (approval_id, project_id)))
        runtime_event(connection, "approval", approval_id, "status_changed", before, updated, reason=action)
        return {"statusCode": 200, "body": {"id": approval_id, "status": "approved", "runnerJobId": runner_job_id, "agentConfigApplicationId": application_id}}

    if action == "reject":
        connection.execute(
            """
            UPDATE approvals
            SET status = ?, reject_reason = ?, rejected_at = ?, updated_at = ?
            WHERE id = ? AND project_id = ?
            """,
            ("rejected", body.get("reason", ""), timestamp, timestamp, approval_id, project_id),
        )
        updated = approval_row_to_api(fetch_one(connection, "SELECT * FROM approvals WHERE id = ? AND project_id = ?", (approval_id, project_id)))
        runtime_event(connection, "approval", approval_id, "status_changed", before, updated, reason=action)
        return {"statusCode": 200, "body": {"id": approval_id, "status": "rejected"}}

    if action == "patch-only":
        patch_artifact_id = f"patch_{approval_id}"
        connection.execute(
            """
            UPDATE approvals
            SET status = ?, patch_artifact_id = ?, patch_only_at = ?, updated_at = ?
            WHERE id = ? AND project_id = ?
            """,
            ("patch_only", patch_artifact_id, timestamp, timestamp, approval_id, project_id),
        )
        updated = approval_row_to_api(fetch_one(connection, "SELECT * FROM approvals WHERE id = ? AND project_id = ?", (approval_id, project_id)))
        runtime_event(connection, "approval", approval_id, "status_changed", before, updated, reason=action)
        return {"statusCode": 200, "body": {"id": approval_id, "status": "patch_only", "patchArtifactId": patch_artifact_id}}

    return {"statusCode": 404, "body": {"error": "unknown_approval_action"}}


def create_agent_change_request(connection):
    agent_id = args["agentId"]
    body = args.get("body", {})
    agent = fetch_one(connection, "SELECT * FROM agents WHERE id = ? AND project_id = ?", (agent_id, project_id))
    if agent is None:
        return {"statusCode": 404, "body": {"error": "agent_not_found"}}

    timestamp = now()
    change_type = body.get("changeType") or "model"
    risk_level = body.get("riskLevel") or ("medium" if change_type == "model" else "high")
    risk_tone = "high" if risk_level == "high" else ("mid" if risk_level == "medium" else "low")
    changes = body.get("changes") if isinstance(body.get("changes"), list) else []
    approval_id = f"approval_agent_{agent_id}_{change_type}"
    diff_preview = [f"~ {item.get('field')}: {item.get('before')} -> {item.get('after')}" for item in changes] or [f"~ {change_type}: 等待补充变更字段"]
    change_request = {"agentId": agent_id, "changeType": change_type, "changes": changes}
    existing = fetch_one(connection, "SELECT * FROM approvals WHERE id = ? AND project_id = ?", (approval_id, project_id))
    before = approval_row_to_api(existing) if existing else None

    values = (
        approval_id,
        project_id,
        "pending",
        risk_level,
        risk_tone,
        agent_id,
        agent["name"],
        "agent_config",
        as_json(["agent_config_change"]),
        body.get("reason") or f"申请修改 {agent['name']} 的 Agent 配置。",
        1,
        0,
        "",
        as_json([f"agent-config/{agent_id}"]),
        f"{len(changes)} fields",
        as_json(diff_preview),
        1 if risk_level == "high" else 0,
        as_json(change_request),
        "",
        "",
        "",
        None,
        None,
        None,
        timestamp if existing is None else existing["created_at"],
        timestamp,
    )
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
        ON CONFLICT(id) DO UPDATE SET
          status = excluded.status,
          risk_level = excluded.risk_level,
          risk_tone = excluded.risk_tone,
          reason = excluded.reason,
          diff_summary = excluded.diff_summary,
          diff_preview = excluded.diff_preview,
          requires_second_confirm = excluded.requires_second_confirm,
          change_request = excluded.change_request,
          updated_at = excluded.updated_at
        """,
        values,
    )
    updated = approval_row_to_api(fetch_one(connection, "SELECT * FROM approvals WHERE id = ? AND project_id = ?", (approval_id, project_id)))
    runtime_event(connection, "approval", approval_id, "created" if before is None else "updated", before, updated, reason="agent_change_request")
    return {"statusCode": 201, "body": {"approval": updated, "message": "Agent change request created. Agent config was not modified."}}


def agent_config_application_action(connection):
    application_id = args["applicationId"]
    action = args["action"]
    body = args.get("body", {})
    row = fetch_one(connection, "SELECT * FROM agent_config_applications WHERE id = ? AND project_id = ?", (application_id, project_id))
    if row is None:
        return {"statusCode": 404, "body": {"error": "agent_config_application_not_found"}}

    before = application_row_to_api(row)
    approval = fetch_one(connection, "SELECT * FROM approvals WHERE id = ? AND project_id = ?", (row["approval_id"], project_id))
    timestamp = now()

    if action == "apply":
        if approval is None:
            return {"statusCode": 409, "body": {"error": "source_approval_not_found", "message": "Agent config application must reference an existing approval."}}
        approval_api = approval_row_to_api(approval)
        if body.get("secondConfirm") is not True:
            return {"statusCode": 409, "body": {"error": "second_confirm_required", "message": "Mock agent config apply requires secondConfirm=true."}}
        if not body.get("confirmText"):
            return {"statusCode": 409, "body": {"error": "confirm_text_required", "message": "Mock agent config apply requires confirmText."}}
        if before["status"] != "pending_apply":
            return {"statusCode": 409, "body": {"error": "application_not_pending_apply", "message": f"Agent config application cannot apply from status {before['status']}."}}
        if approval_api["status"] != "approved" or approval_api["targetService"] != "agent_config" or approval_api["runnerJobId"]:
            return {"statusCode": 409, "body": {"error": "application_preconditions_failed", "message": "Agent config application requires approved agent_config approval without Runner job."}}
        connection.execute(
            """
            UPDATE agent_config_applications
            SET status = ?, applied_at = ?, applied_by = ?, apply_confirm_text = ?, updated_at = ?
            WHERE id = ? AND project_id = ?
            """,
            ("applied", timestamp, body.get("appliedBy") or "local_user", body["confirmText"], timestamp, application_id, project_id),
        )
    elif action == "cancel":
        if before["status"] != "pending_apply":
            return {"statusCode": 409, "body": {"error": "application_not_pending_apply", "message": f"Agent config application cannot cancel from status {before['status']}."}}
        if not body.get("reason"):
            return {"statusCode": 409, "body": {"error": "cancel_reason_required", "message": "Mock agent config cancel requires reason."}}
        connection.execute(
            """
            UPDATE agent_config_applications
            SET status = ?, cancelled_at = ?, cancelled_by = ?, cancel_reason = ?, updated_at = ?
            WHERE id = ? AND project_id = ?
            """,
            ("cancelled", timestamp, body.get("cancelledBy") or "local_user", body["reason"], timestamp, application_id, project_id),
        )
    else:
        return {"statusCode": 404, "body": {"error": "unknown_agent_config_application_action"}}

    updated = application_row_to_api(fetch_one(connection, "SELECT * FROM agent_config_applications WHERE id = ? AND project_id = ?", (application_id, project_id)))
    runtime_event(connection, "agent_config_application", application_id, "status_changed", before, updated, reason=action)
    message = "Mock application status changed to applied. Agent config was not modified." if action == "apply" else "Mock application status changed to cancelled. Agent config was not modified."
    return {"statusCode": 200, "body": {"application": updated, "message": message}}


with sqlite3.connect(db_file) as connection:
    connection.row_factory = sqlite3.Row
    connection.execute("PRAGMA foreign_keys = ON")
    try:
        if command == "transitionTask":
            result = transition_task(connection)
        elif command == "approvalAction":
            result = approval_action(connection)
        elif command == "createAgentChangeRequest":
            result = create_agent_change_request(connection)
        elif command == "agentConfigApplicationAction":
            result = agent_config_application_action(connection)
        else:
            result = {"statusCode": 404, "body": {"error": "unknown_sqlite_write_command"}}

        if result["statusCode"] < 400:
            connection.commit()
        else:
            connection.rollback()
    except Exception:
        connection.rollback()
        raise

print(json.dumps(result, ensure_ascii=False))
