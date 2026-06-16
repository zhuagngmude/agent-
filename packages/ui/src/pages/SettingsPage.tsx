import { Alert, Card, Descriptions, Space, Typography } from "antd";

import type { ProjectSummary } from "@agent-swarm/shared";
import { statusLabel } from "../utils/labels";

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
        <Typography.Text type="secondary">项目信息与能力边界</Typography.Text>
      </div>

      <Card title="项目信息">
        <Descriptions column={1} bordered>
          <Descriptions.Item label="项目名称">{project.name}</Descriptions.Item>
          <Descriptions.Item label="项目状态">{statusLabel(project.status)}</Descriptions.Item>
          <Descriptions.Item label="当前阶段">{project.phase}</Descriptions.Item>
        </Descriptions>
      </Card>

      <Card title="真实模型">
        <Alert
          type="info"
          showIcon
          title="真实模型已通过模型网关和受控模型目录开放"
          description="前端不接收密钥、接口地址、提示词、请求头等敏感配置。模型选择从后端受控模型目录获取，调用经模型网关和脱敏审计链路。"
        />
      </Card>

      <Card title="执行引擎">
        <Alert
          type="info"
          showIcon
          title="执行引擎仅在阶段 34 边界内开放"
          description="仅允许临时沙箱路径、固定版本管理只读命令和人工二次确认。不开放批量/自动执行、自由命令、版本提交/推送、文件删除、网络请求。"
        />
      </Card>
    </Space>
  );
}
