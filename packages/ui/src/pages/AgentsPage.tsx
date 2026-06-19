import { useCallback, useEffect, useMemo, useState } from "react";
import {
  Bot,
  CheckCircle2,
  ChevronDown,
  ChevronRight,
  Cpu,
  Plus,
  Settings2,
  Shield,
  UserCheck,
  X,
} from "lucide-react";

import type { AgentSummary } from "@agent-swarm/shared";
import {
  deleteExecutorModel,
  deleteExecutorSkill,
  isTauriHost,
  listAgentBoundaryChecks,
  listAgentTemplates,
  listExecutorConfigs,
  listExecutorModels,
  listExecutorSkills,
  listProjectAgents,
  removeProjectAgent,
  upsertExecutorModel,
  upsertExecutorSkill,
  upsertProjectAgent,
  type AgentBoundaryCheckSummary,
  type AgentTemplateSummary,
  type ExecutorConfigSummary,
  type ExecutorModelSummary,
  type ExecutorSkillSummary,
  type ProjectAgentSummary,
} from "../utils/desktopHost";
import { agentNameLabel, agentStatusColor, modelLabel, roleLabel, statusLabel } from "../utils/labels";
import { userErrorLabel } from "../utils/userError";

type AgentsPageProps = {
  agents: AgentSummary[];
};

type ExecutorKey = string;

type ExecutorOption = {
  key: ExecutorKey;
  label: string;
  count: number;
  models: string[];
  status: "ready" | "planned" | "missing";
  source: "external" | "gateway";
  note: string;
};

type AgentModelChoice = {
  executor: ExecutorKey;
  model: string;
};

type CoreAgentView = AgentSummary & {
  poolStatus: "connected" | "template";
  specialty: string;
  boundary: string;
  stack: string[];
  defaultExecutor: ExecutorKey;
  defaultModel: string;
  agentTemplateId?: string;
  source?: "core" | "recommended" | "manual";
  moduleScope?: string;
  removedAt?: string | null;
};

type ExpertStatus = "suggested" | "accepted" | "ignored";

type ExpertRecommendation = {
  id: string;
  name: string;
  role: string;
  reason: string;
  priority: "高" | "中";
  executor: ExecutorKey;
  model: string;
  skills: string[];
};

type ExecutorLocalConfig = {
  models: string[];
  agents: string[];
  skills: string[];
};

type ExecutorLocalConfigMap = Partial<Record<ExecutorKey, ExecutorLocalConfig>>;

type AgentConfigState = {
  executors: ExecutorConfigSummary[];
  models: ExecutorModelSummary[];
  templates: AgentTemplateSummary[];
  projectAgents: ProjectAgentSummary[];
  skills: ExecutorSkillSummary[];
  boundaryChecks: AgentBoundaryCheckSummary[];
};

const CHOICES_STORAGE_KEY = "agent-swarm.agent-model-choices.v1";
const EXPERTS_STORAGE_KEY = "agent-swarm.project-expert-status.v1";
const EXECUTOR_CONFIG_STORAGE_KEY = "agent-swarm.executor-config.v1";

const EXECUTOR_OPTIONS: ExecutorOption[] = [
  {
    key: "codex",
    label: "Codex 执行器",
    count: 0,
    models: ["gpt-5-codex", "gpt-5.1-codex", "gpt-4.1"],
    status: "ready",
    source: "external",
    note: "官方 CLI 已验证，后续用显式路径接入，不使用 Codex++。",
  },
  {
    key: "claude",
    label: "Claude 执行器",
    count: 1,
    models: ["claude-sonnet-4.6", "claude-opus-4.8", "claude-3.5-sonnet"],
    status: "ready",
    source: "external",
    note: "已可调用，模型可跟随 Claude 自身配置。",
  },
  {
    key: "openclaw",
    label: "OpenClaw 执行器",
    count: 2,
    models: ["openclaw-default", "openclaw-coder", "openclaw-planner"],
    status: "planned",
    source: "gateway",
    note: "后续通过模型网关读取可用模型目录。",
  },
  {
    key: "hermes",
    label: "Hermes 执行器",
    count: 3,
    models: ["hermes-3", "hermes-coder", "hermes-fast"],
    status: "planned",
    source: "gateway",
    note: "后续通过模型网关统一配置 provider、base URL 和 key。",
  },
  {
    key: "google",
    label: "Google 执行器",
    count: 2,
    models: ["gemini-2.5-pro", "gemini-2.5-flash"],
    status: "planned",
    source: "gateway",
    note: "后续只从系统设置里已配置的模型中选择。",
  },
  {
    key: "cursor",
    label: "Cursor 执行器",
    count: 2,
    models: ["cursor-agent", "cursor-fast"],
    status: "missing",
    source: "external",
    note: "本机暂未检测到命令，后续可手动配置路径。",
  },
  {
    key: "opencode",
    label: "OpenCode 执行器",
    count: 2,
    models: ["opencode-agent", "opencode-coder"],
    status: "missing",
    source: "external",
    note: "本机暂未检测到命令，后续可手动配置路径。",
  },
];

const DEFAULT_EXECUTOR: ExecutorKey = "openclaw";

