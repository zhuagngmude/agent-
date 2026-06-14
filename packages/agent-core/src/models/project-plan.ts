export const PROJECT_PLAN_ROLES = [
  "frontend",
  "backend",
  "qa",
  "docs",
  "reviewer",
] as const;

export type ProjectPlanRole = (typeof PROJECT_PLAN_ROLES)[number];

export const PROJECT_PLAN_AGENT_ASSIGNMENTS: Record<ProjectPlanRole, string> = {
  frontend: "agent_frontend",
  backend: "agent_backend",
  qa: "agent_qa",
  docs: "agent_docs",
  reviewer: "agent_reviewer",
};

export function isProjectPlanApprovalTarget(targetService: string): boolean {
  return targetService === "project_plan";
}
