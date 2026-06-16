import {
  AlertTriangle,
  FileEdit,
  FileMinus,
  FilePlus,
  GitCommit,
  LockKeyhole,
  Network,
  ShieldCheck,
  Sparkles,
  UserCheck,
  Workflow,
} from "lucide-react";

export type AutomationRuleItem = {
  key: string;
  label: string;
  description: string;
  /** 当前展示的开关状态 */
  enabled: boolean;
  /** 该规则在此阶段是否可操作（第一版全部只读） */
  readonly: boolean;
  /** 分组 */
  group: "safe" | "dangerous";
};

const DEFAULT_RULES: AutomationRuleItem[] = [
  // -- 安全自动化 --
  {
    key: "auto_draft_plan",
    label: "自动生成计划草案",
    description: "基于项目种子和想法生成结构化计划草案，不执行任何操作。",
    enabled: true,
    readonly: true,
    group: "safe",
  },
  {
    key: "auto_split_templates",
    label: "自动拆分任务模板",
    description: "将计划草案拆分为独立任务模板，供人工审查分配。",
    enabled: true,
    readonly: true,
    group: "safe",
  },
  {
    key: "auto_create_approval",
    label: "创建审批请求",
    description: "为高风险操作自动生成审批记录，等待人工确认。",
    enabled: true,
    readonly: true,
    group: "safe",
  },
  {
    key: "auto_generate_preview",
    label: "生成只读预演",
    description: "在执行前自动生成只读预演报告，不产生实际变更。",
    enabled: true,
    readonly: true,
    group: "safe",
  },
  {
    key: "auto_summarize_next",
    label: "汇总结果和下一步建议",
    description: "执行完成后自动汇总产出并生成下一步建议。",
    enabled: true,
    readonly: true,
    group: "safe",
  },
  // -- 危险自动化（必须显示为关闭） --
  {
    key: "auto_exec_commands",
    label: "自动执行命令",
    description: "在不经人工确认的情况下自动运行命令。此阶段强制关闭。",
    enabled: false,
    readonly: true,
    group: "dangerous",
  },
  {
    key: "auto_write_files",
    label: "自动写入文件",
    description: "在不经人工确认的情况下自动创建/修改文件。此阶段强制关闭。",
    enabled: false,
    readonly: true,
    group: "dangerous",
  },
  {
    key: "auto_delete_files",
    label: "自动删除文件",
    description: "在不经人工确认的情况下自动删除文件。此阶段强制关闭。",
    enabled: false,
    readonly: true,
    group: "dangerous",
  },
  {
    key: "auto_git_commit",
    label: "自动版本提交 / 推送",
    description: "在不经人工确认的情况下自动提交和推送代码。此阶段强制关闭。",
    enabled: false,
    readonly: true,
    group: "dangerous",
  },
  {
    key: "bypass_approval",
    label: "绕过人工审批",
    description: "跳过审批链直接执行。此阶段强制关闭且不可绕过。",
    enabled: false,
    readonly: true,
    group: "dangerous",
  },
  {
    key: "batch_auto_runner",
    label: "批量自动执行",
    description: "一次性自动执行多个任务。此阶段强制关闭。",
    enabled: false,
    readonly: true,
    group: "dangerous",
  },
];

type AutomationRulesPanelProps = {
  /** 可覆盖默认规则列表；传 undefined 使用默认 */
  rules?: AutomationRuleItem[];
};

export function AutomationRulesPanel({ rules = DEFAULT_RULES }: AutomationRulesPanelProps) {
  const safeRules = rules.filter((r) => r.group === "safe");
  const dangerousRules = rules.filter((r) => r.group === "dangerous");

  return (
    <div className="automation-rules-panel">
      <div className="automation-rules-panel__heading">
        <Workflow size={16} aria-hidden="true" />
        <span>自动化规则</span>
      </div>

      <p className="automation-rules-panel__hint">
        以下规则展示当前可自动推进的环节与安全边界。第一版为只读展示，不保存配置。
      </p>

      {/* 安全自动化 */}
      <div className="automation-rules-panel__section">
        <div className="automation-rules-panel__section-title">
          <ShieldCheck size={14} aria-hidden="true" />
          <span>已开启的安全自动化</span>
        </div>
        {safeRules.map((rule) => (
          <div key={rule.key} className="automation-rule-item">
            <div className="automation-rule-item__icon is-safe">
              {rule.key.includes("draft") ? (
                <FileEdit size={14} aria-hidden="true" />
              ) : rule.key.includes("template") ? (
                <Network size={14} aria-hidden="true" />
              ) : rule.key.includes("approval") ? (
                <UserCheck size={14} aria-hidden="true" />
              ) : rule.key.includes("preview") ? (
                <Sparkles size={14} aria-hidden="true" />
              ) : (
                <Sparkles size={14} aria-hidden="true" />
              )}
            </div>
            <div className="automation-rule-item__body">
              <h4>{rule.label}</h4>
              <p>{rule.description}</p>
            </div>
            <span className={`automation-rule-item__toggle${rule.enabled ? " is-on" : ""}`}>
              {rule.enabled ? "开" : "关"}
            </span>
          </div>
        ))}
      </div>

      {/* 危险自动化（强制关闭） */}
      <div className="automation-rules-panel__section">
        <div className="automation-rules-panel__section-title is-danger">
          <AlertTriangle size={14} aria-hidden="true" />
          <span>强制关闭的高风险项</span>
        </div>
        {dangerousRules.map((rule) => (
          <div key={rule.key} className="automation-rule-item is-dangerous">
            <div className="automation-rule-item__icon is-danger">
              {rule.key.includes("command") ? (
                <LockKeyhole size={14} aria-hidden="true" />
              ) : rule.key.includes("write") ? (
                <FilePlus size={14} aria-hidden="true" />
              ) : rule.key.includes("delete") ? (
                <FileMinus size={14} aria-hidden="true" />
              ) : rule.key.includes("git") ? (
                <GitCommit size={14} aria-hidden="true" />
              ) : rule.key.includes("bypass") ? (
                <AlertTriangle size={14} aria-hidden="true" />
              ) : (
                <LockKeyhole size={14} aria-hidden="true" />
              )}
            </div>
            <div className="automation-rule-item__body">
              <h4>{rule.label}</h4>
              <p>{rule.description}</p>
            </div>
            <span className="automation-rule-item__toggle is-locked-off">关</span>
          </div>
        ))}
      </div>

      {/* 失败策略 */}
      <div className="automation-rules-panel__fail-policy">
        <AlertTriangle size={16} aria-hidden="true" />
        <p>
          任一节点失败或风险升级时，自动化暂停，回到主控对话询问用户，不继续向后执行。
        </p>
      </div>
    </div>
  );
}

export default AutomationRulesPanel;