const CORE_AGENT_TEMPLATES: Array<{
  id: string;
  name: string;
  role: string;
  specialty: string;
  boundary: string;
  stack: string[];
  defaultExecutor: ExecutorKey;
  defaultModel: string;
}> = [
  {
    id: "agent_controller",
    name: "总控 Agent",
    role: "controller",
    specialty: "目标理解、任务调度、进度汇总",
    boundary: "只负责判断、拆分、转派和汇总，不直接越过审批写代码。",
    stack: ["项目分流", "任务拆解", "多 Agent 调度"],
    defaultExecutor: "openclaw",
    defaultModel: "openclaw-planner",
  },
  {
    id: "agent_product",
    name: "产品经理 Agent",
    role: "product",
    specialty: "需求澄清、优先级、验收口径",
    boundary: "只负责需求和范围，不直接决定技术实现或修改代码。",
    stack: ["PRD", "MVP", "用户流程"],
    defaultExecutor: "claude",
    defaultModel: "claude-sonnet-4.6",
  },
  {
    id: "agent_architect",
    name: "架构师 Agent",
    role: "architect",
    specialty: "技术路线、模块边界、风险设计",
    boundary: "只负责架构方案和模块边界，跨模块冲突交回总控裁决。",
    stack: ["系统架构", "模块拆分", "接口边界"],
    defaultExecutor: "codex",
    defaultModel: "gpt-5-codex",
  },
  {
    id: "agent_uiux",
    name: "UI/UX Agent",
    role: "ux",
    specialty: "页面体验、交互、视觉一致性",
    boundary: "只负责界面体验和交互建议，不直接改业务逻辑或数据库。",
    stack: ["Figma", "交互设计", "可用性"],
    defaultExecutor: "claude",
    defaultModel: "claude-sonnet-4.6",
  },
  {
    id: "agent_frontend",
    name: "前端 Agent",
    role: "frontend",
    specialty: "页面、组件、状态和样式",
    boundary: "只负责前端 UI、状态和样式，API 或数据库问题必须转给对应角色。",
    stack: ["React", "TypeScript", "CSS"],
    defaultExecutor: "cursor",
    defaultModel: "cursor-agent",
  },
  {
    id: "agent_backend",
    name: "后端 Agent",
    role: "backend",
    specialty: "API、服务逻辑、权限集成",
    boundary: "只负责服务端逻辑和接口，界面体验和部署策略必须转派。",
    stack: ["Node/Rust", "API", "权限"],
    defaultExecutor: "codex",
    defaultModel: "gpt-5-codex",
  },
  {
    id: "agent_desktop",
    name: "桌面端 Agent",
    role: "desktop",
    specialty: "本地能力、桌面宿主、安装包",
    boundary: "只负责桌面宿主、本地能力和打包，不直接修改业务策略。",
    stack: ["Tauri", "本地文件", "打包"],
    defaultExecutor: "codex",
    defaultModel: "gpt-5-codex",
  },
  {
    id: "agent_database",
    name: "数据库 Agent",
    role: "database",
    specialty: "数据模型、迁移、查询和索引",
    boundary: "只负责数据结构和迁移，业务流程变更必须由总控确认。",
    stack: ["SQLite", "迁移", "索引"],
    defaultExecutor: "openclaw",
    defaultModel: "openclaw-planner",
  },
  {
    id: "agent_ai_prompt",
    name: "AI 提示词 Agent",
    role: "ai_prompt",
    specialty: "模型提示词、工具调用、输出格式",
    boundary: "只负责模型提示词和输出协议，不直接调用真实模型或写文件。",
    stack: ["Prompt", "模型评测", "工具调用"],
    defaultExecutor: "google",
    defaultModel: "gemini-2.5-pro",
  },
  {
    id: "agent_qa",
    name: "质量保证 Agent",
    role: "qa",
    specialty: "测试计划、回归、验收",
    boundary: "只负责验证和复现问题，不直接修复实现代码。",
    stack: ["测试用例", "回归", "验收"],
    defaultExecutor: "openclaw",
    defaultModel: "openclaw-coder",
  },
  {
    id: "agent_security",
    name: "安全 Agent",
    role: "security",
    specialty: "权限、密钥、保护路径和审批",
    boundary: "只负责安全边界和风险判定，高风险动作必须进入审批。",
    stack: ["安全审查", "审批链", "敏感信息"],
    defaultExecutor: "google",
    defaultModel: "gemini-2.5-pro",
  },
  {
    id: "agent_devops",
    name: "部署运维 Agent",
    role: "devops",
    specialty: "部署、CI/CD、环境和监控",
    boundary: "只负责环境和部署方案，不能擅自 push、发布或改业务代码。",
    stack: ["Docker", "CI/CD", "服务器"],
    defaultExecutor: "opencode",
    defaultModel: "opencode-agent",
  },
  {
    id: "agent_docs",
    name: "文档 Agent",
    role: "docs",
    specialty: "说明书、接口文档、维护文档",
    boundary: "只负责文档和交付说明，不直接改变产品行为。",
    stack: ["README", "接口文档", "交付说明"],
    defaultExecutor: "claude",
    defaultModel: "claude-3.5-sonnet",
  },
  {
    id: "agent_reviewer",
    name: "审查 Agent",
    role: "reviewer",
    specialty: "最终审查、变更范围、发布风险",
    boundary: "只负责最终审查和发布风险，不直接承担实现任务。",
    stack: ["Code Review", "风险复核", "发布检查"],
    defaultExecutor: "openclaw",
    defaultModel: "openclaw-coder",
  },
];

const EXPERT_RECOMMENDATIONS: ExpertRecommendation[] = [
  {
    id: "expert-uiux",
    name: "UI/UX 专家 Agent",
    role: "界面体验专家",
    reason: "当前项目正在重做主控台画布、员工配置和模块导航，需要专门把交互和视觉打磨好。",
    priority: "高",
    executor: "claude",
    model: "claude-sonnet-4.6",
    skills: ["界面结构", "交互流程", "视觉一致性"],
  },
  {
    id: "expert-desktop",
    name: "桌面端专家 Agent",
    role: "Tauri / 本地能力专家",
    reason: "这个项目未来要做桌面端、本地文件、输出目录和安装包，需要有人专门盯住桌面边界。",
    priority: "高",
    executor: "codex",
    model: "gpt-5-codex",
    skills: ["Tauri", "本地文件", "桌面打包"],
  },
  {
    id: "expert-security",
    name: "安全专家 Agent",
    role: "Runner 安全专家",
    reason: "专家 Agent 以后会参与写代码，必须把审批、保护路径、密钥和高风险动作看牢。",
    priority: "高",
    executor: "google",
    model: "gemini-2.5-pro",
    skills: ["审批链", "保护路径", "敏感信息"],
  },
  {
    id: "expert-data",
    name: "数据模型专家 Agent",
    role: "数据与状态专家",
    reason: "后续要区分全局员工池、项目成员、专家推荐和运行记录，需要提前设计数据结构。",
    priority: "中",
    executor: "openclaw",
    model: "openclaw-planner",
    skills: ["SQLite", "状态模型", "迁移设计"],
  },
];

