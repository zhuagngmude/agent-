const pages = {
  overview: {
    label: "项目总览",
    count: 3,
    kicker: "Project Console",
    title: "项目总览",
    subtitle: "确认项目状态、Agent 活动、阻塞任务、Runner 风险和最近 Git 保存点。",
    primary: "新建协作任务",
    secondary: "导出记录",
  },
  agents: {
    label: "Agent 编排",
    count: 7,
    kicker: "Agent Orchestration",
    title: "Agent 编排",
    subtitle: "查看父 Agent、子 Agent、任务队列、职责边界和结果回传状态。",
    primary: "新增 Agent 模板",
    secondary: "查看权限矩阵",
  },
  tasks: {
    label: "任务拆解",
    count: 12,
    kicker: "Task Planning",
    title: "任务拆解与分配",
    subtitle: "从大任务拆成模块，分配负责人、风险等级、相关文件和预计步骤。",
    primary: "运行拆解",
    secondary: "批量重分配",
  },
  pipeline: {
    label: "执行流水线",
    count: 1,
    kicker: "Pipeline Replay",
    title: "执行流水线详情",
    subtitle: "回放一次 AI 协作任务的完整过程、耗时、输出摘要和错误信息。",
    primary: "继续执行",
    secondary: "导出流水线",
  },
  runner: {
    label: "Runner 确认",
    count: 2,
    kicker: "Local Runner Approval",
    title: "本地 Runner 执行确认",
    subtitle: "自动改代码前必须展示文件、原因、风险、Git checkpoint 和用户确认动作。",
    primary: "查看差异",
    secondary: "只生成补丁",
  },
  git: {
    label: "Git 历史",
    count: 18,
    kicker: "Git Checkpoints",
    title: "Git 保存与历史记录",
    subtitle: "把关键讨论、功能实现、bug 修复和大改前状态沉淀为可回溯记录。",
    primary: "创建保存点",
    secondary: "查看决策链",
  },
  docs: {
    label: "知识库",
    count: 42,
    kicker: "Knowledge Base",
    title: "知识库 / 文档管理",
    subtitle: "维护人类说明书和 AI 开发维护手册，并追踪功能变更对应更新位置。",
    primary: "同步知识库",
    secondary: "检查过期段落",
  },
  settings: {
    label: "系统设置",
    count: null,
    kicker: "System Settings",
    title: "系统设置",
    subtitle: "配置模型、API Key、权限、云同步、数据保存策略、日志归档和导出清理。",
    primary: "保存设置",
    secondary: "导出配置",
  },
};

const activeAgents = [
  ["架构师 Agent", "正在合并任务拆解方案", "进行中", "ok"],
  ["调度 Agent", "等待 Runner 权限确认", "等待用户", "warn"],
  ["前端 Agent", "生成工作台页面结构", "进行中", "ok"],
  ["测试 Agent", "待接收后端接口变更", "待分配", ""],
];

const tasks = [
  ["工作台首页信息聚合", "前端 Agent", "进行中", "src/pages/overview.tsx", "低", "4"],
  ["Runner 权限确认流程", "后端 Agent", "等待用户确认", "runner/permissions.py", "高", "6"],
  ["Git checkpoint 记录模型", "架构师 Agent", "待审查", "server/git_log.go", "中", "5"],
  ["知识库双文档关联", "文档 Agent", "待分配", "docs/user.md, docs/ai.md", "低", "3"],
  ["失败任务回放", "测试 Agent", "失败", "tests/replay.spec.ts", "中", "4"],
];

const contextByPage = {
  overview: [
    ["阶段", "MVP UI 原型工程化"],
    ["当前阻塞", "Runner 写入审批状态机"],
    ["最近保存", "a5d3f2c · Runner 确认前保存点"],
  ],
  agents: [
    ["选中 Agent", "架构师 Agent"],
    ["模型", "GPT · 高推理"],
    ["子 Agent", "接口设计、数据模型、风险审查"],
  ],
  tasks: [
    ["筛选", "等待用户确认、高风险"],
    ["批量操作", "重新分配、标记阻塞、进入审查"],
    ["当前任务", "Runner 权限确认流程"],
  ],
  pipeline: [
    ["流水线", "建立 Runner 安全确认机制"],
    ["当前步骤", "用户确认"],
    ["耗时", "2m 11s"],
  ],
  runner: [
    ["操作类型", "写文件、更新文档"],
    ["Git checkpoint", "已创建 a5d3f2c"],
    ["执行后果", "修改 Runner 权限逻辑"],
  ],
  git: [
    ["选中 commit", "a5d3f2c"],
    ["类型", "大改前保存点"],
    ["关联任务", "Runner 执行确认"],
  ],
  docs: [
    ["文档状态", "2 处待更新"],
    ["人类说明书", "Runner 确认规则需补充"],
    ["AI 手册", "安全边界待审查"],
  ],
  settings: [
    ["密钥策略", "本地保存、日志脱敏"],
    ["写入权限", "默认需要确认"],
    ["同步策略", "密钥不同步"],
  ],
};

