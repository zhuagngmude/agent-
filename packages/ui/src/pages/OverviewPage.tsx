import { Alert, Card, Col, Row, Space, Statistic, Tag, Typography } from "antd";

import type { AgentSummary, ApprovalSummary, ProjectSummary, TaskSummary } from "../utils/desktopHost";

// ---------------------------------------------------------------------------
// Props
// ---------------------------------------------------------------------------

type OverviewPageProps = {
  project: ProjectSummary;
  tasks: TaskSummary[];
  agents: AgentSummary[];
  approvals: ApprovalSummary[];
  connectionStatus: "loading" | "browser" | "connected" | "error";
  message?: string;
};

// ---------------------------------------------------------------------------
// OverviewPage（总览仪表盘）
// ---------------------------------------------------------------------------

export function OverviewPage({ project, tasks, agents, approvals, connectionStatus, message }: OverviewPageProps) {
  const pendingApprovalCount = approvals.filter((a) => a.status === "pending").length;
  const isLoading = connectionStatus === "loading";

  return (
    <Space orientation="vertical" size={16} className="page-stack">
      <div className="page-heading">
        <Typography.Title level={2}>项目总览</Typography.Title>
        <Typography.Text type="secondary">这里是共享 UI 工程骨架，后续页面会从旧原型迁移到这一套组件体系。</Typography.Text>
      </div>

      <Row gutter={[16, 16]}>
        <Col xs={24} md={8}>
          <Card>
            <Statistic title="任务" value={tasks.length} loading={isLoading} />
          </Card>
        </Col>
        <Col xs={24} md={8}>
          <Card>
            <Statistic title="Agent" value={agents.length} loading={isLoading} />
          </Card>
        </Col>
        <Col xs={24} md={8}>
          <Card>
            <Statistic title="待审" value={pendingApprovalCount} loading={isLoading} />
          </Card>
        </Col>
      </Row>

      <Card title="桌面宿主连接">
        {connectionStatus === "connected" ? (
          <Space size={8} wrap>
            <Tag color="success">已连接</Tag>
            <Typography.Text>{project.name}</Typography.Text>
            <Typography.Text type="secondary">{project.phase}</Typography.Text>
          </Space>
        ) : null}
        {connectionStatus === "loading" ? (
          <Space size={8} wrap>
            <Tag color="processing">连接中</Tag>
            <Typography.Text type="secondary">正在读取 Rust 宿主和 SQLite 数据</Typography.Text>
          </Space>
        ) : null}
        {connectionStatus === "browser" ? (
          <Space size={8} wrap>
            <Tag>浏览器预览</Tag>
            <Typography.Text type="secondary">
              当前未运行在 Tauri 桌面宿主内，正在使用 fallback 数据
            </Typography.Text>
          </Space>
        ) : null}
        {connectionStatus === "error" ? (
          <Space direction="vertical" size={8}>
            <Space size={8} wrap>
              <Tag color="error">连接失败</Tag>
              <Typography.Text type="secondary">{message}</Typography.Text>
            </Space>
            <Alert type="warning" showIcon message="已切换到浏览器 fallback 数据" />
          </Space>
        ) : null}
      </Card>
    </Space>
  );
}
