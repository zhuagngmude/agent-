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
    return datetime.now(timezone.utc).isoformat().replace("+00:00", "Z")


def as_json(value):
    return json.dumps(value if value is not None else [], ensure_ascii=False, separators=(",", ":"))


def from_json(value, fallback):
    if value in (None, ""):
        return fallback
    return json.loads(value)


def pick(row, key, fallback=""):
    try:
        return row[key] if row[key] is not None else fallback
    except (KeyError, IndexError):
        return fallback


def json_pick(row, key, fallback):
    try:
        return from_json(row[key], fallback)
    except (KeyError, IndexError):
        return fallback


def bool_from_int(value):
    return bool(value)


def as_bool(value):
    return 1 if value else 0


ALLOWED_AGENT_CONFIG_FIELDS = {
    "permissions",
    "model",
    "status",
    "maxSubAgents",
    "canSpawnSubAgents",
}

AGENT_CONFIG_FIELD_COLUMNS = {
    "permissions": "permissions",
    "model": "model",
    "status": "status",
    "maxSubAgents": "max_sub_agents",
    "canSpawnSubAgents": "can_spawn_sub_agents",
}

FORBIDDEN_AGENT_CONFIG_FIELD_TOKENS = [
    "apikey",
    "authorization",
    "command",
    "env",
    "file",
    "git",
    "header",
    "network",
    "parent",
    "prompt",
    "providerresponse",
    "rawsecret",
    "reportsto",
    "runner",
    "secret",
    "token",
    "tool",
    "workspace",
]

FORBIDDEN_AGENT_CONFIG_VALUE_TOKENS = [
    "api_key",
    "apikey",
    "authorization",
    "bearer ",
    "raw_secret",
    "rawsecret",
    "secret",
    "provider_response",
    "providerresponse",
    "prompt",
    "/users/",
]

AGENT_PERMISSION_PROFILES = {
    "architect_admin": [
        "canViewProject",
        "canReadKnowledge",
        "canPlanArchitecture",
        "canDraftTasks",
        "canDraftWorkflow",
        "canReviewArchitecture",
        "canProposeModelUse",
        "canCreateTasks",
        "canAssignTasks",
        "canAssignAgents",
        "canSpawnSubAgents",
        "canSetTaskPriority",
        "canRequestAgentConfigChange",
        "canRequestExecution",
        "canRequestModelConnectivity",
        "canRequestModelCall",
        "canReferenceSecretPresence",
        "canRequestSecretUse",
    ],
    "executor_agent": [
        "canViewProject",
        "canReadKnowledge",
        "canDraftTasks",
        "canRequestExecution",
        "canRequestFileWrite",
        "canRequestCommand",
        "canRequestGitOperation",
    ],
    "reviewer_agent": [
        "canViewProject",
        "canReadKnowledge",
        "canReviewArchitecture",
        "canReviewApproval",
        "canRecommendApproval",
    ],
    "all_agents_full_management": [
        "canViewProject",
        "canReadKnowledge",
        "canPlanArchitecture",
        "canDraftTasks",
        "canDraftWorkflow",
        "canReviewArchitecture",
        "canProposeModelUse",
        "canCreateTasks",
        "canAssignTasks",
        "canAssignAgents",
        "canSpawnSubAgents",
        "canSetTaskPriority",
        "canRequestAgentConfigChange",
        "canRequestExecution",
        "canRequestFileWrite",
        "canRequestCommand",
        "canRequestNetwork",
        "canRequestGitOperation",
        "canRequestModelConnectivity",
        "canRequestModelCall",
        "canReviewApproval",
        "canRecommendApproval",
        "canReferenceSecretPresence",
        "canRequestSecretUse",
    ],
}

KNOWN_AGENT_CAPABILITIES = sorted(set(sum(AGENT_PERMISSION_PROFILES.values(), [])))

FORBIDDEN_AGENT_CAPABILITIES = [
    "canApproveHighRisk",
    "canApproveOwnRequest",
    "canExecuteRunnerJob",
    "canWriteFiles",
    "canDeleteFiles",
    "canExecuteCommands",
    "canModifyGit",
    "canMakeNetworkRequests",
    "canAccessRawSecrets",
]


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


def agent_row_to_config_snapshot(row):
    return {
        "permissions": from_json(row["permissions"], []),
        "model": pick(row, "model"),
        "status": row["status"],
        "maxSubAgents": row["max_sub_agents"],
        "canSpawnSubAgents": bool_from_int(row["can_spawn_sub_agents"]),
    }


def field_token(field):
    return "".join(ch for ch in str(field).lower() if ch.isalnum())


def split_permission_text(value):
    if isinstance(value, list):
        result = []
        for item in value:
            result.extend(split_permission_text(item))
        return result
    if not isinstance(value, str):
        return []
    parts = []
    for item in value.replace(",", "/").replace("\n", "/").split("/"):
        text = item.strip()
        if text:
            parts.append(text)
    return parts


def unique(values):
    result = []
    for value in values:
        if value not in result:
            result.append(value)
    return result


def normalize_permissions(value):
    if isinstance(value, dict) and value.get("all") is True:
        return None, ["all=true is not a valid Agent permission contract."]
    if isinstance(value, str) and value.strip() in AGENT_PERMISSION_PROFILES:
        return list(AGENT_PERMISSION_PROFILES[value.strip()]), []

    capabilities = unique(split_permission_text(value))
    validation_errors = []
    if len(capabilities) == 0:
        validation_errors.append("permissions change must include a profile or explicit capabilities.")
    for capability in capabilities:
        if capability not in KNOWN_AGENT_CAPABILITIES:
            validation_errors.append(f"unknown capability: {capability}")
        if capability in FORBIDDEN_AGENT_CAPABILITIES:
            validation_errors.append(f"forbidden Agent capability: {capability}")
    return capabilities, validation_errors


def validate_non_permission_value_safety(change):
    text = json.dumps({"before": change.get("before"), "after": change.get("after")}, ensure_ascii=False).lower()
    validation_errors = []
    for token in FORBIDDEN_AGENT_CONFIG_VALUE_TOKENS:
        if token in text:
            validation_errors.append("change plan contains forbidden Agent config content.")
            break
    if ":\\users\\" in text:
        validation_errors.append("change plan must not contain local private paths.")
    return validation_errors


