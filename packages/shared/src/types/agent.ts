export type AgentSummary = {
  id: string;
  project_id: string;
  name: string;
  role: string;
  status: string;
  model: string | null;
  permissions: string[];
  created_at: string;
  updated_at: string;
};
