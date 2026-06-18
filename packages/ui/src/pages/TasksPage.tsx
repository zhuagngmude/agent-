import { useCallback, useMemo, useState } from "react";
import { App as AntdApp, Button, Popconfirm, Progress } from "antd";
import { Bot, FolderOpen, Play, RefreshCw, Trash2 } from "lucide-react";

import type { AgentSummary, TaskStatus, TaskSummary } from "@agent-swarm/shared";
import { StatusBadge } from "../components/StatusBadge";
import { continueSwarmTasks, deleteTasks, openTaskOutputFolder } from "../utils/desktopHost";
import { agentNameLabel, priorityColor, priorityLabel } from "../utils/labels";
import { userErrorLabel } from "../utils/userError";

type TaskRow = {
  key: string;
  id: string;
  title: string;
  description: string | null;
  owner: string;
  status: TaskStatus;
  priority: string;
  children?: TaskRow[];
  childCount?: number;
  completedCount?: number;
  deleteTaskIds?: string[];
  continuableTaskIds?: string[];
  outputTaskIds?: string[];
};

type MessageApi = ReturnType<typeof AntdApp.useApp>["message"];

const continuableStatuses: TaskStatus[] = ["queued", "running", "blocked", "waiting_user", "failed"];

function showError(messageApi: MessageApi, error: unknown): void {
  messageApi.error(userErrorLabel(error));
}

type TasksPageProps = {
  tasks: TaskSummary[];
  agents: AgentSummary[];
  refresh: () => void;
  canWrite: boolean;
};

