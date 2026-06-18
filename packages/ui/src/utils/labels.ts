/**
 * labels — 前端统一中文化映射工具。
 *
 * 所有用户可见的英文枚举值、技术字段、内部标识符统一通过此模块映射为中文。
 * 各页面组件不得散写 ad-hoc 映射，必须引用此处的函数。
 *
 * ## 映射函数
 * - statusLabel()      — 状态值 → 中文
 * - riskLabel()        — 风险等级 → 中文
 * - riskColor()        — 风险等级 → Tag 颜色
 * - roleLabel()        — Agent/任务角色 → 中文
 * - priorityLabel()    — 优先级 → 中文
 * - priorityColor()    — 优先级 → Tag 颜色
 * - operationTypeLabel() — 操作类型 → 中文
 * - targetServiceLabel() — 审批目标服务 → 中文
 * - agentLabel()       — Agent 名称/角色友好展示
 * - modelLabel()       — 模型标识 → 用户友好名
 * - technicalTermLabel() — 技术术语 → 中文解释
 * - boolLabel()        — boolean → 是/否
 * - draftSourceLabel() — 草案来源 → 中文
 * - blockedReasonLabel() — 阻断原因 → 中文
 * - checkpointStrategyLabel() — 检查点策略 → 中文
 * - approvalStatusColor() — 审批状态 → Tag 颜色
 * - agentStatusColor() — Agent 状态 → Tag 颜色
 */

// ==========================================================================
// 状态标签
// ==========================================================================

/** 任务 / Agent / 审批 / Runner 的所有状态 → 中文映射 */
export function statusLabel(status: string): string {
  const map: Record<string, string> = {
    // 任务状态 (TaskStatus)
    queued: "排队中",
    running: "运行中",
    completed: "已交付",
    blocked: "已阻塞",
    cancelled: "已取消",
    failed: "失败",
    waiting_user: "等你确认",

    // 审批状态
    pending: "待审批",
    approved: "已通过",
    rejected: "已拒绝",

    // Agent 状态
    idle: "空闲",

    // Agent Run 状态
    succeeded: "成功",
    success: "成功",

    // Runner 审查 / 闸门 / 预演 / 锁定 / 最小执行 状态
    revoked: "已撤销",
    locked: "已锁定",
    instantiated: "已实例化",
    draft: "草案",
    draft_ready: "草案已生成",
    pending_apply: "待应用",
    applied: "已应用",
    unknown: "未知",
    blocked_by_stage_boundary: "已被阶段边界阻断",
    in_progress: "进行中",
    waiting: "等待中",
    skipped: "已跳过",
  };
  return map[status] ?? `未识别（${status}）`;
}

// ==========================================================================
// 风险等级标签
// ==========================================================================

/** high / medium / low / none → 中文风险等级 */
export function riskLabel(risk: string): string {
  const map: Record<string, string> = {
    high: "高风险",
    medium: "中风险",
    low: "低风险",
    none: "无风险",
  };
  return map[risk] ?? risk;
}

/** 风险等级 → Ant Design Tag 颜色 */
export function riskColor(risk: string): string {
  const map: Record<string, string> = {
    high: "red",
    medium: "orange",
    low: "green",
    none: "default",
  };
  return map[risk] ?? "default";
}

// ==========================================================================
// 优先级标签
// ==========================================================================

/** high / medium / low → 中文优先级 */
export function priorityLabel(priority: string): string {
  const map: Record<string, string> = {
    high: "高",
    medium: "中",
    low: "低",
  };
  return map[priority] ?? priority;
}

/** 优先级 → Ant Design Tag 颜色 */
export function priorityColor(priority: string): string {
  const map: Record<string, string> = {
    high: "red",
    medium: "orange",
    low: "default",
  };
  return map[priority] ?? "default";
}

// ==========================================================================
// 角色标签（Agent 角色 / 任务模板角色）
// ==========================================================================

