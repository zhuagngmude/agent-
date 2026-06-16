use rusqlite::{params, Connection};
use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};

use super::projects::get_current_project;

const IDEA_MAX_LENGTH: usize = 1000;

#[derive(Debug, Serialize, Clone)]
pub struct ProjectIntakeSession {
    pub id: String,
    pub project_id: String,
    pub raw_idea: String,
    pub normalized_idea: String,
    pub project_type: String,
    pub project_type_label: String,
    pub confidence: i64,
    pub reason: String,
    pub recommended_questions: Vec<String>,
    pub recommended_next_step: String,
    pub status: String,
    pub created_by: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct ClassifyProjectIntakeResponse {
    pub session: ProjectIntakeSession,
    pub side_effects: ProjectIntakeSideEffects,
}

#[derive(Debug, Serialize)]
pub struct ProjectIntakeSideEffects {
    pub calls_real_model: bool,
    pub creates_tasks: bool,
    pub creates_approvals: bool,
    pub executes_runner: bool,
    pub writes_project_files: bool,
    pub modifies_git: bool,
}

#[derive(Debug, Clone)]
struct Classification {
    project_type: &'static str,
    label: &'static str,
    confidence: i64,
    reason: &'static str,
    questions: Vec<&'static str>,
    next_step: &'static str,
}

pub fn classify_project_intake(
    connection: &Connection,
    idea: &str,
) -> Result<ClassifyProjectIntakeResponse, String> {
    let project = get_current_project(connection)?;
    let normalized = normalize_idea(idea)?;
    let classification = classify(&normalized);
    let now = current_timestamp();
    let id = format!("project_intake_{}", timestamp_nanos());
    let questions_json = to_json_array(&classification.questions);

    connection
        .execute(
            "INSERT INTO project_intake_sessions (
                id, project_id, raw_idea, normalized_idea,
                project_type, project_type_label, confidence, reason,
                recommended_questions, recommended_next_step,
                status, created_by, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 'classified', 'local_user', ?11, ?12)",
            params![
                id.as_str(),
                project.id.as_str(),
                idea.trim(),
                normalized.as_str(),
                classification.project_type,
                classification.label,
                classification.confidence,
                classification.reason,
                questions_json.as_str(),
                classification.next_step,
                now.as_str(),
                now.as_str(),
            ],
        )
        .map_err(|e| format!("database_error: insert project_intake_sessions failed: {e}"))?;

    Ok(ClassifyProjectIntakeResponse {
        session: ProjectIntakeSession {
            id,
            project_id: project.id,
            raw_idea: idea.trim().to_string(),
            normalized_idea: normalized,
            project_type: classification.project_type.to_string(),
            project_type_label: classification.label.to_string(),
            confidence: classification.confidence,
            reason: classification.reason.to_string(),
            recommended_questions: classification
                .questions
                .into_iter()
                .map(str::to_string)
                .collect(),
            recommended_next_step: classification.next_step.to_string(),
            status: "classified".to_string(),
            created_by: "local_user".to_string(),
            created_at: now.clone(),
            updated_at: now,
        },
        side_effects: ProjectIntakeSideEffects::none(),
    })
}

pub fn list_project_intakes(connection: &Connection) -> Result<Vec<ProjectIntakeSession>, String> {
    let project = get_current_project(connection)?;
    let mut stmt = connection
        .prepare(
            "SELECT id, project_id, raw_idea, normalized_idea,
                    project_type, project_type_label, confidence, reason,
                    recommended_questions, recommended_next_step,
                    status, created_by, created_at, updated_at
             FROM project_intake_sessions
             WHERE project_id = ?1
             ORDER BY created_at DESC",
        )
        .map_err(|e| format!("database_error: list project_intake_sessions failed: {e}"))?;

    let rows = stmt
        .query_map(params![project.id.as_str()], map_intake_row)
        .map_err(|e| format!("database_error: list project_intake_sessions failed: {e}"))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("database_error: list project_intake_sessions failed: {e}"))
}

impl ProjectIntakeSideEffects {
    fn none() -> Self {
        Self {
            calls_real_model: false,
            creates_tasks: false,
            creates_approvals: false,
            executes_runner: false,
            writes_project_files: false,
            modifies_git: false,
        }
    }
}

fn normalize_idea(idea: &str) -> Result<String, String> {
    let normalized = idea.trim().split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.is_empty() {
        return Err("invalid_request: idea is required".to_string());
    }
    if normalized.chars().count() > IDEA_MAX_LENGTH {
        return Err(format!(
            "invalid_request: idea must be <= {IDEA_MAX_LENGTH} chars"
        ));
    }
    Ok(normalized)
}