export function AgentsPage({ agents }: AgentsPageProps) {
  const canUseTauri = isTauriHost();
  const [choices, setChoices] = useState<Record<string, AgentModelChoice>>(() => canUseTauri ? {} : loadChoices());
  const [expertStatus, setExpertStatus] = useState<Record<string, ExpertStatus>>(() => canUseTauri ? {} : loadExpertStatus());
  const [executorConfig, setExecutorConfig] = useState<ExecutorLocalConfigMap>(() => canUseTauri ? {} : loadExecutorConfig());
  const [executorDrafts, setExecutorDrafts] = useState<Record<string, string>>({});
  const [selectedExecutorKey, setSelectedExecutorKey] = useState<ExecutorKey>("codex");
  const [configState, setConfigState] = useState<AgentConfigState | null>(null);
  const [configLoading, setConfigLoading] = useState(canUseTauri);
  const [configError, setConfigError] = useState<string | null>(null);
  const [savingKey, setSavingKey] = useState<string | null>(null);

  const loadAgentConfig = useCallback(async () => {
    if (!canUseTauri) {
      return;
    }
    setConfigLoading(true);
    setConfigError(null);
    try {
      const [executors, models, templates, projectAgents, skills, boundaryChecks] = await Promise.all([
        listExecutorConfigs(),
        listExecutorModels(),
        listAgentTemplates(),
        listProjectAgents(),
        listExecutorSkills(),
        listAgentBoundaryChecks({ limit: 20 }),
      ]);
      setConfigState({ executors, models, templates, projectAgents, skills, boundaryChecks });
      setSelectedExecutorKey((current) => executors.some((executor) => executor.key === current) ? current : executors[0]?.key ?? current);
    } catch (error) {
      setConfigError(userErrorLabel(error, "读取 AI 员工配置失败"));
    } finally {
      setConfigLoading(false);
    }
  }, [canUseTauri]);

  useEffect(() => {
    void loadAgentConfig();
  }, [loadAgentConfig]);

  const executorOptions = useMemo(() => buildExecutorOptions(configState), [configState]);
  const coreAgents = useMemo(() => buildCoreAgentPool(agents, configState), [agents, configState]);
  const totalConfigured = canUseTauri
    ? coreAgents.filter((agent) => agent.model).length
    : coreAgents.filter((agent) => choices[agent.id]).length;
  const expertStatusById = useMemo(
    () => canUseTauri ? buildRemoteExpertStatus(EXPERT_RECOMMENDATIONS, configState) : expertStatus,
    [canUseTauri, configState, expertStatus],
  );
  const acceptedExperts = EXPERT_RECOMMENDATIONS.filter((expert) => expertStatusById[expert.id] === "accepted");
  const connectedCoreCount = coreAgents.filter((agent) => agent.poolStatus === "connected").length;
  const activeProjectMemberCount = coreAgents.length + acceptedExperts.length;
  const readyExecutorCount = executorOptions.filter((executor) => executor.status === "ready").length;
  const gatewayExecutorCount = executorOptions.filter((executor) => executor.source === "gateway").length;

  const executorByKey = useMemo(
    () => new Map(executorOptions.map((executor) => [executor.key, executor])),
    [executorOptions],
  );
  const selectedExecutor = executorByKey.get(selectedExecutorKey) ?? executorOptions[0] ?? EXECUTOR_OPTIONS[0];
  const activeChoices = canUseTauri ? {} : choices;
  const selectedExecutorAgents = coreAgents.filter((agent) => getChoice(agent, activeChoices, executorOptions).executor === selectedExecutor.key);
  const selectedExecutorConfig = getExecutorConfig(selectedExecutor, selectedExecutorAgents, executorConfig, configState);

  const saveChoice = (agentId: string, choice: AgentModelChoice) => {
    const next = { ...choices, [agentId]: choice };
    setChoices(next);
    window.localStorage.setItem(CHOICES_STORAGE_KEY, JSON.stringify(next));
  };

  const saveExpertStatus = async (expertId: string, status: ExpertStatus) => {
    // Tauri 模式：专家加入/移出走 project_agents 后端
    if (canUseTauri) {
      const expert = EXPERT_RECOMMENDATIONS.find((e) => e.id === expertId);
      if (!expert) return;
      setSavingKey(`expert:${expertId}`);
      setConfigError(null);
      try {
        if (status === "accepted") {
          // 找到对应核心模板或专家模板，加入项目
          const template = configState?.templates.find(
            (t) => t.category === "expert" && isExpertMatch(t, expert),
          ) ?? configState?.templates.find(
            (t) => t.role === normalizeExpertRole(expert.role),
          );
          if (template) {
            const model = configState?.models.find(
              (m) => m.executor_key === expert.executor && m.model_id === expert.model && m.enabled,
            );
            await upsertProjectAgent({
              agent_template_id: template.id,
              name: expert.name,
              role: template.role,
              source: "recommended",
              executor_key: expert.executor,
              model_id: model?.model_id ?? null,
              module_scope: template.module_scope,
              status: "active",
            });
          }
        } else if (status === "suggested" || status === "ignored") {
          // 查找并软移除
          const existingAgent = configState?.projectAgents.find(
            (pa) => pa.name === expert.name && pa.source === "recommended" && pa.removed_at === null,
          );
          if (existingAgent) {
            await removeProjectAgent(existingAgent.id);
          }
        }
        await loadAgentConfig();
      } catch (error) {
        setConfigError(userErrorLabel(error, "专家操作失败"));
      } finally {
        setSavingKey(null);
      }
      return;
    }

    // 浏览器预览模式：仍用 localStorage
    const next = { ...expertStatus, [expertId]: status };
    setExpertStatus(next);
    window.localStorage.setItem(EXPERTS_STORAGE_KEY, JSON.stringify(next));
  };

  const persistProjectAgent = async (agent: CoreAgentView, choice: AgentModelChoice) => {
    if (!canUseTauri) {
      saveChoice(agent.id, choice);
      return;
    }
    if (!agent.agentTemplateId) {
      setConfigError("当前项目 Agent 缺少模板 ID，无法写入真实配置。");
      return;
    }
    const key = `agent:${agent.id}`;
    setSavingKey(key);
    setConfigError(null);
    try {
      const model = selectedModelForExecutor(choice.executor, choice.model, configState);
      await upsertProjectAgent({
        agent_template_id: agent.agentTemplateId,
        name: agentNameLabel(agent.name),
        role: agent.role,
        source: agent.source ?? "core",
        executor_key: choice.executor,
        model_id: model?.model_id ?? null,
        module_scope: agent.moduleScope ?? agent.role,
        status: agent.status === "removed" ? "idle" : normalizeProjectAgentStatus(agent.status),
      });
      await loadAgentConfig();
    } catch (error) {
      setConfigError(userErrorLabel(error, "保存智能体绑定失败"));
    } finally {
      setSavingKey(null);
    }
  };

  const updateExecutor = (agent: CoreAgentView, executorKey: ExecutorKey) => {
    const executor = executorByKey.get(executorKey) ?? EXECUTOR_OPTIONS[0];
    void persistProjectAgent(agent, {
      executor: executor.key,
      model: executor.models[0],
    });
  };

  const updateModel = (agent: CoreAgentView, model: string) => {
    const current = getChoice(agent, choices, executorOptions);
    void persistProjectAgent(agent, {
      ...current,
      model,
    });
  };

  const saveExecutorConfig = (next: ExecutorLocalConfigMap) => {
    // Tauri 模式：不写 localStorage，执行器配置通过 Tauri commands 管理
    if (canUseTauri) {
      setExecutorConfig(next);
      return;
    }
    setExecutorConfig(next);
    window.localStorage.setItem(EXECUTOR_CONFIG_STORAGE_KEY, JSON.stringify(next));
  };

  const updateExecutorConfigList = (
    field: keyof ExecutorLocalConfig,
    updater: (items: string[]) => string[],
  ) => {
    const current = getExecutorConfig(selectedExecutor, selectedExecutorAgents, executorConfig, configState);
    saveExecutorConfig({
      ...executorConfig,
      [selectedExecutor.key]: {
        ...current,
        [field]: updater(current[field]),
      },
    });
  };

  const addExecutorConfigItem = (field: keyof ExecutorLocalConfig) => {
    const draftKey = `${selectedExecutor.key}:${field}`;
    const value = (executorDrafts[draftKey] ?? "").trim();
    if (!value) return;
    if (canUseTauri) {
      void addRemoteExecutorConfigItem(field, value, draftKey);
      return;
    }
    updateExecutorConfigList(field, (items) => (items.includes(value) ? items : [...items, value]));
    setExecutorDrafts((current) => ({ ...current, [draftKey]: "" }));
  };

  const removeExecutorConfigItem = (field: keyof ExecutorLocalConfig, value: string) => {
    if (canUseTauri) {
      void removeRemoteExecutorConfigItem(field, value);
      return;
    }
    updateExecutorConfigList(field, (items) => items.filter((item) => item !== value));
  };

  const addRemoteExecutorConfigItem = async (
    field: keyof ExecutorLocalConfig,
    value: string,
    draftKey: string,
  ) => {
    const key = `${selectedExecutor.key}:${field}:add`;
    setSavingKey(key);
    setConfigError(null);
    try {
      if (field === "models") {
        await upsertExecutorModel({
          executor_key: selectedExecutor.key,
          provider: "openai_compat",
          model_id: value,
          display_name: modelLabel(value),
          purpose: "agent_task",
          enabled: true,
        });
      } else if (field === "skills") {
        await upsertExecutorSkill({
          executor_key: selectedExecutor.key,
          skill_name: value,
          skill_scope: selectedExecutor.key,
          enabled: true,
        });
      } else {
        setConfigError("请在下方员工卡片里调整执行器绑定。");
        return;
      }
      setExecutorDrafts((current) => ({ ...current, [draftKey]: "" }));
      await loadAgentConfig();
    } catch (error) {
      setConfigError(userErrorLabel(error, "保存执行器配置失败"));
    } finally {
      setSavingKey(null);
    }
  };

  const removeRemoteExecutorConfigItem = async (field: keyof ExecutorLocalConfig, value: string) => {
    const key = `${selectedExecutor.key}:${field}:remove`;
    setSavingKey(key);
    setConfigError(null);
    try {
      if (field === "models") {
        const model = configState?.models.find((entry) => entry.executor_key === selectedExecutor.key && entry.model_id === value);
        if (model) {
          await deleteExecutorModel(model.id);
        }
      } else if (field === "skills") {
        const skill = configState?.skills.find((entry) => entry.executor_key === selectedExecutor.key && entry.skill_name === value);
        if (skill) {
          await deleteExecutorSkill(skill.id);
        }
      } else {
        setConfigError("请在下方员工卡片里调整执行器绑定。");
        return;
      }
      await loadAgentConfig();
    } catch (error) {
      setConfigError(userErrorLabel(error, "删除执行器配置失败"));
    } finally {
      setSavingKey(null);
    }
  };

  return (
    <div className="agents-page">
      <header className="agents-page__heading">
        <div>
          <span>AI 员工</span>
          <h1>智能体模型编排</h1>
          <p>{canUseTauri ? "核心员工、执行器、模型和 Skill 已从本地 SQLite 读取；敏感密钥仍只在系统设置和安全存储里处理。" : "浏览器预览只展示示例配置；真实保存请启动 Tauri 桌面宿主。"}</p>
        </div>
        <div className="agents-page__summary">
          <strong>{activeProjectMemberCount}</strong>
          <span>全技术栈员工池</span>
        </div>
      </header>

      <section className="agents-overview-grid" aria-label="AI 员工配置概览">
        <article className="agents-overview-card">
          <span>员工池</span>
          <strong>{connectedCoreCount}/{coreAgents.length}</strong>
          <p>已接入真实 Agent，其余先作为固定岗位模板保留。</p>
        </article>
        <article className="agents-overview-card">
          <span>本页配置</span>
          <strong>{totalConfigured}</strong>
          <p>{canUseTauri ? "已绑定模型的项目 Agent 数量，保存会写入本地 SQLite。" : "已手动调整执行器或模型的员工数量，暂存本地浏览器。"}</p>
        </article>
        <article className="agents-overview-card is-ready">
          <span>可调用执行器</span>
          <strong>{readyExecutorCount}</strong>
          <p>Codex 官方 CLI 与 Claude 已验证，后续进入系统设置。</p>
        </article>
        <article className="agents-overview-card">
          <span>模型网关</span>
          <strong>{gatewayExecutorCount}</strong>
          <p>provider、base URL、key、模型目录后续统一从这里管理。</p>
        </article>
      </section>

      {(configLoading || configError) && (
        <section className="controller-rule-panel" aria-label="AI 员工配置状态">
          <div className="controller-rule-panel__icon">
            <Settings2 size={20} aria-hidden="true" />
          </div>
          <div>
            <strong>{configLoading ? "正在读取真实配置" : "真实配置读取失败"}</strong>
            <p>{configLoading ? "正在通过 Tauri commands 读取执行器、模型目录、Agent 模板、项目成员和 Skill。" : configError}</p>
          </div>
        </section>
      )}

      <section className="controller-rule-panel" aria-label="总控调度规则">
        <div className="controller-rule-panel__icon">
          <Bot size={20} aria-hidden="true" />
        </div>
        <div>
          <strong>总控调度规则</strong>
          <p>所有角色只在自己的固定模块内发挥。跨模块任务必须由总控拆分或转派；高风险动作必须进入审批，不能由单个 Agent 自行越权执行。</p>
        </div>
      </section>

      {canUseTauri && configState && configState.boundaryChecks.length > 0 && (
        <section className="controller-rule-panel" aria-label="最近边界检查记录">
          <div className="controller-rule-panel__icon">
            <Shield size={20} aria-hidden="true" />
          </div>
          <div>
            <strong>最近边界检查记录（只读）</strong>
            <p>
              {configState.boundaryChecks.slice(0, 5).map((check) => (
                <span key={check.id} className={`boundary-check-tag is-${check.decision}`}>
                  {check.decision === "allowed" ? "✓ 允许" : check.decision === "denied" ? "✗ 拒绝" : "⚠ 需审批"}
                  {" "}{check.reason}
                </span>
              ))}
            </p>
          </div>
        </section>
      )}

      <section className="project-expert-panel" aria-label="项目专家推荐">
        <div className="section-heading">
          <div>
            <span>项目专家推荐</span>
            <h2>适合当前项目的专家 Agent</h2>
          </div>
          <strong>{acceptedExperts.length} 已加入</strong>
        </div>

        <div className="expert-recommendation-grid">
          {EXPERT_RECOMMENDATIONS.map((expert) => {
            const status = expertStatusById[expert.id] ?? "suggested";
            const executor = executorByKey.get(expert.executor) ?? EXECUTOR_OPTIONS[0];

            return (
              <article className={`expert-card is-${status}`} key={expert.id}>
                <div className="expert-card__top">
                  <div className="expert-card__avatar">
                    <UserCheck size={18} aria-hidden="true" />
                  </div>
                  <div>
                    <h3>{expert.name}</h3>
                    <p>{expert.role}</p>
                  </div>
                  <span>{expert.priority}</span>
                </div>

                <p className="expert-card__reason">{expert.reason}</p>

                <div className="expert-card__skills">
                  {expert.skills.map((skill) => (
                    <span key={skill}>{skill}</span>
                  ))}
                </div>

                <div className="expert-card__meta">
                  <span>{executor.label}</span>
                  <span>{modelLabel(expert.model)}</span>
                  <span>{sourceLabel(executor.source)}</span>
                </div>

                <div className="expert-card__actions">
                  {status === "accepted" ? (
                    <button type="button" className="is-muted" onClick={() => saveExpertStatus(expert.id, "suggested")}>
                      <X size={14} aria-hidden="true" />
                      移出项目
                    </button>
                  ) : (
                    <button type="button" onClick={() => saveExpertStatus(expert.id, "accepted")}>
                      <Plus size={14} aria-hidden="true" />
                      加入项目
                    </button>
                  )}
                  {status === "ignored" ? (
                    <button type="button" className="is-muted" onClick={() => saveExpertStatus(expert.id, "suggested")}>
                      恢复推荐
                    </button>
                  ) : (
                    <button type="button" className="is-muted" onClick={() => saveExpertStatus(expert.id, "ignored")}>
                      暂不需要
                    </button>
                  )}
                </div>
              </article>
            );
          })}
        </div>
      </section>

      <section className="project-members-panel" aria-label="当前项目成员">
        <div className="section-heading">
          <div>
            <span>项目成员</span>
            <h2>已经参与当前项目的 Agent</h2>
          </div>
          <strong>{activeProjectMemberCount} 个成员</strong>
        </div>

        <div className="project-member-strip">
          {coreAgents.map((agent) => (
            <article className="project-member-chip" key={agent.id}>
              <Bot size={15} aria-hidden="true" />
              <div>
                <strong>{agentNameLabel(agent.name)}</strong>
                <span>{agent.poolStatus === "connected" ? "已接入" : "待接入模板"} / {roleLabel(agent.role)}</span>
              </div>
            </article>
          ))}
          {acceptedExperts.map((expert) => (
            <article className="project-member-chip is-expert" key={expert.id}>
              <UserCheck size={15} aria-hidden="true" />
              <div>
                <strong>{expert.name}</strong>
                <span>项目专家 / {expert.role}</span>
              </div>
            </article>
          ))}
        </div>
      </section>

      <section className="executor-config-workbench" aria-label="执行器配置">
        <div className="section-heading section-heading--wide">
          <div>
            <span>执行器配置</span>
            <h2>执行器 / 模型 / 智能体目录</h2>
          </div>
          <strong>{readyExecutorCount} 已验证 / {executorOptions.length} 执行器</strong>
        </div>

        <div className="executor-config-layout">
          <aside className="executor-config-tree" aria-label="执行器配置树">
            {executorOptions.map((executor) => {
              const isActive = executor.key === selectedExecutor.key;
              const executorAgentCount = coreAgents.filter((agent) => getChoice(agent, activeChoices, executorOptions).executor === executor.key).length;

              return (
                <div className={`executor-tree-group${isActive ? " is-active" : ""}`} key={executor.key}>
                  <button
                    type="button"
                    className="executor-tree-group__head"
                    onClick={() => {
                      setSelectedExecutorKey(executor.key);
                    }}
                  >
                    {isActive ? <ChevronDown size={14} aria-hidden="true" /> : <ChevronRight size={14} aria-hidden="true" />}
                    <strong>{executor.label}</strong>
                    <span>{executor.models.length + executorAgentCount}</span>
                  </button>

                  {isActive ? (
                    <div className="executor-tree-group__items">
                      <div className="is-selected">
                        <Settings2 size={14} aria-hidden="true" />
                        <span>配置</span>
                        <em>{executor.models.length + executorAgentCount}</em>
                      </div>
                    </div>
                  ) : null}
                </div>
              );
            })}
          </aside>

          <article className={`executor-config-detail is-${selectedExecutor.status}`}>
            <div className="executor-config-detail__top">
              <div className="executor-config-detail__icon">
              <Cpu size={18} aria-hidden="true" />
              </div>
              <div>
                <span>配置</span>
                <h3>{selectedExecutor.label}</h3>
                <p>{selectedExecutor.note}</p>
              </div>
              <strong>{executorStatusLabel(selectedExecutor.status)}</strong>
            </div>

            <div className="executor-config-detail__grid">
              <div>
                <span>调用来源</span>
                <strong>{sourceLabel(selectedExecutor.source)}</strong>
              </div>
              <div>
                <span>可选模型</span>
                <strong>{selectedExecutorConfig.models.length} 个</strong>
              </div>
              <div>
                <span>绑定员工</span>
                <strong>{selectedExecutorConfig.agents.length} 个</strong>
              </div>
            </div>

            <div className="executor-config-detail__sections">
              <div className="executor-config-detail__panel">
                <h4>执行器配置</h4>
                <p>
                  {selectedExecutor.source === "gateway"
                    ? "系统设置里配置 provider、base URL 和 key；模型网关保存可用模型目录。密钥不进数据库、不进日志、不显示给前端。"
                    : "外部执行器走本机命令或官方客户端配置；Codex 后续使用显式 exe 路径，Claude 跟随自身配置。"}
                </p>
                <div className="executor-connection-form" aria-label="模型连接配置">
                  <label>
                    <span>Provider</span>
                    <input
                      placeholder={selectedExecutor.source === "gateway" ? "openai-compatible / deepseek / google" : selectedExecutor.key}
                    />
                  </label>
                  <label>
                    <span>{selectedExecutor.source === "gateway" ? "Base URL" : "执行器路径"}</span>
                    <input
                      placeholder={
                        selectedExecutor.source === "gateway"
                          ? "https://api.example.com/v1"
                          : "C:\\Users\\...\\codex.exe"
                      }
                    />
                  </label>
                  <label>
                    <span>API Key / Token</span>
                    <input type="password" placeholder="只用于连接测试，不在前端保存" autoComplete="off" />
                  </label>
                </div>
              </div>

              <div className="executor-config-detail__panel">
                <h4>模型目录</h4>
                <ConfigListEditor
                  items={selectedExecutorConfig.models}
                  placeholder="输入模型 ID"
                  draftValue={executorDrafts[`${selectedExecutor.key}:models`] ?? ""}
                  labelForItem={modelLabel}
                  onDraftChange={(value) => setExecutorDrafts((current) => ({ ...current, [`${selectedExecutor.key}:models`]: value }))}
                  onAdd={() => addExecutorConfigItem("models")}
                  onRemove={(value) => removeExecutorConfigItem("models", value)}
                  disabled={Boolean(savingKey)}
                />
              </div>

              <div className="executor-config-detail__panel">
                <h4>绑定智能体</h4>
                <ConfigListEditor
                  items={selectedExecutorConfig.agents}
                  placeholder="输入智能体名称"
                  draftValue={executorDrafts[`${selectedExecutor.key}:agents`] ?? ""}
                  emptyLabel="暂无员工绑定到这个执行器"
                  onDraftChange={(value) => setExecutorDrafts((current) => ({ ...current, [`${selectedExecutor.key}:agents`]: value }))}
                  onAdd={() => addExecutorConfigItem("agents")}
                  onRemove={(value) => removeExecutorConfigItem("agents", value)}
                  disabled={canUseTauri || Boolean(savingKey)}
                />
              </div>

              <div className="executor-config-detail__panel">
                <h4>Skill 配置</h4>
                <ConfigListEditor
                  items={selectedExecutorConfig.skills}
                  placeholder="输入 Skill 名称"
                  draftValue={executorDrafts[`${selectedExecutor.key}:skills`] ?? ""}
                  emptyLabel="暂无 Skill 配置"
                  onDraftChange={(value) => setExecutorDrafts((current) => ({ ...current, [`${selectedExecutor.key}:skills`]: value }))}
                  onAdd={() => addExecutorConfigItem("skills")}
                  onRemove={(value) => removeExecutorConfigItem("skills", value)}
                  disabled={Boolean(savingKey)}
                />
              </div>
            </div>
          </article>
        </div>
      </section>

      <section className="agent-model-grid" aria-label="智能体模型选择">
        <div className="section-heading section-heading--wide">
          <div>
            <span>核心员工模型</span>
            <h2>全技术栈固定员工池</h2>
          </div>
          <strong>{connectedCoreCount} 已接入 / {coreAgents.length} 员工</strong>
        </div>
        {coreAgents.map((agent) => {
          const choice = getChoice(agent, activeChoices, executorOptions);
          const executor = executorByKey.get(choice.executor) ?? EXECUTOR_OPTIONS[0];
          const agentSaving = savingKey === `agent:${agent.id}`;

          return (
            <article className="agent-model-card" key={agent.id}>
              <div className="agent-model-card__top">
                <div className="agent-model-card__avatar">
                  <Bot size={18} aria-hidden="true" />
                </div>
                <div>
                  <h2>{agentNameLabel(agent.name)}</h2>
                  <p>{roleLabel(agent.role)}</p>
                </div>
                <span className={`agent-model-card__status color-${agentStatusColor(agent.status)}`}>
                  {agent.poolStatus === "connected" ? statusLabel(agent.status) : "待接入"}
                </span>
              </div>

              <p className="agent-model-card__specialty">{agent.specialty}</p>
              <p className="agent-model-card__boundary">{agent.boundary}</p>

              <div className="agent-model-card__stack">
                {agent.stack.map((item) => (
                  <span key={item}>{item}</span>
                ))}
              </div>

              <div className="agent-model-card__source">
                <span>{sourceLabel(executor.source)}</span>
                <p>{executor.note}</p>
              </div>

              <div className="agent-model-card__controls">
                <label>
                  <span>执行器</span>
                  <select value={choice.executor} disabled={agentSaving} onChange={(event) => updateExecutor(agent, event.target.value)}>
                    {executorOptions.map((option) => (
                      <option value={option.key} key={option.key}>
                        {option.label}
                      </option>
                    ))}
                  </select>
                </label>

                <label>
                  <span>模型</span>
                  <select value={choice.model} disabled={agentSaving || executor.models.length === 0} onChange={(event) => updateModel(agent, event.target.value)}>
                    {executor.models.map((model) => (
                      <option value={model} key={model}>
                        {modelLabel(model)}
                      </option>
                    ))}
                  </select>
                </label>
              </div>

              <div className="agent-model-card__footer">
                <span>
                  <CheckCircle2 size={14} aria-hidden="true" />
                  {executor.label} / {executorStatusLabel(executor.status)}
                </span>
                <span className="agent-model-card__config-state">
                  <Settings2 size={14} aria-hidden="true" />
                  {canUseTauri ? (agent.removedAt ? "已软移除" : "真实配置") : agent.poolStatus === "connected" ? "只读预览" : "模板待接入"}
                </span>
              </div>
            </article>
          );
        })}
      </section>
    </div>
  );
}

