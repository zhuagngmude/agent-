import { MessageSquareText, Sparkles } from "lucide-react";
import type { WorkflowNodeData } from "./WorkflowNodeCard";
import { WorkflowNodeCard } from "./WorkflowNodeCard";

type WorkflowCanvasProps = {
  nodes: WorkflowNodeData[];
  activeStageKey?: string;
  /** 顶部摘要信息 */
  summaryText?: string;
  commandValue?: string;
  commandPlaceholder?: string;
  commandDisabled?: boolean;
  commandLoading?: boolean;
  commandButtonLabel?: string;
  onCommandChange?: (value: string) => void;
  onCommandSubmit?: () => void;
};

/**
 * WorkflowCanvas — 蜂群工作流中心画布。
 * 以纵向卡片流展示全部阶段节点，高亮活跃阶段。
 */
export function WorkflowCanvas({
  nodes,
  activeStageKey,
  summaryText,
  commandValue = "",
  commandPlaceholder = "输入一个想法，蜂群会自动分析并生成角色任务...",
  commandDisabled = false,
  commandLoading = false,
  commandButtonLabel = "交给蜂群",
  onCommandChange,
  onCommandSubmit,
}: WorkflowCanvasProps) {
  const activeIndex = nodes.findIndex((n) => n.stageKey === activeStageKey);
  const completedNodes = nodes.filter((n) => n.status === "completed").length;

  return (
    <div className="workflow-canvas">
      {/* 画布顶栏 */}
      <div className="workflow-canvas__header">
        <div>
          <h2>工作流画布</h2>
          <span>
            阶段管道 · {nodes.length} 个阶段 · {completedNodes} 个已完成
          </span>
        </div>
        {summaryText && (
          <div className="workflow-canvas__summary-badge">
            <Sparkles size={14} aria-hidden="true" />
            <span>{summaryText}</span>
          </div>
        )}
      </div>

      {/* 节点流 */}
      <div className="workflow-canvas__flow">
        {nodes.length === 0 ? (
          <div className="workflow-canvas__empty">
            <MessageSquareText size={32} aria-hidden="true" />
            <h3>暂无工作流数据</h3>
            <p>
              从项目计划、审批和执行记录推导工作流阶段。请先创建项目种子或计划草案。
            </p>
          </div>
        ) : (
          nodes.map((node, idx) => (
            <WorkflowNodeCard
              key={node.stageKey}
              node={node}
              isActive={node.stageKey === activeStageKey}
              isLast={idx === nodes.length - 1}
            />
          ))
        )}
      </div>

      {/* 底部对话输入区 */}
      <div className="workflow-canvas__command-box">
        <MessageSquareText size={19} aria-hidden="true" />
        <input
          value={commandValue}
          maxLength={500}
          placeholder={commandPlaceholder}
          disabled={commandDisabled || commandLoading}
          onChange={(event) => onCommandChange?.(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter" && !event.shiftKey) {
              event.preventDefault();
              onCommandSubmit?.();
            }
          }}
        />
        <button
          type="button"
          disabled={commandDisabled || commandLoading || !commandValue.trim()}
          onClick={onCommandSubmit}
        >
          {commandLoading ? "生成中" : commandButtonLabel}
        </button>
      </div>
    </div>
  );
}

export default WorkflowCanvas;
