use serde::{Deserialize, Serialize};

use crate::db::DbState;
use crate::services::model_catalog::{self, ModelCatalogEntry, UpdateModelEnabledInput};
use crate::services::model_gateway::openai_compat::{
    ModelProvider, ModelRequest, OpenAiCompatProvider, ProviderError,
};

const PROVIDER_CREDENTIAL_SERVICE: &str = "agent-swarm";
const PROVIDER_API_KEY_ACCOUNT: &str = "openai_compat_api_key";
const PROVIDER_BASE_URL_ACCOUNT: &str = "openai_compat_base_url";
const PROVIDER_MODEL_ID_ACCOUNT: &str = "openai_compat_model_id";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateModelEnabledCmdInput {
    pub model_record_id: String,
    pub enabled: bool,
    pub second_confirm: bool,
    pub confirm_text: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateRuntimeModelProviderInput {
    pub api_key: String,
    pub base_url: String,
    pub model_id: String,
}

#[derive(Debug, Serialize)]
pub struct RuntimeModelProviderStatus {
    pub has_api_key: bool,
    pub api_key_hint: Option<String>,
    pub base_url: Option<String>,
    pub model_id: String,
}

#[derive(Debug, Serialize)]
pub struct TestRuntimeModelProviderResponse {
    pub ok: bool,
    pub status: String,
    pub message: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChatWithControllerInput {
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct ChatWithControllerResponse {
    pub reply: String,
    pub model_id: String,
}

#[tauri::command]
pub fn list_project_plan_models(
    state: tauri::State<'_, DbState>,
) -> Result<Vec<ModelCatalogEntry>, String> {
    let connection = state.connection()?;
    model_catalog::list_project_plan_models(&connection)
}

#[tauri::command]
pub fn update_project_plan_model_enabled(
    state: tauri::State<'_, DbState>,
    input: UpdateModelEnabledCmdInput,
) -> Result<Vec<ModelCatalogEntry>, String> {
    let mut connection = state.connection()?;
    model_catalog::update_model_enabled(
        &mut connection,
        UpdateModelEnabledInput {
            model_record_id: input.model_record_id,
            enabled: input.enabled,
            second_confirm: input.second_confirm,
            confirm_text: input.confirm_text,
        },
    )
}

#[tauri::command]
pub fn get_runtime_model_provider_status() -> Result<RuntimeModelProviderStatus, String> {
    hydrate_runtime_provider_from_credentials();
    Ok(runtime_status())
}

#[tauri::command]
pub fn update_runtime_model_provider(
    input: UpdateRuntimeModelProviderInput,
) -> Result<RuntimeModelProviderStatus, String> {
    let api_key = input.api_key.trim().to_string();
    let base_url = input.base_url.trim().trim_end_matches('/').to_string();
    let model_id = input.model_id.trim().to_string();

    if !base_url.starts_with("https://") {
        return Err("invalid_input: Base URL 必须以 https:// 开头".into());
    }
    model_catalog::validate_model_id(&model_id)?;

    let effective_api_key = if api_key.is_empty() {
        read_credential(PROVIDER_API_KEY_ACCOUNT)
            .or_else(|| std::env::var("AGENT_SWARM_OPENAI_COMPAT_API_KEY").ok())
            .ok_or_else(|| "invalid_input: 首次配置必须输入 API Key".to_string())?
    } else {
        if api_key.len() < 8 {
            return Err("invalid_input: API Key 太短".into());
        }
        write_credential(PROVIDER_API_KEY_ACCOUNT, &api_key)?;
        api_key
    };

    write_credential(PROVIDER_BASE_URL_ACCOUNT, &base_url)?;
    write_credential(PROVIDER_MODEL_ID_ACCOUNT, &model_id)?;

    std::env::set_var("AGENT_SWARM_OPENAI_COMPAT_API_KEY", effective_api_key);
    std::env::set_var("AGENT_SWARM_OPENAI_COMPAT_BASE_URL", base_url);
    std::env::set_var("AGENT_SWARM_RUNNER_MODEL_ID", model_id);

    Ok(runtime_status())
}

#[tauri::command]
pub fn test_runtime_model_provider() -> Result<TestRuntimeModelProviderResponse, String> {
    hydrate_runtime_provider_from_credentials();
    let status = runtime_status();
    if !status.has_api_key {
        return Ok(TestRuntimeModelProviderResponse {
            ok: false,
            status: "missing_key".into(),
            message: "还没有配置 API Key".into(),
        });
    }
    let Some(base_url) = status.base_url.clone() else {
        return Ok(TestRuntimeModelProviderResponse {
            ok: false,
            status: "missing_base_url".into(),
            message: "还没有配置 Base URL".into(),
        });
    };
    let api_key = std::env::var("AGENT_SWARM_OPENAI_COMPAT_API_KEY")
        .map_err(|_| "provider_config_error: missing key".to_string())?;
    let provider = OpenAiCompatProvider::from_values(api_key, base_url);
    let request = ModelRequest {
        system_prompt: "You are a connection test. Reply with OK only.".into(),
        user_message: "OK".into(),
        model_id: status.model_id.clone(),
    };

    match provider.send(&request, 20, 8 * 1024) {
        Ok(_) => Ok(TestRuntimeModelProviderResponse {
            ok: true,
            status: "ok".into(),
            message: "模型服务连接正常".into(),
        }),
        Err(error) => {
            let category = provider_error_category(&error);
            Ok(TestRuntimeModelProviderResponse {
                ok: false,
                status: category.into(),
                message: provider_error_message(category).into(),
            })
        }
    }
}

#[tauri::command]
pub fn chat_with_controller(
    state: tauri::State<'_, DbState>,
    input: ChatWithControllerInput,
) -> Result<ChatWithControllerResponse, String> {
    hydrate_runtime_provider_from_credentials();
    let user_message = input.message.trim();
    if user_message.is_empty() {
        return Err("invalid_input: message must not be empty".into());
    }
    if user_message.chars().count() > 1000 {
        return Err("invalid_input: message must be at most 1000 characters".into());
    }

    let connection = state.connection()?;
    let model_id = std::env::var("AGENT_SWARM_RUNNER_MODEL_ID")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| model_catalog::get_default_runner_model_id(&connection).ok())
        .unwrap_or_else(|| "deepseek-chat".to_string());
    model_catalog::validate_model_id(&model_id)?;

    let provider = OpenAiCompatProvider::from_env()
        .map_err(|_| "provider_config_error: 模型服务还没有配置完整".to_string())?;
    let request = ModelRequest {
        system_prompt: "你是 agent-swarm 桌面端的总控 Agent。用简洁中文回答用户，先解释概念，再给下一步建议。不要声称已经执行文件、命令、Git 或外部工具；如果用户要执行，提醒需要走 Runner 和审批链。".into(),
        user_message: user_message.to_string(),
        model_id: model_id.clone(),
    };

    let response = provider.send(&request, 45, 64 * 1024).map_err(|error| {
        format!(
            "controller_chat_error: {}",
            provider_error_message(provider_error_category(&error))
        )
    })?;

    Ok(ChatWithControllerResponse {
        reply: response.content.trim().to_string(),
        model_id,
    })
}

fn runtime_status() -> RuntimeModelProviderStatus {
    hydrate_runtime_provider_from_credentials();
    let api_key = std::env::var("AGENT_SWARM_OPENAI_COMPAT_API_KEY").ok();
    let base_url = std::env::var("AGENT_SWARM_OPENAI_COMPAT_BASE_URL").ok();
    let model_id = std::env::var("AGENT_SWARM_RUNNER_MODEL_ID")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "deepseek-chat".into());
    RuntimeModelProviderStatus {
        has_api_key: api_key
            .as_ref()
            .is_some_and(|value| !value.trim().is_empty()),
        api_key_hint: None,
        base_url,
        model_id,
    }
}

fn provider_error_category(error: &ProviderError) -> &'static str {
    match error {
        ProviderError::Timeout => "timeout",
        ProviderError::NetworkError => "network_error",
        ProviderError::ProviderError => "provider_error",
        ProviderError::AuthError => "auth_error",
        ProviderError::RateLimited => "rate_limited",
        ProviderError::ResponseTooLarge => "response_too_large",
        ProviderError::InvalidResponse => "invalid_response",
    }
}

