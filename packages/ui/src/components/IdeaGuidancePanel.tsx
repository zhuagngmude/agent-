// 阶段 37：想法引导官面板
// 权限级别 L1（模型草案），不执行 Runner、不写文件、不改 Git。
// L1 不是免确认：真实模型调用必须二次确认。
// 三步流程：输入想法 → 二次确认 → AI 追问 → 用户回答 → 二次确认 → 生成项目种子

import { useEffect, useMemo, useState } from "react";
import {
  Alert,
  Button,
  Card,
  Checkbox,
  Divider,
  Form,
  Input,
  List,
  Space,
  Tag,
  Typography,
  Descriptions,
  App,
} from "antd";
import { BulbOutlined, QuestionCircleOutlined, FileTextOutlined } from "@ant-design/icons";
import type {
  CreateIdeaGuidanceQuestionsInput,
  CreateIdeaGuidanceQuestionsResponse,
  GenerateProjectSeedInput,
  GenerateProjectSeedResponse,
  IdeaGuidanceQuestion,
  ProjectSeed,
} from "@agent-swarm/shared";
import {
  createIdeaGuidanceQuestions,
  generateProjectSeed,
  saveGuidanceAnswers,
  isTauriHost,
} from "../utils/desktopHost";

const { TextArea } = Input;
const { Title, Text, Paragraph } = Typography;

type Step = "input" | "questions" | "seed";

export type IdeaGuidanceHandoff = {
  idea: string;
  projectTypeLabel?: string | null;
  reason?: string | null;
  questions?: string[];
};

export type ProjectSeedDraftPayload = {
  idea: string;
  constraints: string;
};

type IdeaGuidancePanelProps = {
  canWrite: boolean;
  handoff?: IdeaGuidanceHandoff | null;
  onSeedReadyForDraft?: (payload: ProjectSeedDraftPayload) => void;
};

/** 返回粗粒度中文错误信息，不泄露 raw 后端错误细节 */
function toUserError(e: unknown): string {
  const msg = e instanceof Error ? e.message : String(e);
  if (msg.includes("feature_disabled")) {
    return "真实模型调用功能未启用，请检查环境变量配置";
  }
  if (msg.includes("provider_config_error") || msg.includes("missing_key")) {
    return "模型服务配置异常，请检查桌面宿主环境变量";
  }
  if (msg.includes("provider_error") || msg.includes("timeout") || msg.includes("network_error")) {
    return "模型调用失败，请稍后重试或检查网络连接";
  }
  if (msg.includes("invalid_input") || msg.includes("second_confirm")) {
    return "输入校验未通过，请检查确认信息";
  }
  if (msg.includes("invalid_response")) {
    return "模型返回格式异常，请重试";
  }
  if (msg.includes("audit_write_failed")) {
    return "安全审计写入失败，操作已被拒绝";
  }
  if (msg.includes("not_found")) {
    return "会话记录不存在，请重新开始";
  }
  return "操作失败，请检查桌面宿主或模型配置";
}

function buildOfflineQuestions(idea: string, handoff?: IdeaGuidanceHandoff | null): IdeaGuidanceQuestion[] {
  const sourceQuestions =
    handoff?.questions && handoff.questions.length > 0
      ? handoff.questions
      : [
          "这个项目第一版最想解决的一个问题是什么？",
          "目标用户是谁？他们现在为什么需要它？",
          "第一版必须包含哪些功能？哪些明确不做？",
          "你希望它运行在桌面端、网页端，还是先只做本地工具？",
          "你判断第一版成功的验收标准是什么？",
        ];

  const now = new Date().toISOString();
  return sourceQuestions.slice(0, 8).map((question, index) => ({
    id: `offline_question_${index + 1}`,
    project_id: "offline_preview",
    session_id: `offline_session_${Date.now()}`,
    sort_order: index + 1,
    question,
    answer: null,
    status: "pending",
    created_at: now,
    updated_at: now,
  }));
}

