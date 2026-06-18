// ---------------------------------------------------------------------------
// 想法引导官服务（阶段 37）
// 权限级别 L1（模型草案），不执行 Runner、不写文件、不改 Git。
// L1 不是免确认：真实模型调用前必须二次确认。
// 复用 model_gateway 的子模块：provider_config、openai_compat、redaction、model_calls。
// 所有模型调用必须走 Model Gateway + 受控模型目录 + model_calls 审计。
// ---------------------------------------------------------------------------

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use super::model_catalog;
use super::model_gateway::model_calls::{
    compute_request_hash, insert_safe_model_call, SafeModelCallRecordInput,
};
use super::model_gateway::openai_compat::{
    ModelProvider, ModelRequest, OpenAiCompatProvider, ProviderError,
};
use super::model_gateway::provider_config::{resolve_provider_config, ProviderConfigStatus};
use super::model_gateway::redaction::{
    check_forbidden_value_patterns, redact_secrets, truncate_summary,
};
use super::projects::get_current_project;

// ---------------------------------------------------------------------------
// 常量
// ---------------------------------------------------------------------------

const SUMMARY_MAX_LENGTH: usize = 10000;
const QUESTIONS_COUNT_MIN: usize = 3;
const QUESTIONS_COUNT_MAX: usize = 7;
const QUESTION_MAX_LENGTH: usize = 200;
const PROVIDER_TIMEOUT_SECS: u64 = 60;
const MAX_RESPONSE_BYTES: u64 = 1024 * 1024;

// 二次确认文本
const CONFIRM_QUESTIONS_TEXT: &str = "我确认发起想法引导模型调用";
const CONFIRM_SEED_TEXT: &str = "我确认生成项目种子";

