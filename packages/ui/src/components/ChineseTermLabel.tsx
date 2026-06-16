/**
 * ChineseTermLabel — 统一内部术语到用户中文文案的映射组件。
 *
 * 使用方式：
 *   <ChineseTermLabel term="preflight" />           → "执行前审查"
 *   <ChineseTermLabel term="dry-run" fallback />    → "Dry Run"（回退原文）
 *
 * 也可通过 `labelFor(term)` 在非 JSX 场景获取映射文案。
 *
 * **注意**：状态、风险、角色、操作类型等字段的映射请使用
 * `../utils/fieldLabels` 中的专项函数（statusLabel、riskLabel 等）。
 * 本组件主要用于技术术语的独立映射。
 */

import {
  statusLabel,
  riskLabel,
  roleLabel,
  operationTypeLabel,
  targetServiceLabel,
  priorityLabel,
} from "../utils/labels";

const TERM_MAP: Record<string, string> = {
  // Runner 相关
  preflight: "执行前审查",
  gate: "执行许可闸门",
  "dry-run": "只读预演",
  lock: "执行锁定",
  "minimal-run": "最小范围执行",

  // 项目计划
  "project seed": "项目种子",
  "plan draft": "计划草案",
  "pending approval": "等待审批",
  "Runner request": "执行请求",
  "model_calls audit": "模型调用审计",

  // Git
  "Git checkpoint": "版本保存点",
  "Git commit / push": "版本提交 / 推送",

  // 通用
  API: "接口",
  debug: "调试",

  // 扩展映射（下划线/驼峰兼容）
  preflight_review: "执行前审查",
  execution_gate: "执行许可闸门",
  dry_run: "只读预演",
  execution_lock: "执行锁定",
  minimal_run: "最小范围执行",
  model_call: "模型调用审计",
  runner_preflight: "执行前审查",
  runner_gate: "执行许可闸门",
  runner_dry_run: "只读预演",
  runner_lock: "执行锁定",
  runner_minimal_run: "最小范围执行",

  // 项目阶段
  "MVP-0.4": "MVP 0.4 · 前端主控台",
  "MVP-0.5": "MVP 0.5 · 蜂群工作流",
};

/**
 * 获取中文术语，无匹配时尝试 fieldLabels 映射，最后回退原文。
 */
export function labelFor(term: string): string {
  const key = term.toLowerCase();
  const direct = TERM_MAP[key] ?? TERM_MAP[term];
  if (direct) return direct;

  // 回退到 fieldLabels 映射
  return (
    statusLabel(term) !== `未识别（${term}）` ? statusLabel(term)
    : riskLabel(term) !== term ? riskLabel(term)
    : roleLabel(term) !== term ? roleLabel(term)
    : operationTypeLabel(term) !== term ? operationTypeLabel(term)
    : targetServiceLabel(term) !== term ? targetServiceLabel(term)
    : priorityLabel(term) !== term ? priorityLabel(term)
    : term
  );
}

type ChineseTermLabelProps = {
  term: string;
  fallback?: boolean;
};

export function ChineseTermLabel({ term, fallback }: ChineseTermLabelProps) {
  const label = labelFor(term);
  if (!fallback && label === term) {
    // 无映射时，尽量拆分驼峰/下划线为用户可读形式
    return <span>{term.replace(/_/g, " ").replace(/([a-z])([A-Z])/g, "$1 $2")}</span>;
  }
  return <span>{label}</span>;
}

export default ChineseTermLabel;
