import { Tag } from "antd";

type StatusBadgeProps = {
  status: "queued" | "review" | "blocked";
};

const statusMeta: Record<StatusBadgeProps["status"], { color: string; label: string }> = {
  queued: { color: "default", label: "排队中" },
  review: { color: "blue", label: "待审查" },
  blocked: { color: "red", label: "已阻塞" },
};

export function StatusBadge({ status }: StatusBadgeProps) {
  const meta = statusMeta[status];

  return <Tag color={meta.color}>{meta.label}</Tag>;
}