type ConfigListEditorProps = {
  items: string[];
  placeholder: string;
  draftValue: string;
  disabled?: boolean;
  emptyLabel?: string;
  labelForItem?: (value: string) => string;
  onDraftChange: (value: string) => void;
  onAdd: () => void;
  onRemove: (value: string) => void;
};

function ConfigListEditor({
  items,
  placeholder,
  draftValue,
  disabled = false,
  emptyLabel = "暂无配置",
  labelForItem = (value) => value,
  onDraftChange,
  onAdd,
  onRemove,
}: ConfigListEditorProps) {
  return (
    <div className="config-list-editor">
      <div className="config-list-editor__input">
        <input
          value={draftValue}
          placeholder={placeholder}
          disabled={disabled}
          onChange={(event) => onDraftChange(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter") {
              event.preventDefault();
              onAdd();
            }
          }}
        />
        <button type="button" onClick={onAdd} aria-label="新增" disabled={disabled}>
          <Plus size={14} aria-hidden="true" />
        </button>
      </div>

      <div className="executor-config-detail__chips">
        {items.length === 0 ? (
          <span>{emptyLabel}</span>
        ) : (
          items.map((item) => (
            <span className="is-removable" key={item}>
              {labelForItem(item)}
              <button type="button" onClick={() => onRemove(item)} aria-label={`删除 ${labelForItem(item)}`} disabled={disabled}>
                <X size={11} aria-hidden="true" />
              </button>
            </span>
          ))
        )}
      </div>
    </div>
  );
}