def validate_agent_config_changes(changes):
    validation_errors = []
    normalized = []
    if not isinstance(changes, list) or len(changes) == 0:
        return [], ["changes must be a non-empty array."]

    for index, change in enumerate(changes):
        if not isinstance(change, dict):
            validation_errors.append(f"change {index} must be an object.")
            continue
        field = change.get("field", "")
        if not isinstance(field, str) or not field.strip():
            validation_errors.append(f"change {index} field is required.")
            continue
        field = field.strip()
        token = field_token(field)
        if any(item in token for item in FORBIDDEN_AGENT_CONFIG_FIELD_TOKENS):
            validation_errors.append(f"forbidden Agent config field: {field}")
            continue
        if field not in ALLOWED_AGENT_CONFIG_FIELDS:
            validation_errors.append(f"unsupported Agent config field: {field}")
            continue

        value = change.get("after")
        if field == "permissions":
            value, permission_errors = normalize_permissions(value)
            validation_errors.extend(permission_errors)
        else:
            validation_errors.extend(validate_non_permission_value_safety(change))
        if field == "model" and not isinstance(value, str):
            validation_errors.append("model change must use a string value.")
        if field == "status" and value not in ["running", "idle", "waiting", "failed", "disabled"]:
            validation_errors.append("status change must use a supported Agent status.")
        if field == "maxSubAgents":
            try:
                value = int(value)
            except Exception:
                validation_errors.append("maxSubAgents change must be an integer between 0 and 20.")
            if isinstance(value, int) and (value < 0 or value > 20):
                validation_errors.append("maxSubAgents change must be an integer between 0 and 20.")
        if field == "canSpawnSubAgents" and not isinstance(value, bool):
            validation_errors.append("canSpawnSubAgents change must use a boolean value.")

        normalized.append({"field": field, "value": value})

    return normalized, validation_errors


def all_agent_config_side_effects_false(side_effects):
    expected = [
        "writesAgents",
        "writesAgentConfigVersions",
        "writesRuntimeEvents",
        "writesSqlite",
        "writesRuntimeState",
        "createsApprovals",
        "createsRunnerJobs",
        "executesRunner",
        "callsRealModel",
        "readsRawSecrets",
    ]
    return isinstance(side_effects, dict) and all(side_effects.get(key) is False for key in expected)


def validate_real_apply_body(body, application_id, approval_id, agent_id):
    validation_errors = []
    dry_run = body.get("dryRun")
    git_checkpoint = body.get("gitCheckpoint")

    if body.get("secondConfirm") is not True:
        validation_errors.append("secondConfirm=true is required.")
    if not body.get("confirmText"):
        validation_errors.append("confirmText is required.")
    if not (body.get("requestedBy") or body.get("appliedBy")):
        validation_errors.append("requestedBy is required.")
    if not isinstance(git_checkpoint, dict) or git_checkpoint.get("created") is not True or not git_checkpoint.get("commit"):
        validation_errors.append("gitCheckpoint.created=true and gitCheckpoint.commit are required.")
    if body.get("rollbackPlanAccepted") is not True:
        validation_errors.append("rollbackPlanAccepted=true is required.")
    if not isinstance(dry_run, dict):
        validation_errors.append("dryRun result is required before real apply.")
        return validation_errors

    if dry_run.get("applicationId") != application_id:
        validation_errors.append("dryRun applicationId must match application.")
    if dry_run.get("approvalId") != approval_id:
        validation_errors.append("dryRun approvalId must match source approval.")
    if dry_run.get("agentId") != agent_id:
        validation_errors.append("dryRun agentId must match target Agent.")
    if dry_run.get("dryRun") is not True or dry_run.get("ok") is not False or dry_run.get("canApply") is not False:
        validation_errors.append("dryRun must be the current feature-disabled preview.")
    if "feature_disabled" not in (dry_run.get("blockedReasons") or []):
        validation_errors.append("dryRun must include feature_disabled.")
    if len(dry_run.get("validationErrors") or []) != 0:
        validation_errors.append("dryRun must have no validation errors.")
    if not isinstance(dry_run.get("changePlanValidation"), dict) or dry_run["changePlanValidation"].get("ok") is not True:
        validation_errors.append("dryRun change plan must pass Agent config field validation.")
    if not all_agent_config_side_effects_false(dry_run.get("sideEffects")):
        validation_errors.append("dryRun side effects must all be false.")

    return validation_errors


def fetch_one(connection, sql, params):
    row = connection.execute(sql, params).fetchone()
    return row


def ensure_agent_run_schema(connection):
    connection.executescript(
        """
        CREATE TABLE IF NOT EXISTS agent_runs (
          id TEXT PRIMARY KEY,
          project_id TEXT NOT NULL,
          chain_id TEXT NOT NULL,
          root_run_id TEXT NOT NULL,
          parent_run_id TEXT,
          sequence INTEGER NOT NULL,
          role TEXT NOT NULL,
          agent_id TEXT,
          agent_name TEXT NOT NULL,
          model TEXT NOT NULL,
          status TEXT NOT NULL,
          input_summary TEXT,
          output_summary TEXT,
          token_usage TEXT NOT NULL,
          cost_estimate TEXT NOT NULL,
          error_category TEXT,
          error_message TEXT,
          requested_by TEXT NOT NULL,
          chain_label TEXT,
          created_at TEXT NOT NULL,
          started_at TEXT,
          completed_at TEXT,
          failed_at TEXT,
          updated_at TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_agent_runs_project_id ON agent_runs(project_id);
        CREATE INDEX IF NOT EXISTS idx_agent_runs_chain_id ON agent_runs(chain_id);
        CREATE INDEX IF NOT EXISTS idx_agent_runs_status ON agent_runs(status);
        CREATE INDEX IF NOT EXISTS idx_agent_runs_created_at ON agent_runs(created_at);
        """
    )


def ensure_column(connection, table, column, definition):
    columns = {
        row[1]
        for row in connection.execute(f"PRAGMA table_info({table})").fetchall()
    }
    if column not in columns:
        connection.execute(f"ALTER TABLE {table} ADD COLUMN {column} {definition}")


def ensure_runner_status_schema(connection):
    ensure_column(connection, "runner_status", "workspace_alias", "TEXT")
    ensure_column(connection, "runner_status", "capabilities", "TEXT NOT NULL DEFAULT '{}'")
    ensure_column(connection, "runner_status", "git_status", "TEXT NOT NULL DEFAULT '{}'")
    ensure_column(connection, "runner_status", "validation_commands", "TEXT NOT NULL DEFAULT '[]'")


