import { GitBranch, Layers } from "lucide-react";
import type { WorkflowNodeData } from "./WorkflowNodeCard";

type WorkflowStageRailProps = {
  nodes: WorkflowNodeData[];
  activeStageKey?: string;
  completedCount: number;
  totalCount: number;
};

/**
 * WorkflowStageRail — 蜂群工作流左侧阶段轨道。
 * 展示完整阶段管线概览，高亮当前阶段并统计进度。
 */
export function WorkflowStageRail({ nodes, activeStageKey, completedCount, totalCount }: WorkflowStageRailProps) {
  const progressPct = totalCount > 0 ? Math.round((completedCount / totalCount) * 100) : 0;

  return (
    <div className="workflow-stage-rail">
      <div className="workflow-stage-rail__heading">
        <Layers size={16} aria-hidden="true" />
        <span>阶段轨道</span>
      </div>

      {/* 进度摘要 */}
      <div className="workflow-stage-rail__summary">
        <div className="workflow-stage-rail__progress-ring">
          <svg viewBox="0 0 64 64" aria-label={`阶段完成 ${progressPct}%`}>
            <circle
              cx="32"
              cy="32"
              r="26"
              fill="none"
              stroke="rgba(177,196,220,0.35)"
              strokeWidth="5"
            />
            <circle
              cx="32"
              cy="32"
              r="26"
              fill="none"
              stroke="url(#wf-progress-grad)"
              strokeWidth="5"
              strokeLinecap="round"
              strokeDasharray={`${(progressPct / 100) * 163.36} 163.36`}
              transform="rotate(-90 32 32)"
            />
            <defs>
              <linearGradient id="wf-progress-grad" x1="0" y1="0" x2="1" y2="0">
                <stop offset="0%" stopColor="var(--as-blue)" />
                <stop offset="100%" stopColor="var(--as-mint)" />
              </linearGradient>
            </defs>
          </svg>
          <strong>{progressPct}%</strong>
        </div>
        <span>
          {completedCount} / {totalCount} 阶段完成
        </span>
      </div>

      {/* 阶段列表 */}
      <div className="workflow-stage-rail__list">
        {nodes.map((node) => {
          const isActive = node.stageKey === activeStageKey;
          const dotTone =
            node.status === "completed"
              ? "dot-done"
              : node.status === "in_progress"
                ? "dot-active"
                : node.status === "locked"
                  ? "dot-locked"
                  : node.status === "waiting"
                    ? "dot-waiting"
                    : "dot-pending";

          return (
            <button
              key={node.stageKey}
              type="button"
              className={`workflow-stage-rail__item${isActive ? " is-active" : ""}`}
              disabled={node.status === "pending"}
            >
              <span className={`workflow-stage-rail__dot ${dotTone}`} aria-hidden="true" />
              <span className="workflow-stage-rail__label">{node.stageLabel}</span>
              {node.status === "completed" && (
                <span className="workflow-stage-rail__badge">✓</span>
              )}
              {node.status === "in_progress" && (
                <span className="workflow-stage-rail__badge is-live">进行中</span>
              )}
              {node.status === "waiting" && (
                <span className="workflow-stage-rail__badge is-waiting">等待</span>
              )}
            </button>
          );
        })}
      </div>

      {/* 图例 */}
      <div className="workflow-stage-rail__legend">
        <span>
          <GitBranch size={12} aria-hidden="true" /> 阶段管线
        </span>
        <span>从已有审批和执行记录推导</span>
      </div>
    </div>
  );
}

export default WorkflowStageRail;