function buildOfflineSeed(
  idea: string,
  constraints: string,
  questions: IdeaGuidanceQuestion[],
  answers: Record<string, string>,
  handoff?: IdeaGuidanceHandoff | null,
): ProjectSeed {
  const now = new Date().toISOString();
  const answered = questions
    .map((question) => {
      const answer = answers[question.id]?.trim();
      return answer ? `${question.question}：${answer}` : null;
    })
    .filter((item): item is string => Boolean(item));
  const features = answered.length > 0 ? answered.slice(0, 4) : ["澄清目标", "生成计划草案", "拆解任务", "受控执行预演"];

  return {
    id: `offline_seed_${Date.now()}`,
    project_id: "offline_preview",
    session_id: "offline_session",
    status: "ready",
    product_goal: idea,
    target_users: answered[1] ?? "待进一步明确目标用户",
    mvp_scope: answered[2] ?? "先完成最小可用闭环，再扩展自动化能力",
    non_goals: constraints || "第一版不自动改 Git、不执行自由命令、不绕过审批链",
    key_features: JSON.stringify(features),
    pages_or_modules: JSON.stringify(["总控对话", "想法澄清", "项目计划草案", "任务分配"]),
    data_entities: JSON.stringify(["项目种子", "计划草案", "任务", "智能体"]),
    technical_constraints: constraints || "复用现有桌面端、SQLite、Tauri command 和受控模型目录",
    acceptance_criteria: "可以从总控输入想法，完成澄清，生成项目种子，并继续创建项目计划草案",
    risk_points: "离线模式仅用于体验流程，不写入模型审计，不代表真实模型输出",
    open_questions: answered.length > 0 ? answered.join("\n") : "仍需继续补充关键问题答案",
    recommended_next_step: handoff?.reason ?? "用这个项目种子创建项目计划草案",
    model_call_id: null,
    created_at: now,
    updated_at: now,
  };
}

function buildDraftPayloadFromSeed(seed: ProjectSeed): ProjectSeedDraftPayload {
  const idea = seed.product_goal?.trim() || seed.recommended_next_step?.trim() || "根据项目种子创建项目计划";
  const constraints = [
    seed.target_users ? `目标用户：${seed.target_users}` : null,
    seed.mvp_scope ? `MVP 范围：${seed.mvp_scope}` : null,
    seed.non_goals ? `明确不做：${seed.non_goals}` : null,
    seed.key_features ? `核心功能：${tryParseJsonArray(seed.key_features).join("、")}` : null,
    seed.pages_or_modules ? `页面/模块：${tryParseJsonArray(seed.pages_or_modules).join("、")}` : null,
    seed.data_entities ? `数据实体：${tryParseJsonArray(seed.data_entities).join("、")}` : null,
    seed.technical_constraints ? `技术约束：${seed.technical_constraints}` : null,
    seed.acceptance_criteria ? `验收标准：${seed.acceptance_criteria}` : null,
    seed.risk_points ? `风险点：${seed.risk_points}` : null,
    seed.open_questions ? `待澄清问题：${seed.open_questions}` : null,
  ].filter((item): item is string => Boolean(item));

  return {
    idea: idea.slice(0, 500),
    constraints: constraints.join("\n").slice(0, 2000),
  };
}

