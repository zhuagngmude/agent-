window.AGENT_SWARM_NAV = {
  overview: ["项目概览", "全局视图 / 项目概览"],
  agents: ["智能体管理", "全局视图 / 智能体管理"],
  tasks: ["任务管理", "全局视图 / 任务管理"],
  workflow: ["工作流编排", "全局视图 / 工作流编排"],
  knowledge: ["知识库管理", "全局视图 / 知识库管理"],
  code: ["代码与知识", "全局视图 / 代码与知识"],
  runtime: ["运行与调度", "全局视图 / 运行与调度"],
  approval: ["审批与确认", "全局视图 / 审批与确认"],
  monitor: ["监控与日志", "全局视图 / 监控与日志"],
  billing: ["费用与用量", "全局视图 / 费用与用量"],
  integrations: ["集成与扩展", "全局视图 / 集成与扩展"],
  settings: ["系统设置", "全局视图 / 系统设置"],
};

window.AGENT_SWARM_STATUS = {
  approval: {
    draft: { label: "草稿", tone: "neutral", action: "继续编辑" },
    pending: { label: "等待审批", tone: "warn", action: "进入审查" },
    approved: { label: "已批准", tone: "ok", action: "等待执行" },
    rejected: { label: "已拒绝", tone: "danger", action: "查看原因" },
    patch_only: { label: "只生成补丁", tone: "neutral", action: "下载补丁" },
    executed: { label: "已执行", tone: "ok", action: "查看结果" },
    rolled_back: { label: "已回滚", tone: "danger", action: "查看回滚" },
    expired: { label: "已过期", tone: "neutral", action: "重新申请" },
  },
  task: {
    queued: { label: "排队中", tone: "neutral" },
    running: { label: "进行中", tone: "ok" },
    blocked: { label: "已阻塞", tone: "danger" },
    waiting_user: { label: "等待用户", tone: "warn" },
    completed: { label: "已完成", tone: "ok" },
    failed: { label: "失败", tone: "danger" },
    cancelled: { label: "已取消", tone: "neutral" },
  },
  agent: {
    running: { label: "运行中", tone: "ok" },
    idle: { label: "空闲中", tone: "neutral" },
    waiting: { label: "等待中", tone: "warn" },
    failed: { label: "异常", tone: "danger" },
  },
};

window.AGENT_SWARM_DATA = {
  project: {
    name: "agent蜂群 MVP",
    status: "运行中",
    phase: "MVP-0.2 前端工程化与 Runner 审批原型",
    description: "多 AI 智能体协作调度系统，优先打磨任务流、审批确认和知识库联动。",
  },
  dashboardMetrics: [
    { label: "活跃智能体", value: "-", note: "等待本地 API 数据", tone: "purple", icon: "A" },
    { label: "待确认事项", value: "-", note: "Runner 审批优先", tone: "orange", icon: "!" },
    { label: "活跃任务", value: "-", note: "等待任务状态", tone: "blue", icon: "T" },
    { label: "Git 检查点", value: "-", note: "等待项目保存点", tone: "green", icon: "G" },
    { label: "Token 消耗", value: "-", note: "真实模型调用未接入", tone: "violet", icon: "K" },
    { label: "模型使用", value: "-", note: "模型 API 未接入", tone: "cyan", icon: "M" },
  ],
  workflow: {
    steps: [
      { name: "需求分析智能体", detail: "2 个实例运行中", progress: "100%", tone: "purple" },
      { name: "方案设计智能体", detail: "3 个实例运行中", progress: "78%", tone: "blue" },
      { name: "开发实现智能体", detail: "5 个实例运行中", progress: "62%", tone: "green" },
      { name: "测试验证智能体", detail: "4 个实例运行中", progress: "45%", tone: "orange" },
      { name: "归档审查智能体", detail: "2 个实例运行中", progress: "20%", tone: "cyan" },
    ],
    stats: [
      ["总智能体数", "-"],
      ["总任务数", "-"],
      ["完成任务", "-"],
      ["协作成功率", "-"],
      ["平均响应时间", "-"],
    ],
  },
  approvalRequests: [],
  taskQueue: [],
  agents: [
    { avatar: "A", name: "架构师 Agent", version: "v0.2.0", status: "running" },
    { avatar: "B", name: "前端 Agent", version: "v0.2.0", status: "running" },
    { avatar: "C", name: "文档 Agent", version: "v0.1.5", status: "idle" },
    { avatar: "D", name: "审查 Agent", version: "v0.1.8", status: "running" },
  ],
  gitCheckpoints: [],
  knowledgeUpdates: [
    { mark: "路线", tone: "purple", title: "下一步开发路线", detail: "新增 MVP-0.2 开发顺序", time: "刚刚" },
    { mark: "架构", tone: "blue", title: "前端交互反推架构", detail: "更新 12 模块系统边界", time: "今天" },
    { mark: "AI", tone: "orange", title: "AI 开发维护手册", detail: "补充 Runner 审批原则", time: "今天" },
  ],
  apiKeys: [
    { name: "OpenAI API", status: "未接入", detail: "当前不保存真实模型密钥，界面只保留配置入口占位。", usage: "0%" },
    { name: "Anthropic API", status: "未接入", detail: "当前不保存真实模型密钥，日志脱敏规则仍是后续能力。", usage: "0%" },
  ],
};
