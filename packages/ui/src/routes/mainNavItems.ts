import type { LucideIcon } from "lucide-react";
import { Activity, Bot, ClipboardList, Compass, GitBranch, Lightbulb, Settings, ShieldCheck } from "lucide-react";

export type PageKey =
  | "overview"
  | "tasks"
  | "projectPlan"
  | "workflow"
  | "runs"
  | "agents"
  | "approvals"
  | "settings";

export type MainNavItem = {
  key: PageKey;
  label: string;
  icon: LucideIcon;
};

export const mainNavItems: MainNavItem[] = [
  { key: "overview", label: "主控台", icon: Compass },
  { key: "tasks", label: "任务拆解", icon: Lightbulb },
  { key: "projectPlan", label: "项目计划", icon: ClipboardList },
  { key: "workflow", label: "流程蓝图", icon: GitBranch },
  { key: "runs", label: "运行输出", icon: Activity },
  { key: "agents", label: "AI 员工", icon: Bot },
  { key: "approvals", label: "审批与安全", icon: ShieldCheck },
  { key: "settings", label: "系统设置", icon: Settings },
];