export function TasksPage({ tasks, agents, refresh, canWrite }: TasksPageProps) {
  const { message } = AntdApp.useApp();
  const [deletingKey, setDeletingKey] = useState<string | null>(null);
  const [continuingKey, setContinuingKey] = useState<string | null>(null);
  const [openingKey, setOpeningKey] = useState<string | null>(null);

  const taskRows = useMemo(() => toTaskRows(tasks, agents), [tasks, agents]);

  const totalTasks = taskRows.reduce((sum, row) => sum + (row.childCount ?? 1), 0);
  const completedTasks = taskRows.reduce((sum, row) => sum + (row.completedCount ?? (row.status === "completed" ? 1 : 0)), 0);
  const runningAgents = new Set(
    taskRows
      .flatMap((row) => row.children ?? [row])
      .filter((row) => row.status === "running" || row.status === "waiting_user")
      .map((row) => row.owner),
  );
  const pendingCount = taskRows
    .flatMap((row) => row.children ?? [row])
    .filter((row) => row.status === "waiting_user" || row.status === "blocked")
    .length;
  const progress = totalTasks > 0 ? Math.round((completedTasks / totalTasks) * 100) : 0;

  const handleDeleteTasks = useCallback(
    async (row: TaskRow) => {
      const taskIds = row.deleteTaskIds ?? row.children?.map((child) => child.id) ?? [row.id];
      setDeletingKey(row.key);
      try {
        await deleteTasks({ task_ids: taskIds });
        message.success(row.children ? "任务组和产物文件夹已删除" : "任务和产物文件夹已删除");
        refresh();
      } catch (error) {
        showError(message, error);
      } finally {
        setDeletingKey(null);
      }
    },
    [refresh, message],
  );

  const handleContinueTasks = useCallback(
    async (row: TaskRow) => {
      const taskIds = row.continuableTaskIds ?? (continuableStatuses.includes(row.status) ? [row.id] : []);
      if (taskIds.length === 0) {
        message.info("这个任务已经没有需要继续的子任务");
        return;
      }

      setContinuingKey(row.key);
      try {
        const response = await continueSwarmTasks({
          task_ids: taskIds,
          requested_by: "tasks_page_continue",
        });
        const succeededCount = response.task_results.filter((item) => item.status === "succeeded").length;
        message.success(
          response.status === "succeeded"
            ? `已继续完成 ${succeededCount} 个任务`
            : `已继续处理 ${response.task_results.length} 个任务，部分还需要检查`,
        );
        refresh();
      } catch (error) {
        showError(message, error);
      } finally {
        setContinuingKey(null);
      }
    },
    [refresh, message],
  );

  const handleOpenOutputFolder = useCallback(
    async (row: Pick<TaskRow, "key" | "id" | "deleteTaskIds" | "outputTaskIds">) => {
      const taskIds = row.outputTaskIds ?? row.deleteTaskIds ?? [row.id];
      setOpeningKey(row.key);
      try {
        const response = await openTaskOutputFolder({ task_ids: taskIds });
        message.success(`已打开：${response.path}`);
      } catch (error) {
        showError(message, error);
      } finally {
        setOpeningKey(null);
      }
    },
    [message],
  );

  return (
    <div className="task-workbench">
      <header className="task-workbench__heading">
        <div>
          <span>任务拆解</span>
          <h1>项目任务与 AI 智能体执行状态</h1>
          <p>一个项目任务下面展开分配出去的 AI 智能体，直接看谁在工作、做到哪一步、产物放在哪。</p>
        </div>
        <div className="task-workbench__actions">
          <Button icon={<RefreshCw size={14} />} onClick={refresh}>
            刷新
          </Button>
          <Button type="primary" disabled={!canWrite}>
            生成任务
          </Button>
        </div>
      </header>

      <section className="task-overview-grid" aria-label="任务执行总览">
        <article>
          <span>输出目录</span>
          <strong>workspace/generated</strong>
          <p>所有项目产物按任务归档。</p>
        </article>
        <article>
          <span>任务进度</span>
          <strong>{progress}%</strong>
          <Progress percent={progress} size="small" />
        </article>
        <article>
          <span>运行智能体</span>
          <strong>{runningAgents.size}</strong>
          <p>{runningAgents.size > 0 ? Array.from(runningAgents).map(agentNameLabel).join("、") : "暂无运行中智能体"}</p>
        </article>
        <article>
          <span>等待确认</span>
          <strong>{pendingCount}</strong>
          <p>阻塞和等你确认的任务会在这里计数。</p>
        </article>
      </section>

      <section className="task-workbench__grid">
        <div className="task-board">
          <div className="task-section-heading">
            <div>
              <span>执行看板</span>
              <h2>智能体任务状态</h2>
            </div>
            <strong>{taskRows.length} 个任务组</strong>
          </div>

          <div className="task-card-list">
            {taskRows.length === 0 ? (
              <div className="task-empty-state">暂无任务。可以先从主控台生成基础工作流，再让总控拆解任务。</div>
            ) : (
              taskRows.map((row) => (
                <article className="task-execution-card" key={row.key}>
                  <div className="task-execution-card__top">
                    <div className="task-execution-card__icon">
                      <Bot size={17} aria-hidden="true" />
                    </div>
                    <div>
                      <span>项目任务</span>
                      <h3>{row.title}</h3>
                      <p>{agentNameLabel(row.owner)}</p>
                    </div>
                    <StatusBadge status={row.status} />
                  </div>

                  <div className="task-execution-card__progress">
                    <Progress
                      percent={Math.round(((row.completedCount ?? 0) / Math.max(row.childCount ?? 1, 1)) * 100)}
                      size="small"
                    />
                    <span>{row.completedCount ?? 0}/{row.childCount ?? 1} 子任务完成</span>
                  </div>

                  <div className="task-execution-card__meta">
                    <span className={`color-${priorityColor(row.priority)}`}>优先级：{priorityLabel(row.priority)}</span>
                    <span>产物：{row.outputTaskIds?.length ?? 1} 个</span>
                    <span>路径：{outputFolderFor(row.key)}</span>
                  </div>

                  <div className="task-child-heading">
                    <span>分配的 AI 员工</span>
                    <strong>{row.childCount ?? 1} 个员工/任务</strong>
                  </div>
                  <div className="task-child-list">
                    {(row.children ?? [row]).map((child) => (
                      <div className="task-assignee-row" key={child.id}>
                        <div className="task-assignee-row__avatar">
                          <Bot size={15} aria-hidden="true" />
                        </div>
                        <div className="task-assignee-row__agent">
                          <strong>{agentNameLabel(child.owner)}</strong>
                          <span>{child.owner === "未分配" ? "等待总控分配" : "已分配 AI 员工"}</span>
                        </div>
                        <div className="task-assignee-row__work">
                          <span>负责任务</span>
                          <strong>{child.title}</strong>
                        </div>
                        <StatusBadge status={child.status} />
                      </div>
                    ))}
                  </div>

                  <div className="task-execution-card__actions">
                    {(row.continuableTaskIds?.length ?? 0) > 0 ? (
                      <Button
                        icon={<Play size={14} />}
                        size="small"
                        type="primary"
                        loading={continuingKey === row.key}
                        disabled={!canWrite || continuingKey !== null || deletingKey !== null}
                        onClick={() => handleContinueTasks(row)}
                      >
                        继续做
                      </Button>
                    ) : null}
                    <Button
                      icon={<FolderOpen size={14} />}
                      size="small"
                      loading={openingKey === row.key}
                      disabled={openingKey !== null}
                      onClick={() => handleOpenOutputFolder(row)}
                    >
                      打开文件夹
                    </Button>
                    {canWrite ? (
                      <Popconfirm
                        title={`确定删除「${row.title}」及其任务记录？`}
                        okText="删除"
                        cancelText="取消"
                        okButtonProps={{ danger: true }}
                        onConfirm={() => handleDeleteTasks(row)}
                      >
                        <Button icon={<Trash2 size={14} />} size="small" danger loading={deletingKey === row.key}>
                          删除
                        </Button>
                      </Popconfirm>
                    ) : null}
                  </div>
                </article>
              ))
            )}
          </div>
        </div>
      </section>
    </div>
  );
}

