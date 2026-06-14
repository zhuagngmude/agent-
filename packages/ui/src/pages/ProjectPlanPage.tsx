import { useCallback, useEffect, useMemo, useState } from "react";
import {
  Alert,
  App as AntdApp,
  Button,
  Card,
  Checkbox,
  Form,
  Input,
  Space,
  Table,
  Tag,
  Typography,
} from "antd";
import type { ColumnsType } from "antd/es/table";
import { Network } from "lucide-react";

import { isProjectPlanApprovalTarget } from "@agent-swarm/agent-core";
import type {
  ApprovalSummary,
  CreateProjectPlanDraftResponse,
  PlannedTaskSummary,
  ProjectPlanDraftSummary,
  RunnerRequestSummary,
} from "@agent-swarm/shared";
import {
  approveProjectPlan,
  createProjectPlanDraft,
  isTauriHost,
  listProjectPlanDrafts,
  listRunnerRequests,
} from "../utils/desktopHost";

type ProjectPlanPageProps = {
  approvals: ApprovalSummary[];
  refreshOverview: () => void;
  canWrite: boolean;
};

type DraftFormValues = {
  idea: string;
  constraints?: string;
};

type ConfirmFormValues = {
  confirmText: string;
  secondConfirm: boolean;
};

type DraftRow = ProjectPlanDraftSummary & {
  key: string;
  approval_status: string;
};

const taskColumns: ColumnsType<PlannedTaskSummary> = [
  { title: "角色", dataIndex: "role", width: 100, render: (role: string) => <Tag>{role}</Tag> },
  { title: "任务", dataIndex: "title" },
  { title: "负责 Agent", dataIndex: "assigned_agent_id", render: (id: string) => <Tag>{id}</Tag> },
  { title: "优先级", dataIndex: "priority", width: 90 },
  { title: "风险", dataIndex: "risk_level", width: 90 },
];

const runnerRequestColumns: ColumnsType<RunnerRequestSummary> = [
  { title: "队列记录", dataIndex: "id" },
  { title: "任务", dataIndex: "task_id" },
  {
    title: "状态",
    dataIndex: "status",
    width: 90,
    render: (status: string) => <Tag>{status}</Tag>,
  },
  {
    title: "操作类型",
    dataIndex: "operation_types",
    render: (operations: string[]) => (
      <Space size={4} wrap>
        {operations.map((operation) => (
          <Tag key={operation}>{operation}</Tag>
        ))}
      </Space>
    ),
  },
];

