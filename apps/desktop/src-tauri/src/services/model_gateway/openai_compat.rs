// ---------------------------------------------------------------------------
// OpenAI-compatible 适配器（阶段 25.1）
// 内部读取 env raw key / base URL，构造固定请求，发起 HTTP POST，解析响应。
// raw key / base URL 不进入任何 Serialize struct。
// 生产实现使用 ureq；测试通过 FakeModelProvider 注入预设结果。
// ---------------------------------------------------------------------------

use std::io::Read;

// ---------------------------------------------------------------------------
// 共享类型
// ---------------------------------------------------------------------------

/// 模型请求（由后端固定构造，前端不可控）
pub struct ModelRequest {
    pub system_prompt: String,
    pub user_message: String,
    /// 模型名——来自后端 model_catalog 校验后的 model_id，不来自前端自由文本。
    pub model_id: String,
}

/// 模型响应（只返回 assistant message content，不含 raw response）
#[derive(Debug)]
pub struct ModelResponse {
    pub content: String,
}

/// Provider 错误（粗粒度，不携带 raw provider body / header / status text）
#[derive(Debug)]
pub enum ProviderError {
    Timeout,
    NetworkError,
    ProviderError,
    ResponseTooLarge, // 与阶段 21 错误分类 response_too_large 对齐
    InvalidResponse,
}

/// 模型调用 trait，用于注入 fake provider 做测试。
/// 生产实现通过 ureq 发起真实 HTTP 请求；测试实现返回预设结果。
pub trait ModelProvider {
    fn send(
        &self,
        request: &ModelRequest,
        timeout_secs: u64,
        max_response_bytes: u64,
    ) -> Result<ModelResponse, ProviderError>;
}

// ---------------------------------------------------------------------------
// 生产实现：OpenAI-compatible HTTP provider（使用 ureq）
// ---------------------------------------------------------------------------

pub struct OpenAiCompatProvider {
    /// NOT Serialize, internal only — 绝不进入任何返回值或日志
    api_key: String,
    /// NOT Serialize, internal only — 绝不进入任何返回值或日志
    base_url: String,
}

impl OpenAiCompatProvider {
    /// 从环境变量构造。raw key / base URL 只存于实例字段，不会泄露。
    pub fn from_env() -> Result<Self, &'static str> {
        let api_key = std::env::var("AGENT_SWARM_OPENAI_COMPAT_API_KEY")
            .map_err(|_| "AGENT_SWARM_OPENAI_COMPAT_API_KEY 环境变量未设置")?;
        let base_url = std::env::var("AGENT_SWARM_OPENAI_COMPAT_BASE_URL")
            .map_err(|_| "AGENT_SWARM_OPENAI_COMPAT_BASE_URL 环境变量未设置")?;
        Ok(Self { api_key, base_url })
    }
}

impl ModelProvider for OpenAiCompatProvider {
    fn send(
        &self,
        request: &ModelRequest,
        timeout_secs: u64,
        max_response_bytes: u64,
    ) -> Result<ModelResponse, ProviderError> {
        // 构造 URL：base_url 统一补 /v1，再拼 /chat/completions
        let base = self.base_url.trim_end_matches('/');
        let url = if base.ends_with("/v1") {
            format!("{base}/chat/completions")
        } else {
            format!("{base}/v1/chat/completions")
        };

        // 固定请求体：provider=openai_compat, model 来自后端受控目录
        let body = serde_json::json!({
            "model": request.model_id,
            "messages": [
                {"role": "system", "content": request.system_prompt},
                {"role": "user", "content": request.user_message}
            ],
            "temperature": 0.2,
            "stream": false
        });

        let body_string =
            serde_json::to_string(&body).map_err(|_| ProviderError::InvalidResponse)?;

        // 发起 POST，设置 Authorization header 和 timeout
        let response = ureq::post(&url)
            .set("Authorization", &format!("Bearer {}", self.api_key))
            .set("Content-Type", "application/json")
            .timeout(std::time::Duration::from_secs(timeout_secs))
            .send_string(&body_string)
            .map_err(|e| match &e {
                ureq::Error::Status(_status_code, _response) => ProviderError::ProviderError,
                ureq::Error::Transport(_transport) => {
                    let msg = e.to_string().to_lowercase();
                    if msg.contains("timeout") || msg.contains("timed out") {
                        ProviderError::Timeout
                    } else {
                        ProviderError::NetworkError
                    }
                }
            })?;

        // 检查 HTTP 状态码：非 200 只返回粗粒度错误，不读取/返回 provider error body
        if response.status() != 200 {
            return Err(ProviderError::ProviderError);
        }

        // 限长读取响应体（max_response_bytes + 1 用于检测超限）
        let mut reader = response.into_reader().take(max_response_bytes + 1);
        let mut body_bytes = Vec::new();
        reader
            .read_to_end(&mut body_bytes)
            .map_err(|_| ProviderError::NetworkError)?;

        if body_bytes.len() as u64 > max_response_bytes {
            return Err(ProviderError::ResponseTooLarge);
        }

        let body_text =
            String::from_utf8(body_bytes).map_err(|_| ProviderError::InvalidResponse)?;

        let content = parse_chat_completion_content(&body_text)?;

        Ok(ModelResponse { content })
    }
}

// ---------------------------------------------------------------------------
// 纯函数：从 OpenAI-compatible JSON 响应中解析 assistant content
// 可供测试直接调用，不依赖网络或 ureq
// ---------------------------------------------------------------------------

