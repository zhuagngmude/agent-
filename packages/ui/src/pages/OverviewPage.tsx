import { Alert, Card, Col, Row, Space, Statistic, Table, Tag, Typography } from "antd";
import type { TableProps } from "antd";

import { StatusBadge } from "../components/StatusBadge";
import { zhText } from "../i18n/zh";
import type { AgentSummary, ApprovalSummary, TaskSummary } from "../utils/desktopHost";
import { useDesktopHostOverview } from "../utils/desktopHost";

type TaskRow = {
  key: string;
  title: string;
  owner: string;
  status: TaskSummary["status"];
  priority: string;
};

type AgentRow = {
  key: string;
  name: string;
  role: string;
  status: string;
  model: string;
};

type ApprovalRow = {
  key: string;
  service: string;
  risk: string;
  status: string;
  operations: string[];
};

const taskColumns: TableProps<TaskRow>["columns"] = [
  {
    title: "任务",
    dataIndex: "title",
  },
  {
    title: "负责 Agent",
    dataIndex: "owner",
    render: (owner: string) => <Tag>{owner}</Tag>,
  },
  {
    title: "优先级",
    dataIndex: "priority",
    render: (priority: string) => <Tag color={priority === "high" ? "red" : "default"}>{priority}</Tag>,
  },
  {
    title: "状态",
    dataIndex: "status",
    render: (status: TaskRow["status"]) => <StatusBadge status={status} />,
  },
];

const agentColumns: TableProps<AgentRow>["columns"] = [
  {
    title: "Agent",
    dataIndex: "name",
  },
  {
    title: "角色",
    dataIndex: "role",
    render: (role: string) => <Tag color="blue">{role}</Tag>,
  },
  {
    title: "状态",
    dataIndex: "status",
    render: (status: string) => <Tag color={status === "running" ? "success" : "default"}>{status}</Tag>,
  },
  {
    title: "模型",
    dataIndex: "model",
  },
];

const approvalColumns: TableProps<ApprovalRow>["columns"] = [
  {
    title: "目标服务",
    dataIndex: "service",
    render: (service: string) => <Tag>{service}</Tag>,
  },
  {
    title: "风险",
    dataIndex: "risk",
    render: (risk: string) => <Tag color={risk === "high" ? "red" : "orange"}>{risk}</Tag>,
  },
  {
    title: "状态",
    dataIndex: "status",
    render: (status: string) => <Tag color={status === "pending" ? "processing" : "default"}>{status}</Tag>,
  },
  {
    title: "操作类型",
    dataIndex: "operations",
    render: (operations: string[]) => (
      <Space size={4} wrap>
        {operations.map((operation) => (
          <Tag key={operation}>{operation}</Tag>
        ))}
      </Space>
    ),
  },
];

export function OverviewPage() {
  const desktopHost = useDesktopHostOverview();
  const isLoading = desktopHost.status === "loading";
  const data = desktopHost.status === "loading" ? null : desktopHost;
  const taskRows = data ? toTaskRows(data.tasks, data.agents) : [];
  const agentRows = data ? toAgentRows(data.agents) : [];
  const approvalRows = data ? toApprovalRows(data.approvals) : [];

  return (
    <Space orientation="vertical" size={16} className="page-stack">
      <div className="page-heading">
        <Typography.Title level={2}>{zhText.overview.title}</Typography.Title>
        <Typography.Text type="secondary">{zhText.overview.subtitle}</Typography.Text>
      </div>

      <Row gutter={[16, 16]}>
        <Col xs={24} md={8}>
          <Card>
            <Statistic title="任务" value={taskRows.length} loading={isLoading} />
          </Card>
        </Col>
        <Col xs={24} md={8}>
          <Card>
            <Statistic title="Agent" value={agentRows.length} loading={isLoading} />
          </Card>
        </Col>
        <Col xs={24} md={8}>
          <Card>
            <Statistic title="待审" value={approvalRows.length} loading={isLoading} />
          </Card>
        </Col>
      </Row>

      <Card title="桌面宿主连接">
        {desktopHost.status === "connected" ? (
          <Space size={8} wrap>
            <Tag color="success">已连接</Tag>
            <Typography.Text>{desktopHost.project.name}</Typography.Text>
            <Typography.Text type="secondary">{desktopHost.project.phase}</Typography.Text>
          </Space>
        ) : null}
        {desktopHost.status === "loading" ? (
          <Space size={8} wrap>
            <Tag color="processing">连接中</Tag>
            <Typography.Text type="secondary">正在读取 Rust 宿主和 SQLite 数据</Typography.Text>
          </Space>
        ) : null}
        {desktopHost.status === "browser" ? (
          <Space size={8} wrap>
            <Tag>浏览器预览</Tag>
            <Typography.Text type="secondary">当前未运行在 Tauri 桌面宿主内，正在使用 fallback 数据</Typography.Text>
          </Space>
        ) : null}
        {desktopHost.status === "error" ? (
          <Space direction="vertical" size={8}>
            <Space size={8} wrap>
              <Tag color="error">连接失败</Tag>
              <Typography.Text type="secondary">{desktopHost.message}</Typography.Text>
            </Space>
            <Alert type="warning" showIcon message="已切换到浏览器 fallback 数据" />
          </Space>
        ) : null}
      </Card>

      <Card title="任务拆解">
        <Table<TaskRow>
          columns={taskColumns}
          dataSource={taskRows}
          loading={isLoading}
          pagination={false}
          locale={{ emptyText: "暂无任务" }}
        />
      </Card>

      <Row gutter={[16, 16]}>
        <Col xs={24} xl={12}>
          <Card title="Agent 编排">
            <Table<AgentRow>
              columns={agentColumns}
              dataSource={agentRows}
              loading={isLoading}
              pagination={false}
              locale={{ emptyText: "暂无 Agent" }}
            />
          </Card>
        </Col>
        <Col xs={24} xl={12}>
          <Card title="Runner 审批">
            <Table<ApprovalRow>
              columns={approvalColumns}
              dataSource={approvalRows}
              loading={isLoading}
              pagination={false}
              locale={{ emptyText: "暂无审批" }}
            />
          </Card>
        </Col>
      </Row>
    </Space>
  );
}

function toTaskRows(tasks: TaskSummary[], agents: AgentSummary[]): TaskRow[] {
  const agentNameById = new Map(agents.map((agent) => [agent.id, agent.name]));

  return tasks.map((task) => ({
    key: task.id,
    title: task.title,
    owner: task.assigned_agent_id ? (agentNameById.get(task.assigned_agent_id) ?? task.assigned_agent_id) : "未分配",
    status: task.status,
    priority: task.priority,
  }));
}

function toAgentRows(agents: AgentSummary[]): AgentRow[] {
  return agents.map((agent) => ({
    key: agent.id,
    name: agent.name,
    role: agent.role,
    status: agent.status,
    model: agent.model ?? "未指定",
  }));
}

function toApprovalRows(approvals: ApprovalSummary[]): ApprovalRow[] {
  return approvals.map((approval) => ({
    key: approval.id,
    service: approval.target_service,
    risk: approval.risk_level,
    status: approval.status,
    operations: approval.operation_types,
  }));
}