const risksByPage = {
  runner: [
    ["高", "将修改本地 Runner 权限逻辑", "danger"],
    ["中", "审计日志格式会影响后续回放", "warn"],
  ],
  settings: [
    ["高", "API Key 不能进入日志和云同步", "danger"],
    ["中", "模型切换可能影响任务输出一致性", "warn"],
  ],
  default: [
    ["中", "Runner 写入审批仍待确认", "warn"],
    ["低", "知识库文档有 2 处待同步", ""],
  ],
};

const nav = document.querySelector("#nav");
const content = document.querySelector("#content");
const pageTitle = document.querySelector("#page-title");
const pageSubtitle = document.querySelector("#page-subtitle");
const pageKicker = document.querySelector("#page-kicker");
const primaryAction = document.querySelector("#primary-action");
const secondaryAction = document.querySelector("#secondary-action");
const contextBody = document.querySelector("#context-body");
const contextTitle = document.querySelector("#context-title");
const riskList = document.querySelector("#risk-list");

Object.entries(pages).forEach(([id, page], index) => {
  const button = document.createElement("button");
  button.type = "button";
  button.dataset.page = id;
  button.className = index === 0 ? "active" : "";
  button.innerHTML = `<span>${page.label}</span>${page.count === null ? "" : `<small>${page.count}</small>`}`;
  nav.appendChild(button);
});

nav.addEventListener("click", (event) => {
  const button = event.target.closest("button[data-page]");
  if (!button) return;
  setPage(button.dataset.page);
});

function chip(text, tone = "") {
  return `<span class="chip ${tone}">${text}</span>`;
}

function renderOverview() {
  const agentRows = activeAgents.map(([name, desc, status, tone]) => `
    <div class="list-row">
      <div><h3>${name}</h3><div class="muted">${desc}</div></div>
      ${chip(status, tone)}
    </div>
  `).join("");

  return `
    <div class="metric-grid">
      <div class="metric"><span>活跃 Agent</span><strong>6</strong></div>
      <div class="metric"><span>阻塞任务</span><strong>2</strong></div>
      <div class="metric"><span>待确认事项</span><strong>3</strong></div>
      <div class="metric"><span>最近保存点</span><strong>a5d3f2c</strong></div>
    </div>
    <div class="grid two">
      <section class="panel pad"><h2>活跃 Agent</h2><div class="list">${agentRows}</div></section>
      <section class="panel pad">
        <h2>阻塞与确认</h2>
        <div class="list">
          <div class="list-row"><div><h3>修改 runner/permissions.py</h3><div class="muted">涉及本地文件写入权限，需要用户确认。</div></div>${chip("高风险", "danger")}</div>
          <div class="list-row"><div><h3>API Key 存储策略</h3><div class="muted">平台默认 key 与用户 key 的优先级未确认。</div></div>${chip("产品决策", "warn")}</div>
          <div class="list-row"><div><h3>Git checkpoint</h3><div class="muted">已创建 checkpoint，等待允许执行。</div></div>${chip("已保护", "ok")}</div>
        </div>
      </section>
    </div>
    <section class="panel pad">
      <h2>项目想法输入</h2>
      <div class="muted">把 agent蜂群 第一版做成桌面端 Web App：支持任务拆解、Agent 分配、执行确认、Git 历史和知识库同步。</div>
    </section>
  `;
}

function renderAgents() {
  return `
    <div class="agent-map">
      ${agentNode("架构师 Agent", "负责系统边界、模块拆分、技术方案和风险判断。", ["接口设计子 Agent", "数据模型子 Agent", "风险审查子 Agent"])}
      ${agentNode("调度 Agent", "负责 Agent 分配、队列、依赖关系和失败重试。", ["优先级子 Agent", "资源子 Agent", "汇总子 Agent"], "等待确认")}
      <section class="empty-state"><strong>空状态</strong><span>还没有为测试 Agent 派生子 Agent。选择一个父 Agent 后可创建最多 3 个子 Agent。</span></section>
    </div>
  `;
}