function buildExecutorOptions(configState: AgentConfigState | null): ExecutorOption[] {
  if (!configState) {
    return EXECUTOR_OPTIONS;
  }
  const modelsByExecutor = groupBy(configState.models, (model) => model.executor_key);
  return configState.executors.map((executor) => {
    const models = (modelsByExecutor.get(executor.key) ?? [])
      .filter((model) => model.enabled)
      .map((model) => model.model_id);
    return {
      key: executor.key,
      label: executor.label,
      count: models.length,
      models,
      status: executor.status === "active" ? "ready" : executor.status === "error" ? "missing" : "planned",
      source: executor.kind === "model_gateway" ? "gateway" : "external",
      note: executor.provider
        ? `${executor.provider} / ${executor.base_url_status}`
        : executor.executable_path ?? executor.base_url_status,
    };
  });
}

function buildCoreAgentPool(agents: AgentSummary[], configState: AgentConfigState | null): CoreAgentView[] {
  if (configState) {
    const templateById = new Map(configState.templates.map((template) => [template.id, template]));
    return configState.projectAgents.map((projectAgent) => {
      const template = templateById.get(projectAgent.agent_template_id);
      return {
        id: projectAgent.id,
        project_id: projectAgent.project_id,
        name: projectAgent.name,
        role: projectAgent.role,
        status: projectAgent.status,
        model: projectAgent.model_id,
        permissions: template?.allowed_task_types ?? [],
        created_at: projectAgent.created_at,
        updated_at: projectAgent.updated_at,
        poolStatus: "connected",
        specialty: template?.specialty ?? roleLabel(projectAgent.role),
        boundary: buildBoundaryText(template),
        stack: splitStack(template?.stack),
        defaultExecutor: projectAgent.executor_key || template?.default_executor_key || DEFAULT_EXECUTOR,
        defaultModel: projectAgent.model_id || template?.default_model_id || "",
        agentTemplateId: projectAgent.agent_template_id,
        source: projectAgent.source,
        moduleScope: projectAgent.module_scope,
        removedAt: projectAgent.removed_at,
      };
    });
  }

  const byRole = new Map(agents.map((agent) => [agent.role, agent]));
  const byId = new Map(agents.map((agent) => [agent.id, agent]));

  return CORE_AGENT_TEMPLATES.map((template) => {
    const connected = byId.get(template.id) ?? byRole.get(template.role);

    if (connected) {
      return {
        ...connected,
        poolStatus: "connected",
        specialty: template.specialty,
        boundary: template.boundary,
        stack: template.stack,
        defaultExecutor: template.defaultExecutor,
        defaultModel: template.defaultModel,
      };
    }

    return {
      id: template.id,
      project_id: "core_agent_template",
      name: template.name,
      role: template.role,
      status: "template",
      model: template.defaultModel,
      permissions: ["read_project"],
      created_at: "",
      updated_at: "",
      poolStatus: "template",
      specialty: template.specialty,
      boundary: template.boundary,
      stack: template.stack,
      defaultExecutor: template.defaultExecutor,
      defaultModel: template.defaultModel,
    };
  });
}

