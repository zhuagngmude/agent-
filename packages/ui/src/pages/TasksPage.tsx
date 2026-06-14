import { useCallback, useState } from "react";
import { App as AntdApp, Button, Card, Popconfirm, Space, Table, Tag, Typography } from "antd";
import type { ColumnsType } from "antd/es/table";

import { CreateTaskModal } from "../components/CreateTaskModal";
import { StatusBadge } from "../components/StatusBadge";
import type { AgentSummary, TaskStatus, TaskSummary } from "../utils/desktopHost";
import { updateTaskStatus } from "../utils/desktopHost";

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
// 列定义
// ---------------------------------------------------------------------------

const taskColumns: ColumnsType<TaskRow> = [
  { title: "任务", dataIndex: "title" },
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

// ---------------------------------------------------------------------------
// 错误处理
// ---------------------------------------------------------------------------

type MessageApi = ReturnType<typeof AntdApp.useApp>["message"];

function showError(messageApi: MessageApi, error: unknown): void {
  const text = error instanceof Error ? error.message : String(error);
  messageApi.error(text);
}

// ---------------------------------------------------------------------------
// Props
// ---------------------------------------------------------------------------

type TasksPageProps = {
  tasks: TaskSummary[];
  agents: AgentSummary[];
  refresh: () => void;
  canWrite: boolean;
};

// ---------------------------------------------------------------------------
// TasksPage
// ---------------------------------------------------------------------------

export function TasksPage({ tasks, agents, refresh, canWrite }: TasksPageProps) {
  const { message } = AntdApp.useApp();
  const [createModalOpen, setCreateModalOpen] = useState(false);

  const taskRows = toTaskRows(tasks, agents);

  const handleStatusChange = useCallback(
    async (taskId: string, newStatus: TaskStatus) => {
      try {
        await updateTaskStatus({ id: taskId, status: newStatus });
        message.success("任务状态已更新");
        refresh();
      } catch (error) {
        showError(message, error);
      }
    },
    [refresh, message],
  );

  const taskActionColumn: ColumnsType<TaskRow> = canWrite
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

  return (
    <Space orientation="vertical" size={16} className="page-stack">
      <div className="page-heading">
        <Typography.Title level={2}>任务拆解</Typography.Title>
        <Typography.Text type="secondary">任务列表、新建与状态操作</Typography.Text>
      </div>

      <Card
        title="任务拆解"
        extra={
          <Button
            type="primary"
            disabled={!canWrite}
            title={canWrite ? undefined : "浏览器预览模式不支持写入操作"}
            onClick={() => setCreateModalOpen(true)}
          >
            + 新建任务
          </Button>
        }
      >
        <Table<TaskRow>
          columns={[...taskColumns, ...taskActionColumn]}
          dataSource={taskRows}
          pagination={false}
          locale={{ emptyText: "暂无任务" }}
        />
      </Card>

      <CreateTaskModal
        open={createModalOpen}
        agents={agents}
        onClose={() => setCreateModalOpen(false)}
        onCreated={() => {
          setCreateModalOpen(false);
          refresh();
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
