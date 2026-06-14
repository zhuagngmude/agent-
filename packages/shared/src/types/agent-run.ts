/** Agent Run 状态（与旧 proto 对齐，使用 succeeded 而非 completed） */
export type AgentRunStatus = "queued" | "running" | "succeeded" | "failed" | "blocked";

export type AgentRunSummary = {
  id: string;
  project_id: string;
  chain_id: string;
  root_run_id: string;
  parent_run_id: string | null;
  sequence: number;
  role: string;
  agent_id: string | null;
  agent_name: string;
  model: string;
  status: AgentRunStatus;
  input_summary: string | null;
  output_summary: string | null;
  /** JSON 文本，存储 { prompt, completion, total } */
  token_usage: string;
  /** JSON 文本，存储 { amount, currency } */
  cost_estimate: string;
  error_category: string | null;
  error_message: string | null;
  requested_by: string;
  chain_label: string | null;
  created_at: string;
  started_at: string | null;
  completed_at: string | null;
  failed_at: string | null;
  updated_at: string;
};

export type RuntimeEventSummary = {
  id: string;
  project_id: string;
  entity_type: string;
  entity_id: string;
  event_type: string;
  /** JSON 文本 */
  before_state: string | null;
  /** JSON 文本 */
  after_state: string | null;
  actor: string | null;
  reason: string | null;
  created_at: string;
};