function getChoice(
  agent: CoreAgentView,
  choices: Record<string, AgentModelChoice>,
  executorOptions: ExecutorOption[],
): AgentModelChoice {
  const saved = choices[agent.id];
  if (saved && executorOptions.some((executor) => executor.key === saved.executor && executor.models.includes(saved.model))) {
    return saved;
  }

  const matched = executorOptions.find((executor) => agent.model && executor.models.includes(agent.model));
  const executor =
    matched ??
    executorOptions.find((option) => option.key === agent.defaultExecutor) ??
    executorOptions.find((option) => option.key === DEFAULT_EXECUTOR) ??
    executorOptions[0] ??
    EXECUTOR_OPTIONS[0];

  return {
    executor: executor.key,
    model: agent.model && executor.models.includes(agent.model) ? agent.model : executor.models[0] ?? agent.defaultModel,
  };
}

function executorStatusLabel(status: ExecutorOption["status"]): string {
  const map: Record<ExecutorOption["status"], string> = {
    ready: "已验证",
    planned: "待接入",
    missing: "待配置",
  };
  return map[status];
}

function sourceLabel(source: ExecutorOption["source"]): string {
  const map: Record<ExecutorOption["source"], string> = {
    external: "跟随执行器配置",
    gateway: "通过模型网关",
  };
  return map[source];
}