export function ProjectPlanPage({ approvals, refreshOverview, canWrite }: ProjectPlanPageProps) {
  const { message } = AntdApp.useApp();
  const [draftForm] = Form.useForm<DraftFormValues>();
  const [confirmForm] = Form.useForm<ConfirmFormValues>();
  const [loading, setLoading] = useState(false);
  const [creating, setCreating] = useState(false);
  const [approving, setApproving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [drafts, setDrafts] = useState<ProjectPlanDraftSummary[]>([]);
  const [runnerRequests, setRunnerRequests] = useState<RunnerRequestSummary[]>([]);
  const [latestPreview, setLatestPreview] = useState<CreateProjectPlanDraftResponse | null>(null);
  const [selectedApprovalId, setSelectedApprovalId] = useState<string | null>(null);

  const projectPlanApprovalById = useMemo(() => {
    return new Map(
      approvals
        .filter((approval) => isProjectPlanApprovalTarget(approval.target_service))
        .map((approval) => [approval.id, approval]),
    );
  }, [approvals]);

  const selectedDraft = useMemo(() => {
    if (!selectedApprovalId) return null;
    return drafts.find((draft) => draft.approval_id === selectedApprovalId) ?? null;
  }, [drafts, selectedApprovalId]);

  const selectedApproval = selectedApprovalId
    ? projectPlanApprovalById.get(selectedApprovalId) ?? latestPreview?.approval ?? null
    : latestPreview?.approval ?? null;

  const plannedTasks = latestPreview?.planned_tasks ?? [];
  const plannedRunnerRequests = latestPreview?.planned_runner_requests ?? [];
  const canApproveSelected =
    canWrite &&
    Boolean(selectedApproval) &&
    selectedApproval?.status === "pending" &&
    selectedDraft?.status !== "instantiated";

  const loadData = useCallback(async () => {
    if (!isTauriHost()) {
      setDrafts([]);
      setRunnerRequests([]);
      setLoading(false);
      return;
    }

    setLoading(true);
    try {
      const [nextDrafts, nextRequests] = await Promise.all([
        listProjectPlanDrafts(),
        listRunnerRequests(),
      ]);
      setDrafts(nextDrafts);
      setRunnerRequests(nextRequests);
      setError(null);
      setSelectedApprovalId((current) => current ?? nextDrafts[0]?.approval_id ?? null);
    } catch (err) {
      setError(errorText(err));
      setDrafts([]);
      setRunnerRequests([]);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadData();
  }, [loadData]);

  const handleCreate = useCallback(
    async (values: DraftFormValues) => {
      setCreating(true);
      try {
        const response = await createProjectPlanDraft({
          idea: values.idea,
          constraints: values.constraints ?? null,
          requested_by: "local_user",
        });
        setLatestPreview(response);
        setSelectedApprovalId(response.approval.id);
        message.success("项目计划草案已生成");
        draftForm.resetFields();
        await loadData();
        refreshOverview();
      } catch (err) {
        message.error(errorText(err));
      } finally {
        setCreating(false);
      }
    },
    [draftForm, loadData, message, refreshOverview],
  );

  const handleApprove = useCallback(async () => {
    if (!selectedApprovalId) return;
    const values = await confirmForm.validateFields();
    setApproving(true);
    try {
      await approveProjectPlan({
        approval_id: selectedApprovalId,
        second_confirm: values.secondConfirm,
        confirm_text: values.confirmText,
      });
      message.success("项目计划已批准，任务和只读队列已生成");
      confirmForm.resetFields();
      setLatestPreview(null);
      await loadData();
      refreshOverview();
    } catch (err) {
      message.error(errorText(err));
    } finally {
      setApproving(false);
    }
  }, [confirmForm, loadData, message, refreshOverview, selectedApprovalId]);

  const draftRows: DraftRow[] = drafts.map((draft) => ({
    ...draft,
    key: draft.id,
    approval_status: projectPlanApprovalById.get(draft.approval_id)?.status ?? "unknown",
  }));

  const draftColumns: ColumnsType<DraftRow> = [
    { title: "摘要", dataIndex: "summary" },
    {
      title: "草案状态",
      dataIndex: "status",
      width: 110,
      render: (status: string) => <Tag>{status}</Tag>,
    },
    {
      title: "审批状态",
      dataIndex: "approval_status",
      width: 110,
      render: (status: string) => <Tag color={status === "pending" ? "processing" : "default"}>{status}</Tag>,
    },
    { title: "请求人", dataIndex: "requested_by", width: 120 },
  ];

  return (
    <Space orientation="vertical" size={16} className="page-stack">
      <div className="page-heading">
        <Typography.Title level={2}>
          <Network size={22} style={{ marginRight: 8, verticalAlign: "middle" }} />
          项目计划
        </Typography.Title>
        <Typography.Text type="secondary">
          本地确定性计划草案、二次确认审批和只读 Runner request 队列
        </Typography.Text>
      </div>

      {!canWrite && (
        <Alert
          type="info"
          showIcon
          message="浏览器预览模式只显示结构，生成草案和批准计划需要 Tauri 桌面宿主。"
        />
      )}

      {error && <Alert type="error" showIcon message="读取项目计划失败" description={error} />}

      <Card title="生成项目计划草案">
        <Form<DraftFormValues> form={draftForm} layout="vertical" onFinish={handleCreate}>
          <Form.Item
            name="idea"
            label="项目想法"
            rules={[
              { required: true, message: "请输入项目想法" },
              { max: 500, message: "项目想法不能超过 500 字" },
            ]}
          >
            <Input.TextArea rows={4} maxLength={500} showCount />
          </Form.Item>
          <Form.Item
            name="constraints"
            label="约束"
            rules={[{ max: 2000, message: "约束不能超过 2000 字" }]}
          >
            <Input.TextArea rows={3} maxLength={2000} showCount />
          </Form.Item>
          <Button type="primary" htmlType="submit" loading={creating} disabled={!canWrite}>
            生成草案
          </Button>
        </Form>
      </Card>

      <Card title="计划草案">
        <Table<DraftRow>
          loading={loading}
          columns={draftColumns}
          dataSource={draftRows}
          pagination={false}
          rowKey="approval_id"
          rowSelection={{
            type: "radio",
            selectedRowKeys: selectedApprovalId ? [selectedApprovalId] : [],
            onChange: (_, rows) => setSelectedApprovalId(rows[0]?.approval_id ?? null),
          }}
          locale={{ emptyText: "暂无项目计划草案" }}
          expandable={{
            expandedRowRender: (draft) => (
              <Space orientation="vertical" size={8}>
                <Typography.Text>{draft.idea}</Typography.Text>
                <Typography.Text type="secondary">{draft.constraints ?? "无额外约束"}</Typography.Text>
              </Space>
            ),
          }}
        />
      </Card>

      {latestPreview && (
        <Card title="刚生成的内存预览">
          <Space orientation="vertical" size={12} style={{ width: "100%" }}>
            <Alert
              type="success"
              showIcon
              message="草案已创建，下面是审批前预览。任务和 Runner request 还没有落入对应表。"
            />
            <Table<PlannedTaskSummary>
              columns={taskColumns}
              dataSource={plannedTasks}
              pagination={false}
              rowKey="id"
              size="small"
            />
            <Table<RunnerRequestSummary>
              columns={runnerRequestColumns}
              dataSource={plannedRunnerRequests}
              pagination={false}
              rowKey="id"
              size="small"
            />
          </Space>
        </Card>
      )}

      <Card title="批准生成任务">
        <Space orientation="vertical" size={12} style={{ width: "100%" }}>
          <Alert
            type="warning"
            showIcon
            message="批准后只会创建 5 个 queued 任务和 5 条只读 runner_requests，不会执行 Runner、调用模型、写文件或修改 Git。"
          />
          <Form<ConfirmFormValues> form={confirmForm} layout="vertical">
            <Form.Item
              name="secondConfirm"
              valuePropName="checked"
              rules={[{ validator: (_, value) => (value ? Promise.resolve() : Promise.reject(new Error("请勾选二次确认"))) }]}
            >
              <Checkbox>我确认只生成任务和只读队列，不执行 Runner</Checkbox>
            </Form.Item>
            <Form.Item
              name="confirmText"
              label="确认文本"
              rules={[
                { required: true, message: "请输入确认文本" },
                {
                  validator: (_, value: string | undefined) =>
                    value?.includes("生成任务")
                      ? Promise.resolve()
                      : Promise.reject(new Error("确认文本必须包含“生成任务”")),
                },
              ]}
            >
              <Input placeholder="请输入：确认生成任务" />
            </Form.Item>
            <Button
              type="primary"
              danger
              loading={approving}
              disabled={!canApproveSelected}
              onClick={handleApprove}
            >
              批准生成任务
            </Button>
          </Form>
        </Space>
      </Card>

      <Card title="只读 Runner request 队列">
        <Table<RunnerRequestSummary>
          loading={loading}
          columns={runnerRequestColumns}
          dataSource={runnerRequests}
          pagination={false}
          rowKey="id"
          locale={{ emptyText: "暂无只读 Runner request" }}
          expandable={{
            expandedRowRender: (request) => (
              <Typography.Text type="secondary">{request.safety_note}</Typography.Text>
            ),
          }}
        />
      </Card>
    </Space>
  );
}

function errorText(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