pub fn parse_chat_completion_content(body_text: &str) -> Result<String, ProviderError> {
    let parsed: serde_json::Value =
        serde_json::from_str(body_text).map_err(|_| ProviderError::InvalidResponse)?;

    let content = parsed["choices"]
        .as_array()
        .and_then(|choices| choices.first())
        .and_then(|choice| choice["message"]["content"].as_str())
        .unwrap_or("")
        .to_string();

    if content.trim().is_empty() {
        return Err(ProviderError::InvalidResponse);
    }

    Ok(content)
}

// ---------------------------------------------------------------------------
// 测试用 Fake provider（不发起真实 HTTP 请求）
// ---------------------------------------------------------------------------

#[allow(dead_code)]
pub struct FakeModelProvider {
    /// 预设的成功响应内容
    pub content: Option<String>,
    /// 预设的错误（按 variant 类型匹配，不依赖 Clone）
    pub error: Option<ProviderError>,
}

impl ModelProvider for FakeModelProvider {
    fn send(
        &self,
        _request: &ModelRequest,
        _timeout_secs: u64,
        _max_response_bytes: u64,
    ) -> Result<ModelResponse, ProviderError> {
        match &self.error {
            Some(ProviderError::Timeout) => Err(ProviderError::Timeout),
            Some(ProviderError::NetworkError) => Err(ProviderError::NetworkError),
            Some(ProviderError::ProviderError) => Err(ProviderError::ProviderError),
            Some(ProviderError::ResponseTooLarge) => Err(ProviderError::ResponseTooLarge),
            Some(ProviderError::InvalidResponse) => Err(ProviderError::InvalidResponse),
            None => Ok(ModelResponse {
                content: self.content.clone().unwrap_or_default(),
            }),
        }
    }
}

// ---------------------------------------------------------------------------
// 测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------
    // FakeModelProvider 行为验证
    // -------------------------------------------------------

    #[test]
    fn fake_provider_returns_preset_content() {
        let fake = FakeModelProvider {
            content: Some("测试摘要".into()),
            error: None,
        };
        let req = ModelRequest {
            system_prompt: "sys".into(),
            user_message: "user".into(),
            model_id: "gpt-5.4-mini".into(),
        };
        let result = fake.send(&req, 10, 1024).unwrap();
        assert_eq!(result.content, "测试摘要");
    }

    #[test]
    fn fake_provider_returns_timeout_error() {
        let fake = FakeModelProvider {
            content: None,
            error: Some(ProviderError::Timeout),
        };
        let req = ModelRequest {
            system_prompt: "sys".into(),
            user_message: "user".into(),
            model_id: "gpt-5.4-mini".into(),
        };
        let result = fake.send(&req, 10, 1024);
        assert!(result.is_err());
        match result.unwrap_err() {
            ProviderError::Timeout => {} // expected
            other => panic!("expected Timeout, got {:?}", other),
        }
    }

    #[test]
    fn fake_provider_returns_provider_error() {
        let fake = FakeModelProvider {
            content: None,
            error: Some(ProviderError::ProviderError),
        };
        let req = ModelRequest {
            system_prompt: "sys".into(),
            user_message: "user".into(),
            model_id: "gpt-5.4-mini".into(),
        };
        let result = fake.send(&req, 10, 1024);
        assert!(result.is_err());
        match result.unwrap_err() {
            ProviderError::ProviderError => {}
            other => panic!("expected ProviderError, got {:?}", other),
        }
    }

    // -------------------------------------------------------
    // OpenAiCompatProvider from_env 验证
    // -------------------------------------------------------

    /// 验证 InvalidResponse 作为 ProviderError 能被正确匹配
    #[test]
    fn fake_provider_returns_invalid_response() {
        let fake = FakeModelProvider {
            content: None,
            error: Some(ProviderError::InvalidResponse),
        };
        let req = ModelRequest {
            system_prompt: "sys".into(),
            user_message: "user".into(),
            model_id: "gpt-5.4-mini".into(),
        };
        let result = fake.send(&req, 10, 1024);
        assert!(result.is_err());
        match result.unwrap_err() {
            ProviderError::InvalidResponse => {}
            other => panic!("expected InvalidResponse, got {:?}", other),
        }
    }

    // -------------------------------------------------------
    // parse_chat_completion_content 纯函数测试
    // -------------------------------------------------------

    #[test]
    fn parse_empty_choices_array_returns_invalid_response() {
        let result = parse_chat_completion_content(r#"{"choices":[]}"#);
        assert!(result.is_err());
        match result.unwrap_err() {
            ProviderError::InvalidResponse => {}
            other => panic!("expected InvalidResponse, got {:?}", other),
        }
    }

    #[test]
    fn parse_empty_content_string_returns_invalid_response() {
        let result = parse_chat_completion_content(r#"{"choices":[{"message":{"content":""}}]}"#);
        assert!(result.is_err());
    }

    #[test]
    fn parse_whitespace_only_content_returns_invalid_response() {
        let result = parse_chat_completion_content(r#"{"choices":[{"message":{"content":"  "}}]}"#);
        assert!(result.is_err());
    }

    #[test]
    fn parse_non_json_returns_invalid_response() {
        let result = parse_chat_completion_content("not json at all");
        assert!(result.is_err());
        match result.unwrap_err() {
            ProviderError::InvalidResponse => {}
            other => panic!("expected InvalidResponse, got {:?}", other),
        }
    }

    #[test]
    fn parse_valid_content_returns_ok() {
        let result = parse_chat_completion_content(
            r#"{"choices":[{"message":{"content":"项目计划摘要..."}}]}"#,
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "项目计划摘要...");
    }

    #[test]
    fn openai_compat_from_env_does_not_panic() {
        // 只验证不 panic。返回值依赖本机 env，不硬断言成功/失败。
        let _ = OpenAiCompatProvider::from_env();
    }
}
