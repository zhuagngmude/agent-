import type { AgentRunStatus } from "@agent-swarm/shared";

/** Agent Run 状态值列表（与 Rust normalize 对齐的允许值） */
export const AGENT_RUN_STATUS_VALUES: AgentRunStatus[] = [
  "queued",
  "running",
  "succeeded",
  "failed",
  "blocked",
];

/** Agent Run 角色值列表 */
export const AGENT_RUN_ROLE_VALUES = [
  "architect",
  "scheduler",
  "frontend",
  "backend",
  "qa",
  "docs",
  "reviewer",
] as const;

export type AgentRunRole = (typeof AGENT_RUN_ROLE_VALUES)[number];

/** 终态判断（succeeded 或 failed） */
export function isTerminalAgentRunStatus(status: AgentRunStatus): boolean {
  return status === "succeeded" || status === "failed";
}