function toTaskRows(tasks: TaskSummary[], agents: AgentSummary[]): TaskRow[] {
  const agentNameById = new Map(agents.map((agent) => [agent.id, agent.name]));

  const childRows = tasks.map((task) => ({
    key: task.id,
    id: task.id,
    title: task.title,
    description: task.description,
    owner: task.assigned_agent_id
      ? (agentNameById.get(task.assigned_agent_id) ?? task.assigned_agent_id)
      : "未分配",
    status: task.status,
    priority: task.priority,
  }));

  const groups = new Map<string, TaskRow[]>();
  for (const row of childRows) {
    const groupKey = planGroupKey(row.id);
    const rows = groups.get(groupKey) ?? [];
    rows.push(row);
    groups.set(groupKey, rows);
  }

  return Array.from(groups.entries()).map(([groupKey, rows]) => {
    const latestIndex = latestRunIndex(rows);
    const latestRows =
      latestIndex === null ? rows : rows.filter((row) => extractRunIndex(row.id) === latestIndex);
    const completedCount = latestRows.filter((row) => row.status === "completed").length;
    const failedCount = latestRows.filter((row) => row.status === "failed").length;
    const runningCount = latestRows.filter((row) => row.status === "running").length;
    const queuedCount = latestRows.filter((row) => row.status === "queued").length;
    const status: TaskStatus =
      failedCount > 0
        ? "failed"
        : runningCount > 0
          ? "running"
          : queuedCount > 0
            ? "queued"
            : completedCount === latestRows.length
              ? "completed"
              : "blocked";
    const priority = highestPriority(latestRows.map((row) => row.priority));
    const owners = Array.from(new Set(latestRows.map((row) => row.owner)));

    return {
      key: groupKey,
      id: groupKey,
      title: groupTitle(groupKey, latestRows),
      description: null,
      owner: owners.length > 1 ? `多智能体协作（${owners.length} 个）` : owners[0],
      status,
      priority,
      childCount: latestRows.length,
      completedCount,
      children: latestRows,
      deleteTaskIds: rows.map((row) => row.id),
      outputTaskIds: latestRows.map((row) => row.id),
      continuableTaskIds: rows
        .filter((row) => continuableStatuses.includes(row.status))
        .map((row) => row.id),
    };
  });
}

function outputFolderFor(key: string): string {
  return `workspace/generated/${key.replace(/[^a-zA-Z0-9_-]/g, "_")}`;
}

function planGroupKey(taskId: string): string {
  const match = taskId.match(/^task_(project_plan_.+?)(?:_\d+|_[a-z]+)$/);
  return match?.[1] ?? taskId;
}

function extractRunIndex(taskId: string): number | null {
  const match = taskId.match(/_run_(\d+)_/);
  return match ? Number(match[1]) : null;
}

function latestRunIndex(rows: TaskRow[]): number | null {
  const indexes = rows.map((row) => extractRunIndex(row.id)).filter((value): value is number => value !== null);
  return indexes.length > 0 ? Math.max(...indexes) : null;
}

function groupTitle(groupKey: string, rows: TaskRow[]): string {
  const idea = rows.map((row) => extractIdea(row.description)).find(Boolean);
  if (idea) return normalizeTaskTitle(idea);

  const fromPlanId = readablePlanName(groupKey);
  if (fromPlanId) return normalizeTaskTitle(fromPlanId);

  const first = rows[0]?.title ?? "项目任务";
  if (rows.length <= 1) return first;
  const compact = first
    .replace(/^(前端|后端|测试|文档|审查|安全|UX|DevOps|数据|验收|风险)(页面|接口|功能|检查|实现|切片)?[:：\s-]*/i, "")
    .trim();
  return normalizeTaskTitle(compact || `${first} 等 ${rows.length} 项`);
}

function extractIdea(description: string | null): string | null {
  if (!description) return null;
  const match = description.match(/项目想法：([^\n\r]+)/);
  const idea = match?.[1]?.trim();
  return idea || null;
}

function readablePlanName(groupKey: string): string | null {
  const prefix = "project_plan_";
  if (!groupKey.startsWith(prefix)) return null;

  const raw = groupKey.slice(prefix.length).replace(/^run_\d+_/, "");
  const words = raw
    .split("_")
    .filter(Boolean)
    .filter((word) => !/^\d+$/.test(word));

  if (words.length === 0) return null;

  return words.join(" ");
}

function normalizeTaskTitle(value: string): string {
  const title = value.trim();
  if (!title) return "项目任务";
  return title.length > 60 ? `${title.slice(0, 60)}...` : title;
}

function highestPriority(priorities: string[]): string {
  if (priorities.includes("high")) return "high";
  if (priorities.includes("medium")) return "medium";
  return priorities[0] ?? "low";
}
