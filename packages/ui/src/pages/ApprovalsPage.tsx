import { useCallback } from "react";
import { App as AntdApp, Button, Card, Popconfirm, Space, Table, Tag, Typography } from "antd";
import type { ColumnsType } from "antd/es/table";

import type { ApprovalSummary } from "@agent-swarm/shared";
import { approveApproval, patchOnlyApproval, rejectApproval } from "../utils/desktopHost";

// ---------------------------------------------------------------------------
// 行数据类型
// ---------------------------------------------------------------------------

type ApprovalRow = {
  key: string;
  id: string;
  status: string;
  service: string;
  risk: string;
  operations: string[];
};

// ---------------------------------------------------------------------------
// 列定义
// ---------------------------------------------------------------------------

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

type ApprovalsPageProps = {
  approvals: ApprovalSummary[];
  refresh: () => void;
  canWrite: boolean;
};

// ---------------------------------------------------------------------------
// ApprovalsPage
// ---------------------------------------------------------------------------

export function ApprovalsPage({ approvals, refresh, canWrite }: ApprovalsPageProps) {
  const { message } = AntdApp.useApp();
  const approvalRows = toApprovalRows(approvals);

  const handleApprove = useCallback(
    async (approvalId: string) => {
      try {
        await approveApproval(approvalId);
        message.success("审批已通过");
        refresh();
      } catch (error) {
        showError(message, error);
      }
    },
    [refresh, message],
  );

  const handleReject = useCallback(
    async (approvalId: string) => {
      try {
        await rejectApproval(approvalId);
        message.success("审批已拒绝");
        refresh();
      } catch (error) {
        showError(message, error);
      }
    },
    [refresh, message],
  );

  const handlePatchOnly = useCallback(
    async (approvalId: string) => {
      try {
        await patchOnlyApproval(approvalId);
        message.success("已标记为仅补丁");
        refresh();
      } catch (error) {
        showError(message, error);
      }
    },
    [refresh, message],
  );

  const approvalActionColumn: ColumnsType<ApprovalRow> = canWrite
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
        <Typography.Title level={2}>审批确认</Typography.Title>
        <Typography.Text type="secondary">SQLite 审批记录与终态操作</Typography.Text>
      </div>

      <Card title="审批确认">
        <Table<ApprovalRow>
          columns={[...approvalColumns, ...approvalActionColumn]}
          dataSource={approvalRows}
          pagination={false}
          locale={{ emptyText: "暂无审批" }}
        />
      </Card>
    </Space>
  );
}

// ---------------------------------------------------------------------------
// 行数据转换
// ---------------------------------------------------------------------------

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
