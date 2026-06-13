import { Tag } from "antd";

type StatusBadgeProps = {
  status:
    | "queued"
    | "blocked"
    | "running"
    | "completed"
    | "cancelled"
    | "failed"
    | "waiting_user";
};

const statusMeta: Record<StatusBadgeProps["status"], { color: string; label: string }> = {
  queued: { color: "default", label: "排队中" },
  blocked: { color: "red", label: "已阻塞" },
  running: { color: "processing", label: "运行中" },
  completed: { color: "success", label: "已完成" },
  cancelled: { color: "default", label: "已取消" },
  failed: { color: "error", label: "失败" },
  waiting_user: { color: "warning", label: "等待用户" },
};

export function StatusBadge({ status }: StatusBadgeProps) {
  const meta = statusMeta[status];

  return <Tag color={meta.color}>{meta.label}</Tag>;
}
