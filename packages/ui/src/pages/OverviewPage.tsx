import { Card, Col, Row, Space, Statistic, Table, Tag, Typography } from "antd";
import type { TableProps } from "antd";

import { StatusBadge } from "../components/StatusBadge";
import { zhText } from "../i18n/zh";

type TaskRow = {
  key: string;
  name: string;
  owner: string;
  status: "queued" | "review" | "blocked";
};

const taskRows: TaskRow[] = [
  { key: "1", name: "共享 UI 骨架", owner: "frontend", status: "review" },
  { key: "2", name: "Tauri 宿主边界", owner: "desktop", status: "queued" },
  { key: "3", name: "SQLite 迁移方案", owner: "storage", status: "queued" },
];

const columns: TableProps<TaskRow>["columns"] = [
  {
    title: "任务",
    dataIndex: "name",
  },
  {
    title: "负责域",
    dataIndex: "owner",
    render: (owner: string) => <Tag>{owner}</Tag>,
  },
  {
    title: "状态",
    dataIndex: "status",
    render: (status: TaskRow["status"]) => <StatusBadge status={status} />,
  },
];

export function OverviewPage() {
  return (
    <Space orientation="vertical" size={16} className="page-stack">
      <div className="page-heading">
        <Typography.Title level={2}>{zhText.overview.title}</Typography.Title>
        <Typography.Text type="secondary">{zhText.overview.subtitle}</Typography.Text>
      </div>

      <Row gutter={[16, 16]}>
        <Col xs={24} md={8}>
          <Card>
            <Statistic title="当前阶段" value="UI 骨架" />
          </Card>
        </Col>
        <Col xs={24} md={8}>
          <Card>
            <Statistic title="真实模型" value="关闭" />
          </Card>
        </Col>
        <Col xs={24} md={8}>
          <Card>
            <Statistic title="Runner" value="关闭" />
          </Card>
        </Col>
      </Row>

      <Card title="下一批工程任务">
        <Table<TaskRow> columns={columns} dataSource={taskRows} pagination={false} />
      </Card>
    </Space>
  );
}
