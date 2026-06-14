import type { LucideIcon } from "lucide-react";
import { Activity, Bot, ClipboardCheck, LayoutDashboard, ListChecks, Network, Settings } from "lucide-react";

export type PageKey =
  | "overview"
  | "projectPlan"
  | "tasks"
  | "agents"
  | "approvals"
  | "settings"
  | "agentRuns";

export type MainNavItem = {
  key: PageKey;
  label: string;
  icon: LucideIcon;
};

export const mainNavItems: MainNavItem[] = [
  { key: "overview", label: "项目总览", icon: LayoutDashboard },
  { key: "projectPlan", label: "项目计划", icon: Network },
  { key: "tasks", label: "任务拆解", icon: ListChecks },
  { key: "agents", label: "Agent 编排", icon: Bot },
  { key: "approvals", label: "审批确认", icon: ClipboardCheck },
  { key: "settings", label: "系统设置", icon: Settings },
  { key: "agentRuns", label: "运行记录", icon: Activity },
];
