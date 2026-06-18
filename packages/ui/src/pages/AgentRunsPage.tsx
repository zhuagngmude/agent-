import { useEffect, useState } from "react";
import { Alert, Card, Space, Spin, Table, Tag, Typography } from "antd";
import type { ColumnsType } from "antd/es/table";
import { Activity } from "lucide-react";

import type { AgentRunSummary, RuntimeEventSummary } from "@agent-swarm/shared";
import { isTauriHost, listAgentRuns, listRuntimeEvents } from "../utils/desktopHost";
import { roleLabel, statusLabel, agentNameLabel } from "../utils/labels";
import { userErrorLabel } from "../utils/userError";

// ---------------------------------------------------------------------------
// 行数据类型
// ---------------------------------------------------------------------------

type ChainRow = {
  key: string;
  chain_id: string;
  chain_label: string;
  requested_by: string;
  run_count: number;
  succeeded: number;
  failed: number;
  blocked: number;
  created_at: string;
};

type RunRow = {
  key: string;
  id: string;
  sequence: number;
  role: string;
  agent_name: string;
  model: string;
  status: string;
  input_summary: string | null;
  output_summary: string | null;
  token_usage: string;
  cost_estimate: string;
};

// ---------------------------------------------------------------------------
// AgentRunsPage
// ---------------------------------------------------------------------------

export function AgentRunsPage() {
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [runs, setRuns] = useState<AgentRunSummary[]>([]);
  const [events, setEvents] = useState<RuntimeEventSummary[]>([]);

  useEffect(() => {
    if (!isTauriHost()) {
      setRuns([]);
      setEvents([]);
      setLoading(false);
      return;
    }

    let cancelled = false;

    async function fetchData() {
      try {
        const [r, e] = await Promise.all([
          listAgentRuns(),
          listRuntimeEvents(),
        ]);
        if (!cancelled) {
          setRuns(r);
          setEvents(e);
          setError(null);
        }
      } catch (err) {
        if (!cancelled) {
          setRuns([]);
          setEvents([]);
          setError(userErrorLabel(err, "读取运行记录失败，请稍后重试"));
        }
      } finally {
        if (!cancelled) setLoading(false);
      }
    }

    fetchData();
    return () => { cancelled = true; };
  }, []);

  // -----------------------------------------------------------------------
  // 链视图（按 chain_id 分组）
  // -----------------------------------------------------------------------

  const chains = buildChainRows(runs);
  const eventsByRun = indexEventsByRun(events);

  const chainColumns: ColumnsType<ChainRow> = [
    { title: "链名称", dataIndex: "chain_label" },
    { title: "请求人", dataIndex: "requested_by" },
    { title: "智能体数", dataIndex: "run_count" },
    {
      title: "状态",
      key: "status",
      render: (_: unknown, row: ChainRow) => (
        <Space size={4}>
          {row.succeeded > 0 && <Tag color="success">{row.succeeded} 成功</Tag>}
          {row.failed > 0 && <Tag color="error">{row.failed} 失败</Tag>}
          {row.blocked > 0 && <Tag color="warning">{row.blocked} 阻塞</Tag>}
          {row.succeeded === 0 && row.failed === 0 && row.blocked === 0 && (
            <Tag>运行中</Tag>
          )}
        </Space>
      ),
    },
    { title: "创建时间", dataIndex: "created_at" },
  ];

  const runColumns: ColumnsType<RunRow> = [
    { title: "序号", dataIndex: "sequence", width: 60 },
    { title: "角色", dataIndex: "role", render: (r: string) => roleLabel(r) },
    { title: "智能体", dataIndex: "agent_name", render: (name: string) => agentNameLabel(name) },
    { title: "模型", dataIndex: "model" },
    {
      title: "状态",
      dataIndex: "status",
      render: (s: string) => statusTag(s),
    },
    {
      title: "输入摘要",
      dataIndex: "input_summary",
      render: (text: string | null) => text ?? "-",
    },
    {
      title: "输出摘要",
      dataIndex: "output_summary",
      render: (text: string | null) => text ?? "-",
    },
  ];

  // -----------------------------------------------------------------------
  // Render
  // -----------------------------------------------------------------------

  if (loading) {
    return <Spin size="large" style={{ display: "block", marginTop: 120 }} />;
  }

  return (
    <Space orientation="vertical" size={16} className="page-stack">
      <div className="page-heading">
        <Typography.Title level={2}>
          <Activity size={22} style={{ marginRight: 8, verticalAlign: "middle" }} />
          运行记录
        </Typography.Title>
        <Typography.Text type="secondary">
          智能体运行链记录与运行时审计事件（只读）
        </Typography.Text>
      </div>

      {error && (
        <Alert
          type="error"
          showIcon
          title="读取运行记录失败"
          description={error}
          closable
        />
      )}

      <Card title="运行链">
        <Table<ChainRow>
          columns={chainColumns}
          dataSource={chains}
          pagination={false}
          locale={{ emptyText: "暂无运行记录" }}
          expandable={{
            expandedRowRender: (chain) => {
              const chainRuns = toRunRows(
                runs.filter((r) => r.chain_id === chain.chain_id),
              );
              return (
                <Table<RunRow>
                  columns={runColumns}
                  dataSource={chainRuns}
                  pagination={false}
                  size="small"
                  rowKey="key"
                  expandable={{
                    expandedRowRender: (run) => {
                      const runEvents = eventsByRun.get(run.id) ?? [];
                      if (runEvents.length === 0) return <Typography.Text type="secondary">暂无审计事件</Typography.Text>;

                      const eventColumns: ColumnsType<RuntimeEventSummary> = [
                        { title: "事件类型", dataIndex: "event_type" },
                        { title: "操作人", dataIndex: "actor", render: (v: string | null) => v ?? "-" },
                        { title: "原因", dataIndex: "reason", render: (v: string | null) => v ?? "-" },
                        { title: "时间", dataIndex: "created_at" },
                      ];

                      return (
                        <Table<RuntimeEventSummary>
                          columns={eventColumns}
                          dataSource={runEvents}
                          pagination={false}
                          size="small"
                          rowKey="id"
                        />
                      );
                    },
                  }}
                />
              );
            },
          }}
        />
      </Card>
    </Space>
  );
}

