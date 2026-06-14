import type { AgentSummary } from "./agent";
import type { ApprovalSummary } from "./approval";
import type { ProjectSummary } from "./project";
import type { TaskSummary } from "./task";

export type DesktopHostOverviewData = {
  project: ProjectSummary;
  agents: AgentSummary[];
  tasks: TaskSummary[];
  approvals: ApprovalSummary[];
};

export type DesktopHostOverviewState =
  | ({ status: "browser" } & DesktopHostOverviewData)
  | { status: "loading" }
  | ({ status: "connected" } & DesktopHostOverviewData)
  | ({ status: "error"; message: string } & DesktopHostOverviewData);