fn provider_error_message(category: &str) -> &'static str {
    match category {
        "auth_error" => "API Key 失效、过期，或没有权限",
        "rate_limited" => "额度不足、限流，或余额不够",
        "timeout" => "模型服务超时",
        "network_error" => "网络连接失败",
        "invalid_response" => "模型服务返回格式不兼容",
        "response_too_large" => "模型返回内容过大",
        _ => "模型服务返回错误",
    }
}

pub fn hydrate_runtime_provider_from_credentials() {
    if std::env::var("AGENT_SWARM_OPENAI_COMPAT_API_KEY")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .is_none()
    {
        if let Some(api_key) = read_credential(PROVIDER_API_KEY_ACCOUNT) {
            std::env::set_var("AGENT_SWARM_OPENAI_COMPAT_API_KEY", api_key);
        }
    }

    if std::env::var("AGENT_SWARM_OPENAI_COMPAT_BASE_URL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .is_none()
    {
        if let Some(base_url) = read_credential(PROVIDER_BASE_URL_ACCOUNT) {
            std::env::set_var("AGENT_SWARM_OPENAI_COMPAT_BASE_URL", base_url);
        }
    }

    if std::env::var("AGENT_SWARM_RUNNER_MODEL_ID")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .is_none()
    {
        if let Some(model_id) = read_credential(PROVIDER_MODEL_ID_ACCOUNT) {
            std::env::set_var("AGENT_SWARM_RUNNER_MODEL_ID", model_id);
        }
    }
}

fn read_credential(account: &str) -> Option<String> {
    keyring::Entry::new(PROVIDER_CREDENTIAL_SERVICE, account)
        .ok()?
        .get_password()
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn write_credential(account: &str, value: &str) -> Result<(), String> {
    keyring::Entry::new(PROVIDER_CREDENTIAL_SERVICE, account)
        .map_err(|_| "credential_error: 无法打开系统凭据存储".to_string())?
        .set_password(value)
        .map_err(|_| "credential_error: 无法保存到系统凭据存储".to_string())
}