def agent_run_row_to_api(row):
    return {
        "id": row["id"],
        "projectId": row["project_id"],
        "chainId": row["chain_id"],
        "rootRunId": row["root_run_id"],
        "parentRunId": pick(row, "parent_run_id"),
        "sequence": row["sequence"],
        "role": row["role"],
        "agentId": pick(row, "agent_id"),
        "agentName": row["agent_name"],
        "model": row["model"],
        "status": row["status"],
        "inputSummary": pick(row, "input_summary"),
        "outputSummary": pick(row, "output_summary"),
        "tokenUsage": from_json(row["token_usage"], {}),
        "costEstimate": from_json(row["cost_estimate"], {}),
        "errorCategory": pick(row, "error_category"),
        "errorMessage": pick(row, "error_message"),
        "requestedBy": pick(row, "requested_by"),
        "chainLabel": pick(row, "chain_label"),
        "createdAt": pick(row, "created_at"),
        "startedAt": pick(row, "started_at"),
        "completedAt": pick(row, "completed_at"),
        "failedAt": pick(row, "failed_at"),
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
        "workspaceAlias": pick(row, "workspace_alias"),
        "permissions": from_json(row["permissions"], {}),
        "capabilities": json_pick(row, "capabilities", {}),
        "gitStatus": json_pick(row, "git_status", {}),
        "validationCommands": json_pick(row, "validation_commands", []),
        "lastHeartbeatAt": pick(row, "last_heartbeat_at"),
    }


def runtime_event_row_to_api(row):
    return {
        "id": row["id"],
        "projectId": row["project_id"],
        "entityType": row["entity_type"],
        "entityId": row["entity_id"],
        "eventType": row["event_type"],
        "beforeState": from_json(row["before_state"], None) if row["before_state"] else None,
        "afterState": from_json(row["after_state"], None) if row["after_state"] else None,
        "actor": pick(row, "actor"),
        "reason": pick(row, "reason"),
        "createdAt": pick(row, "created_at"),
    }


def upsert_runtime_event_payload(connection, event):
    connection.execute(
        """
        INSERT INTO runtime_events (
          id, project_id, entity_type, entity_id, event_type,
          before_state, after_state, actor, reason, created_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET
          entity_type = excluded.entity_type,
          entity_id = excluded.entity_id,
          event_type = excluded.event_type,
          before_state = excluded.before_state,
          after_state = excluded.after_state,
          actor = excluded.actor,
          reason = excluded.reason,
          created_at = excluded.created_at
        """,
        (
            event["id"],
            project_id,
            event["entityType"],
            event["entityId"],
            event["eventType"],
            as_json(event.get("beforeState")) if event.get("beforeState") is not None else None,
            as_json(event.get("afterState")) if event.get("afterState") is not None else None,
            event.get("actor", "api"),
            event.get("reason", ""),
            event.get("createdAt", now()),
        ),
    )


def create_agent_run_request(connection):
    ensure_agent_run_schema(connection)

    agent_runs = args.get("agentRuns")
    runtime_events = args.get("runtimeEvents")
    chain = args.get("chain") or {}
    if not isinstance(agent_runs, list) or len(agent_runs) == 0:
        return {"statusCode": 422, "body": {"error": "agent_run_request_invalid", "message": "agentRuns are required."}}
    if not isinstance(runtime_events, list):
        return {"statusCode": 422, "body": {"error": "agent_run_request_invalid", "message": "runtimeEvents must be an array."}}

    for agent_run in agent_runs:
        connection.execute(
            """
            INSERT INTO agent_runs (
              id, project_id, chain_id, root_run_id, parent_run_id, sequence,
              role, agent_id, agent_name, model, status, input_summary,
              output_summary, token_usage, cost_estimate, error_category,
              error_message, requested_by, chain_label, created_at, started_at,
              completed_at, failed_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
              chain_id = excluded.chain_id,
              root_run_id = excluded.root_run_id,
              parent_run_id = excluded.parent_run_id,
              sequence = excluded.sequence,
              role = excluded.role,
              agent_id = excluded.agent_id,
              agent_name = excluded.agent_name,
              model = excluded.model,
              status = excluded.status,
              input_summary = excluded.input_summary,
              output_summary = excluded.output_summary,
              token_usage = excluded.token_usage,
              cost_estimate = excluded.cost_estimate,
              error_category = excluded.error_category,
              error_message = excluded.error_message,
              requested_by = excluded.requested_by,
              chain_label = excluded.chain_label,
              created_at = excluded.created_at,
              started_at = excluded.started_at,
              completed_at = excluded.completed_at,
              failed_at = excluded.failed_at,
              updated_at = excluded.updated_at
            """,
            (
                agent_run["id"],
                project_id,
                agent_run["chainId"],
                agent_run["rootRunId"],
                agent_run.get("parentRunId") or None,
                int(agent_run.get("sequence", 0)),
                agent_run.get("role", ""),
                agent_run.get("agentId") or None,
                agent_run.get("agentName", ""),
                agent_run.get("model", ""),
                agent_run.get("status", ""),
                agent_run.get("inputSummary", ""),
                agent_run.get("outputSummary", ""),
                as_json(agent_run.get("tokenUsage", {})),
                as_json(agent_run.get("costEstimate", {})),
                agent_run.get("errorCategory", ""),
                agent_run.get("errorMessage", ""),
                agent_run.get("requestedBy", "local_user"),
                agent_run.get("chainLabel", ""),
                agent_run.get("createdAt", now()),
                agent_run.get("startedAt") or None,
                agent_run.get("completedAt") or None,
                agent_run.get("failedAt") or None,
                agent_run.get("updatedAt", now()),
            ),
        )

    for event in runtime_events:
        upsert_runtime_event_payload(connection, event)

    return {
        "statusCode": 201,
        "body": {
            "chain": chain,
            "agentRuns": [agent_run_row_to_api(fetch_one(connection, "SELECT * FROM agent_runs WHERE id = ? AND project_id = ?", (agent_run["id"], project_id))) for agent_run in agent_runs],
            "runtimeEvents": [runtime_event_row_to_api(fetch_one(connection, "SELECT * FROM runtime_events WHERE id = ?", (event["id"],))) for event in runtime_events],
            "sideEffects": {
                "writesAgents": False,
                "writesAgentConfigVersions": False,
                "writesAgentConfigApplications": False,
                "writesRuntimeEvents": True,
                "writesSqlite": True,
                "writesRuntimeState": False,
                "writesProjectFiles": False,
                "modifiesGit": False,
                "createsApprovals": False,
                "createsTasks": False,
                "createsRunnerJobs": False,
                "triggersAgents": False,
                "executesRunner": False,
                "callsRealModel": False,
                "readsRawSecrets": False,
            },
            "message": "Agent Run chain recorded in SQLite. No real Runner or model call was executed.",
        },
    }