// ---------------------------------------------------------------------------
// 数据类型
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Clone)]
pub struct IdeaGuidanceSession {
    pub id: String,
    pub project_id: String,
    pub status: String,
    pub source: String,
    pub idea_summary: String,
    pub constraints_summary: Option<String>,
    pub model_call_id: Option<String>,
    pub created_by: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct IdeaGuidanceQuestion {
    pub id: String,
    pub project_id: String,
    pub session_id: String,
    pub sort_order: i64,
    pub question: String,
    pub answer: Option<String>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct ProjectSeed {
    pub id: String,
    pub project_id: String,
    pub session_id: String,
    pub status: String,
    pub product_goal: Option<String>,
    pub target_users: Option<String>,
    pub mvp_scope: Option<String>,
    pub non_goals: Option<String>,
    pub key_features: Option<String>,
    pub pages_or_modules: Option<String>,
    pub data_entities: Option<String>,
    pub technical_constraints: Option<String>,
    pub acceptance_criteria: Option<String>,
    pub risk_points: Option<String>,
    pub open_questions: Option<String>,
    pub recommended_next_step: Option<String>,
    pub model_call_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct CreateIdeaGuidanceQuestionsResponse {
    pub session: IdeaGuidanceSession,
    pub questions: Vec<IdeaGuidanceQuestion>,
    pub audit_record_id: Option<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct GenerateProjectSeedResponse {
    pub seed: ProjectSeed,
    pub session: IdeaGuidanceSession,
    pub audit_record_id: Option<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct QuestionAnswer {
    pub question_id: String,
    pub answer: String,
}

// ---------------------------------------------------------------------------
// 公开 API（被 Tauri command 调用）
// ---------------------------------------------------------------------------

/// 创建想法引导追问
pub fn create_idea_guidance_questions(
    idea: &str,
    constraints: &Option<String>,
    connection: &Connection,
    model_record_id: Option<&str>,
    second_confirm: bool,
    confirm_text: &Option<String>,
) -> Result<CreateIdeaGuidanceQuestionsResponse, String> {
    let flag = std::env::var("AGENT_SWARM_ENABLE_REAL_MODEL_PROJECT_PLAN")
        .unwrap_or_else(|_| "false".into());
    create_questions_core(
        idea,
        constraints,
        connection,
        &flag,
        model_record_id,
        None,
        None,
        None,
        second_confirm,
        confirm_text,
    )
}

/// 生成项目种子（从 DB 读取 session 和问题，构造 prompt 调用模型）
pub fn generate_project_seed(
    session_id: &str,
    connection: &Connection,
    model_record_id: Option<&str>,
    second_confirm: bool,
    confirm_text: &Option<String>,
) -> Result<GenerateProjectSeedResponse, String> {
    let flag = std::env::var("AGENT_SWARM_ENABLE_REAL_MODEL_PROJECT_PLAN")
        .unwrap_or_else(|_| "false".into());
    generate_seed_core(
        session_id,
        connection,
        &flag,
        model_record_id,
        None,
        None,
        None,
        second_confirm,
        confirm_text,
    )
}

/// 列出当前项目的所有种子
pub fn list_project_seeds(connection: &Connection) -> Result<Vec<ProjectSeed>, String> {
    let project_id = current_project_id(connection)?;
    let mut stmt = connection
        .prepare(
            "SELECT id, project_id, session_id, status,
                    product_goal, target_users, mvp_scope, non_goals,
                    key_features, pages_or_modules, data_entities,
                    technical_constraints, acceptance_criteria, risk_points,
                    open_questions, recommended_next_step, model_call_id,
                    created_at, updated_at
             FROM project_seeds
             WHERE project_id = ?1
             ORDER BY created_at DESC",
        )
        .map_err(|e| format!("database_error: list project_seeds failed: {e}"))?;

    let rows = stmt
        .query_map(params![project_id.as_str()], map_seed_row)
        .map_err(|e| format!("database_error: list project_seeds failed: {e}"))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("database_error: list project_seeds failed: {e}"))
}

/// 保存用户对追问的回答（不调用模型，无需二次确认）
pub fn save_guidance_answers(
    connection: &Connection,
    session_id: &str,
    answers: &[QuestionAnswer],
) -> Result<IdeaGuidanceSession, String> {
    let project_id = current_project_id(connection)?;

    // 校验 session 存在、状态为 questions_ready、且属于当前项目
    let session = load_session(connection, session_id, Some(&project_id))?;
    if session.status != "questions_ready" {
        return Err(format!(
            "invalid_state: session status is {}, expected questions_ready",
            session.status
        ));
    }

    let now = current_timestamp();
    for a in answers {
        let normalized = a.answer.trim().to_string();
        if normalized.is_empty() {
            let rows = connection
                .execute(
                    "UPDATE idea_guidance_questions
                     SET status = 'skipped', updated_at = ?1
                     WHERE id = ?2 AND session_id = ?3 AND project_id = ?4",
                    params![
                        now.as_str(),
                        a.question_id.as_str(),
                        session_id,
                        project_id.as_str()
                    ],
                )
                .map_err(|e| format!("database_error: update question failed: {e}"))?;
            if rows != 1 {
                return Err(format!(
                    "invalid_input: question_id '{}' not found in session '{}'",
                    a.question_id, session_id
                ));
            }
        } else {
            check_forbidden_value_patterns(&normalized)?;
            if normalized.chars().count() > 2000 {
                return Err("invalid_input: answer exceeds max length 2000".into());
            }
            let rows = connection
                .execute(
                    "UPDATE idea_guidance_questions
                     SET answer = ?1, status = 'answered', updated_at = ?2
                     WHERE id = ?3 AND session_id = ?4 AND project_id = ?5",
                    params![
                        normalized.as_str(),
                        now.as_str(),
                        a.question_id.as_str(),
                        session_id,
                        project_id.as_str()
                    ],
                )
                .map_err(|e| format!("database_error: update question failed: {e}"))?;
            if rows != 1 {
                return Err(format!(
                    "invalid_input: question_id '{}' not found in session '{}'",
                    a.question_id, session_id
                ));
            }
        }
    }

    load_session(connection, session_id, Some(&project_id))
}

// ---------------------------------------------------------------------------
// 二次确认校验
// ---------------------------------------------------------------------------

fn validate_second_confirm(
    second_confirm: bool,
    confirm_text: &Option<String>,
    expected_text: &str,
) -> Result<(), String> {
    if !second_confirm {
        return Err("invalid_input: second_confirm is required for real model calls".into());
    }
    let text = confirm_text.as_ref().map(|t| t.trim()).unwrap_or("");
    if text != expected_text {
        return Err("invalid_input: confirm_text does not match required confirmation".into());
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// 核心实现（可注入依赖，供测试使用）
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
pub(crate) fn create_questions_core(
    idea: &str,
    constraints: &Option<String>,
    connection: &Connection,
    flag_value: &str,
    model_record_id: Option<&str>,
    provider_override: Option<Box<dyn ModelProvider>>,
    config_key: Option<&str>,
    config_base_url: Option<&str>,
    second_confirm: bool,
    confirm_text: &Option<String>,
) -> Result<CreateIdeaGuidanceQuestionsResponse, String> {
    // 1. 输入校验
    validate_idea_input(idea, constraints)?;

    // 2. feature flag
    if flag_value != "true" {
        return Err("feature_disabled: 真实模型调用未启用".into());
    }

    // 3. 二次确认
    validate_second_confirm(second_confirm, confirm_text, CONFIRM_QUESTIONS_TEXT)?;

    // 4. provider 配置
    let config = if config_key.is_some() || config_base_url.is_some() {
        super::model_gateway::provider_config::resolve(config_key, config_base_url)
    } else {
        resolve_provider_config()
    };

    if config.status != ProviderConfigStatus::Configured {
        return Err(format!(
            "provider_config_error: {}",
            match config.status {
                ProviderConfigStatus::MissingKey => "missing_key",
                ProviderConfigStatus::MissingBaseUrl => "missing_base_url",
                ProviderConfigStatus::InvalidBaseUrl => "invalid_base_url",
                _ => "provider_config_error",
            }
        ));
    }

    // 5. 获取 project_id
    let project_id = current_project_id(connection)?;

    // 6. 解析模型
    let resolved_model_id =
        resolve_model(connection, &project_id, model_record_id, "idea_guidance")?;

    // 7. 构造 provider
    let provider: Box<dyn ModelProvider> = match provider_override {
        Some(p) => p,
        None => Box::new(
            OpenAiCompatProvider::from_env()
                .map_err(|_| "无法从环境变量构造 provider".to_string())?,
        ),
    };

    // 8. 构造 prompt
    let (system_prompt, user_message) = build_questions_prompt(idea, constraints);

    let request = ModelRequest {
        system_prompt,
        user_message,
        model_id: resolved_model_id.clone(),
    };

    // 9. 计算审计哈希
    let idea_len = idea.chars().count();
    let has_constraints = constraints.is_some();
    let request_hash = compute_request_hash(
        "idea_guidance",
        "openai_compat",
        &resolved_model_id,
        idea_len,
        has_constraints,
    );

    // 10. 调用 provider
    match provider.send(&request, PROVIDER_TIMEOUT_SECS, MAX_RESPONSE_BYTES) {
        Ok(response) => {
            let redacted = redact_secrets(&response.content);
            let summary = truncate_summary(&redacted, SUMMARY_MAX_LENGTH);

            // 11. 解析追问列表
            let questions_texts = match parse_questions_response(&summary) {
                Ok(qt) => qt,
                Err(parse_err) => {
                    // 模型已真实调用，解析失败也必须写审计
                    let audit_input = SafeModelCallRecordInput {
                        project_id: project_id.clone(),
                        purpose: "idea_guidance".into(),
                        provider: "openai_compat".into(),
                        model: resolved_model_id.clone(),
                        status: "failed".into(),
                        error_category: Some("invalid_response".into()),
                        structured_summary: None,
                        request_hash: request_hash.clone(),
                    };
                    insert_safe_model_call(connection, audit_input).map_err(|_| {
                        "audit_write_failed: 真实模型调用审计记录写入失败".to_string()
                    })?;
                    return Err(parse_err);
                }
            };

            // 12. 写入 model_calls 审计
            let audit_input = SafeModelCallRecordInput {
                project_id: project_id.clone(),
                purpose: "idea_guidance".into(),
                provider: "openai_compat".into(),
                model: resolved_model_id.clone(),
                status: "success".into(),
                error_category: None,
                structured_summary: Some(format!("generated {} questions", questions_texts.len())),
                request_hash: request_hash.clone(),
            };
            let audit_id = insert_safe_model_call(connection, audit_input)
                .map_err(|_| "audit_write_failed: 真实模型调用审计记录写入失败".to_string())?;

            // 13. 写入 DB：session + questions
            let (session, questions) = insert_session_and_questions(
                connection,
                &project_id,
                idea,
                constraints,
                &questions_texts,
                Some(&audit_id),
            )?;

            Ok(CreateIdeaGuidanceQuestionsResponse {
                session,
                questions,
                audit_record_id: Some(audit_id),
                warnings: vec!["已写入安全审计记录".into()],
            })
        }
        Err(e) => {
            let error_category = provider_error_category(&e);

            // 写入 failed 审计记录
            let audit_input = SafeModelCallRecordInput {
                project_id: project_id.clone(),
                purpose: "idea_guidance".into(),
                provider: "openai_compat".into(),
                model: resolved_model_id.clone(),
                status: "failed".into(),
                error_category: Some(error_category.to_string()),
                structured_summary: None,
                request_hash,
            };
            // 审计写入失败必须返回 audit_write_failed，不能吞掉
            insert_safe_model_call(connection, audit_input)
                .map_err(|_| "audit_write_failed: 真实模型调用审计记录写入失败".to_string())?;

            // 返回 Err，不让前端进入成功流程
            Err(format!("provider_error: {}", error_category))
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn generate_seed_core(
    session_id: &str,
    connection: &Connection,
    flag_value: &str,
    model_record_id: Option<&str>,
    provider_override: Option<Box<dyn ModelProvider>>,
    config_key: Option<&str>,
    config_base_url: Option<&str>,
    second_confirm: bool,
    confirm_text: &Option<String>,
) -> Result<GenerateProjectSeedResponse, String> {
    let project_id = current_project_id(connection)?;

    // 1. 读取 session，必须属于当前项目
    let session = load_session(connection, session_id, Some(&project_id))?;
    if session.status != "questions_ready" {
        return Err(format!(
            "invalid_state: session status is {}, expected questions_ready",
            session.status
        ));
    }

    // 2. feature flag
    if flag_value != "true" {
        return Err("feature_disabled: 真实模型调用未启用".into());
    }

    // 3. 二次确认
    validate_second_confirm(second_confirm, confirm_text, CONFIRM_SEED_TEXT)?;

    // 4. provider 配置
    let config = if config_key.is_some() || config_base_url.is_some() {
        super::model_gateway::provider_config::resolve(config_key, config_base_url)
    } else {
        resolve_provider_config()
    };

    if config.status != ProviderConfigStatus::Configured {
        return Err(format!(
            "provider_config_error: {}",
            match config.status {
                ProviderConfigStatus::MissingKey => "missing_key",
                ProviderConfigStatus::MissingBaseUrl => "missing_base_url",
                ProviderConfigStatus::InvalidBaseUrl => "invalid_base_url",
                _ => "provider_config_error",
            }
        ));
    }

    // 5. 读取问题及答案（跨项目安全：project_id 已校验）
    let questions = load_questions_for_session(connection, session_id, &project_id)?;

    // 6. 解析模型
    let resolved_model_id = resolve_model(
        connection,
        &project_id,
        model_record_id,
        "project_seed_generation",
    )?;

    // 7. 构造 provider
    let provider: Box<dyn ModelProvider> = match provider_override {
        Some(p) => p,
        None => Box::new(
            OpenAiCompatProvider::from_env()
                .map_err(|_| "无法从环境变量构造 provider".to_string())?,
        ),
    };

    // 8. 构造 prompt
    let qa_pairs: Vec<(String, Option<String>)> = questions
        .iter()
        .map(|q| (q.question.clone(), q.answer.clone()))
        .collect();
    let (system_prompt, user_message) = build_seed_prompt(
        &session.idea_summary,
        session.constraints_summary.as_deref(),
        &qa_pairs,
    );

    let request = ModelRequest {
        system_prompt,
        user_message,
        model_id: resolved_model_id.clone(),
    };

    // 9. 计算审计哈希
    let request_hash = compute_request_hash(
        "project_seed_generation",
        "openai_compat",
        &resolved_model_id,
        session.idea_summary.chars().count(),
        session.constraints_summary.is_some(),
    );

    // 10. 调用 provider
    match provider.send(&request, PROVIDER_TIMEOUT_SECS, MAX_RESPONSE_BYTES) {
        Ok(response) => {
            let redacted = redact_secrets(&response.content);
            let summary = truncate_summary(&redacted, SUMMARY_MAX_LENGTH);

            // 11. 解析种子字段
            let seed_fields = match parse_seed_response(&summary) {
                Ok(sf) => sf,
                Err(parse_err) => {
                    // 模型已真实调用，解析失败也必须写审计
                    let audit_input = SafeModelCallRecordInput {
                        project_id: project_id.clone(),
                        purpose: "project_seed_generation".into(),
                        provider: "openai_compat".into(),
                        model: resolved_model_id.clone(),
                        status: "failed".into(),
                        error_category: Some("invalid_response".into()),
                        structured_summary: None,
                        request_hash: request_hash.clone(),
                    };
                    insert_safe_model_call(connection, audit_input).map_err(|_| {
                        "audit_write_failed: 真实模型调用审计记录写入失败".to_string()
                    })?;
                    return Err(parse_err);
                }
            };

            // 12. 写审计
            let audit_input = SafeModelCallRecordInput {
                project_id: project_id.clone(),
                purpose: "project_seed_generation".into(),
                provider: "openai_compat".into(),
                model: resolved_model_id.clone(),
                status: "success".into(),
                error_category: None,
                structured_summary: Some(format!(
                    "generated seed: {}",
                    seed_fields
                        .product_goal
                        .chars()
                        .take(80)
                        .collect::<String>()
                )),
                request_hash: request_hash.clone(),
            };
            let audit_id = insert_safe_model_call(connection, audit_input)
                .map_err(|_| "audit_write_failed: 真实模型调用审计记录写入失败".to_string())?;

            // 13. 写入 DB
            let seed = insert_seed_and_update_session(
                connection,
                &project_id,
                session_id,
                &seed_fields,
                Some(&audit_id),
            )?;
            let updated_session = load_session(connection, session_id, Some(&project_id))?;

            Ok(GenerateProjectSeedResponse {
                seed,
                session: updated_session,
                audit_record_id: Some(audit_id),
                warnings: vec!["已写入安全审计记录".into()],
            })
        }
        Err(e) => {
            let error_category = provider_error_category(&e);

            // 写入 failed 审计
            let audit_input = SafeModelCallRecordInput {
                project_id: project_id.clone(),
                purpose: "project_seed_generation".into(),
                provider: "openai_compat".into(),
                model: resolved_model_id.clone(),
                status: "failed".into(),
                error_category: Some(error_category.to_string()),
                structured_summary: None,
                request_hash,
            };
            // 审计写入失败必须返回 audit_write_failed，不能吞掉
            insert_safe_model_call(connection, audit_input)
                .map_err(|_| "audit_write_failed: 真实模型调用审计记录写入失败".to_string())?;

            // 标记 session 为 failed
            let now = current_timestamp();
            let _ = connection.execute(
                "UPDATE idea_guidance_sessions SET status = 'failed', updated_at = ?1 WHERE id = ?2 AND project_id = ?3",
                params![now.as_str(), session_id, project_id.as_str()],
            );

            // 返回 Err，不让前端进入成功流程
            Err(format!("provider_error: {}", error_category))
        }
    }
}

// ---------------------------------------------------------------------------
// Prompt 模板构造
// ---------------------------------------------------------------------------

fn build_questions_prompt(idea: &str, constraints: &Option<String>) -> (String, String) {
    let system_prompt = r#"你是一位项目想法引导官。用户会给你一个粗略的项目想法，你需要提出 3-7 个澄清问题，帮助用户把想法扩展成更完整的项目定义。

要求：
1. 问题必须是中文。
2. 问题要帮助用户扩宽想法，不是机械收集字段。例如：目标用户是谁？核心场景是什么？有哪些技术约束？
3. 不要问密钥、账号、密码、支付凭据等敏感信息。
4. 不要问用户上传源码或文件。
5. 输出必须是严格的 JSON 数组，每个元素是一个问题字符串。数组长度 3-7。
6. 不要输出任何 JSON 以外的内容。

输出格式示例：
["你的项目目标用户是谁？","核心使用场景是什么？","有什么技术栈偏好吗？"]"#
        .to_string();

    let mut user_message = format!("用户的粗略项目想法：\n{}", idea);
    if let Some(c) = constraints {
        if !c.trim().is_empty() {
            user_message.push_str(&format!("\n\n用户的约束条件：\n{}", c));
        }
    }
    user_message.push_str("\n\n请根据以上信息，生成 3-7 个中文澄清问题。只输出 JSON 数组。");

    (system_prompt, user_message)
}

fn build_seed_prompt(
    idea_summary: &str,
    constraints_summary: Option<&str>,
    qa_pairs: &[(String, Option<String>)],
) -> (String, String) {
    let system_prompt = r#"你是一位项目规划专家。用户提供了项目想法、约束条件以及对澄清问题的回答。你需要基于这些信息生成一份结构化的项目种子草案。

输出必须是严格的 JSON 对象，包含以下字段（每个字段的值都是中文字符串）：
- product_goal: 产品目标（1-2句话）
- target_users: 目标用户描述
- mvp_scope: MVP 范围定义
- non_goals: 明确不做的事情
- key_features: 核心功能列表，JSON 数组字符串格式
- pages_or_modules: 页面或模块列表，JSON 数组字符串格式
- data_entities: 数据实体列表，JSON 数组字符串格式
- technical_constraints: 技术约束
- acceptance_criteria: 验收标准
- risk_points: 已识别风险点
- open_questions: 仍待澄清的问题
- recommended_next_step: 推荐的下一步行动

不要输出任何 JSON 以外的内容。所有字段都必须填写，不能为空。
如果某个字段确实没有内容，填入"暂无"。"#
        .to_string();

    let mut user_message = format!("项目想法：\n{}", idea_summary);
    if let Some(c) = constraints_summary {
        if !c.trim().is_empty() {
            user_message.push_str(&format!("\n\n约束条件：\n{}", c));
        }
    }
    if !qa_pairs.is_empty() {
        user_message.push_str("\n\n用户对澄清问题的回答：");
        for (i, (q, a)) in qa_pairs.iter().enumerate() {
            user_message.push_str(&format!(
                "\n{}. 问：{}\n   答：{}",
                i + 1,
                q,
                a.as_deref().unwrap_or("（未回答）")
            ));
        }
    }
    user_message.push_str("\n\n请基于以上全部信息，生成一份完整的项目种子草案。只输出 JSON 对象。");

    (system_prompt, user_message)
}

// ---------------------------------------------------------------------------
// 响应解析
// ---------------------------------------------------------------------------

fn parse_questions_response(raw: &str) -> Result<Vec<String>, String> {
    let trimmed = raw.trim();

    // 尝试直接解析 JSON 数组
    if let Ok(list) = serde_json::from_str::<Vec<String>>(trimmed) {
        return validate_and_filter_questions(list);
    }

    // 尝试从 markdown 代码块中提取 JSON
    if let Some(json_start) = trimmed.find('[') {
        if let Some(json_end) = trimmed.rfind(']') {
            let json_str = &trimmed[json_start..=json_end];
            if let Ok(list) = serde_json::from_str::<Vec<String>>(json_str) {
                return validate_and_filter_questions(list);
            }
        }
    }

    Err("invalid_response: 无法从模型输出中解析有效的问题列表，请确保输出为有效 JSON 数组".into())
}

fn validate_and_filter_questions(list: Vec<String>) -> Result<Vec<String>, String> {
    let valid: Vec<String> = list
        .into_iter()
        .map(|q| q.trim().to_string())
        .filter(|q| !q.is_empty())
        .collect();

    if valid.len() < QUESTIONS_COUNT_MIN {
        return Err(format!(
            "invalid_response: parsed {} questions, need at least {}",
            valid.len(),
            QUESTIONS_COUNT_MIN
        ));
    }
    if valid.len() > QUESTIONS_COUNT_MAX {
        return Err(format!(
            "invalid_response: parsed {} questions, max {}",
            valid.len(),
            QUESTIONS_COUNT_MAX
        ));
    }

    // 每个问题都要做长度和敏感内容校验（含 markdown 解析路径）
    for q in &valid {
        if q.chars().count() > QUESTION_MAX_LENGTH {
            return Err(format!(
                "invalid_response: question exceeds max length {}",
                QUESTION_MAX_LENGTH
            ));
        }
        check_forbidden_value_patterns(q)
            .map_err(|e| format!("invalid_response: question contains forbidden content: {e}"))?;
    }

    Ok(valid)
}

#[derive(Debug, Default)]
struct SeedFields {
    product_goal: String,
    target_users: String,
    mvp_scope: String,
    non_goals: String,
    key_features: String,
    pages_or_modules: String,
    data_entities: String,
    technical_constraints: String,
    acceptance_criteria: String,
    risk_points: String,
    open_questions: String,
    recommended_next_step: String,
}

fn parse_seed_response(raw: &str) -> Result<SeedFields, String> {
    let trimmed = raw.trim();

    // 尝试直接解析 JSON
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) {
        return extract_seed_fields(&value);
    }

    // 尝试从 markdown 代码块中提取 JSON
    if let Some(json_start) = trimmed.find('{') {
        if let Some(json_end) = trimmed.rfind('}') {
            let json_str = &trimmed[json_start..=json_end];
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(json_str) {
                return extract_seed_fields(&value);
            }
        }
    }

    Err("invalid_response: 无法从模型输出中解析有效的项目种子 JSON".into())
}

fn extract_seed_fields(value: &serde_json::Value) -> Result<SeedFields, String> {
    let obj = value
        .as_object()
        .ok_or("invalid_response: seed output is not a JSON object")?;

    let required_fields = [
        "product_goal",
        "target_users",
        "mvp_scope",
        "non_goals",
        "key_features",
        "pages_or_modules",
        "data_entities",
        "technical_constraints",
        "acceptance_criteria",
        "risk_points",
        "open_questions",
        "recommended_next_step",
    ];

    for field in &required_fields {
        if !obj.contains_key(*field) {
            return Err(format!(
                "invalid_response: missing required field '{}'",
                field
            ));
        }
    }

    let get_string = |key: &str| -> String {
        obj.get(key)
            .map(|v| match v {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Array(arr) => serde_json::to_string(arr).unwrap_or_default(),
                other => other.to_string(),
            })
            .unwrap_or_default()
    };

    let fields = SeedFields {
        product_goal: get_string("product_goal"),
        target_users: get_string("target_users"),
        mvp_scope: get_string("mvp_scope"),
        non_goals: get_string("non_goals"),
        key_features: get_string("key_features"),
        pages_or_modules: get_string("pages_or_modules"),
        data_entities: get_string("data_entities"),
        technical_constraints: get_string("technical_constraints"),
        acceptance_criteria: get_string("acceptance_criteria"),
        risk_points: get_string("risk_points"),
        open_questions: get_string("open_questions"),
        recommended_next_step: get_string("recommended_next_step"),
    };

    // 对所有输出字段做敏感内容校验（不只是 3 个字段）
    let all_field_values = [
        ("product_goal", &fields.product_goal),
        ("target_users", &fields.target_users),
        ("mvp_scope", &fields.mvp_scope),
        ("non_goals", &fields.non_goals),
        ("key_features", &fields.key_features),
        ("pages_or_modules", &fields.pages_or_modules),
        ("data_entities", &fields.data_entities),
        ("technical_constraints", &fields.technical_constraints),
        ("acceptance_criteria", &fields.acceptance_criteria),
        ("risk_points", &fields.risk_points),
        ("open_questions", &fields.open_questions),
        ("recommended_next_step", &fields.recommended_next_step),
    ];
    for (field_name, value) in &all_field_values {
        check_forbidden_value_patterns(value).map_err(|e| {
            format!(
                "invalid_response: field '{}' contains forbidden content: {}",
                field_name, e
            )
        })?;
    }

    Ok(fields)
}

// ---------------------------------------------------------------------------
// DB 操作
// ---------------------------------------------------------------------------

fn insert_session_and_questions(
    connection: &Connection,
    project_id: &str,
    idea: &str,
    constraints: &Option<String>,
    questions: &[String],
    model_call_id: Option<&str>,
) -> Result<(IdeaGuidanceSession, Vec<IdeaGuidanceQuestion>), String> {
    let now = current_timestamp();
    let session_id = format!("igs_{}", now);

    connection
        .execute(
            "INSERT INTO idea_guidance_sessions (
                id, project_id, status, source, idea_summary, constraints_summary,
                model_call_id, created_by, created_at, updated_at
            ) VALUES (?1, ?2, 'questions_ready', 'model_guided', ?3, ?4, ?5, 'local_user', ?6, ?6)",
            params![
                session_id.as_str(),
                project_id,
                truncate_idea(idea).as_str(),
                constraints.as_ref().map(|c| truncate_idea(c)).as_deref(),
                model_call_id,
                now.as_str(),
            ],
        )
        .map_err(|e| format!("database_error: insert session failed: {e}"))?;

    let mut question_rows = Vec::new();
    for (i, q) in questions.iter().enumerate() {
        let q_id = format!("igq_{}_{}", session_id, i);
        connection
            .execute(
                "INSERT INTO idea_guidance_questions (
                    id, project_id, session_id, sort_order, question, status, created_at, updated_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, 'pending', ?6, ?6)",
                params![
                    q_id.as_str(),
                    project_id,
                    session_id.as_str(),
                    i as i64,
                    q.as_str(),
                    now.as_str(),
                ],
            )
            .map_err(|e| format!("database_error: insert question failed: {e}"))?;

        question_rows.push(IdeaGuidanceQuestion {
            id: q_id,
            project_id: project_id.into(),
            session_id: session_id.clone(),
            sort_order: i as i64,
            question: q.clone(),
            answer: None,
            status: "pending".into(),
            created_at: now.clone(),
            updated_at: now.clone(),
        });
    }

    let session = IdeaGuidanceSession {
        id: session_id,
        project_id: project_id.into(),
        status: "questions_ready".into(),
        source: "model_guided".into(),
        idea_summary: truncate_idea(idea),
        constraints_summary: constraints.as_ref().map(|c| truncate_idea(c)),
        model_call_id: model_call_id.map(|s| s.into()),
        created_by: "local_user".into(),
        created_at: now.clone(),
        updated_at: now,
    };

    Ok((session, question_rows))
}

fn insert_seed_and_update_session(
    connection: &Connection,
    project_id: &str,
    session_id: &str,
    fields: &SeedFields,
    model_call_id: Option<&str>,
) -> Result<ProjectSeed, String> {
    let now = current_timestamp();
    let seed_id = format!("ps_{}", now);

    connection
        .execute(
            "INSERT INTO project_seeds (
                id, project_id, session_id, status,
                product_goal, target_users, mvp_scope, non_goals,
                key_features, pages_or_modules, data_entities,
                technical_constraints, acceptance_criteria, risk_points,
                open_questions, recommended_next_step, model_call_id,
                created_at, updated_at
            ) VALUES (
                ?1, ?2, ?3, 'ready',
                ?4, ?5, ?6, ?7,
                ?8, ?9, ?10,
                ?11, ?12, ?13,
                ?14, ?15, ?16,
                ?17, ?17
            )",
            params![
                seed_id.as_str(),
                project_id,
                session_id,
                fields.product_goal.as_str(),
                fields.target_users.as_str(),
                fields.mvp_scope.as_str(),
                fields.non_goals.as_str(),
                fields.key_features.as_str(),
                fields.pages_or_modules.as_str(),
                fields.data_entities.as_str(),
                fields.technical_constraints.as_str(),
                fields.acceptance_criteria.as_str(),
                fields.risk_points.as_str(),
                fields.open_questions.as_str(),
                fields.recommended_next_step.as_str(),
                model_call_id,
                now.as_str(),
            ],
        )
        .map_err(|e| format!("database_error: insert seed failed: {e}"))?;

    // 更新 session 状态（限制当前项目）
    connection
        .execute(
            "UPDATE idea_guidance_sessions SET status = 'seed_ready', updated_at = ?1
             WHERE id = ?2 AND project_id = ?3",
            params![now.as_str(), session_id, project_id],
        )
        .map_err(|e| format!("database_error: update session failed: {e}"))?;

    Ok(ProjectSeed {
        id: seed_id,
        project_id: project_id.into(),
        session_id: session_id.into(),
        status: "ready".into(),
        product_goal: Some(fields.product_goal.clone()),
        target_users: Some(fields.target_users.clone()),
        mvp_scope: Some(fields.mvp_scope.clone()),
        non_goals: Some(fields.non_goals.clone()),
        key_features: Some(fields.key_features.clone()),
        pages_or_modules: Some(fields.pages_or_modules.clone()),
        data_entities: Some(fields.data_entities.clone()),
        technical_constraints: Some(fields.technical_constraints.clone()),
        acceptance_criteria: Some(fields.acceptance_criteria.clone()),
        risk_points: Some(fields.risk_points.clone()),
        open_questions: Some(fields.open_questions.clone()),
        recommended_next_step: Some(fields.recommended_next_step.clone()),
        model_call_id: model_call_id.map(|s| s.into()),
        created_at: now.clone(),
        updated_at: now,
    })
}

fn load_session(
    connection: &Connection,
    session_id: &str,
    project_id: Option<&str>,
) -> Result<IdeaGuidanceSession, String> {
    let (sql, params_vec): (String, Vec<Box<dyn rusqlite::types::ToSql>>) =
        if let Some(pid) = project_id {
            (
                "SELECT id, project_id, status, source, idea_summary, constraints_summary,
                    model_call_id, created_by, created_at, updated_at
             FROM idea_guidance_sessions WHERE id = ?1 AND project_id = ?2"
                    .into(),
                vec![Box::new(session_id.to_string()), Box::new(pid.to_string())],
            )
        } else {
            (
                "SELECT id, project_id, status, source, idea_summary, constraints_summary,
                    model_call_id, created_by, created_at, updated_at
             FROM idea_guidance_sessions WHERE id = ?1"
                    .into(),
                vec![Box::new(session_id.to_string())],
            )
        };

    let params_refs: Vec<&dyn rusqlite::types::ToSql> =
        params_vec.iter().map(|p| p.as_ref()).collect();

    connection
        .query_row(&sql, params_refs.as_slice(), |row| {
            Ok(IdeaGuidanceSession {
                id: row.get(0)?,
                project_id: row.get(1)?,
                status: row.get(2)?,
                source: row.get(3)?,
                idea_summary: row.get(4)?,
                constraints_summary: row.get(5)?,
                model_call_id: row.get(6)?,
                created_by: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        })
        .map_err(|e| format!("not_found: session '{}' not found: {}", session_id, e))
}

fn load_questions_for_session(
    connection: &Connection,
    session_id: &str,
    project_id: &str,
) -> Result<Vec<IdeaGuidanceQuestion>, String> {
    let mut stmt = connection
        .prepare(
            "SELECT id, project_id, session_id, sort_order, question, answer, status, created_at, updated_at
             FROM idea_guidance_questions
             WHERE session_id = ?1 AND project_id = ?2
             ORDER BY sort_order",
        )
        .map_err(|e| format!("database_error: load questions failed: {e}"))?;

    let rows = stmt
        .query_map(params![session_id, project_id], |row| {
            Ok(IdeaGuidanceQuestion {
                id: row.get(0)?,
                project_id: row.get(1)?,
                session_id: row.get(2)?,
                sort_order: row.get(3)?,
                question: row.get(4)?,
                answer: row.get(5)?,
                status: row.get(6)?,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        })
        .map_err(|e| format!("database_error: load questions failed: {e}"))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("database_error: load questions failed: {e}"))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn validate_idea_input(idea: &str, constraints: &Option<String>) -> Result<(), String> {
    let trimmed = idea.trim();
    if trimmed.is_empty() {
        return Err("invalid_input: idea must not be empty".into());
    }
    if trimmed.chars().count() > 2000 {
        return Err("invalid_input: idea must be at most 2000 characters".into());
    }
    check_forbidden_value_patterns(trimmed)?;

    if let Some(c) = constraints {
        let ct = c.trim();
        if ct.chars().count() > 2000 {
            return Err("invalid_input: constraints must be at most 2000 characters".into());
        }
        if !ct.is_empty() {
            check_forbidden_value_patterns(ct)?;
        }
    }
    Ok(())
}

fn truncate_idea(text: &str) -> String {
    truncate_summary(text, 1000)
}

fn resolve_model(
    connection: &Connection,
    project_id: &str,
    model_record_id: Option<&str>,
    purpose: &str,
) -> Result<String, String> {
    if let Some(record_id) = model_record_id {
        model_catalog::validate_model_for_call_with_purpose(
            connection, project_id, record_id, purpose,
        )
    } else {
        model_catalog::get_default_model_id_for_purpose(connection, purpose)
    }
}

fn provider_error_category(e: &ProviderError) -> &str {
    match e {
        ProviderError::Timeout => "timeout",
        ProviderError::NetworkError => "network_error",
        ProviderError::ProviderError => "provider_error",
        ProviderError::AuthError => "auth_error",
        ProviderError::RateLimited => "rate_limited",
        ProviderError::ResponseTooLarge => "response_too_large",
        ProviderError::InvalidResponse => "provider_error",
    }
}

fn current_project_id(connection: &Connection) -> Result<String, String> {
    get_current_project(connection).map(|p| p.id)
}

fn current_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
        .to_string()
}

fn map_seed_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ProjectSeed> {
    Ok(ProjectSeed {
        id: row.get(0)?,
        project_id: row.get(1)?,
        session_id: row.get(2)?,
        status: row.get(3)?,
        product_goal: row.get(4)?,
        target_users: row.get(5)?,
        mvp_scope: row.get(6)?,
        non_goals: row.get(7)?,
        key_features: row.get(8)?,
        pages_or_modules: row.get(9)?,
        data_entities: row.get(10)?,
        technical_constraints: row.get(11)?,
        acceptance_criteria: row.get(12)?,
        risk_points: row.get(13)?,
        open_questions: row.get(14)?,
        recommended_next_step: row.get(15)?,
        model_call_id: row.get(16)?,
        created_at: row.get(17)?,
        updated_at: row.get(18)?,
    })
}

// ---------------------------------------------------------------------------
// 测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::services::model_gateway::openai_compat::FakeModelProvider;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    const TEST_KEY: &str = "test-key";
    const TEST_URL: &str = "https://api.cheng.pink";
    const CONFIRM_QUESTIONS: &str = "我确认发起想法引导模型调用";
    const CONFIRM_SEED: &str = "我确认生成项目种子";

    fn test_db() -> (db::DbState, std::path::PathBuf) {
        let test_dir = std::env::temp_dir().join(format!(
            "agent-swarm-ig-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time")
                .as_nanos()
        ));
        let state = db::initialize(test_dir.clone()).expect("sqlite should initialize");
        (state, test_dir)
    }

    fn fake_questions_provider() -> Box<dyn ModelProvider> {
        Box::new(FakeModelProvider {
            content: Some(
                r#"["这个项目的目标用户是谁？","核心使用场景是什么？","有什么技术偏好吗？"]"#
                    .into(),
            ),
            error: None,
        })
    }

    fn fake_seed_provider() -> Box<dyn ModelProvider> {
        Box::new(FakeModelProvider {
            content: Some(
                r#"{"product_goal":"帮助小团队管理客户线索","target_users":"销售人员和客户经理","mvp_scope":"创建、查看、搜索和编辑客户线索","non_goals":"不做自动邮件、不做支付集成","key_features":"[\"客户线索CRUD\",\"搜索和过滤\",\"标签管理\"]","pages_or_modules":"[\"线索列表页\",\"线索编辑页\",\"搜索过滤栏\",\"标签管理\"]","data_entities":"[\"Lead\",\"Tag\"]","technical_constraints":"本地运行，SQLite存储，Tauri桌面应用","acceptance_criteria":"能创建线索、按关键字搜索、修改状态","risk_points":"数据量大时搜索可能变慢","open_questions":"是否需要数据导出功能？是否需要多用户？","recommended_next_step":"先做线索列表页的UI原型"}"#
                    .into(),
            ),
            error: None,
        })
    }

    // -------------------------------------------------------
    // 输入校验
    // -------------------------------------------------------

    #[test]
    fn rejects_empty_idea() {
        let err = validate_idea_input("", &None).expect_err("empty idea should be rejected");
        assert!(err.contains("invalid_input"));
    }

    #[test]
    fn rejects_forbidden_patterns_in_idea() {
        let err = validate_idea_input("sk-abcdefghijklmnopqrstuvwxyz123456", &None)
            .expect_err("sk- key should be rejected");
        assert!(err.contains("API key"));
    }

    #[test]
    fn accepts_normal_idea() {
        let result = validate_idea_input("我想做一个本地客户线索管理工具", &None);
        assert!(result.is_ok());
    }

    #[test]
    fn rejects_too_long_idea() {
        let long = "想".repeat(2001);
        let err = validate_idea_input(&long, &None).expect_err("too long idea should be rejected");
        assert!(err.contains("invalid_input"));
    }

    // -------------------------------------------------------
    // 追问解析
    // -------------------------------------------------------

    #[test]
    fn parse_questions_accepts_valid_json_array() {
        let input = r#"["问题一","问题二","问题三"]"#;
        let result = parse_questions_response(input).expect("should parse");
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], "问题一");
    }

    #[test]
    fn parse_questions_rejects_fewer_than_3() {
        let input = r#"["问题一","问题二"]"#;
        let err = parse_questions_response(input).expect_err("too few should be rejected");
        assert!(err.contains("invalid_response"));
    }

    #[test]
    fn parse_questions_rejects_more_than_7() {
        let input = r#"["1","2","3","4","5","6","7","8"]"#;
        let err = parse_questions_response(input).expect_err("too many should be rejected");
        assert!(err.contains("invalid_response"));
    }

    #[test]
    fn parse_questions_rejects_non_json() {
        let input = "这不是 JSON";
        let err = parse_questions_response(input).expect_err("non-JSON should be rejected");
        assert!(err.contains("invalid_response"));
    }

    #[test]
    fn parse_questions_extracts_from_markdown_block() {
        let input = "```json\n[\"问题一\",\"问题二\",\"问题三\"]\n```";
        let result = parse_questions_response(input).expect("should extract from markdown");
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn parse_questions_markdown_revalidates_sensitive_content() {
        // markdown 中提取的追问同样走 validate_and_filter_questions 的校验路径
        let input = "```json\n[\"正常问题\",\"sk-abcdefghijklmnopqrstuvwxyz123456\"]\n```";
        let result = parse_questions_response(input);
        assert!(
            result.is_err(),
            "markdown extracted sensitive question should be rejected"
        );
    }

    // -------------------------------------------------------
    // 种子解析
    // -------------------------------------------------------

    #[test]
    fn parse_seed_accepts_valid_json_object() {
        let input = r#"{
            "product_goal": "测试产品目标",
            "target_users": "测试用户",
            "mvp_scope": "测试MVP",
            "non_goals": "暂无",
            "key_features": "[\"功能1\"]",
            "pages_or_modules": "[\"页面1\"]",
            "data_entities": "[\"实体1\"]",
            "technical_constraints": "暂无",
            "acceptance_criteria": "测试验收",
            "risk_points": "暂无",
            "open_questions": "暂无",
            "recommended_next_step": "下一步"
        }"#;
        let result = parse_seed_response(input).expect("should parse");
        assert_eq!(result.product_goal, "测试产品目标");
    }

    #[test]
    fn parse_seed_rejects_missing_fields() {
        let input = r#"{"product_goal": "只有这一个字段"}"#;
        let err = parse_seed_response(input).expect_err("missing fields should be rejected");
        assert!(err.contains("missing required field"));
    }

    #[test]
    fn parse_seed_rejects_non_json() {
        let input = "这不是 JSON";
        let err = parse_seed_response(input).expect_err("non-JSON should be rejected");
        assert!(err.contains("invalid_response"));
    }

    #[test]
    fn parse_seed_extracts_from_markdown_block() {
        let input = r#"```json
{
    "product_goal": "测试目标",
    "target_users": "测试用户",
    "mvp_scope": "测试MVP",
    "non_goals": "暂无",
    "key_features": "[]",
    "pages_or_modules": "[]",
    "data_entities": "[]",
    "technical_constraints": "暂无",
    "acceptance_criteria": "测试验收",
    "risk_points": "暂无",
    "open_questions": "暂无",
    "recommended_next_step": "下一步"
}
```"#;
        let result = parse_seed_response(input).expect("should extract from markdown");
        assert_eq!(result.product_goal, "测试目标");
    }

    #[test]
    fn parse_seed_rejects_sensitive_in_any_field() {
        let input = r#"{
            "product_goal": "正常",
            "target_users": "正常",
            "mvp_scope": "正常",
            "non_goals": "正常",
            "key_features": "[]",
            "pages_or_modules": "[]",
            "data_entities": "[]",
            "technical_constraints": "sk-abcdefghijklmnopqrstuvwxyz123456",
            "acceptance_criteria": "正常",
            "risk_points": "正常",
            "open_questions": "正常",
            "recommended_next_step": "正常"
        }"#;
        let err = parse_seed_response(input).expect_err("sensitive field should be rejected");
        assert!(err.contains("forbidden content"));
    }

    // -------------------------------------------------------
    // 二次确认
    // -------------------------------------------------------

    #[test]
    fn rejects_missing_second_confirm() {
        let err = validate_second_confirm(false, &None, CONFIRM_QUESTIONS)
            .expect_err("no second_confirm should fail");
        assert!(err.contains("invalid_input"));
    }

    #[test]
    fn rejects_wrong_confirm_text() {
        let err = validate_second_confirm(true, &Some("随便".into()), CONFIRM_QUESTIONS)
            .expect_err("wrong confirm_text should fail");
        assert!(err.contains("invalid_input"));
    }

    #[test]
    fn accepts_correct_confirm() {
        let result =
            validate_second_confirm(true, &Some(CONFIRM_QUESTIONS.into()), CONFIRM_QUESTIONS);
        assert!(result.is_ok());
    }

    // -------------------------------------------------------
    // feature flag 关闭时
    // -------------------------------------------------------

    #[test]
    fn feature_disabled_returns_error_for_questions() {
        let (state, test_dir) = test_db();
        {
            let conn = state.connection().expect("connection");
            let result = create_questions_core(
                "测试想法",
                &None,
                &conn,
                "false",
                None,
                Some(fake_questions_provider()),
                Some(TEST_KEY),
                Some(TEST_URL),
                true,
                &Some(CONFIRM_QUESTIONS.into()),
            );
            assert!(result.is_err());
            assert!(result.unwrap_err().contains("feature_disabled"));
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }

    // -------------------------------------------------------
    // 完整流程：questions generation (fake provider)
    // -------------------------------------------------------

    #[test]
    fn create_questions_with_fake_provider_succeeds() {
        let (state, test_dir) = test_db();
        {
            let conn = state.connection().expect("connection");
            let result = create_questions_core(
                "测试想法",
                &None,
                &conn,
                "true",
                None,
                Some(fake_questions_provider()),
                Some(TEST_KEY),
                Some(TEST_URL),
                true,
                &Some(CONFIRM_QUESTIONS.into()),
            )
            .expect("should succeed");

            assert_eq!(result.session.status, "questions_ready");
            assert_eq!(result.questions.len(), 3);
            assert!(result.audit_record_id.is_some());
            assert!(result.warnings.iter().any(|w| w.contains("安全审计")));

            let loaded = load_session(&conn, &result.session.id, None).expect("should load");
            assert_eq!(loaded.status, "questions_ready");

            let questions =
                load_questions_for_session(&conn, &result.session.id, &result.session.project_id)
                    .expect("should load questions");
            assert_eq!(questions.len(), 3);
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn create_questions_writes_model_calls_audit() {
        let (state, test_dir) = test_db();
        {
            let conn = state.connection().expect("connection");
            let before: i64 = conn
                .query_row("SELECT COUNT(*) FROM model_calls", [], |row| row.get(0))
                .expect("count");

            let _result = create_questions_core(
                "测试想法",
                &None,
                &conn,
                "true",
                None,
                Some(fake_questions_provider()),
                Some(TEST_KEY),
                Some(TEST_URL),
                true,
                &Some(CONFIRM_QUESTIONS.into()),
            )
            .expect("should succeed");

            let after: i64 = conn
                .query_row("SELECT COUNT(*) FROM model_calls", [], |row| row.get(0))
                .expect("count");
            assert_eq!(after, before + 1, "应写入 1 条 model_calls 审计记录");
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn create_questions_does_not_create_tasks_or_runner_requests() {
        let (state, test_dir) = test_db();
        {
            let conn = state.connection().expect("connection");
            let tasks_before: i64 = conn
                .query_row("SELECT COUNT(*) FROM tasks", [], |row| row.get(0))
                .expect("count");
            let rr_before: i64 = conn
                .query_row("SELECT COUNT(*) FROM runner_requests", [], |row| row.get(0))
                .expect("count");

            let _result = create_questions_core(
                "测试想法",
                &None,
                &conn,
                "true",
                None,
                Some(fake_questions_provider()),
                Some(TEST_KEY),
                Some(TEST_URL),
                true,
                &Some(CONFIRM_QUESTIONS.into()),
            )
            .expect("should succeed");

            assert_eq!(
                conn.query_row::<i64, _, _>("SELECT COUNT(*) FROM tasks", [], |row| row.get(0))
                    .expect("count"),
                tasks_before,
                "不应创建 tasks"
            );
            assert_eq!(
                conn.query_row::<i64, _, _>("SELECT COUNT(*) FROM runner_requests", [], |row| row
                    .get(0))
                    .expect("count"),
                rr_before,
                "不应创建 runner_requests"
            );
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn create_questions_rejects_without_second_confirm() {
        let (state, test_dir) = test_db();
        {
            let conn = state.connection().expect("connection");
            let err = create_questions_core(
                "测试想法",
                &None,
                &conn,
                "true",
                None,
                Some(fake_questions_provider()),
                Some(TEST_KEY),
                Some(TEST_URL),
                false,
                &None,
            )
            .expect_err("missing second_confirm should fail");
            assert!(err.contains("invalid_input"));
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }

    // -------------------------------------------------------
    // provider 失败返回 Err
    // -------------------------------------------------------

    #[test]
    fn create_questions_returns_err_on_provider_error() {
        let (state, test_dir) = test_db();
        {
            let conn = state.connection().expect("connection");
            let failing_provider: Box<dyn ModelProvider> = Box::new(FakeModelProvider {
                content: None,
                error: Some(ProviderError::Timeout),
            });
            let result = create_questions_core(
                "测试想法",
                &None,
                &conn,
                "true",
                None,
                Some(failing_provider),
                Some(TEST_KEY),
                Some(TEST_URL),
                true,
                &Some(CONFIRM_QUESTIONS.into()),
            );
            assert!(result.is_err(), "provider error should return Err");
            assert!(result.unwrap_err().contains("provider_error"));
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }

    // -------------------------------------------------------
    // save_guidance_answers
    // -------------------------------------------------------

    #[test]
    fn save_answers_updates_question_status() {
        let (state, test_dir) = test_db();
        {
            let conn = state.connection().expect("connection");
            let result = create_questions_core(
                "测试想法",
                &None,
                &conn,
                "true",
                None,
                Some(fake_questions_provider()),
                Some(TEST_KEY),
                Some(TEST_URL),
                true,
                &Some(CONFIRM_QUESTIONS.into()),
            )
            .expect("should succeed");

            let answers: Vec<QuestionAnswer> = result
                .questions
                .iter()
                .map(|q| QuestionAnswer {
                    question_id: q.id.clone(),
                    answer: format!("回答: {}", q.question),
                })
                .collect();

            let session =
                save_guidance_answers(&conn, &result.session.id, &answers).expect("should save");
            assert_eq!(session.status, "questions_ready");

            let questions =
                load_questions_for_session(&conn, &result.session.id, &result.session.project_id)
                    .expect("should load");
            assert!(questions.iter().all(|q| q.status == "answered"));
            assert!(questions.iter().all(|q| q.answer.is_some()));
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn save_answers_rejects_forbidden_content() {
        let (state, test_dir) = test_db();
        {
            let conn = state.connection().expect("connection");
            let result = create_questions_core(
                "测试想法",
                &None,
                &conn,
                "true",
                None,
                Some(fake_questions_provider()),
                Some(TEST_KEY),
                Some(TEST_URL),
                true,
                &Some(CONFIRM_QUESTIONS.into()),
            )
            .expect("should succeed");

            let answers = vec![QuestionAnswer {
                question_id: result.questions[0].id.clone(),
                answer: "sk-abcdefghijklmnopqrstuvwxyz123456".into(),
            }];

            let err = save_guidance_answers(&conn, &result.session.id, &answers)
                .expect_err("forbidden content should be rejected");
            assert!(err.contains("API key"));
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }

    // -------------------------------------------------------
    // 完整流程：seed generation (fake provider)
    // -------------------------------------------------------

    #[test]
    fn generate_seed_with_fake_provider_succeeds() {
        let (state, test_dir) = test_db();
        {
            let conn = state.connection().expect("connection");

            let q_result = create_questions_core(
                "测试想法",
                &None,
                &conn,
                "true",
                None,
                Some(fake_questions_provider()),
                Some(TEST_KEY),
                Some(TEST_URL),
                true,
                &Some(CONFIRM_QUESTIONS.into()),
            )
            .expect("should succeed");

            let answers: Vec<QuestionAnswer> = q_result
                .questions
                .iter()
                .map(|q| QuestionAnswer {
                    question_id: q.id.clone(),
                    answer: format!("回答: {}", q.question),
                })
                .collect();
            save_guidance_answers(&conn, &q_result.session.id, &answers).expect("should save");

            let s_result = generate_seed_core(
                &q_result.session.id,
                &conn,
                "true",
                None,
                Some(fake_seed_provider()),
                Some(TEST_KEY),
                Some(TEST_URL),
                true,
                &Some(CONFIRM_SEED.into()),
            )
            .expect("should succeed");

            assert_eq!(s_result.seed.status, "ready");
            assert_eq!(s_result.session.status, "seed_ready");
            assert!(s_result.seed.product_goal.is_some());
            assert!(s_result.audit_record_id.is_some());

            let seeds = list_project_seeds(&conn).expect("should list");
            assert_eq!(seeds.len(), 1);
            assert_eq!(seeds[0].id, s_result.seed.id);
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn generate_seed_writes_model_calls_audit() {
        let (state, test_dir) = test_db();
        {
            let conn = state.connection().expect("connection");

            let q_result = create_questions_core(
                "测试想法",
                &None,
                &conn,
                "true",
                None,
                Some(fake_questions_provider()),
                Some(TEST_KEY),
                Some(TEST_URL),
                true,
                &Some(CONFIRM_QUESTIONS.into()),
            )
            .expect("should succeed");

            let answers: Vec<QuestionAnswer> = q_result
                .questions
                .iter()
                .map(|q| QuestionAnswer {
                    question_id: q.id.clone(),
                    answer: "测试回答".into(),
                })
                .collect();
            save_guidance_answers(&conn, &q_result.session.id, &answers).expect("should save");

            let before: i64 = conn
                .query_row("SELECT COUNT(*) FROM model_calls", [], |row| row.get(0))
                .expect("count");

            let _s_result = generate_seed_core(
                &q_result.session.id,
                &conn,
                "true",
                None,
                Some(fake_seed_provider()),
                Some(TEST_KEY),
                Some(TEST_URL),
                true,
                &Some(CONFIRM_SEED.into()),
            )
            .expect("should succeed");

            let after: i64 = conn
                .query_row("SELECT COUNT(*) FROM model_calls", [], |row| row.get(0))
                .expect("count");
            assert_eq!(after, before + 1, "应再写入 1 条审计记录（种子生成）");
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn generate_seed_does_not_create_tasks_or_runner_requests() {
        let (state, test_dir) = test_db();
        {
            let conn = state.connection().expect("connection");

            let q_result = create_questions_core(
                "测试想法",
                &None,
                &conn,
                "true",
                None,
                Some(fake_questions_provider()),
                Some(TEST_KEY),
                Some(TEST_URL),
                true,
                &Some(CONFIRM_QUESTIONS.into()),
            )
            .expect("should succeed");

            let answers: Vec<QuestionAnswer> = q_result
                .questions
                .iter()
                .map(|q| QuestionAnswer {
                    question_id: q.id.clone(),
                    answer: "测试回答".into(),
                })
                .collect();
            save_guidance_answers(&conn, &q_result.session.id, &answers).expect("should save");

            let tasks_before: i64 = conn
                .query_row("SELECT COUNT(*) FROM tasks", [], |row| row.get(0))
                .expect("count");
            let rr_before: i64 = conn
                .query_row("SELECT COUNT(*) FROM runner_requests", [], |row| row.get(0))
                .expect("count");

            let _s_result = generate_seed_core(
                &q_result.session.id,
                &conn,
                "true",
                None,
                Some(fake_seed_provider()),
                Some(TEST_KEY),
                Some(TEST_URL),
                true,
                &Some(CONFIRM_SEED.into()),
            )
            .expect("should succeed");

            assert_eq!(
                conn.query_row::<i64, _, _>("SELECT COUNT(*) FROM tasks", [], |row| row.get(0))
                    .expect("count"),
                tasks_before,
                "不应创建 tasks"
            );
            assert_eq!(
                conn.query_row::<i64, _, _>("SELECT COUNT(*) FROM runner_requests", [], |row| row
                    .get(0))
                    .expect("count"),
                rr_before,
                "不应创建 runner_requests"
            );
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn generate_seed_returns_err_on_provider_error() {
        let (state, test_dir) = test_db();
        {
            let conn = state.connection().expect("connection");

            let q_result = create_questions_core(
                "测试想法",
                &None,
                &conn,
                "true",
                None,
                Some(fake_questions_provider()),
                Some(TEST_KEY),
                Some(TEST_URL),
                true,
                &Some(CONFIRM_QUESTIONS.into()),
            )
            .expect("should succeed");

            let answers: Vec<QuestionAnswer> = q_result
                .questions
                .iter()
                .map(|q| QuestionAnswer {
                    question_id: q.id.clone(),
                    answer: "测试回答".into(),
                })
                .collect();
            save_guidance_answers(&conn, &q_result.session.id, &answers).expect("should save");

            let failing_provider: Box<dyn ModelProvider> = Box::new(FakeModelProvider {
                content: None,
                error: Some(ProviderError::NetworkError),
            });
            let result = generate_seed_core(
                &q_result.session.id,
                &conn,
                "true",
                None,
                Some(failing_provider),
                Some(TEST_KEY),
                Some(TEST_URL),
                true,
                &Some(CONFIRM_SEED.into()),
            );
            assert!(result.is_err(), "provider error should return Err");
            assert!(result.unwrap_err().contains("provider_error"));
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }

    // -------------------------------------------------------
    // 状态机校验
    // -------------------------------------------------------

    #[test]
    fn generate_seed_rejects_nonexistent_session() {
        let (state, test_dir) = test_db();
        {
            let conn = state.connection().expect("connection");
            let err = generate_seed_core(
                "nonexistent_session",
                &conn,
                "true",
                None,
                Some(fake_seed_provider()),
                Some(TEST_KEY),
                Some(TEST_URL),
                true,
                &Some(CONFIRM_SEED.into()),
            )
            .expect_err("nonexistent session should fail");
            assert!(err.contains("not_found"));
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }

    // -------------------------------------------------------
    // 跨项目隔离
    // -------------------------------------------------------

    #[test]
    fn save_answers_rejects_cross_project_session() {
        let (state, test_dir) = test_db();
        {
            let conn = state.connection().expect("connection");

            // 创建属于当前项目的 session
            let result = create_questions_core(
                "测试想法",
                &None,
                &conn,
                "true",
                None,
                Some(fake_questions_provider()),
                Some(TEST_KEY),
                Some(TEST_URL),
                true,
                &Some(CONFIRM_QUESTIONS.into()),
            )
            .expect("should succeed");

            // 插入另一个项目
            conn.execute(
                "INSERT INTO projects (id, name, status, created_at, updated_at)
                 VALUES ('other_proj', 'Other', 'planning', '2099-01-01', '2099-01-01')",
                [],
            )
            .expect("insert other project");

            // session 属于当前项目（非 other_proj），但从 other_proj 的视角请求
            // 由于 current_project_id 返回的是第一个项目，save_guidance_answers 用当前项目过滤
            // session 属于当前项目，所以应该成功
            // 真正跨项目测试：手动把 session 的 project_id 改成 other_proj
            conn.execute(
                "UPDATE idea_guidance_sessions SET project_id = 'other_proj' WHERE id = ?1",
                params![result.session.id.as_str()],
            )
            .expect("update session project");

            let answers = vec![QuestionAnswer {
                question_id: result.questions[0].id.clone(),
                answer: "测试回答".into(),
            }];

            let err = save_guidance_answers(&conn, &result.session.id, &answers)
                .expect_err("cross-project should fail");
            assert!(err.contains("not_found"));
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn generate_seed_rejects_cross_project_session() {
        let (state, test_dir) = test_db();
        {
            let conn = state.connection().expect("connection");

            let q_result = create_questions_core(
                "测试想法",
                &None,
                &conn,
                "true",
                None,
                Some(fake_questions_provider()),
                Some(TEST_KEY),
                Some(TEST_URL),
                true,
                &Some(CONFIRM_QUESTIONS.into()),
            )
            .expect("should succeed");

            // 把 session 移到另一个项目
            conn.execute(
                "INSERT INTO projects (id, name, status, created_at, updated_at)
                 VALUES ('other_proj', 'Other', 'planning', '2099-01-01', '2099-01-01')",
                [],
            )
            .expect("insert other project");
            conn.execute(
                "UPDATE idea_guidance_sessions SET project_id = 'other_proj' WHERE id = ?1",
                params![q_result.session.id.as_str()],
            )
            .expect("update session project");

            let err = generate_seed_core(
                &q_result.session.id,
                &conn,
                "true",
                None,
                Some(fake_seed_provider()),
                Some(TEST_KEY),
                Some(TEST_URL),
                true,
                &Some(CONFIRM_SEED.into()),
            )
            .expect_err("cross-project should fail");
            assert!(err.contains("not_found"));
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }

    // -------------------------------------------------------
    // list_project_seeds
    // -------------------------------------------------------

    #[test]
    fn list_seeds_returns_empty_initially() {
        let (state, test_dir) = test_db();
        {
            let conn = state.connection().expect("connection");
            let seeds = list_project_seeds(&conn).expect("should list");
            assert!(seeds.is_empty());
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }

    // -------------------------------------------------------
    // 迁移建表验证
    // -------------------------------------------------------

    #[test]
    fn parse_seed_rejects_sensitive_from_model_output() {
        // 模型输出中的敏感内容由 redact_secrets 处理，
        // 脱敏后的 [REDACTED_SECRET] 能通过 check_forbidden_value_patterns。
        // 这是正确的行为：redaction 是主防线，sensitive check 是二次校验。
        let input = r#"{
            "product_goal": "正常","target_users":"正常","mvp_scope":"正常",
            "non_goals":"正常","key_features":"[]","pages_or_modules":"[]",
            "data_entities":"[]",
            "technical_constraints":"sk-abcdefghijklmnopqrstuvwxyz123456",
            "acceptance_criteria":"正常","risk_points":"正常",
            "open_questions":"正常","recommended_next_step":"正常"
        }"#;
        // 未脱敏的原文本应被拒绝
        let err = parse_seed_response(input).expect_err("raw sensitive should be rejected");
        assert!(err.contains("forbidden content") || err.contains("API key"));
    }

    // -------------------------------------------------------
    // 迁移建表验证
    // -------------------------------------------------------

    #[test]
    fn idea_guidance_tables_exist() {
        let (state, test_dir) = test_db();
        {
            let conn = state.connection().expect("connection");
            let tables = [
                "idea_guidance_sessions",
                "idea_guidance_questions",
                "project_seeds",
            ];
            for table in &tables {
                let count: i64 = conn
                    .query_row(
                        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
                        params![table],
                        |row| row.get(0),
                    )
                    .expect("should query");
                assert!(count > 0, "table {table} should exist");
            }
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }

    // -------------------------------------------------------
    // Bug 修复：provider 成功但解析失败时写入 failed 审计
    // -------------------------------------------------------

    #[test]
    fn parse_failure_after_provider_success_writes_failed_audit() {
        let (state, test_dir) = test_db();
        {
            let conn = state.connection().expect("connection");
            // 返回非 JSON 内容，触发解析失败
            let bad_provider: Box<dyn ModelProvider> = Box::new(FakeModelProvider {
                content: Some("这不是有效的 JSON 输出".into()),
                error: None,
            });

            let before: i64 = conn
                .query_row("SELECT COUNT(*) FROM model_calls", [], |row| row.get(0))
                .expect("count");

            let result = create_questions_core(
                "测试想法",
                &None,
                &conn,
                "true",
                None,
                Some(bad_provider),
                Some(TEST_KEY),
                Some(TEST_URL),
                true,
                &Some(CONFIRM_QUESTIONS.into()),
            );
            assert!(result.is_err(), "解析失败应返回 Err");
            assert!(
                result.unwrap_err().contains("invalid_response"),
                "错误应包含 invalid_response"
            );

            let after: i64 = conn
                .query_row("SELECT COUNT(*) FROM model_calls", [], |row| row.get(0))
                .expect("count");
            assert_eq!(
                after,
                before + 1,
                "即使解析失败，模型已真实调用，必须写入 1 条 failed 审计"
            );

            // 验证审计记录内容
            let failed_status: String = conn
                .query_row(
                    "SELECT status FROM model_calls ORDER BY created_at DESC LIMIT 1",
                    [],
                    |row| row.get(0),
                )
                .expect("status");
            assert_eq!(failed_status, "failed");
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }

    // -------------------------------------------------------
    // Bug 修复：load_questions_for_session 按 project_id 过滤
    // -------------------------------------------------------

    #[test]
    fn load_questions_filters_by_project_id_rejects_dirty_data() {
        let (state, test_dir) = test_db();
        {
            let conn = state.connection().expect("connection");
            let project_id = current_project_id(&conn).expect("project_id");

            let result = create_questions_core(
                "测试想法",
                &None,
                &conn,
                "true",
                None,
                Some(fake_questions_provider()),
                Some(TEST_KEY),
                Some(TEST_URL),
                true,
                &Some(CONFIRM_QUESTIONS.into()),
            )
            .expect("should succeed");

            // 插入另一个项目
            conn.execute(
                "INSERT INTO projects (id, name, status, created_at, updated_at)
                 VALUES ('other_proj', 'Other', 'planning', '2099-01-01', '2099-01-01')",
                [],
            )
            .expect("insert other project");

            // 污染数据：把 question 的 project_id 改成 other_proj
            conn.execute(
                "UPDATE idea_guidance_questions SET project_id = 'other_proj' WHERE session_id = ?1",
                params![result.session.id.as_str()],
            )
            .expect("update question project");

            // 用当前 project_id 查询应查不到这些 question（它们已被移到 other_proj）
            let questions = load_questions_for_session(&conn, &result.session.id, &project_id)
                .expect("should load");
            assert!(
                questions.is_empty(),
                "污染到其他项目的 questions 不应被加载"
            );
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }

    // -------------------------------------------------------
    // Bug 修复：save_guidance_answers 对未知 question_id 报错
    // -------------------------------------------------------

    #[test]
    fn save_answers_rejects_unknown_question_id() {
        let (state, test_dir) = test_db();
        {
            let conn = state.connection().expect("connection");
            let result = create_questions_core(
                "测试想法",
                &None,
                &conn,
                "true",
                None,
                Some(fake_questions_provider()),
                Some(TEST_KEY),
                Some(TEST_URL),
                true,
                &Some(CONFIRM_QUESTIONS.into()),
            )
            .expect("should succeed");

            let answers = vec![QuestionAnswer {
                question_id: "nonexistent_question_id".into(),
                answer: "测试回答".into(),
            }];

            let err = save_guidance_answers(&conn, &result.session.id, &answers)
                .expect_err("unknown question_id should be rejected");
            assert!(
                err.contains("invalid_input"),
                "应返回 invalid_input，实际: {err}"
            );
            assert!(
                err.contains("not found"),
                "应提示 question_id not found，实际: {err}"
            );
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }

    // -------------------------------------------------------
    // Bug 修复：failed audit 写入失败不能吞掉，必须返回 audit_write_failed
    // -------------------------------------------------------

    #[test]
    fn parse_failure_audit_write_failure_returns_audit_write_failed() {
        let (state, test_dir) = test_db();
        {
            let conn = state.connection().expect("connection");
            let bad_provider: Box<dyn ModelProvider> = Box::new(FakeModelProvider {
                content: Some("这不是有效的 JSON".into()),
                error: None,
            });

            // 删除 model_calls 表使审计写入失败
            conn.execute("DROP TABLE model_calls", [])
                .expect("drop model_calls");

            let result = create_questions_core(
                "测试想法",
                &None,
                &conn,
                "true",
                None,
                Some(bad_provider),
                Some(TEST_KEY),
                Some(TEST_URL),
                true,
                &Some(CONFIRM_QUESTIONS.into()),
            );
            assert!(result.is_err(), "应返回 Err");
            let err = result.unwrap_err();
            assert!(
                err.contains("audit_write_failed"),
                "审计写入失败应返回 audit_write_failed，实际: {err}"
            );
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn provider_error_audit_write_failure_returns_audit_write_failed() {
        let (state, test_dir) = test_db();
        {
            let conn = state.connection().expect("connection");
            let failing_provider: Box<dyn ModelProvider> = Box::new(FakeModelProvider {
                content: None,
                error: Some(ProviderError::Timeout),
            });

            // 删除 model_calls 表使审计写入失败
            conn.execute("DROP TABLE model_calls", [])
                .expect("drop model_calls");

            let result = create_questions_core(
                "测试想法",
                &None,
                &conn,
                "true",
                None,
                Some(failing_provider),
                Some(TEST_KEY),
                Some(TEST_URL),
                true,
                &Some(CONFIRM_QUESTIONS.into()),
            );
            assert!(result.is_err(), "应返回 Err");
            let err = result.unwrap_err();
            assert!(
                err.contains("audit_write_failed"),
                "provider error 时审计写入失败应返回 audit_write_failed，实际: {err}"
            );
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }
}