/** Agent 角色 / 任务模板 role → 中文角色名 */
export function roleLabel(role: string): string {
  const map: Record<string, string> = {
    // 主要角色
    controller: "总控调度官",
    product: "产品经理",
    architect: "产品规划官",
    uiux: "界面体验官",
    frontend: "前端实现官",
    backend: "后端实现官",
    desktop: "桌面端实现官",
    database: "数据库工程官",
    ai_prompt: "AI 提示词工程官",
    qa: "质量保证官",
    docs: "文档编写官",
    reviewer: "安全审查官",
    security: "安全审查官",
    devops: "运维部署官",
    ux: "用户体验官",
    data: "数据处理官",
    planner: "项目计划官",
    runner: "执行观察官",

    // agent_xxx 前缀变体（来自旧任务模板 / Agent Run）— 全中文
    agent_frontend: "前端智能体",
    agent_backend: "后端智能体",
    agent_qa: "质量保证智能体",
    agent_docs: "文档智能体",
    agent_reviewer: "审查智能体",
    agent_security: "安全智能体",
    agent_devops: "运维智能体",
    agent_ux: "体验智能体",
    agent_data: "数据智能体",
  };
  return map[role] ?? role;
}

// ==========================================================================
// Agent 名称标签（把 raw agent.name 映射为中文）
// ==========================================================================

/** 常见 Agent 原始名称 → 中文 */
export function agentNameLabel(name: string): string {
  const map: Record<string, string> = {
    // 英文通用名
    "Backend Agent": "后端智能体",
    "QA Agent": "质量保证智能体",
    "Frontend Agent": "前端智能体",
    "Docs Agent": "文档智能体",
    "Reviewer Agent": "审查智能体",
    "Security Agent": "安全智能体",
    "DevOps Agent": "运维智能体",
    "UX Agent": "体验智能体",
    "Data Agent": "数据智能体",
    "Planner Agent": "计划智能体",
    "Runner Agent": "执行智能体",
    "Architect Agent": "规划智能体",

    // 内部智能体 ID
    agent_architect: "规划智能体",
    agent_frontend: "前端智能体",
    agent_backend: "后端智能体",
    agent_qa: "质量保证智能体",
    agent_docs: "文档智能体",
    agent_reviewer: "审查智能体",
    agent_security: "安全智能体",
    agent_devops: "运维智能体",
    agent_ux: "体验智能体",
    agent_data: "数据智能体",
    agent_runner: "执行智能体",

    // 模型/技术标识名（不应直接暴露，但若有则映射）
    "gpt-api": "GPT 接口智能体",
    "claude-ui": "Claude 界面智能体",

    // "示例 Agent：xxx" 模式 — 保留中文部分，去掉"示例 Agent："前缀
  };

  if (map[name]) return map[name];

  // 去掉"示例 Agent："、"示例 Agent:" 前缀
  const cleaned = name.replace(/^示例\s*Agent[:：]?\s*/i, "").trim();
  if (cleaned !== name) return cleaned;

  return name;
}

// ==========================================================================
// Agent 标签（Agent 名称/角色友好展示）
// ==========================================================================

/**
 * 返回 Agent 的用户友好展示名。
 * 优先用 agentNameLabel 映射 name，回退到 roleLabel 映射 role。
 */
export function agentLabel(agent: { name: string; role: string }): string {
  // 先尝试 name 映射
  const nameLabel = agentNameLabel(agent.name);
  if (nameLabel !== agent.name) return nameLabel;

  // 如果 name 已经是中文则直接使用
  if (/[一-鿿]/.test(agent.name)) return agent.name;

  // 否则显示角色名
  return roleLabel(agent.role);
}

// ==========================================================================
// 模型标签
// ==========================================================================