export default function IdeaGuidancePanel({ canWrite, handoff, onSeedReadyForDraft }: IdeaGuidancePanelProps) {
  const { message } = App.useApp();
  const [step, setStep] = useState<Step>("input");
  const [idea, setIdea] = useState(handoff?.idea ?? "");
  const [constraints, setConstraints] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // 二次确认
  const [confirmChecked, setConfirmChecked] = useState(false);
  const [confirmText, setConfirmText] = useState("");

  const CONFIRM_QUESTIONS_TEXT = "我确认发起想法引导模型调用";
  const CONFIRM_SEED_TEXT = "我确认生成项目种子";

  // Step 2 state
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [questions, setQuestions] = useState<IdeaGuidanceQuestion[]>([]);
  const [answers, setAnswers] = useState<Record<string, string>>({});

  // Step 3 state
  const [seed, setSeed] = useState<ProjectSeed | null>(null);

  const isDesktop = isTauriHost();
  const canCallModel = canWrite && isDesktop;
  const hasHandoffQuestions = Boolean(handoff?.questions?.length);
  const canUseOfflineMode = Boolean(idea.trim());
  const offlineHint = useMemo(() => {
    if (canCallModel) return null;
    return hasHandoffQuestions
      ? "当前可使用总控分流带来的预设问题继续离线体验；不会调用模型、不会写审计。"
      : "当前可使用离线预设问题体验流程；不会调用模型、不会写审计。";
  }, [canCallModel, hasHandoffQuestions]);

  useEffect(() => {
    if (!handoff?.idea) return;
    setIdea(handoff.idea);
    setConstraints((current) => {
      if (current.trim()) return current;
      const parts = [
        handoff.projectTypeLabel ? `总控识别类型：${handoff.projectTypeLabel}` : null,
        handoff.reason ? `总控分流理由：${handoff.reason}` : null,
      ].filter(Boolean);
      return parts.join("\n");
    });
  }, [handoff]);

  const resetConfirm = () => {
    setConfirmChecked(false);
    setConfirmText("");
  };

  const handleGenerateOfflineQuestions = () => {
    const trimmedIdea = idea.trim();
    if (!trimmedIdea) {
      message.warning("请输入项目想法");
      return;
    }
    const offlineQuestions = buildOfflineQuestions(trimmedIdea, handoff);
    setSessionId(`offline_session_${Date.now()}`);
    setQuestions(offlineQuestions);
    setAnswers({});
    setStep("questions");
    resetConfirm();
    setError(null);
    message.success(`已生成 ${offlineQuestions.length} 个离线澄清问题`);
  };

  const handleGenerateOfflineSeed = () => {
    if (!sessionId) return;
    const offlineSeed = buildOfflineSeed(idea.trim(), constraints.trim(), questions, answers, handoff);
    setSeed(offlineSeed);
    setStep("seed");
    resetConfirm();
    setError(null);
    message.success("离线项目种子已生成");
  };

  // Step 1 → Step 2: 生成追问
  const handleGenerateQuestions = async () => {
    const trimmedIdea = idea.trim();
    if (!trimmedIdea) {
      message.warning("请输入项目想法");
      return;
    }
    if (!canCallModel) {
      handleGenerateOfflineQuestions();
      return;
    }
    setLoading(true);
    setError(null);
    try {
      const input: CreateIdeaGuidanceQuestionsInput = {
        idea: trimmedIdea,
        constraints: constraints.trim() || undefined,
        second_confirm: confirmChecked,
        confirm_text: confirmText.trim() || undefined,
      };
      const result: CreateIdeaGuidanceQuestionsResponse =
        await createIdeaGuidanceQuestions(input);
      setSessionId(result.session.id);
      setQuestions(result.questions);
      setAnswers({});
      setStep("questions");
      resetConfirm();
      message.success(`已生成 ${result.questions.length} 个澄清问题`);
    } catch (e) {
      setError(toUserError(e));
      message.error(toUserError(e));
    } finally {
      setLoading(false);
    }
  };

  // 更新单个答案
  const handleAnswerChange = (questionId: string, value: string) => {
    setAnswers((prev) => ({ ...prev, [questionId]: value }));
  };

  // 跳过问题
  const handleSkipQuestion = (questionId: string) => {
    setAnswers((prev) => ({ ...prev, [questionId]: "" }));
  };

  // Step 2 → Step 3: 生成项目种子
  const handleGenerateSeed = async () => {
    if (!sessionId) return;
    if (!canCallModel) {
      handleGenerateOfflineSeed();
      return;
    }
    setLoading(true);
    setError(null);
    try {
      // 先保存答案
      const answerList = questions
        .filter((q) => answers[q.id] !== undefined)
        .map((q) => ({
          question_id: q.id,
          answer: answers[q.id] || "",
        }));
      if (answerList.length > 0) {
        await saveGuidanceAnswers({
          session_id: sessionId,
          answers: answerList,
        });
      }

      // 生成种子
      const input: GenerateProjectSeedInput = {
        session_id: sessionId,
        second_confirm: confirmChecked,
        confirm_text: confirmText.trim() || undefined,
      };
      const result: GenerateProjectSeedResponse = await generateProjectSeed(input);
      setSeed(result.seed);
      setStep("seed");
      resetConfirm();
      message.success("项目种子已生成");
    } catch (e) {
      setError(toUserError(e));
      message.error(toUserError(e));
    } finally {
      setLoading(false);
    }
  };

  // 重置
  const handleReset = () => {
    setStep("input");
    setIdea(handoff?.idea ?? "");
    setConstraints("");
    setSessionId(null);
    setQuestions([]);
    setAnswers({});
    setSeed(null);
    setError(null);
    resetConfirm();
  };

  // 二次确认区域
  const renderConfirm = (confirmLabel: string) => (
    <div style={{ marginTop: 16 }}>
      <Checkbox
        checked={confirmChecked}
        onChange={(e) => {
          setConfirmChecked(e.target.checked);
          if (!e.target.checked) setConfirmText("");
        }}
      >
        {confirmLabel}
      </Checkbox>
      {confirmChecked && (
        <div style={{ marginTop: 8 }}>
          <Input
            placeholder={`请输入确认文本：${confirmLabel}`}
            value={confirmText}
            onChange={(e) => setConfirmText(e.target.value)}
            disabled={loading}
            style={{ maxWidth: 480 }}
          />
        </div>
      )}
    </div>
  );

  return (
    <Card
      title={
        <Space>
          <BulbOutlined style={{ color: "#7367f0" }} />
          <span>想法引导官</span>
          <Tag color="purple" style={{ marginLeft: 8 }}>
            L1 · 模型草案
          </Tag>
        </Space>
      }
      style={{ marginBottom: 24 }}
    >
      <Alert
        type="info"
        showIcon
        title="通过 AI 提问帮助你完善项目想法，生成结构化的项目种子草案。真实模型调用需要二次确认。此功能不会启动执行引擎、不会写文件、不会修改版本。"
        style={{ marginBottom: 16 }}
      />

      {!canCallModel && (
        <Alert
          type="warning"
          showIcon
          title={offlineHint ?? "想法引导功能需要启动 Tauri 桌面宿主，浏览器预览模式下将使用离线预设问题。"}
          style={{ marginBottom: 16 }}
        />
      )}

      {/* Step 1: 输入想法 */}
      {step === "input" && (
        <div>
          <Form layout="vertical">
            <Form.Item
              label="项目想法"
              required
              help="请用中文描述你的粗略项目想法，AI 会提出澄清问题帮助你扩展"
            >
              <TextArea
                rows={4}
                maxLength={2000}
                showCount
                placeholder="例如：我想做一个本地客户线索管理工具，帮助小团队跟踪销售机会..."
                value={idea}
                onChange={(e) => setIdea(e.target.value)}
                disabled={loading}
              />
            </Form.Item>
            <Form.Item
              label="约束条件（可选）"
              help="如有技术偏好、时间限制、预算等约束可以在此填写"
            >
              <TextArea
                rows={2}
                maxLength={2000}
                showCount
                placeholder="例如：必须本地运行、使用 Tauri 框架、两周内完成 MVP..."
                value={constraints}
                onChange={(e) => setConstraints(e.target.value)}
                disabled={loading}
              />
            </Form.Item>

            {canCallModel ? renderConfirm(CONFIRM_QUESTIONS_TEXT) : null}

            <Button
              type="primary"
              icon={<QuestionCircleOutlined />}
              loading={loading}
              onClick={handleGenerateQuestions}
              disabled={
                !idea.trim() ||
                (canCallModel && (!confirmChecked || confirmText.trim() !== CONFIRM_QUESTIONS_TEXT))
              }
              style={{ marginTop: 8 }}
            >
              {canCallModel ? "生成澄清问题" : "使用离线问题继续"}
            </Button>
            {canCallModel && (
              <Button
                icon={<QuestionCircleOutlined />}
                onClick={handleGenerateOfflineQuestions}
                disabled={!idea.trim() || loading}
                style={{ marginTop: 8, marginLeft: 8 }}
              >
                跳过模型，使用离线问题
              </Button>
            )}
          </Form>

          {error && (
            <Alert type="error" showIcon title={error} style={{ marginTop: 16 }} />
          )}
        </div>
      )}

      {/* Step 2: 回答问题 */}
      {step === "questions" && (
        <div>
          <Title level={5}>
            <QuestionCircleOutlined style={{ marginRight: 8 }} />
            请回答以下澄清问题（{questions.length} 个）
          </Title>
          <Text type="secondary">
            你可以选择回答或跳过每个问题。回答越详细，生成的项目种子越精确。
          </Text>

          <List
            style={{ marginTop: 16 }}
            dataSource={questions}
            renderItem={(q, index) => (
              <List.Item key={q.id} style={{ display: "block", padding: "12px 0" }}>
                <div style={{ marginBottom: 8 }}>
                  <Tag color="blue">{index + 1}</Tag>
                  <Text strong>{q.question}</Text>
                </div>
                <Space.Compact style={{ width: "100%" }}>
                  <TextArea
                    rows={2}
                    maxLength={500}
                    showCount
                    placeholder="输入你的回答..."
                    value={answers[q.id] || ""}
                    onChange={(e) => handleAnswerChange(q.id, e.target.value)}
                    disabled={loading}
                    style={{ flex: 1 }}
                  />
                  <Button
                    onClick={() => handleSkipQuestion(q.id)}
                    disabled={loading || !!answers[q.id]}
                    style={{ height: "auto" }}
                  >
                    跳过
                  </Button>
                </Space.Compact>
              </List.Item>
            )}
          />

          <Divider />

          {canCallModel ? renderConfirm(CONFIRM_SEED_TEXT) : null}

          <Space style={{ marginTop: 8 }}>
            <Button
              type="primary"
              icon={<FileTextOutlined />}
              loading={loading}
              onClick={handleGenerateSeed}
              disabled={
                canCallModel && (!confirmChecked || confirmText.trim() !== CONFIRM_SEED_TEXT)
              }
            >
              {canCallModel ? "生成项目种子" : "生成离线项目种子"}
            </Button>
            {canCallModel && (
              <Button icon={<FileTextOutlined />} onClick={handleGenerateOfflineSeed} disabled={loading}>
                跳过模型，生成离线项目种子
              </Button>
            )}
            <Button onClick={handleReset} disabled={loading}>
              重新开始
            </Button>
          </Space>

          <Paragraph type="secondary" style={{ marginTop: 8, fontSize: 12 }}>
            {canCallModel ? "已写入安全审计记录" : "离线模式不会写入模型审计记录"}
          </Paragraph>

          {error && (
            <Alert type="error" showIcon title={error} style={{ marginTop: 16 }} />
          )}
        </div>
      )}

      {/* Step 3: 查看种子 */}
      {step === "seed" && seed && (
        <div>
          <Title level={5}>
            <FileTextOutlined style={{ marginRight: 8 }} />
            项目种子草案
          </Title>

          <Descriptions bordered size="small" column={1} style={{ marginTop: 16 }}>
            <Descriptions.Item label="产品目标">{seed.product_goal}</Descriptions.Item>
            <Descriptions.Item label="目标用户">{seed.target_users}</Descriptions.Item>
            <Descriptions.Item label="MVP 范围">{seed.mvp_scope}</Descriptions.Item>
            <Descriptions.Item label="明确不做">{seed.non_goals}</Descriptions.Item>
            <Descriptions.Item label="核心功能">
              {seed.key_features ? (
                <Space wrap>
                  {tryParseJsonArray(seed.key_features).map((f, i) => (
                    <Tag key={i} color="blue">
                      {f}
                    </Tag>
                  ))}
                </Space>
              ) : null}
            </Descriptions.Item>
            <Descriptions.Item label="页面/模块">
              {seed.pages_or_modules ? (
                <Space wrap>
                  {tryParseJsonArray(seed.pages_or_modules).map((m, i) => (
                    <Tag key={i} color="green">
                      {m}
                    </Tag>
                  ))}
                </Space>
              ) : null}
            </Descriptions.Item>
            <Descriptions.Item label="数据实体">
              {seed.data_entities ? (
                <Space wrap>
                  {tryParseJsonArray(seed.data_entities).map((e, i) => (
                    <Tag key={i} color="orange">
                      {e}
                    </Tag>
                  ))}
                </Space>
              ) : null}
            </Descriptions.Item>
            <Descriptions.Item label="技术约束">
              {seed.technical_constraints}
            </Descriptions.Item>
            <Descriptions.Item label="验收标准">
              {seed.acceptance_criteria}
            </Descriptions.Item>
            <Descriptions.Item label="风险点">{seed.risk_points}</Descriptions.Item>
            <Descriptions.Item label="待澄清问题">
              {seed.open_questions}
            </Descriptions.Item>
            <Descriptions.Item label="推荐下一步">
              {seed.recommended_next_step}
            </Descriptions.Item>
          </Descriptions>

          <Paragraph type="secondary" style={{ marginTop: 8, fontSize: 12 }}>
            {seed.model_call_id ? "已写入安全审计记录" : "离线模式生成，未调用真实模型"}
          </Paragraph>

          <Divider />

          <Space>
            {onSeedReadyForDraft && (
              <Button
                type="primary"
                icon={<FileTextOutlined />}
                onClick={() => onSeedReadyForDraft(buildDraftPayloadFromSeed(seed))}
              >
                用这个种子创建项目计划草案
              </Button>
            )}
            <Button type="primary" onClick={handleReset}>
              开始新的想法引导
            </Button>
          </Space>
        </div>
      )}
    </Card>
  );
}

/** 安全解析 JSON 字符串数组，失败返回原始字符串分割 */
function tryParseJsonArray(text: string | null): string[] {
  if (!text) return [];
  try {
    const parsed = JSON.parse(text);
    if (Array.isArray(parsed)) {
      return parsed.map(String);
    }
  } catch {
    // 不是有效 JSON，尝试按逗号分割
  }
  return text
    .split(/[,，]/)
    .map((s) => s.trim())
    .filter(Boolean);
}
