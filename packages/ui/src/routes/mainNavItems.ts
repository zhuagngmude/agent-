import type { LucideIcon } from "lucide-react";
import { Activity, Bot, ClipboardCheck, Compass, Lightbulb, Network, Settings, Workflow } from "lucide-react";

export type PageKey =
  | "overview"
  | "workflow"
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
  { key: "overview", label: "主控台", icon: Compass },
  { key: "workflow", label: "蜂群工作流", icon: Workflow },
  { key: "projectPlan", label: "项目计划", icon: Network },
  { key: "tasks", label: "任务拆解", icon: Lightbulb },
  { key: "agents", label: "智能体队列", icon: Bot },
  { key: "approvals", label: "审批闸门", icon: ClipboardCheck },
  { key: "agentRuns", label: "运行记录", icon: Activity },
  { key: "settings", label: "系统设置", icon: Settings },
];