/** 模型标识 → 用户友好名称 */
export function modelLabel(model: string): string {
  const map: Record<string, string> = {
    // OpenAI
    "gpt-4": "GPT-4",
    "gpt-4o": "GPT-4o",
    "gpt-4o-mini": "GPT-4o Mini",
    "gpt-3.5-turbo": "GPT-3.5 Turbo",
    "gpt-4-turbo": "GPT-4 Turbo",
    "gpt-4.1": "GPT-4.1",
    "gpt-5-codex": "GPT-5 Codex",
    "gpt-5.1-codex": "GPT-5.1 Codex",
    // Anthropic
    "claude-3-opus": "Claude 3 Opus",
    "claude-3-sonnet": "Claude 3 Sonnet",
    "claude-3-haiku": "Claude 3 Haiku",
    "claude-3.5-sonnet": "Claude 3.5 Sonnet",
    "claude-opus-4-8": "Claude Opus 4.8",
    "claude-sonnet-4-6": "Claude Sonnet 4.6",
    "claude-opus-4.8": "Claude Opus 4.8",
    "claude-sonnet-4.6": "Claude Sonnet 4.6",
    // DeepSeek
    "deepseek-v3": "DeepSeek V3",
    "deepseek-v4-pro": "DeepSeek V4 Pro",
    "deepseek-r1": "DeepSeek R1",
    // Model gateway / executor catalog
    "openclaw-default": "OpenClaw 默认模型",
    "openclaw-coder": "OpenClaw 代码模型",
    "openclaw-planner": "OpenClaw 规划模型",
    "hermes-3": "Hermes 3",
    "hermes-coder": "Hermes 代码模型",
    "hermes-fast": "Hermes 快速模型",
    "gemini-2.5-pro": "Gemini 2.5 Pro",
    "gemini-2.5-flash": "Gemini 2.5 Flash",
    "cursor-agent": "Cursor Agent",
    "cursor-fast": "Cursor 快速模型",
    "opencode-agent": "OpenCode Agent",
    "opencode-coder": "OpenCode 代码模型",
    // 通用回退
    "gpt-api": "GPT API",
    "claude-ui": "Claude UI",
  };
  return map[model] ?? model;
}

// ==========================================================================
// 操作类型标签
// ==========================================================================

/** file_write / git_checkpoint / … → 中文操作描述 */
export function operationTypeLabel(op: string): string {
  const map: Record<string, string> = {
    // 文件操作
    file_read: "读取文件",
    file_write: "写入文件",
    file_delete: "删除文件",

    // Git 操作
    git_checkpoint: "版本保存点",
    git_commit: "版本提交",
    git_push: "版本推送",
    git_status: "版本状态查询",
    git_diff: "版本差异对比",

    // 命令执行
    command_exec: "执行命令",
    runner_exec: "执行引擎执行",

    // 模型调用
    model_call: "模型调用",

    // 网络
    network_request: "网络请求",

    // 审批 / 队列相关
    patch_only: "仅补丁",
    approval_review: "审批审查",
    full_approval: "完整审批",
    audit_log_update: "审计日志更新",
    runner_request_queue: "执行请求排队",
    project_plan_approval: "项目计划审批",
    agent_task_assignment: "智能体任务分配",
    agent_config: "智能体配置",
  };
  return map[op] ?? op;
}

// ==========================================================================
// 目标服务标签（审批 target_service）
// ==========================================================================

/** 审批 target_service → 中文服务名 */
export function targetServiceLabel(service: string): string {
  const map: Record<string, string> = {
    project_plan: "项目计划",
    runner: "执行引擎",
    runner_preflight: "预检",
    runner_gate: "放行",
    runner_execution_gate: "放行",
    runner_dry_run: "试跑",
    runner_lock: "锁定",
    runner_execution_lock: "锁定",
    runner_minimal_run: "真干",
    model_call: "模型调用",
    idea_guidance: "想法引导",
    task_update: "任务更新",
    agent_assign: "智能体分配",
    agent_config: "智能体配置",
  };
  return map[service] ?? service;
}