function loadChoices(): Record<string, AgentModelChoice> {
  try {
    const raw = window.localStorage.getItem(CHOICES_STORAGE_KEY);
    if (!raw) {
      return {};
    }

    const parsed = JSON.parse(raw) as Record<string, AgentModelChoice>;
    return Object.fromEntries(
      Object.entries(parsed).filter(([, choice]) =>
        EXECUTOR_OPTIONS.some((executor) => executor.key === choice.executor && executor.models.includes(choice.model)),
      ),
    );
  } catch {
    return {};
  }
}

function getExecutorConfig(
  executor: ExecutorOption,
  assignedAgents: CoreAgentView[],
  savedConfig: ExecutorLocalConfigMap,
  configState: AgentConfigState | null,
): ExecutorLocalConfig {
  if (configState) {
    return {
      models: configState.models
        .filter((model) => model.executor_key === executor.key)
        .map((model) => model.model_id),
      agents: assignedAgents.map((agent) => agentNameLabel(agent.name)),
      skills: configState.skills
        .filter((skill) => skill.executor_key === executor.key)
        .map((skill) => skill.skill_name),
    };
  }
  const saved = savedConfig[executor.key];
  return {
    models: saved?.models ?? executor.models,
    agents: saved?.agents ?? assignedAgents.map((agent) => agentNameLabel(agent.name)),
    skills: saved?.skills ?? defaultSkillsForExecutor(executor),
  };
}