fn classify(idea: &str) -> Classification {
    let lower = idea.to_lowercase();
    let scores = [
        ("software_product", score(&lower, SOFTWARE_KEYWORDS)),
        ("ai_automation", score(&lower, AI_AUTOMATION_KEYWORDS)),
        ("content_creation", score(&lower, CONTENT_KEYWORDS)),
        ("business_plan", score(&lower, BUSINESS_KEYWORDS)),
    ];
    let (project_type, best_score) = scores
        .into_iter()
        .max_by_key(|(_, score)| *score)
        .unwrap_or(("general_goal", 0));

    match project_type {
        "software_product" if best_score > 0 => classification_software(best_score),
        "ai_automation" if best_score > 0 => classification_automation(best_score),
        "content_creation" if best_score > 0 => classification_content(best_score),
        "business_plan" if best_score > 0 => classification_business(best_score),
        _ => classification_general(),
    }
}

fn classification_software(score: i64) -> Classification {
    Classification {
        project_type: "software_product",
        label: "软件产品",
        confidence: confidence(score),
        reason: "你的想法里出现了网站、应用、系统、页面、功能或用户工具等软件产品信号。",
        questions: vec![
            "目标用户是谁？他们现在最痛的地方是什么？",
            "第一版必须解决哪一个核心问题？",
            "你希望它运行在桌面端、网页、移动端，还是多端？",
            "第一版必须有哪 3 个功能？哪些明确不做？",
            "你希望多久看到可用的第一版？",
        ],
        next_step: "进入想法引导官，把软件产品范围收敛成项目种子。",
    }
}

fn classification_automation(score: i64) -> Classification {
    Classification {
        project_type: "ai_automation",
        label: "AI 自动化",
        confidence: confidence(score),
        reason: "你的想法里出现了自动、脚本、批量、工作流、智能体或数据处理等自动化信号。",
        questions: vec![
            "这个自动化的输入是什么？来自文件、网页、接口还是人工输入？",
            "你希望最终输出什么结果？",
            "它应该由什么事件触发：手动、定时、文件变化，还是任务队列？",
            "哪些动作有风险，必须人工确认？",
            "第一版只跑在本机，还是需要和外部服务连接？",
        ],
        next_step: "进入想法引导官，先画清输入、输出、触发方式和风险边界。",
    }
}

fn classification_content(score: i64) -> Classification {
    Classification {
        project_type: "content_creation",
        label: "内容创作",
        confidence: confidence(score),
        reason: "你的想法里出现了视频、小说、文案、课程、账号、脚本或选题等内容创作信号。",
        questions: vec![
            "内容面向谁？他们为什么会关注？",
            "你要做什么主题或系列？",
            "内容形式是文章、短视频、课程、小说，还是混合？",
            "你想要什么风格：专业、爽感、陪伴、故事化，还是实验性？",
            "第一周要产出哪些具体内容？",
        ],
        next_step: "进入想法引导官，把内容定位、栏目和第一批产出列清楚。",
    }
}

fn classification_business(score: i64) -> Classification {
    Classification {
        project_type: "business_plan",
        label: "商业方案",
        confidence: confidence(score),
        reason: "你的想法里出现了创业、产品、用户、竞品、商业模式、预算或增长等商业方案信号。",
        questions: vec![
            "目标客户是谁？他们愿意为什么付费？",
            "你解决的痛点是否足够高频或高价值？",
            "现有竞品或替代方案是什么？",
            "第一版 MVP 如何验证需求，而不是直接做大？",
            "你能投入多少时间、预算和资源？",
        ],
        next_step: "进入想法引导官，把商业假设、验证路径和 MVP 边界写成项目种子。",
    }
}

fn classification_general() -> Classification {
    Classification {
        project_type: "general_goal",
        label: "通用目标",
        confidence: 45,
        reason: "当前想法还比较开放，暂时无法稳定归入具体项目类型。",
        questions: vec![
            "你最终想得到一个工具、内容、方案，还是一个长期系统？",
            "这个想法主要服务你自己，还是服务其他用户？",
            "你最想先解决的一个具体问题是什么？",
            "有什么明确不能做、不能碰或不想投入的边界？",
            "如果一周内看到第一版，你希望它长什么样？",
        ],
        next_step: "进入想法引导官，先把目标、对象和第一版结果澄清。",
    }
}

fn score(text: &str, keywords: &[&str]) -> i64 {
    keywords
        .iter()
        .filter(|keyword| text.contains(**keyword))
        .count() as i64
}

fn confidence(score: i64) -> i64 {
    (55 + score * 12).clamp(55, 92)
}

fn to_json_array(values: &[&str]) -> String {
    let escaped = values
        .iter()
        .map(|v| format!("\"{}\"", v.replace('\\', "\\\\").replace('"', "\\\"")))
        .collect::<Vec<_>>()
        .join(",");
    format!("[{escaped}]")
}

fn parse_json_array(value: String) -> rusqlite::Result<Vec<String>> {
    serde_json::from_str::<Vec<String>>(&value).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
    })
}

