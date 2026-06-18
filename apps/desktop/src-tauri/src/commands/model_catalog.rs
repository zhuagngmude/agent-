use serde::{Deserialize, Serialize};

use crate::db::DbState;
use crate::services::model_catalog::{self, ModelCatalogEntry, UpdateModelEnabledInput};
use crate::services::model_gateway::openai_compat::{
    ModelProvider, ModelRequest, OpenAiCompatProvider, ProviderError,
};

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
    Ok(runtime_status())
}

#[tauri::command]
pub fn update_runtime_model_provider(
    input: UpdateRuntimeModelProviderInput,
) -> Result<RuntimeModelProviderStatus, String> {
    let api_key = input.api_key.trim().to_string();
    let base_url = input.base_url.trim().trim_end_matches('/').to_string();
    let model_id = input.model_id.trim().to_string();

    if api_key.len() < 8 {
        return Err("invalid_input: API Key 太短".into());
    }
    if !base_url.starts_with("https://") {
        return Err("invalid_input: Base URL 必须以 https:// 开头".into());
    }
    model_catalog::validate_model_id(&model_id)?;

    std::env::set_var("AGENT_SWARM_OPENAI_COMPAT_API_KEY", api_key);
    std::env::set_var("AGENT_SWARM_OPENAI_COMPAT_BASE_URL", base_url);
    std::env::set_var("AGENT_SWARM_RUNNER_MODEL_ID", model_id);

    Ok(runtime_status())
}

#[tauri::command]
pub fn test_runtime_model_provider() -> Result<TestRuntimeModelProviderResponse, String> {
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

fn runtime_status() -> RuntimeModelProviderStatus {
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
        api_key_hint: api_key.as_deref().map(mask_key),
        base_url,
        model_id,
    }
}

fn mask_key(value: &str) -> String {
    let trimmed = value.trim();
    let tail: String = trimmed
        .chars()
        .rev()
        .take(4)
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    format!("****{tail}")
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