// ---------------------------------------------------------------------------
// 辅助函数
// ---------------------------------------------------------------------------

function buildChainRows(runs: AgentRunSummary[]): ChainRow[] {
  const map = new Map<string, ChainRow>();

  for (const run of runs) {
    const existing = map.get(run.chain_id);
    if (existing) {
      existing.run_count += 1;
      if (run.status === "succeeded") existing.succeeded += 1;
      else if (run.status === "failed") existing.failed += 1;
      else if (run.status === "blocked") existing.blocked += 1;
    } else {
      map.set(run.chain_id, {
        key: run.chain_id,
        chain_id: run.chain_id,
        chain_label: run.chain_label ?? run.chain_id,
        requested_by: run.requested_by,
        run_count: 1,
        succeeded: run.status === "succeeded" ? 1 : 0,
        failed: run.status === "failed" ? 1 : 0,
        blocked: run.status === "blocked" ? 1 : 0,
        created_at: run.created_at,
      });
    }
  }

  return Array.from(map.values()).sort(
    (a, b) => b.created_at.localeCompare(a.created_at),
  );
}

function toRunRows(runs: AgentRunSummary[]): RunRow[] {
  return runs
    .sort((a, b) => a.sequence - b.sequence)
    .map((run) => ({
      key: run.id,
      id: run.id,
      sequence: run.sequence,
      role: run.role,
      agent_name: run.agent_name,
      model: run.model,
      status: run.status,
      input_summary: run.input_summary,
      output_summary: run.output_summary,
      token_usage: run.token_usage,
      cost_estimate: run.cost_estimate,
    }));
}

function indexEventsByRun(
  events: RuntimeEventSummary[],
): Map<string, RuntimeEventSummary[]> {
  const map = new Map<string, RuntimeEventSummary[]>();
  for (const event of events) {
    if (event.entity_type !== "agent_run") continue;
    const list = map.get(event.entity_id);
    if (list) {
      list.push(event);
    } else {
      map.set(event.entity_id, [event]);
    }
  }
  return map;
}

function statusTag(status: string) {
  const meta: Record<string, { color: string; label: string }> = {
    queued: { color: "default", label: "排队中" },
    running: { color: "processing", label: "运行中" },
    succeeded: { color: "success", label: "已完成" },
    failed: { color: "error", label: "已失败" },
    blocked: { color: "warning", label: "已阻塞" },
  };
  const m = meta[status] ?? { color: "default", label: status };
  return <Tag color={m.color}>{m.label}</Tag>;
}
