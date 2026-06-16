// 阶段 37：想法引导官 Tauri 命令
// 权限级别 L1（模型草案），不执行 Runner、不写文件、不改 Git。
// L1 不是免确认：真实模型调用前必须二次确认。
// 所有输入结构体使用 deny_unknown_fields 防止前端传入自由字段。

use serde::Deserialize;

use crate::db::DbState;
use crate::services::idea_guidance::{
    self, CreateIdeaGuidanceQuestionsResponse, GenerateProjectSeedResponse, IdeaGuidanceSession,
    ProjectSeed, QuestionAnswer,
};

/// 创建想法引导追问
/// 真实模型调用必须二次确认。
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateIdeaGuidanceQuestionsInput {
    pub idea: String,
    #[serde(default)]
    pub constraints: Option<String>,
    #[serde(default)]
    pub model_record_id: Option<String>,
    #[serde(default)]
    pub second_confirm: bool,
    #[serde(default)]
    pub confirm_text: Option<String>,
}

#[tauri::command]
pub fn create_idea_guidance_questions(
    state: tauri::State<'_, DbState>,
    input: CreateIdeaGuidanceQuestionsInput,
) -> Result<CreateIdeaGuidanceQuestionsResponse, String> {
    let connection = state.connection()?;
    idea_guidance::create_idea_guidance_questions(
        &input.idea,
        &input.constraints,
        &connection,
        input.model_record_id.as_deref(),
        input.second_confirm,
        &input.confirm_text,
    )
}

/// 生成项目种子
/// 从 DB 读取 session 和已回答的问题，构造 prompt 调用模型。
/// 真实模型调用必须二次确认。
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GenerateProjectSeedInput {
    pub session_id: String,
    #[serde(default)]
    pub model_record_id: Option<String>,
    #[serde(default)]
    pub second_confirm: bool,
    #[serde(default)]
    pub confirm_text: Option<String>,
}

#[tauri::command]
pub fn generate_project_seed(
    state: tauri::State<'_, DbState>,
    input: GenerateProjectSeedInput,
) -> Result<GenerateProjectSeedResponse, String> {
    let connection = state.connection()?;
    idea_guidance::generate_project_seed(
        &input.session_id,
        &connection,
        input.model_record_id.as_deref(),
        input.second_confirm,
        &input.confirm_text,
    )
}

/// 保存用户对追问的回答（不调用模型，无需二次确认）
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SaveGuidanceAnswersInput {
    pub session_id: String,
    pub answers: Vec<QuestionAnswerInput>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuestionAnswerInput {
    pub question_id: String,
    pub answer: String,
}

#[tauri::command]
pub fn save_guidance_answers(
    state: tauri::State<'_, DbState>,
    input: SaveGuidanceAnswersInput,
) -> Result<IdeaGuidanceSession, String> {
    let connection = state.connection()?;
    let answers: Vec<QuestionAnswer> = input
        .answers
        .into_iter()
        .map(|a| QuestionAnswer {
            question_id: a.question_id,
            answer: a.answer,
        })
        .collect();
    idea_guidance::save_guidance_answers(&connection, &input.session_id, &answers)
}

/// 列出当前项目的所有种子
#[tauri::command]
pub fn list_project_seeds(state: tauri::State<'_, DbState>) -> Result<Vec<ProjectSeed>, String> {
    let connection = state.connection()?;
    idea_guidance::list_project_seeds(&connection)
}