def runner_status_action(connection):
    ensure_agent_run_schema(connection)
    ensure_runner_status_schema(connection)

    action = args["action"]
    body = args.get("body", {})
    current = body.get("currentRunnerStatus")
    row = fetch_one(connection, "SELECT * FROM runner_status WHERE project_id = ? ORDER BY updated_at DESC LIMIT 1", (project_id,))
    current_status = runner_status_row_to_api(row) or current or {
        "connected": False,
        "runnerId": "",
        "version": "",
        "workspacePath": "",
        "permissions": {},
        "lastHeartbeatAt": "",
    }
    before = dict(current_status)
    before["permissions"] = dict(current_status.get("permissions") or {})
    timestamp = now()
    requested_by = body.get("requestedBy") or body.get("updatedBy") or "local_user"
    runner_id = str(body.get("runnerId") or "").strip()
    version = str(body.get("version") or "").strip()
    workspace_path = str(body.get("workspacePath") or "").strip()
    workspace_alias = str(body.get("workspaceAlias") or "").strip()
    has_permissions_update = isinstance(body.get("permissions"), dict)
    has_capabilities_update = isinstance(body.get("capabilities"), dict)
    has_git_status_update = isinstance(body.get("gitStatus"), dict)
    has_validation_commands_update = isinstance(body.get("validationCommands"), list)
    reason = str(body.get("reason") or body.get("note") or "").strip()

    if action == "heartbeat" and current_status.get("connected") is not True:
        return {"statusCode": 409, "body": {"error": "runner_status_not_connected", "message": "Runner heartbeat requires a connected Runner."}}
    if action == "disconnect" and current_status.get("connected") is not True:
        return {"statusCode": 409, "body": {"error": "runner_status_already_disconnected", "message": "Runner is already disconnected."}}

    after = dict(current_status)
    after["permissions"] = dict(current_status.get("permissions") or {})
    after["capabilities"] = dict(current_status.get("capabilities") or {})
    after["gitStatus"] = dict(current_status.get("gitStatus") or {})
    after["validationCommands"] = list(current_status.get("validationCommands") or [])
    if action == "connect":
        after["connected"] = True
        if runner_id:
            after["runnerId"] = runner_id
        if version:
            after["version"] = version
        if workspace_path:
            after["workspacePath"] = workspace_path
        if workspace_alias:
            after["workspaceAlias"] = workspace_alias
        if has_permissions_update:
            after["permissions"].update(body.get("permissions") or {})
        if has_capabilities_update:
            after["capabilities"].update(body.get("capabilities") or {})
        if has_git_status_update:
            after["gitStatus"].update(body.get("gitStatus") or {})
        if has_validation_commands_update:
            after["validationCommands"] = body.get("validationCommands") or []
        after["lastHeartbeatAt"] = timestamp
    elif action == "heartbeat":
        if runner_id:
            after["runnerId"] = runner_id
        if version:
            after["version"] = version
        if workspace_path:
            after["workspacePath"] = workspace_path
        if workspace_alias:
            after["workspaceAlias"] = workspace_alias
        if has_permissions_update:
            after["permissions"].update(body.get("permissions") or {})
        if has_capabilities_update:
            after["capabilities"].update(body.get("capabilities") or {})
        if has_git_status_update:
            after["gitStatus"].update(body.get("gitStatus") or {})
        if has_validation_commands_update:
            after["validationCommands"] = body.get("validationCommands") or []
        after["lastHeartbeatAt"] = timestamp
    elif action == "disconnect":
        after["connected"] = False
    else:
        return {"statusCode": 404, "body": {"error": "unknown_runner_status_action", "message": "Unknown runner status action."}}

    connection.execute(
        """
        INSERT INTO runner_status (
          id, project_id, connected, runner_id, version, workspace_path,
          workspace_alias, permissions, capabilities, git_status,
          validation_commands, last_heartbeat_at, created_at, updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET
          connected = excluded.connected,
          runner_id = excluded.runner_id,
          version = excluded.version,
          workspace_path = excluded.workspace_path,
          workspace_alias = excluded.workspace_alias,
          permissions = excluded.permissions,
          capabilities = excluded.capabilities,
          git_status = excluded.git_status,
          validation_commands = excluded.validation_commands,
          last_heartbeat_at = excluded.last_heartbeat_at,
          updated_at = excluded.updated_at
        """,
        (
            f"runner_status_{project_id}",
            project_id,
            as_bool(after.get("connected")),
            after.get("runnerId", ""),
            after.get("version", ""),
            after.get("workspacePath", ""),
            after.get("workspaceAlias", ""),
            as_json(after.get("permissions", {})),
            as_json(after.get("capabilities", {})),
            as_json(after.get("gitStatus", {})),
            as_json(after.get("validationCommands", [])),
            after.get("lastHeartbeatAt", timestamp),
            row["created_at"] if row else timestamp,
            timestamp,
        ),
    )

    after_row = fetch_one(connection, "SELECT * FROM runner_status WHERE project_id = ? ORDER BY updated_at DESC LIMIT 1", (project_id,))
    after_api = runner_status_row_to_api(after_row)
    runtime_event(
        connection,
        "runner_status",
        f"runner_status_{project_id}",
        "connected" if action == "connect" else "heartbeat" if action == "heartbeat" else "disconnected",
        before,
        after_api,
        actor=requested_by,
        reason=reason or f"runner_status_{action}",
    )
    return {
        "statusCode": 200,
        "body": {
            "runnerStatus": after_api,
            "runtimeEvent": runtime_event_row_to_api(fetch_one(connection, "SELECT * FROM runtime_events WHERE entity_type = ? AND entity_id = ? ORDER BY created_at DESC, id DESC LIMIT 1", ("runner_status", f"runner_status_{project_id}"))),
            "sideEffects": {
                "writesAgents": False,
                "writesAgentConfigVersions": False,
                "writesAgentConfigApplications": False,
                "writesRuntimeEvents": True,
                "writesSqlite": True,
                "writesRuntimeState": False,
                "writesProjectFiles": False,
                "modifiesGit": False,
                "createsApprovals": False,
                "createsTasks": False,
                "createsRunnerJobs": False,
                "triggersAgents": False,
                "executesRunner": False,
                "callsRealModel": False,
                "readsRawSecrets": False,
            },
            "message": "Runner status updated in SQLite local trial mode.",
        },
    }


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
        before = {
            "id": existing["id"],
            "approvalId": existing["approval_id"],
            "taskId": pick(existing, "task_id"),
            "status": existing["status"],
            "operationTypes": from_json(existing["operation_types"], []),
            "affectedFiles": from_json(existing["affected_files"], []),
            "checkpoint": pick(existing, "checkpoint_commit"),
            "safetyNote": pick(existing, "safety_note"),
            "createdAt": pick(existing, "created_at"),
            "updatedAt": pick(existing, "updated_at"),
        }
        connection.execute(
            """
            UPDATE runner_jobs
            SET operation_types = ?, affected_files = ?, checkpoint_commit = ?, updated_at = ?
            WHERE id = ? AND project_id = ?
            """,
            (as_json(approval["operationTypes"]), as_json(approval["affectedFiles"]), approval["checkpoint"]["commit"], timestamp, job_id, project_id),
        )
        updated = fetch_one(connection, "SELECT * FROM runner_jobs WHERE id = ? AND project_id = ?", (job_id, project_id))
        runtime_event(
            connection,
            "runner_job",
            job_id,
            "updated",
            before,
            {
                "id": updated["id"],
                "approvalId": updated["approval_id"],
                "taskId": pick(updated, "task_id"),
                "status": updated["status"],
                "operationTypes": from_json(updated["operation_types"], []),
                "affectedFiles": from_json(updated["affected_files"], []),
                "checkpoint": pick(updated, "checkpoint_commit"),
                "safetyNote": pick(updated, "safety_note"),
                "createdAt": pick(updated, "created_at"),
                "updatedAt": pick(updated, "updated_at"),
            },
            reason=approval["id"],
        )
    return job_id


