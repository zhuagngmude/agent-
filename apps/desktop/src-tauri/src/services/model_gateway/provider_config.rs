// ---------------------------------------------------------------------------
// Provider 配置解析（只读 env，不返回 raw key / base URL）
// ---------------------------------------------------------------------------

/// Provider 配置状态（粗粒度，前端只能看到此枚举值）
#[derive(PartialEq, Debug)]
pub enum ProviderConfigStatus {
    Configured,
    MissingKey,
    MissingBaseUrl,
    InvalidBaseUrl,
}

pub struct ProviderConfig {
    pub status: ProviderConfigStatus,
    pub default_model: String,
    pub allowed_models: Vec<String>,
    pub allowed_purposes: Vec<String>,
}

pub fn resolve_provider_config() -> ProviderConfig {
    let key = std::env::var("AGENT_SWARM_OPENAI_COMPAT_API_KEY").ok();
    let base_url = std::env::var("AGENT_SWARM_OPENAI_COMPAT_BASE_URL").ok();

    let status = match (&key, &base_url) {
        (None, _) => ProviderConfigStatus::MissingKey,
        (_, None) => ProviderConfigStatus::MissingBaseUrl,
        (Some(_), Some(url)) if !is_valid_base_url(url) => {
            ProviderConfigStatus::InvalidBaseUrl
        }
        _ => ProviderConfigStatus::Configured,
    };

    ProviderConfig {
        status,
        default_model: "gpt-5.4-mini".into(),
        allowed_models: vec!["gpt-5.4-mini".into()],
        allowed_purposes: vec!["project_plan_generation".into()],
    }
}

fn is_valid_base_url(url: &str) -> bool {
    let lower = url.to_lowercase();
    // 必须是 https
    if !lower.starts_with("https://") {
        return false;
    }
    // 不允许 localhost 或 loopback
    if lower.contains("localhost") || lower.contains("127.0.0.1") {
        return false;
    }
    // 不允许私有 IP
    if lower.contains("192.168.") || lower.contains("10.") || lower.contains("172.16.") {
        return false;
    }
    true
}

// ---------------------------------------------------------------------------
// 测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_config_returns_missing_key_when_env_not_set() {
        // 测试前清除环境变量
        std::env::remove_var("AGENT_SWARM_OPENAI_COMPAT_API_KEY");
        std::env::remove_var("AGENT_SWARM_OPENAI_COMPAT_BASE_URL");

        let config = resolve_provider_config();
        assert_eq!(config.status, ProviderConfigStatus::MissingKey);
    }

    #[test]
    fn resolve_config_returns_missing_base_url_when_only_key_set() {
        std::env::set_var("AGENT_SWARM_OPENAI_COMPAT_API_KEY", "test-key-123");
        std::env::remove_var("AGENT_SWARM_OPENAI_COMPAT_BASE_URL");

        let config = resolve_provider_config();
        assert_eq!(config.status, ProviderConfigStatus::MissingBaseUrl);

        std::env::remove_var("AGENT_SWARM_OPENAI_COMPAT_API_KEY");
    }

    #[test]
    fn resolve_config_rejects_non_https_base_url() {
        std::env::set_var("AGENT_SWARM_OPENAI_COMPAT_API_KEY", "test-key");
        std::env::set_var("AGENT_SWARM_OPENAI_COMPAT_BASE_URL", "http://example.com");

        let config = resolve_provider_config();
        assert_eq!(config.status, ProviderConfigStatus::InvalidBaseUrl);

        std::env::remove_var("AGENT_SWARM_OPENAI_COMPAT_API_KEY");
        std::env::remove_var("AGENT_SWARM_OPENAI_COMPAT_BASE_URL");
    }

    #[test]
    fn resolve_config_rejects_localhost_base_url() {
        std::env::set_var("AGENT_SWARM_OPENAI_COMPAT_API_KEY", "test-key");
        std::env::set_var("AGENT_SWARM_OPENAI_COMPAT_BASE_URL", "https://127.0.0.1:8080");

        let config = resolve_provider_config();
        assert_eq!(config.status, ProviderConfigStatus::InvalidBaseUrl);

        std::env::remove_var("AGENT_SWARM_OPENAI_COMPAT_API_KEY");
        std::env::remove_var("AGENT_SWARM_OPENAI_COMPAT_BASE_URL");
    }

    #[test]
    fn resolve_config_rejects_private_ip_base_url() {
        std::env::set_var("AGENT_SWARM_OPENAI_COMPAT_API_KEY", "test-key");
        std::env::set_var("AGENT_SWARM_OPENAI_COMPAT_BASE_URL", "https://192.168.1.1/api");

        let config = resolve_provider_config();
        assert_eq!(config.status, ProviderConfigStatus::InvalidBaseUrl);

        std::env::remove_var("AGENT_SWARM_OPENAI_COMPAT_API_KEY");
        std::env::remove_var("AGENT_SWARM_OPENAI_COMPAT_BASE_URL");
    }

    #[test]
    fn resolve_config_accepts_valid_https_base_url() {
        std::env::set_var("AGENT_SWARM_OPENAI_COMPAT_API_KEY", "test-key");
        std::env::set_var("AGENT_SWARM_OPENAI_COMPAT_BASE_URL", "https://api.openai.com");

        let config = resolve_provider_config();
        assert_eq!(config.status, ProviderConfigStatus::Configured);
        assert_eq!(config.default_model, "gpt-5.4-mini");
        assert!(config.allowed_models.contains(&"gpt-5.4-mini".into()));
        assert!(config.allowed_purposes.contains(&"project_plan_generation".into()));

        std::env::remove_var("AGENT_SWARM_OPENAI_COMPAT_API_KEY");
        std::env::remove_var("AGENT_SWARM_OPENAI_COMPAT_BASE_URL");
    }

    #[test]
    fn resolve_config_does_not_expose_key_value() {
        std::env::set_var("AGENT_SWARM_OPENAI_COMPAT_API_KEY", "sk-secret-abc123");
        std::env::set_var("AGENT_SWARM_OPENAI_COMPAT_BASE_URL", "https://api.openai.com");

        let config = resolve_provider_config();
        // 验证 status 是粗粒度枚举值
        assert_eq!(config.status, ProviderConfigStatus::Configured);
        // ProviderConfig 结构体本身不包含 key 或 base_url 字段——
        // 调用方只能拿到 Configured/MissingKey/MissingBaseUrl/InvalidBaseUrl
        // 无法拿到 raw key、key suffix、masked fragment 或 base URL 原文

        std::env::remove_var("AGENT_SWARM_OPENAI_COMPAT_API_KEY");
        std::env::remove_var("AGENT_SWARM_OPENAI_COMPAT_BASE_URL");
    }
}
