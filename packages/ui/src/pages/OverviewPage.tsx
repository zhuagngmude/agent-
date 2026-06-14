import { useCallback, useState } from "react";
import { Alert, App as AntdApp, Button, Card, Col, Popconfirm, Row, Space, Statistic, Table, Tag, Typography } from "antd";
import type { ColumnsType } from "antd/es/table";

import { CreateTaskModal } from "../components/CreateTaskModal";
import { StatusBadge } from "../components/StatusBadge";
import { zhText } from "../i18n/zh";
import type { AgentSummary, ApprovalSummary, TaskStatus, TaskSummary } from "../utils/desktopHost";
import {
  approveApproval,
  isTauriHost,
  patchOnlyApproval,
  rejectApproval,
  updateTaskStatus,
  useDesktopHostOverview,
} from "../utils/desktopHost";

// ---------------------------------------------------------------------------
// 行数据类型
// ---------------------------------------------------------------------------

type TaskRow = {
  key: string;
  id: string;
  title: string;
  owner: string;
  status: TaskStatus;
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
  id: string;
  status: string;
  service: string;
  risk: string;
  operations: string[];
};

// ---------------------------------------------------------------------------
// 静态列定义
// ---------------------------------------------------------------------------

const taskColumns: ColumnsType<TaskRow> = [
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

const agentColumns: ColumnsType<AgentRow> = [
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

const approvalColumns: ColumnsType<ApprovalRow> = [
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

// ---------------------------------------------------------------------------
// 状态转换映射
// ---------------------------------------------------------------------------

const statusTransitions: Record<string, Array<{ status: TaskStatus; label: string; danger?: boolean }>> = {
  queued: [
    { status: "running", label: "开始" },
    { status: "cancelled", label: "取消", danger: true },
  ],
  running: [
    { status: "completed", label: "完成" },
    { status: "blocked", label: "阻塞" },
    { status: "waiting_user", label: "等待用户" },
    { status: "failed", label: "失败", danger: true },
    { status: "cancelled", label: "取消", danger: true },
  ],
  blocked: [{ status: "running", label: "恢复" }],
  waiting_user: [
    { status: "running", label: "恢复" },
    { status: "cancelled", label: "取消", danger: true },
  ],
};

// ---------------------------------------------------------------------------
// 错误处理
// ---------------------------------------------------------------------------

type MessageApi = ReturnType<typeof AntdApp.useApp>["message"];

function showError(messageApi: MessageApi, error: unknown): void {
  const text = error instanceof Error ? error.message : String(error);
  messageApi.error(text);
}

// ---------------------------------------------------------------------------
// OverviewPage
// ---------------------------------------------------------------------------

export function OverviewPage() {
  const { message } = AntdApp.useApp();
  const desktopHost = useDesktopHostOverview();
  const isLoading = desktopHost.status === "loading";
  const data = desktopHost.status === "loading" ? null : desktopHost;
  const taskRows = data ? toTaskRows(data.tasks, data.agents) : [];
  const agentRows = data ? toAgentRows(data.agents) : [];
  const approvalRows = data ? toApprovalRows(data.approvals) : [];
  const pendingApprovalCount = approvalRows.filter((row) => row.status === "pending").length;
  const [createModalOpen, setCreateModalOpen] = useState(false);
  const showWriteUI = isTauriHost() && desktopHost.status === "connected";

  const handleStatusChange = useCallback(
    async (taskId: string, newStatus: TaskStatus) => {
      try {
        await updateTaskStatus({ id: taskId, status: newStatus });
        message.success("任务状态已更新");
        desktopHost.refresh();
      } catch (error) {
        showError(message, error);
      }
    },
    [desktopHost.refresh, message],
  );

  const handleApprove = useCallback(
    async (approvalId: string) => {
      try {
        await approveApproval(approvalId);
        message.success("审批已通过");
        desktopHost.refresh();
      } catch (error) {
        showError(message, error);
      }
    },
    [desktopHost.refresh, message],
  );

  const handleReject = useCallback(
    async (approvalId: string) => {
      try {
        await rejectApproval(approvalId);
        message.success("审批已拒绝");
        desktopHost.refresh();
      } catch (error) {
        showError(message, error);
      }
    },
    [desktopHost.refresh, message],
  );

  const handlePatchOnly = useCallback(
    async (approvalId: string) => {
      try {
        await patchOnlyApproval(approvalId);
        message.success("已标记为仅补丁");
        desktopHost.refresh();
      } catch (error) {
        showError(message, error);
      }
    },
    [desktopHost.refresh, message],
  );

  // 动态操作列（需要访问组件内的 handler）
  const taskActionColumn: ColumnsType<TaskRow> = showWriteUI
    ? [
        {
          title: "操作",
          key: "actions",
          width: 240,
          render: (_: unknown, row: TaskRow) => {
            const transitions = statusTransitions[row.status];
            if (!transitions || transitions.length === 0) return null;

            return (
              <Space size={4} wrap>
                {transitions.map((t) => (
                  <Popconfirm
                    key={t.status}
                    title={`确定将任务状态改为「${t.label}」？`}
                    onConfirm={() => handleStatusChange(row.id, t.status)}
                  >
                    <Button size="small" danger={t.danger}>
                      {t.label}
                    </Button>
                  </Popconfirm>
                ))}
              </Space>
            );
          },
        },
      ]
    : [];

  const approvalActionColumn: ColumnsType<ApprovalRow> = showWriteUI
    ? [
        {
          title: "操作",
          key: "actions",
          width: 220,
          render: (_: unknown, row: ApprovalRow) => {
            if (row.status !== "pending") return null;

            return (
              <Space size={4} wrap>
                <Popconfirm title="确定通过此审批？" onConfirm={() => handleApprove(row.id)}>
                  <Button size="small" type="primary">
                    通过
                  </Button>
                </Popconfirm>
                <Popconfirm title="确定拒绝此审批？" onConfirm={() => handleReject(row.id)}>
                  <Button size="small" danger>
                    拒绝
                  </Button>
                </Popconfirm>
                <Popconfirm title="确定标记为仅补丁？" onConfirm={() => handlePatchOnly(row.id)}>
                  <Button size="small">仅补丁</Button>
                </Popconfirm>
              </Space>
            );
          },
        },
      ]
    : [];

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
            <Statistic title="待审" value={pendingApprovalCount} loading={isLoading} />
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
            <Typography.Text type="secondary">
              当前未运行在 Tauri 桌面宿主内，正在使用 fallback 数据
            </Typography.Text>
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

      <Card
        title="任务拆解"
        extra={
          <Button
            type="primary"
            disabled={!showWriteUI}
            title={showWriteUI ? undefined : "浏览器预览模式不支持写入操作"}
            onClick={() => setCreateModalOpen(true)}
          >
            + 新建任务
          </Button>
        }
      >
        <Table<TaskRow>
          columns={[...taskColumns, ...taskActionColumn]}
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
              columns={[...approvalColumns, ...approvalActionColumn]}
              dataSource={approvalRows}
              loading={isLoading}
              pagination={false}
              locale={{ emptyText: "暂无审批" }}
            />
          </Card>
        </Col>
      </Row>

      <CreateTaskModal
        open={createModalOpen}
        agents={"agents" in desktopHost ? (desktopHost as { agents: AgentSummary[] }).agents : []}
        onClose={() => setCreateModalOpen(false)}
        onCreated={() => {
          setCreateModalOpen(false);
          desktopHost.refresh();
        }}
      />
    </Space>
  );
}

// ---------------------------------------------------------------------------
// 行数据转换
// ---------------------------------------------------------------------------

function toTaskRows(tasks: TaskSummary[], agents: AgentSummary[]): TaskRow[] {
  const agentNameById = new Map(agents.map((agent) => [agent.id, agent.name]));

  return tasks.map((task) => ({
    key: task.id,
    id: task.id,
    title: task.title,
    owner: task.assigned_agent_id
      ? (agentNameById.get(task.assigned_agent_id) ?? task.assigned_agent_id)
      : "未分配",
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
    id: approval.id,
    status: approval.status,
    service: approval.target_service,
    risk: approval.risk_level,
    operations: approval.operation_types,
  }));
}