def runner_job_terminal_status(status):
    return status in {"blocked", "failed", "completed", "cancelled"}


def runner_job_action_body_required(action):
    return action in {"start", "review", "pause", "complete", "fail", "cancel", "block"}


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


def runner_job_action(connection):
    job_id = args["jobId"]
    action = args["action"]
    body = args.get("body", {})
    row = fetch_one(connection, "SELECT * FROM runner_jobs WHERE id = ? AND project_id = ?", (job_id, project_id))
    if row is None:
        return {"statusCode": 404, "body": {"error": "runner_job_not_found"}}

    before = runner_job_row_to_api(row)
    approval = fetch_one(connection, "SELECT * FROM approvals WHERE id = ? AND project_id = ?", (row["approval_id"], project_id))
    timestamp = now()
    status = row["status"]
    next_status = None
    event_type = None
    reason = body.get("reason") or ""
    requested_by = body.get("requestedBy") or body.get("reviewedBy") or "local_user"

    if action == "review":
        if status not in {"queued", "blocked", "paused"}:
            return {"statusCode": 409, "body": {"error": "runner_job_terminal", "message": f"Runner job is already {status}."}}
        next_status = "blocked" if body.get("blocked") is True else "reviewed"
        event_type = "blocked" if body.get("blocked") is True else "reviewed"
    elif action == "start":
        if not body.get("requestedBy"):
            return {"statusCode": 409, "body": {"error": "runner_job_requested_by_required", "message": "requestedBy is required."}}
        if approval is None or approval["status"] != "approved":
            return {"statusCode": 409, "body": {"error": "runner_job_approval_required", "message": "Runner job requires an approved source approval."}}
        if body.get("scopeLockAccepted") is not True:
            return {"statusCode": 409, "body": {"error": "runner_job_scope_lock_required", "message": "scopeLockAccepted=true is required."}}
        checkpoint_commit = pick(approval, "checkpoint_commit") or pick(row, "checkpoint_commit")
        checkpoint_required = bool_from_int(approval["checkpoint_required"]) or bool(checkpoint_commit)
        if checkpoint_required and body.get("gitCheckpointCommit") != checkpoint_commit:
            return {"statusCode": 409, "body": {"error": "runner_job_checkpoint_required", "message": "gitCheckpointCommit must match the locked checkpoint."}}
        second_confirm_required = bool_from_int(approval["requires_second_confirm"]) or any(item in {"file_write", "git_checkpoint", "network_request", "command"} for item in from_json(row["operation_types"], []))
        if second_confirm_required and body.get("secondConfirm") is not True:
            return {"statusCode": 409, "body": {"error": "runner_job_second_confirm_required", "message": "secondConfirm=true is required for this job."}}
        if status not in {"queued", "reviewed", "paused"}:
            return {"statusCode": 409, "body": {"error": "runner_job_cannot_start", "message": f"Runner job cannot start from status {status}."}}
        next_status = "running"
        event_type = "started"
        reason = reason or "launch_gate_passed"
    elif action == "pause":
        if status != "running":
            return {"statusCode": 409, "body": {"error": "runner_job_cannot_pause", "message": "Only running runner jobs can pause."}}
        next_status = "paused"
        event_type = "paused"
    elif action == "complete":
        if status != "running":
            return {"statusCode": 409, "body": {"error": "runner_job_cannot_complete", "message": "Only running runner jobs can complete."}}
        next_status = "completed"
        event_type = "completed"
    elif action == "fail":
        if runner_job_terminal_status(status):
            return {"statusCode": 409, "body": {"error": "runner_job_terminal", "message": f"Runner job is already {status}."}}
        next_status = "failed"
        event_type = "failed"
        reason = reason or "execution_failed"
    elif action == "cancel":
        if runner_job_terminal_status(status):
            return {"statusCode": 409, "body": {"error": "runner_job_terminal", "message": f"Runner job is already {status}."}}
        next_status = "cancelled"
        event_type = "cancelled"
    elif action == "block":
        if runner_job_terminal_status(status):
            return {"statusCode": 409, "body": {"error": "runner_job_terminal", "message": f"Runner job is already {status}."}}
        next_status = "blocked"
        event_type = "blocked"
        reason = reason or "safety_check_failed"
    else:
        return {"statusCode": 404, "body": {"error": "unknown_runner_job_action", "message": "Unknown Runner job action."}}

    connection.execute(
        """
        UPDATE runner_jobs
        SET status = ?, updated_at = ?
        WHERE id = ? AND project_id = ?
        """,
        (next_status, timestamp, job_id, project_id),
    )
    updated = runner_job_row_to_api(fetch_one(connection, "SELECT * FROM runner_jobs WHERE id = ? AND project_id = ?", (job_id, project_id)))
    runtime_event(connection, "runner_job", job_id, event_type, before, updated, actor=requested_by, reason=reason)
    return {
        "statusCode": 200,
        "body": {
            "job": updated,
            "executionRequest": {
                "id": updated["id"],
                "approvalId": updated["approvalId"],
                "taskId": updated["taskId"],
                "status": updated["status"],
            },
        },
    }