function selectedModelForExecutor(
  executorKey: string,
  modelId: string,
  configState: AgentConfigState | null,
): ExecutorModelSummary | undefined {
  return configState?.models.find((model) =>
    model.executor_key === executorKey &&
    model.model_id === modelId &&
    model.enabled
  );
}

function normalizeProjectAgentStatus(status: string): ProjectAgentSummary["status"] {
  if (status === "active" || status === "idle" || status === "disabled" || status === "removed") {
    return status;
  }
  if (status === "running") {
    return "active";
  }
  return "idle";
}

function buildBoundaryText(template: AgentTemplateSummary | undefined): string {
  if (!template) {
    return "按项目成员配置执行；后续 Runner 会继续做边界校验。";
  }
  const allowed = template.allowed_paths.length > 0 ? `允许路径：${template.allowed_paths.join("、")}` : "允许路径待配置";
  const forbidden = template.forbidden_actions.length > 0 ? `禁止动作：${template.forbidden_actions.join("、")}` : "禁止动作待配置";
  return `${allowed}；${forbidden}`;
}

function splitStack(stack: string | null | undefined): string[] {
  if (!stack) {
    return ["未配置"];
  }
  return stack.split(/[,\s/]+/).map((item) => item.trim()).filter(Boolean);
}

function groupBy<T>(items: T[], keyOf: (item: T) => string): Map<string, T[]> {
  const map = new Map<string, T[]>();
  for (const item of items) {
    const key = keyOf(item);
    const group = map.get(key) ?? [];
    group.push(item);
    map.set(key, group);
  }
  return map;
}

function defaultSkillsForExecutor(executor: ExecutorOption): string[] {
  if (executor.key === "codex") return ["代码实现", "文件修改", "构建检查"];
  if (executor.key === "claude") return ["需求分析", "文档整理", "方案审查"];
  if (executor.source === "gateway") return ["模型调用", "任务拆解"];
  return [];
}

function loadExecutorConfig(): ExecutorLocalConfigMap {
  try {
    const raw = window.localStorage.getItem(EXECUTOR_CONFIG_STORAGE_KEY);
    if (!raw) {
      return {};
    }

    const parsed = JSON.parse(raw) as ExecutorLocalConfigMap;
    return Object.fromEntries(
      Object.entries(parsed).filter(([key, value]) =>
        EXECUTOR_OPTIONS.some((executor) => executor.key === key) &&
        Array.isArray(value?.models) &&
        Array.isArray(value?.agents) &&
        Array.isArray(value?.skills),
      ),
    ) as ExecutorLocalConfigMap;
  } catch {
    return {};
  }
}

function loadExpertStatus(): Record<string, ExpertStatus> {
  try {
    const raw = window.localStorage.getItem(EXPERTS_STORAGE_KEY);
    if (!raw) {
      return {};
    }

    const parsed = JSON.parse(raw) as Record<string, ExpertStatus>;
    return Object.fromEntries(
      Object.entries(parsed).filter(([, status]) =>
        status === "suggested" || status === "accepted" || status === "ignored",
      ),
    );
  } catch {
    return {};
  }
}

function buildRemoteExpertStatus(
  experts: ExpertRecommendation[],
  configState: AgentConfigState | null,
): Record<string, ExpertStatus> {
  const statuses: Record<string, ExpertStatus> = {};
  if (!configState) {
    return statuses;
  }

  for (const expert of experts) {
    const template = configState.templates.find((entry) =>
      entry.category === "expert" && isExpertMatch(entry, expert)
    ) ?? configState.templates.find((entry) => entry.role === normalizeExpertRole(expert.role));
    const activeAgent = configState.projectAgents.find((agent) =>
      agent.source === "recommended" &&
      agent.removed_at === null &&
      (
        agent.name === expert.name ||
        (template ? agent.agent_template_id === template.id : false)
      )
    );
    statuses[expert.id] = activeAgent ? "accepted" : "suggested";
  }

  return statuses;
}

/** 将专家推荐匹配到 agent_templates 中的专家模板 */
function isExpertMatch(template: AgentTemplateSummary, expert: ExpertRecommendation): boolean {
  if (template.category !== "expert") return false;
  const expertRole = normalizeExpertRole(expert.role);
  if (template.role === expertRole) return true;
  if (template.name.includes(expert.name) || expert.name.includes(template.name)) return true;
  return false;
}

/** 将页面显示的角色名归一化为模板 role */
function normalizeExpertRole(displayRole: string): string {
  const map: Record<string, string> = {
    "界面体验专家": "ux",
    "Tauri / 本地能力专家": "desktop",
    "Runner 安全专家": "security",
    "数据与状态专家": "data",
  };
  return map[displayRole] ?? displayRole;
}
