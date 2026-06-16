import type { AgentSummary, ApprovalSummary, ProjectSummary, TaskSummary } from "@agent-swarm/shared";
import type { PageKey } from "../routes/mainNavItems";

import { ConsoleDashboardPage } from "./ConsoleDashboardPage";

type OverviewPageProps = {
  project: ProjectSummary;
  tasks: TaskSummary[];
  agents: AgentSummary[];
  approvals: ApprovalSummary[];
  connectionStatus: "loading" | "browser" | "connected" | "error";
  message?: string;
  onNavigate: (page: PageKey) => void;
  onRefresh?: () => void;
};

export function OverviewPage({ project, tasks, agents, approvals, connectionStatus, message, onNavigate, onRefresh }: OverviewPageProps) {
  return (
    <ConsoleDashboardPage
      project={project}
      tasks={tasks}
      agents={agents}
      approvals={approvals}
      connectionStatus={connectionStatus}
      message={message}
      onNavigate={onNavigate}
      onRefresh={onRefresh}
    />
  );
}