def is_project_plan_approval(approval):
    change_request = approval.get("changeRequest") or {}
    return (
        approval.get("targetService") == "project_plan"
        or change_request.get("type") == "project_plan"
        or change_request.get("changeType") == "project_plan"
    )


def no_project_plan_request_side_effects():
    return {
        "writesProjectFiles": False,
        "modifiesGit": False,
        "executesRunner": False,
        "callsRealModel": False,
        "readsRawSecrets": False,
        "makesNetworkRequests": False,
        "triggersAgents": False,
    }


def upsert_project_plan_task(connection, planned_task, timestamp):
    task_id = planned_task["id"]
    existing = fetch_one(connection, "SELECT * FROM tasks WHERE id = ? AND project_id = ?", (task_id, project_id))
    task_status = existing["status"] if existing else planned_task.get("status", "queued")
    values = (
        task_id,
        project_id,
        planned_task.get("title", task_id),
        planned_task.get("description", ""),
        task_status,
        planned_task.get("priority", "medium"),
        planned_task.get("assignedAgentId") or None,
        planned_task.get("riskLevel", "low"),
        as_json(planned_task.get("relatedFiles", [])),
        as_bool(planned_task.get("requiresApproval")),
        as_json(planned_task.get("dependsOn", [])),
        None,
        None,
        None,
        None,
        None,
        existing["created_at"] if existing else timestamp,
        timestamp,
    )
    connection.execute(
        """
        INSERT INTO tasks (
          id, project_id, title, description, status, priority, assigned_agent_id,
          risk_level, related_files, requires_approval, depends_on,
          started_at, completed_at, failed_at, cancelled_at, failure_reason,
          created_at, updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET
          title = excluded.title,
          description = excluded.description,
          priority = excluded.priority,
          assigned_agent_id = excluded.assigned_agent_id,
          risk_level = excluded.risk_level,
          related_files = excluded.related_files,
          requires_approval = excluded.requires_approval,
          depends_on = excluded.depends_on,
          updated_at = excluded.updated_at
        """,
        values,
    )
    if existing is None:
        runtime_event(connection, "task", task_id, "created", None, {"id": task_id, "status": task_status}, reason="project_plan_approval")
    return existing is None


def upsert_project_plan_runner_job(connection, approval, runner_request, timestamp):
    job_id = runner_request["id"]
    existing = fetch_one(connection, "SELECT * FROM runner_jobs WHERE id = ? AND project_id = ?", (job_id, project_id))
    job_status = existing["status"] if existing else runner_request.get("status", "queued")
    values = (
        job_id,
        project_id,
        approval["id"],
        runner_request.get("taskId") or None,
        job_status,
        as_json(runner_request.get("operationTypes", ["runner_request_readonly"])),
        as_json(runner_request.get("affectedFiles", [])),
        "",
        runner_request.get("safetyNote") or "SQLite MVP-0.3 read-only Runner request. No command, file write, network request, or Git change is executed.",
        existing["created_at"] if existing else timestamp,
        timestamp,
    )
    connection.execute(
        """
        INSERT INTO runner_jobs (
          id, project_id, approval_id, task_id, status, operation_types,
          affected_files, checkpoint_commit, safety_note, created_at, updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET
          approval_id = excluded.approval_id,
          task_id = excluded.task_id,
          operation_types = excluded.operation_types,
          affected_files = excluded.affected_files,
          checkpoint_commit = excluded.checkpoint_commit,
          safety_note = excluded.safety_note,
          updated_at = excluded.updated_at
        """,
        values,
    )
    if existing is None:
        runtime_event(connection, "runner_job", job_id, "created", None, {"id": job_id, "status": job_status}, reason=approval["id"])
    return existing is None


def validate_project_plan_payload(planned_tasks, runner_requests):
    task_ids = set()
    for planned_task in planned_tasks:
        if not isinstance(planned_task, dict) or not str(planned_task.get("id", "")).strip():
            return {
                "error": "invalid_project_plan_approval",
                "message": "Project plan approval contains a task without an id.",
            }
        task_id = planned_task["id"]
        if task_id in task_ids:
            return {
                "error": "invalid_project_plan_approval",
                "message": "Project plan approval contains duplicate task ids.",
            }
        task_ids.add(task_id)

    runner_job_ids = set()
    for runner_request in runner_requests:
        if not isinstance(runner_request, dict) or not str(runner_request.get("id", "")).strip():
            return {
                "error": "invalid_project_plan_approval",
                "message": "Project plan approval contains a Runner request without an id.",
            }
        runner_job_id = runner_request["id"]
        if runner_job_id in runner_job_ids:
            return {
                "error": "invalid_project_plan_approval",
                "message": "Project plan approval contains duplicate Runner request ids.",
            }
        if runner_request.get("taskId") not in task_ids:
            return {
                "error": "invalid_project_plan_approval",
                "message": "Project plan Runner request must reference a planned task.",
            }
        operation_types = runner_request.get("operationTypes")
        if not isinstance(operation_types, list) or "runner_request_readonly" not in operation_types:
            return {
                "error": "invalid_project_plan_approval",
                "message": "Project plan Runner request must remain read-only.",
            }
        runner_job_ids.add(runner_job_id)

    return None