function agentNode(name, desc, children, status = "进行中") {
  return `
    <article class="agent-node">
      <div class="list-row">
        <div><h3>${name}</h3><div class="muted">${desc}</div></div>
        ${chip(status, status === "等待确认" ? "warn" : "ok")}
      </div>
      <div class="child-agents">
        ${children.map((child) => `<article><h3>${child}</h3><div class="muted">结果回传给 ${name} 汇总</div></article>`).join("")}
      </div>
    </article>
  `;
}

function renderTasks() {
  const rows = tasks.map(([name, owner, status, file, risk, steps]) => `
    <tr>
      <td><strong>${name}</strong></td>
      <td>${owner}</td>
      <td>${statusChip(status)}</td>
      <td>${file}</td>
      <td>${riskChip(risk)}</td>
      <td>${steps}</td>
    </tr>
  `).join("");
  return `
    <div class="toolbar">
      ${chip("状态：全部")} ${chip("Agent：全部")} ${chip("风险：全部")} ${chip("只看阻塞", "warn")}
    </div>
    <div class="table-wrap">
      <table>
        <thead><tr><th>任务</th><th>负责人</th><th>状态</th><th>相关文件</th><th>风险</th><th>预计步骤</th></tr></thead>
        <tbody>${rows}</tbody>
      </table>
    </div>
  `;
}

function renderPipeline() {
  const steps = [
    ["需求输入", "用户要求 Runner 修改代码前必须展示文件、原因、风险和 Git checkpoint。", "完成 · 12s", "ok"],
    ["架构设计", "新增 ExecutionApproval 模型，所有写操作走审批状态机。", "完成 · 1m 08s", "ok"],
    ["任务拆解", "拆分为权限 UI、后端策略、Git checkpoint、审查日志四个模块。", "完成 · 38s", "ok"],
    ["Agent 执行", "后端 Agent 请求修改 runner/permissions.py 和 server/audit_log.go。", "暂停", "warn"],
    ["用户确认", "需要选择允许执行、拒绝、只生成补丁或查看差异。", "当前步骤", "warn"],
  ];
  return `<div class="pipeline">${steps.map((step, index) => `
    <article class="step">
      <span class="step-index">${index + 1}</span>
      <div><h3>${step[0]}</h3><div class="muted">${step[1]}</div></div>
      ${chip(step[2], step[3])}
    </article>
  `).join("")}</div>`;
}

function renderRunner() {
  return `
    <div class="warning-box"><strong>安全边界：</strong>Runner 请求写入 3 个本地文件。Git checkpoint 已创建：<strong>a5d3f2c</strong>。执行前必须确认。</div>
    <div class="grid two">
      <section class="panel pad">
        <h2>拟修改文件</h2>
        <div class="list">
          <div class="list-row"><div><h3>runner/permissions.py</h3><div class="muted">新增写入审批状态机。</div></div>${chip("高", "danger")}</div>
          <div class="list-row"><div><h3>server/audit_log.go</h3><div class="muted">记录审批人、时间、风险说明。</div></div>${chip("中", "warn")}</div>
          <div class="list-row"><div><h3>docs/ai-maintenance.md</h3><div class="muted">更新 AI 维护手册中的 Runner 规则。</div></div>${chip("低")}</div>
        </div>
      </section>
      <section class="panel pad">
        <h2>执行决策</h2>
        <p class="muted">建议先查看差异。如果只想保留 AI 产物但不写入工作区，可以选择只生成补丁。</p>
        <div class="toolbar">
          <button class="primary-button">查看差异</button>
          <button class="secondary-button">只生成补丁</button>
          <button class="secondary-button">拒绝</button>
          <button class="primary-button">允许执行</button>
        </div>
      </section>
    </div>
    <div class="diff">
      <div class="meta">diff --git a/runner/permissions.py b/runner/permissions.py</div>
      <div class="del">- return runner.execute(command)</div>
      <div class="add">+ approval = require_user_approval(command, changed_files)</div>
      <div class="add">+ return runner.execute(command) if approval.allowed else PatchOnlyResult()</div>
    </div>
  `;
}

