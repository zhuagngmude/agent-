export type ApprovalSummary = {
  id: string;
  project_id: string;
  task_id: string | null;
  request_agent_id: string;
  target_service: string;
  operation_types: string[];
  status: string;
  risk_level: string;
  reason: string | null;
  reject_reason: string | null;
  approved_at: string | null;
  rejected_at: string | null;
  created_at: string;
  updated_at: string;
};