def instantiate_project_plan_approval(connection, approval, timestamp):
    change_request = approval.get("changeRequest") or {}
    plan = change_request.get("plan") or {}
    planned_tasks = plan.get("tasks") if isinstance(plan.get("tasks"), list) else []
    runner_requests = plan.get("runnerRequests") if isinstance(plan.get("runnerRequests"), list) else []
    if len(planned_tasks) == 0 or len(runner_requests) == 0:
        return {
            "error": "invalid_project_plan_approval",
            "message": "Project plan approval must contain plan tasks and runnerRequests.",
        }

    validation_error = validate_project_plan_payload(planned_tasks, runner_requests)
    if validation_error:
        return validation_error

    created_task_ids = []
    created_runner_job_ids = []
    for planned_task in planned_tasks:
        if upsert_project_plan_task(connection, planned_task, timestamp):
            created_task_ids.append(planned_task["id"])
    for runner_request in runner_requests:
        if upsert_project_plan_runner_job(connection, approval, runner_request, timestamp):
            created_runner_job_ids.append(runner_request["id"])

    return {
        "planId": plan.get("id", ""),
        "createdTaskIds": created_task_ids,
        "createdRunnerJobIds": created_runner_job_ids,
        "taskIds": [task.get("id", "") for task in planned_tasks],
        "runnerJobIds": [job.get("id", "") for job in runner_requests],
    }


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
        project_plan_result = None
        if before["targetService"] == "agent_config":
            application_id = upsert_agent_config_application(connection, before)
        elif is_project_plan_approval(before):
            project_plan_result = instantiate_project_plan_approval(connection, before, timestamp)
            if project_plan_result.get("error"):
                return {"statusCode": 409, "body": project_plan_result}
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
        body = {
            "id": approval_id,
            "status": "approved",
            "runnerJobId": runner_job_id,
            "agentConfigApplicationId": application_id,
        }
        if project_plan_result:
            body.update({
                "createdTaskIds": project_plan_result["createdTaskIds"],
                "createdRunnerJobIds": project_plan_result["createdRunnerJobIds"],
                "sideEffects": {
                    **no_project_plan_request_side_effects(),
                    "writesRuntimeState": False,
                    "writesSqlite": True,
                    "createsApproval": False,
                    "createsTasks": len(project_plan_result["createdTaskIds"]) > 0,
                    "createsRunnerJobs": len(project_plan_result["createdRunnerJobIds"]) > 0,
                },
            })
        return {"statusCode": 200, "body": body}

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
    change_request = {
        "agentId": agent_id,
        "changeType": change_type,
        "changes": changes,
        "permissionProfile": body.get("permissionProfile") or body.get("profile") or "",
        "capabilities": body.get("capabilities") if isinstance(body.get("capabilities"), list) else [],
        "permissionValidation": body.get("permissionValidation"),
    }
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
    return {
        "statusCode": 201,
        "body": {
            "approval": updated,
            "permissionValidation": body.get("permissionValidation"),
            "message": "Agent change request created. Agent config was not modified.",
        },
    }