// ==========================================================================
// 技术术语标签
// ==========================================================================

/**
 * 技术术语 → 中文解释。
 * 技术词如 Runner/Git 可保留但必须有中文解释。
 */
export function technicalTermLabel(term: string): string {
  const map: Record<string, string> = {
    "Model Gateway": "模型网关",
    "model gateway": "模型网关",
    API: "接口（API）",
    SQLite: "本地数据库（SQLite）",
    Runner: "执行引擎",
    runner: "执行引擎",
    Git: "版本管理",
    git: "版本管理",
    model_calls: "模型调用审计",
    "Tauri command": "桌面宿主命令",
    sandbox: "沙箱",
    checkpoint: "保存点",
  };
  const key = term.trim();
  return map[key] ?? map[key.toLowerCase()] ?? term;
}

// ==========================================================================
// 布尔值标签
// ==========================================================================

export function boolLabel(v: boolean): string {
  return v ? "是" : "否";
}

// ==========================================================================
// 草案来源标签
// ==========================================================================

export function draftSourceLabel(source: string): string {
  const map: Record<string, string> = {
    local_template: "本地模板",
    real_model_preview: "真实模型预览",
  };
  return map[source] ?? source;
}

// ==========================================================================
// 阻断原因标签
// ==========================================================================

export function blockedReasonLabel(reason: string): string {
  const map: Record<string, string> = {
    runner_execution_disabled_by_stage_boundary: "当前阶段边界禁止执行",
    runner_execution_disabled_until_gate_approved: "放行未通过",
    runner_execution_disabled_until_dry_run_reviewed: "试跑未确认",
    runner_execution_not_allowed_by_stage: "当前阶段不允许执行",
    manual_checkpoint_required_before_stage34: "阶段 34 前需人工检查点",
    command_failed: "命令执行失败",
    scope_violation: "超出允许范围",
    audit_write_failed: "审计写入失败",
    provider_error: "模型服务返回错误",
    provider_config_error: "模型服务配置异常",
    response_too_large: "响应过大",
    invalid_request: "请求无效",
    invalid_state: "状态无效",
    feature_disabled: "功能开关未开启",
    git_unavailable: "版本管理不可用",
    timeout: "执行超时",
    file_write_not_allowed: "文件写入不被允许",
    file_delete_not_allowed: "文件删除不被允许",
    network_request_not_allowed: "网络请求不被允许",
  };
  return map[reason] ?? reason;
}

// ==========================================================================
// 检查点策略标签
// ==========================================================================

export function checkpointStrategyLabel(strategy: string): string {
  const map: Record<string, string> = {
    manual_checkpoint_required_before_stage34: "阶段 34 前需人工检查点",
    manual_checkpoint_required: "需人工检查点",
    not_required_for_readonly_preview: "试跑无需检查点",
    no_checkpoint_required: "无需检查点",
    auto_checkpoint_before_execution: "执行前自动检查点",
  };
  return map[strategy] ?? strategy;
}

// ==========================================================================
// 颜色辅助函数
// ==========================================================================

/** 审批状态 → Tag 颜色 */
export function approvalStatusColor(status: string): string {
  const map: Record<string, string> = {
    pending: "processing",
    approved: "success",
    rejected: "error",
  };
  return map[status] ?? "default";
}

/** Agent 状态 → Tag 颜色 */
export function agentStatusColor(status: string): string {
  const map: Record<string, string> = {
    running: "success",
    idle: "default",
    error: "error",
  };
  return map[status] ?? "default";
}

// ==========================================================================
// 通用可选文本（多级回退）
// ==========================================================================

export function optionalTextLabel(value: string | null | undefined): string {
  if (value === null || value === undefined) return "-";
  const r = blockedReasonLabel(value);
  if (r !== value) return r;
  const s = statusLabel(value);
  if (s !== `未识别（${value}）`) return s;
  return value;
}