fn map_intake_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ProjectIntakeSession> {
    let questions: String = row.get(8)?;
    Ok(ProjectIntakeSession {
        id: row.get(0)?,
        project_id: row.get(1)?,
        raw_idea: row.get(2)?,
        normalized_idea: row.get(3)?,
        project_type: row.get(4)?,
        project_type_label: row.get(5)?,
        confidence: row.get(6)?,
        reason: row.get(7)?,
        recommended_questions: parse_json_array(questions)?,
        recommended_next_step: row.get(9)?,
        status: row.get(10)?,
        created_by: row.get(11)?,
        created_at: row.get(12)?,
        updated_at: row.get(13)?,
    })
}

fn current_timestamp() -> String {
    format!("{}Z", timestamp_nanos())
}

fn timestamp_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0)
}

const SOFTWARE_KEYWORDS: &[&str] = &[
    "网站",
    "网页",
    "app",
    "应用",
    "软件",
    "系统",
    "小程序",
    "桌面端",
    "页面",
    "功能",
    "平台",
    "工具",
    "前端",
    "后端",
    "数据库",
    "用户登录",
];

const AI_AUTOMATION_KEYWORDS: &[&str] = &[
    "ai",
    "智能体",
    "自动",
    "自动化",
    "脚本",
    "批量",
    "工作流",
    "爬虫",
    "整理",
    "处理",
    "触发",
    "定时",
    "机器人",
    "agent",
];

const CONTENT_KEYWORDS: &[&str] = &[
    "视频",
    "短视频",
    "小说",
    "文案",
    "课程",
    "账号",
    "脚本",
    "选题",
    "公众号",
    "小红书",
    "抖音",
    "内容",
    "文章",
    "故事",
    "人设",
];

const BUSINESS_KEYWORDS: &[&str] = &[
    "商业",
    "创业",
    "产品",
    "方案",
    "用户",
    "客户",
    "竞品",
    "市场",
    "商业模式",
    "盈利",
    "预算",
    "增长",
    "mvp",
    "立项",
];

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    const PROJECT_INTAKE_MIGRATION_SQL: &str =
        include_str!("../../../../../data/migrations/014_add_project_intake.sql");

    fn setup_connection() -> Connection {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch(
                "
                PRAGMA foreign_keys = ON;
                CREATE TABLE projects (
                  id TEXT PRIMARY KEY,
                  name TEXT NOT NULL,
                  status TEXT NOT NULL,
                  phase TEXT,
                  created_at TEXT NOT NULL
                );
                INSERT INTO projects (id, name, status, phase, created_at)
                VALUES ('project_agent_swarm', 'agent蜂群', 'active', 'stage38', '2026-06-16T00:00:00Z');
                ",
            )
            .unwrap();
        connection
            .execute_batch(PROJECT_INTAKE_MIGRATION_SQL)
            .unwrap();
        connection
    }

    #[test]
    fn classifies_software_product() {
        let c = classify("我想做一个桌面端网站生成工具");
        assert_eq!(c.project_type, "software_product");
        assert_eq!(c.label, "软件产品");
        assert_eq!(c.questions.len(), 5);
    }

    #[test]
    fn classifies_content_creation() {
        let c = classify("我想做短视频账号选题和脚本");
        assert_eq!(c.project_type, "content_creation");
        assert!(c.reason.contains("内容创作"));
    }

    #[test]
    fn empty_idea_is_rejected() {
        let err = normalize_idea("   ").unwrap_err();
        assert!(err.contains("invalid_request"));
    }

    #[test]
    fn classify_project_intake_inserts_row_without_side_effects() {
        let connection = setup_connection();
        let response = classify_project_intake(&connection, "我想做一个桌面端软件工具").unwrap();

        assert_eq!(response.session.project_type, "software_product");
        assert!(!response.side_effects.calls_real_model);
        assert!(!response.side_effects.creates_tasks);
        assert!(!response.side_effects.creates_approvals);
        assert!(!response.side_effects.executes_runner);
        assert!(!response.side_effects.writes_project_files);
        assert!(!response.side_effects.modifies_git);

        let count: i64 = connection
            .query_row("SELECT COUNT(*) FROM project_intake_sessions", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn list_project_intakes_returns_current_project_rows() {
        let connection = setup_connection();
        classify_project_intake(&connection, "我想做短视频脚本工具").unwrap();

        connection
            .execute(
                "INSERT INTO projects (id, name, status, phase, created_at)
                 VALUES ('other_project', '其他项目', 'active', 'stage38', '2026-06-16T00:00:01Z')",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO project_intake_sessions (
                    id, project_id, raw_idea, normalized_idea,
                    project_type, project_type_label, confidence, reason,
                    recommended_questions, recommended_next_step,
                    status, created_by, created_at, updated_at
                 ) VALUES (
                    'other_intake', 'other_project', '其他想法', '其他想法',
                    'general_goal', '通用目标', 45, '其他项目数据',
                    '[]', '无', 'classified', 'local_user',
                    '2026-06-16T00:00:02Z', '2026-06-16T00:00:02Z'
                 )",
                [],
            )
            .unwrap();

        let rows = list_project_intakes(&connection).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].project_id, "project_agent_swarm");
    }
}
