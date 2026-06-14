import type { TaskStatus } from "../../../shared/src/types/task";

export const taskStatusTransitions: Record<TaskStatus, TaskStatus[]> = {
  queued: ["running", "cancelled"],
  running: ["completed", "blocked", "waiting_user", "failed", "cancelled"],
  blocked: ["running"],
  waiting_user: ["running", "cancelled"],
  completed: [],
  cancelled: [],
  failed: [],
};

export function isValidTransition(current: TaskStatus, next: TaskStatus): boolean {
  const allowed = taskStatusTransitions[current];
  return allowed !== undefined && allowed.includes(next);
}

export function isTerminalStatus(status: TaskStatus): boolean {
  return taskStatusTransitions[status]?.length === 0;
}
