import { Alert, Card, Descriptions, Space, Typography } from "antd";

import type { ProjectSummary } from "@agent-swarm/shared";

// ---------------------------------------------------------------------------
// Props
// ---------------------------------------------------------------------------

type SettingsPageProps = {
  project: ProjectSummary;
};

// ---------------------------------------------------------------------------
// SettingsPage
// ---------------------------------------------------------------------------

export function SettingsPage({ project }: SettingsPageProps) {
  return (
    <Space orientation="vertical" size={16} className="page-stack">
      <div className="page-heading">
        <Typography.Title level={2}>系统设置</Typography.Title>
        <Typography.Text type="secondary">项目信息只读展示</Typography.Text>
      </div>

      <Card title="系统设置">
        <Descriptions column={1} bordered>
          <Descriptions.Item label="项目名称">{project.name}</Descriptions.Item>
          <Descriptions.Item label="项目状态">{project.status}</Descriptions.Item>
          <Descriptions.Item label="当前阶段">{project.phase}</Descriptions.Item>
        </Descriptions>
        <Alert
          type="info"
          showIcon
          message="真实 provider、raw key、Model Gateway 实现当前不开放，后续必须另走真实模型准入。"
          style={{ marginTop: 16 }}
        />
      </Card>
    </Space>
  );
}