def create_project_plan_request(connection):
    approval = args.get("approval") or {}
    plan = args.get("plan") or {}
    approval_id = approval.get("id")
    if not approval_id:
        return {"statusCode": 422, "body": {"error": "project_plan_approval_required"}}

    project = fetch_one(connection, "SELECT id FROM projects WHERE id = ?", (project_id,))
    if project is None:
        return {"statusCode": 404, "body": {"error": "project_not_found"}}

    existing = fetch_one(connection, "SELECT * FROM approvals WHERE id = ? AND project_id = ?", (approval_id, project_id))
    before = approval_row_to_api(existing) if existing else None
    if before and before["status"] != "pending":
        return {
            "statusCode": 409,
            "body": {
                "error": "project_plan_approval_already_closed",
                "message": "Existing project plan approval is no longer pending.",
                "approval": before,
                "sideEffects": {
                    **no_project_plan_request_side_effects(),
                    "writesRuntimeState": False,
                    "writesSqlite": False,
                    "createsApproval": False,
                    "createsTasks": False,
                    "createsRunnerJobs": False,
                },
            },
        }

    timestamp = now()
    checkpoint = approval.get("checkpoint") or {}
    values = (
        approval_id,
        project_id,
        "pending",
        approval.get("riskLevel", "medium"),
        approval.get("riskTone", "mid"),
        approval.get("requestAgentId") or None,
        approval.get("requestAgentName", ""),
        "project_plan",
        as_json(approval.get("operationTypes", [])),
        approval.get("reason", ""),
        as_bool(checkpoint.get("required")),
        as_bool(checkpoint.get("created")),
        checkpoint.get("commit", ""),
        as_json(approval.get("affectedFiles", [])),
        approval.get("diffSummary", ""),
        as_json(approval.get("diffPreview", [])),
        as_bool(approval.get("requiresSecondConfirm")),
        as_json(approval.get("changeRequest")),
        "",
        "",
        "",
        None,
        None,
        None,
        existing["created_at"] if existing else approval.get("createdAt", timestamp),
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
          request_agent_id = excluded.request_agent_id,
          request_agent_name = excluded.request_agent_name,
          target_service = excluded.target_service,
          operation_types = excluded.operation_types,
          reason = excluded.reason,
          checkpoint_required = excluded.checkpoint_required,
          checkpoint_created = excluded.checkpoint_created,
          checkpoint_commit = excluded.checkpoint_commit,
          affected_files = excluded.affected_files,
          diff_summary = excluded.diff_summary,
          diff_preview = excluded.diff_preview,
          requires_second_confirm = excluded.requires_second_confirm,
          change_request = excluded.change_request,
          runner_job_id = excluded.runner_job_id,
          patch_artifact_id = excluded.patch_artifact_id,
          reject_reason = excluded.reject_reason,
          approved_at = excluded.approved_at,
          rejected_at = excluded.rejected_at,
          patch_only_at = excluded.patch_only_at,
          updated_at = excluded.updated_at
        """,
        values,
    )
    updated = approval_row_to_api(fetch_one(connection, "SELECT * FROM approvals WHERE id = ? AND project_id = ?", (approval_id, project_id)))
    runtime_event(connection, "approval", approval_id, "created" if before is None else "updated", before, updated, reason="project_plan_request")
    return {
        "statusCode": 201,
        "body": {
            "approval": updated,
            "plan": plan,
            "sideEffects": {
                **no_project_plan_request_side_effects(),
                "writesRuntimeState": False,
                "writesSqlite": True,
                "createsApproval": before is None,
                "createsTasks": False,
                "createsRunnerJobs": False,
            },
            "message": "Project plan approval created in SQLite. No Agent was triggered and no Runner request was executed.",
        },
    }


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


def agent_config_application_real_apply(connection):
    application_id = args["applicationId"]
    body = args.get("body", {})
    row = fetch_one(connection, "SELECT * FROM agent_config_applications WHERE id = ? AND project_id = ?", (application_id, project_id))
    if row is None:
        return {"statusCode": 404, "body": {"error": "agent_config_application_not_found"}}

    before_application = application_row_to_api(row)
    approval = fetch_one(connection, "SELECT * FROM approvals WHERE id = ? AND project_id = ?", (row["approval_id"], project_id))
    if approval is None:
        return {"statusCode": 409, "body": {"error": "source_approval_not_found", "message": "Agent config application must reference an existing approval."}}

    approval_api = approval_row_to_api(approval)
    agent = fetch_one(connection, "SELECT * FROM agents WHERE id = ? AND project_id = ?", (row["agent_id"], project_id))
    if agent is None:
        return {"statusCode": 409, "body": {"error": "target_agent_not_found", "message": "Agent config application target Agent must exist."}}

    if before_application["status"] != "pending_apply":
        return {"statusCode": 409, "body": {"error": "application_not_pending_apply", "message": f"Agent config application cannot real-apply from status {before_application['status']}."}}
    if approval_api["status"] != "approved" or approval_api["targetService"] != "agent_config" or approval_api["runnerJobId"]:
        return {"statusCode": 409, "body": {"error": "application_preconditions_failed", "message": "Agent config real apply requires approved agent_config approval without Runner job."}}

    validation_errors = validate_real_apply_body(body, application_id, row["approval_id"], row["agent_id"])
    normalized_changes, change_errors = validate_agent_config_changes(before_application["changes"])
    validation_errors.extend(change_errors)
    if validation_errors:
        return {"statusCode": 409, "body": {"error": "agent_config_real_apply_preconditions_failed", "validationErrors": validation_errors}}

    timestamp = now()
    applied_by = body.get("requestedBy") or body.get("appliedBy") or "local_user"
    before_agent_snapshot = agent_row_to_config_snapshot(agent)
    after_agent_snapshot = dict(before_agent_snapshot)
    updates = {}

    for change in normalized_changes:
        field = change["field"]
        value = change["value"]
        after_agent_snapshot[field] = value
        if field == "permissions":
            updates[AGENT_CONFIG_FIELD_COLUMNS[field]] = as_json(value)
        elif field == "canSpawnSubAgents":
            updates[AGENT_CONFIG_FIELD_COLUMNS[field]] = 1 if value else 0
        else:
            updates[AGENT_CONFIG_FIELD_COLUMNS[field]] = value

    if not updates:
        return {"statusCode": 409, "body": {"error": "agent_config_real_apply_empty_write_set", "message": "Agent config real apply requires at least one allowed field update."}}

    updates["updated_at"] = timestamp
    assignments = ", ".join(f"{column} = ?" for column in updates)
    connection.execute(
        f"UPDATE agents SET {assignments} WHERE id = ? AND project_id = ?",
        [*updates.values(), row["agent_id"], project_id],
    )
    if connection.total_changes <= 0:
        return {"statusCode": 409, "body": {"error": "target_agent_not_updated", "message": "Target Agent update did not apply."}}

    next_version_row = fetch_one(
        connection,
        "SELECT COALESCE(MAX(version), 0) + 1 AS next_version FROM agent_config_versions WHERE agent_id = ? AND project_id = ?",
        (row["agent_id"], project_id),
    )
    next_version = int(next_version_row["next_version"])
    version_id = f"agent_config_version_{application_id}_{next_version}"
    existing_version = fetch_one(
        connection,
        "SELECT id FROM agent_config_versions WHERE agent_id = ? AND version = ?",
        (row["agent_id"], next_version),
    )
    if existing_version is not None:
        return {"statusCode": 409, "body": {"error": "agent_config_version_conflict", "message": "Agent config version already exists."}}

    connection.execute(
        """
        INSERT INTO agent_config_versions (
          id, project_id, agent_id, version, approval_id, application_id,
          config_snapshot, changes, applied_by, applied_at, created_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        """,
        (
            version_id,
            project_id,
            row["agent_id"],
            next_version,
            row["approval_id"],
            application_id,
            as_json(after_agent_snapshot),
            as_json(before_application["changes"]),
            applied_by,
            timestamp,
            timestamp,
        ),
    )

    connection.execute(
        """
        UPDATE agent_config_applications
        SET status = ?, applied_at = ?, applied_by = ?, apply_confirm_text = ?, updated_at = ?
        WHERE id = ? AND project_id = ? AND status = ?
        """,
        ("applied", timestamp, applied_by, body["confirmText"], timestamp, application_id, project_id, "pending_apply"),
    )

    updated = application_row_to_api(fetch_one(connection, "SELECT * FROM agent_config_applications WHERE id = ? AND project_id = ?", (application_id, project_id)))
    runtime_event(
        connection,
        "agent_config_application",
        application_id,
        "real_applied",
        {
            "application": before_application,
            "agentConfig": before_agent_snapshot,
        },
        {
            "application": updated,
            "agentConfig": after_agent_snapshot,
            "version": next_version,
            "versionId": version_id,
        },
        actor=applied_by,
        reason="agent_config_real_apply",
    )

    return {
        "statusCode": 200,
        "body": {
            "application": updated,
            "version": {
                "id": version_id,
                "projectId": project_id,
                "agentId": row["agent_id"],
                "version": next_version,
                "approvalId": row["approval_id"],
                "applicationId": application_id,
                "configSnapshot": after_agent_snapshot,
                "changes": before_application["changes"],
                "appliedBy": applied_by,
                "appliedAt": timestamp,
                "createdAt": timestamp,
            },
            "sideEffects": {
                "writesAgents": True,
                "writesAgentConfigVersions": True,
                "writesAgentConfigApplications": True,
                "writesRuntimeEvents": True,
                "writesSqlite": True,
                "writesRuntimeState": False,
                "createsApprovals": False,
                "createsRunnerJobs": False,
                "executesRunner": False,
                "callsRealModel": False,
                "readsRawSecrets": False,
            },
            "message": "Agent config real apply completed in one SQLite transaction.",
        },
    }


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
        elif command == "createAgentRunRequest":
            result = create_agent_run_request(connection)
        elif command == "createProjectPlanRequest":
            result = create_project_plan_request(connection)
        elif command == "agentConfigApplicationAction":
            result = agent_config_application_action(connection)
        elif command == "agentConfigApplicationRealApply":
            result = agent_config_application_real_apply(connection)
        elif command == "runnerStatusAction":
            result = runner_status_action(connection)
        elif command == "runnerJobAction":
            result = runner_job_action(connection)
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
