import { useState } from "react";
import { Spin } from "antd";

import { AppShell } from "../layouts/AppShell";
import type { PageKey } from "../routes/mainNavItems";
import { AgentRunsPage } from "../pages/AgentRunsPage";
import { AgentsPage } from "../pages/AgentsPage";
import { ApprovalsPage } from "../pages/ApprovalsPage";
import { OverviewPage } from "../pages/OverviewPage";
import { SettingsPage } from "../pages/SettingsPage";
import { TasksPage } from "../pages/TasksPage";
import { WorkflowPage } from "../pages/WorkflowPage";
import { useDesktopHostOverview } from "../utils/desktopHost";

export function App() {
  const [activePage, setActivePage] = useState<PageKey>("overview");
  const overviewState = useDesktopHostOverview();

  const renderPage = () => {
    if (overviewState.status === "loading") {
      return <Spin size="large" style={{ display: "block", marginTop: 120 }} />;
    }

    const { refresh, ...data } = overviewState;
    const canWrite = overviewState.status === "connected";

    switch (activePage) {
      case "overview":
        return <OverviewPage {...data} connectionStatus={overviewState.status} message={"message" in overviewState ? overviewState.message : undefined} onNavigate={setActivePage} onRefresh={refresh} />;
      case "tasks":
        return <TasksPage tasks={data.tasks} agents={data.agents} refresh={refresh} canWrite={canWrite} />;
      case "workflow":
        return <WorkflowPage {...data} connectionStatus={overviewState.status} message={"message" in overviewState ? overviewState.message : undefined} onRefresh={refresh} />;
      case "runs":
        return <AgentRunsPage />;
      case "agents":
        return <AgentsPage agents={data.agents} />;
      case "approvals":
        return <ApprovalsPage approvals={data.approvals} refresh={refresh} canWrite={canWrite} />;
      case "settings":
        return <SettingsPage project={data.project} />;
    }
  };

  return (
    <AppShell activePage={activePage} onNavigate={setActivePage}>
      {renderPage()}
    </AppShell>
  );
}
