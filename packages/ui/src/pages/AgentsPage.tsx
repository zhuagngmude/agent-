import { Card, Space, Table, Tag, Typography } from "antd";
import type { ColumnsType } from "antd/es/table";

import type { AgentSummary } from "@agent-swarm/shared";
import { roleLabel, statusLabel, modelLabel, agentStatusColor, agentNameLabel } from "../utils/labels";

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
  { title: "智能体名称", dataIndex: "name", render: (name: string) => agentNameLabel(name) },
  {
    title: "角色",
    dataIndex: "role",
    render: (role: string) => <Tag color="blue">{roleLabel(role)}</Tag>,
  },
  {
    title: "状态",
    dataIndex: "status",
    render: (status: string) => <Tag color={agentStatusColor(status)}>{statusLabel(status)}</Tag>,
  },
  { title: "模型", dataIndex: "model", render: (model: string) => modelLabel(model) },
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
        <Typography.Title level={2}>智能体编排</Typography.Title>
        <Typography.Text type="secondary">智能体列表（第一版只读）</Typography.Text>
      </div>

      <Card title="智能体编排">
        <Table<AgentRow>
          columns={agentColumns}
          dataSource={agentRows}
          pagination={false}
          locale={{ emptyText: "暂无智能体" }}
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
