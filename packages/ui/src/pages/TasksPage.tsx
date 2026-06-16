import { useCallback, useState } from "react";
import { App as AntdApp, Button, Card, Popconfirm, Space, Table, Tag, Typography } from "antd";
import type { ColumnsType } from "antd/es/table";

import { taskStatusTransitions } from "@agent-swarm/agent-core";
import type { AgentSummary, TaskStatus, TaskSummary } from "@agent-swarm/shared";
import { CreateTaskModal } from "../components/CreateTaskModal";
import { StatusBadge } from "../components/StatusBadge";
import { updateTaskStatus } from "../utils/desktopHost";
import { priorityLabel, priorityColor, agentNameLabel } from "../utils/labels";
import { userErrorLabel } from "../utils/userError";

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
// 状态转换展示元数据（中文标签、危险样式），转换规则在 @agent-swarm/agent-core
// ---------------------------------------------------------------------------

const transitionMeta: Record<string, { label: string; danger?: boolean }> = {
  "queued->running": { label: "开始" },
  "queued->cancelled": { label: "取消", danger: true },
  "running->completed": { label: "完成" },
  "running->blocked": { label: "阻塞" },
  "running->waiting_user": { label: "等待用户" },
  "running->failed": { label: "失败", danger: true },
  "running->cancelled": { label: "取消", danger: true },
  "blocked->running": { label: "恢复" },
  "waiting_user->running": { label: "恢复" },
  "waiting_user->cancelled": { label: "取消", danger: true },
};

// ---------------------------------------------------------------------------
// 列定义
// ---------------------------------------------------------------------------

const taskColumns: ColumnsType<TaskRow> = [
  { title: "任务", dataIndex: "title" },
  {
    title: "负责智能体",
    dataIndex: "owner",
    render: (owner: string) => <Tag>{agentNameLabel(owner)}</Tag>,
  },
  {
    title: "优先级",
    dataIndex: "priority",
    render: (priority: string) => <Tag color={priorityColor(priority)}>{priorityLabel(priority)}</Tag>,
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
  messageApi.error(userErrorLabel(error));
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
            const nextStatuses = taskStatusTransitions[row.status];
            if (!nextStatuses || nextStatuses.length === 0) return null;

            return (
              <Space size={4} wrap>
                {nextStatuses.map((nextStatus) => {
                  const meta = transitionMeta[`${row.status}->${nextStatus}`];
                  if (!meta) return null;
                  return (
                    <Popconfirm
                      key={nextStatus}
                      title={`确定将任务状态改为「${meta.label}」？`}
                      onConfirm={() => handleStatusChange(row.id, nextStatus)}
                    >
                      <Button size="small" danger={meta.danger}>
                        {meta.label}
                      </Button>
                    </Popconfirm>
                  );
                })}
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
