export type TaskStatus =
  | "queued"
  | "running"
  | "completed"
  | "blocked"
  | "cancelled"
  | "failed"
  | "waiting_user";

export type TaskSummary = {
  id: string;
  project_id: string;
  title: string;
  description: string | null;
  status: TaskStatus;
  priority: string;
  assigned_agent_id: string | null;
  depends_on: string[];
  risk_level: string | null;
  created_at: string;
  updated_at: string;
};

export type CreateTaskInput = {
  title: string;
  description?: string | null;
  priority: "low" | "medium" | "high";
  assigned_agent_id?: string | null;
  depends_on?: string[];
  risk_level?: "low" | "medium" | "high" | null;
};

export type UpdateTaskStatusInput = {
  id: string;
  status: TaskStatus;
};

export type DeleteTasksInput = {
  task_ids: string[];
};

export type DeleteTasksResponse = {
  deleted_task_ids: string[];
  deleted_output_paths: string[];
};

export type OpenTaskOutputFolderInput = {
  task_ids: string[];
};

export type OpenTaskOutputFolderResponse = {
  path: string;
};
