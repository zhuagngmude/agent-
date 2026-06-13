import {
  Bot,
  ClipboardCheck,
  Database,
  GitPullRequest,
  LayoutDashboard,
  ListChecks,
  Settings,
} from "lucide-react";
import type { LucideIcon } from "lucide-react";

type MainNavItem = {
  key: string;
  label: string;
  icon: LucideIcon;
  active?: boolean;
};

export const mainNavItems: MainNavItem[] = [
  { key: "overview", label: "项目总览", icon: LayoutDashboard, active: true },
  { key: "agents", label: "Agent 编排", icon: Bot },
  { key: "tasks", label: "任务拆解", icon: ListChecks },
  { key: "approvals", label: "Runner 审批", icon: ClipboardCheck },
  { key: "git", label: "Git 保存点", icon: GitPullRequest },
  { key: "knowledge", label: "知识库", icon: Database },
  { key: "settings", label: "系统设置", icon: Settings },
];
