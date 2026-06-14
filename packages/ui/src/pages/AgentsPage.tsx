import { Card, Space, Table, Tag, Typography } from "antd";
import type { ColumnsType } from "antd/es/table";

import type { AgentSummary } from "../utils/desktopHost";

// ---------------------------------------------------------------------------
// 行数据类型
// ---------------------------------------------------------------------------

type AgentRow = {
  key: string;
  name: string;
  role: string;
  status: string;
  model: string;
};

// ---------------------------------------------------------------------------
// 列定义
// ---------------------------------------------------------------------------

const agentColumns: ColumnsType<AgentRow> = [
  { title: "Agent", dataIndex: "name" },
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
  { title: "模型", dataIndex: "model" },
];

// ---------------------------------------------------------------------------
// Props
// ---------------------------------------------------------------------------

type AgentsPageProps = {
  agents: AgentSummary[];
};

// ---------------------------------------------------------------------------
// AgentsPage
// ---------------------------------------------------------------------------

export function AgentsPage({ agents }: AgentsPageProps) {
  const agentRows = toAgentRows(agents);

  return (
    <Space orientation="vertical" size={16} className="page-stack">
      <div className="page-heading">
        <Typography.Title level={2}>Agent 编排</Typography.Title>
        <Typography.Text type="secondary">Agent 列表（第一版只读）</Typography.Text>
      </div>

      <Card title="Agent 编排">
        <Table<AgentRow>
          columns={agentColumns}
          dataSource={agentRows}
          pagination={false}
          locale={{ emptyText: "暂无 Agent" }}
        />
      </Card>
    </Space>
  );
}

// ---------------------------------------------------------------------------
// 行数据转换
// ---------------------------------------------------------------------------

function toAgentRows(agents: AgentSummary[]): AgentRow[] {
  return agents.map((agent) => ({
    key: agent.id,
    name: agent.name,
    role: agent.role,
    status: agent.status,
    model: agent.model ?? "未指定",
  }));
}