function renderGit() {
  return `
    <div class="table-wrap">
      <table>
        <thead><tr><th>Commit</th><th>类型</th><th>摘要</th><th>关联文档</th><th>时间</th></tr></thead>
        <tbody>
          <tr><td>a5d3f2c</td><td>大改前</td><td>Runner 执行确认前保存点</td><td>AI开发维护手册 / Runner 安全</td><td>2 分钟前</td></tr>
          <tr><td>9c1e8aa</td><td>功能实现</td><td>新增 Agent 编排模型与父子层级约束</td><td>人类说明书 / Agent 编排</td><td>今天 08:50</td></tr>
          <tr><td>6ba12fe</td><td>决策记录</td><td>确定任务流不采用聊天记录形态</td><td>产品决策 / 工作流回放</td><td>昨天 22:18</td></tr>
        </tbody>
      </table>
    </div>
  `;
}

function renderDocs() {
  return `
    <div class="grid two">
      <section class="panel pad">
        <h2>人类说明书</h2>
        <div class="list">
          <div class="list-row"><div><h3>项目工作台如何使用</h3><div class="muted">关联：AI 手册 / overview-state</div></div>${chip("已同步", "ok")}</div>
          <div class="list-row"><div><h3>Runner 执行确认规则</h3><div class="muted">关联：AI 手册 / runner-approval</div></div>${chip("需更新", "warn")}</div>
        </div>
      </section>
      <section class="panel pad">
        <h2>AI 开发维护手册</h2>
        <div class="list">
          <div class="list-row"><div><h3>Agent 层级约束</h3><div class="muted">父 Agent 最多 3 个子 Agent，子层级最多 2 层。</div></div>${chip("已同步", "ok")}</div>
          <div class="list-row"><div><h3>文件写入安全边界</h3><div class="muted">所有本地写入必须经过 ExecutionApproval。</div></div>${chip("待审查", "warn")}</div>
        </div>
      </section>
    </div>
  `;
}

function renderSettings() {
  return `
    <div class="grid two">
      ${settingsPanel("模型配置", [["架构师", "GPT · 高推理"], ["前端", "Claude · UI 代码"], ["审查", "Gemini · 长上下文"]])}
      ${settingsPanel("API Key 管理", [["平台默认", "启用"], ["用户 Key", "仅本地保存"], ["泄露防护", "日志自动脱敏，禁止回显完整 key"]])}
      ${settingsPanel("权限系统", [["读文件", "允许"], ["写文件", "需要确认"], ["执行命令", "高风险需二次确认"]])}
      ${settingsPanel("数据策略", [["云同步", "项目元数据同步，密钥不同步"], ["日志归档", "每日 02:30 自动归档"], ["导出清理", "支持一键导出 / 一键清理"]])}
    </div>
  `;
}

function settingsPanel(title, rows) {
  return `<section class="panel pad"><h2>${title}</h2>${rows.map(([key, value]) => `<div class="kv"><span>${key}</span><strong>${value}</strong></div>`).join("")}</section>`;
}

function statusChip(status) {
  const tone = status.includes("失败") ? "danger" : status.includes("等待") || status.includes("审查") ? "warn" : status.includes("进行") ? "ok" : "";
  return chip(status, tone);
}

function riskChip(risk) {
  return chip(risk, risk === "高" ? "danger" : risk === "中" ? "warn" : "");
}

function renderPage(id) {
  const renderers = {
    overview: renderOverview,
    agents: renderAgents,
    tasks: renderTasks,
    pipeline: renderPipeline,
    runner: renderRunner,
    git: renderGit,
    docs: renderDocs,
    settings: renderSettings,
  };
  return renderers[id]();
}

function setPage(id) {
  const page = pages[id];
  pageKicker.textContent = page.kicker;
  pageTitle.textContent = page.title;
  pageSubtitle.textContent = page.subtitle;
  primaryAction.textContent = page.primary;
  secondaryAction.textContent = page.secondary;
  content.innerHTML = renderPage(id);

  document.querySelectorAll(".nav button").forEach((button) => {
    button.classList.toggle("active", button.dataset.page === id);
  });

  renderContext(id);
}

function renderContext(id) {
  contextTitle.textContent = `${pages[id].label}上下文`;
  contextBody.innerHTML = contextByPage[id].map(([key, value]) => `<div class="kv"><span>${key}</span><strong>${value}</strong></div>`).join("");
  const risks = risksByPage[id] || risksByPage.default;
  riskList.innerHTML = risks.map(([level, text, tone]) => `<div class="risk-item">${chip(level, tone)} <span>${text}</span></div>`).join("");
}

setPage("overview");
