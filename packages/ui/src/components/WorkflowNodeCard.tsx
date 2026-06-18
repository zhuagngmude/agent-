import {
  AlertTriangle,
  ArrowRight,
  Bot,
  CheckCircle2,
  Clock3,
  LockKeyhole,
  ShieldCheck,
} from "lucide-react";
import { labelFor } from "./ChineseTermLabel";
import { riskLabel } from "../utils/labels";

export type WorkflowNodeStatus =
  | "completed"
  | "in_progress"
  | "waiting"
  | "locked"
  | "pending"
  | "skipped";

export type WorkflowAssignedAgent = {
  name: string;
  responsibility: string;
  statusLabel?: string;
};

export type WorkflowNodeData = {
  /** 阶段标识（英文 key，如 "idea_clarify"） */
  stageKey: string;
  /** 用户可见中文阶段名 */
  stageLabel: string;
  /** 阶段在管道中的序号（从 0 开始） */
  stageIndex: number;
  /** 当前状态 */
  status: WorkflowNodeStatus;
  /** 状态中文文案 */
  statusLabel: string;
  /** 负责 Agent 名称（null 表示未分配） */
  agentName: string | null;
  /** 本阶段分配的 AI 员工 */
  assignedAgents: WorkflowAssignedAgent[];
  /** 是否可自动推进 */
  canAutoAdvance: boolean;
  /** 是否需要审批 */
  requiresApproval: boolean;
  /** 风险级别 */
  riskLevel: "low" | "medium" | "high" | "none";
  /** 产出物列表 */
  artifacts: string[];
  /** 与旧 MVP 数据的对应关系说明 */
  sourceData: string;
};

type WorkflowNodeCardProps = {
  node: WorkflowNodeData;
  isActive?: boolean;
  isLast?: boolean;
};

const statusIconMap: Record<WorkflowNodeStatus, typeof CheckCircle2> = {
  completed: CheckCircle2,
  in_progress: Bot,
  waiting: Clock3,
  locked: LockKeyhole,
  pending: Clock3,
  skipped: ArrowRight,
};

export function WorkflowNodeCard({ node, isActive, isLast }: WorkflowNodeCardProps) {
  const StatusIcon = statusIconMap[node.status];
  const visibleArtifacts = node.artifacts.slice(0, 6);
  const hiddenArtifactCount = Math.max(0, node.artifacts.length - visibleArtifacts.length);

  const statusTone =
    node.status === "completed"
      ? "tone-completed"
      : node.status === "in_progress"
        ? "tone-active"
        : node.status === "locked"
          ? "tone-locked"
          : node.status === "waiting"
            ? "tone-waiting"
            : "tone-pending";

  return (
    <div className={`workflow-node-card ${statusTone}${isActive ? " is-active" : ""}`}>
      {/* 连接线 */}
      {!isLast && <div className="workflow-node-card__connector" aria-hidden="true" />}

      {/* 状态图标 */}
      <div className="workflow-node-card__icon">
        <StatusIcon size={16} aria-hidden="true" />
      </div>

      {/* 主体 */}
      <div className="workflow-node-card__body">
        <div className="workflow-node-card__header">
          <h3>{node.stageLabel}</h3>
          <span className={`workflow-node-card__status workflow-node-card__status--${node.status}`}>
            {node.statusLabel}
          </span>
        </div>

        {/* 元信息行 */}
        <div className="workflow-node-card__meta">
          {node.agentName && (
            <span className="workflow-node-card__agent">
              <Bot size={12} aria-hidden="true" />
              {node.agentName}
            </span>
          )}
          {node.riskLevel !== "none" && (
            <span className={`workflow-node-card__risk risk-${node.riskLevel}`}>
              <AlertTriangle size={12} aria-hidden="true" />
              {riskLabel(node.riskLevel)}
            </span>
          )}
          {node.requiresApproval && (
            <span className="workflow-node-card__needs-approval">
              <ShieldCheck size={12} aria-hidden="true" />
              需审批
            </span>
          )}
          {node.canAutoAdvance && (
            <span className="workflow-node-card__auto">
              <ArrowRight size={12} aria-hidden="true" />
              可自动推进
            </span>
          )}
        </div>

        {node.assignedAgents.length > 0 && (
          <div className="workflow-node-card__agents">
            <span className="workflow-node-card__label">本阶段 AI 员工</span>
            {isActive ? (
              <div className="workflow-node-card__agent-list">
                {node.assignedAgents.map((agent, index) => (
                  <div className="workflow-node-card__agent-item" key={`${agent.name}-${index}`}>
                    <Bot size={14} aria-hidden="true" />
                    <div>
                      <strong>{agent.name}</strong>
                      <span>{agent.responsibility}</span>
                    </div>
                    {agent.statusLabel ? <em>{agent.statusLabel}</em> : null}
                  </div>
                ))}
              </div>
            ) : (
              <div className="workflow-node-card__agent-chips">
                {node.assignedAgents.slice(0, 4).map((agent, index) => (
                  <span key={`${agent.name}-${index}`}>
                    <Bot size={11} aria-hidden="true" />
                    {agent.name}
                  </span>
                ))}
                {node.assignedAgents.length > 4 ? <span>还有 {node.assignedAgents.length - 4} 个</span> : null}
              </div>
            )}
          </div>
        )}

        {/* 产出物 */}
        {node.artifacts.length > 0 && (
          <div className="workflow-node-card__artifacts">
            <span className="workflow-node-card__label">产出物</span>
            <div className="workflow-node-card__artifact-list">
              {visibleArtifacts.map((a, i) => (
                <span key={i} className="workflow-node-card__artifact-tag" title={a}>
                  {labelFor(a)}
                </span>
              ))}
              {hiddenArtifactCount > 0 ? (
                <span className="workflow-node-card__artifact-tag">
                  还有 {hiddenArtifactCount} 项
                </span>
              ) : null}
            </div>
          </div>
        )}

        {/* 数据来源 */}
        <div className="workflow-node-card__source">
          <span className="workflow-node-card__label">数据来源</span>
          <code>{node.sourceData}</code>
        </div>
      </div>
    </div>
  );
}

export default WorkflowNodeCard;
