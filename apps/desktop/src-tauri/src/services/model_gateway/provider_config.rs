// ---------------------------------------------------------------------------
// Provider 配置解析（可注入 env 值，避免测试并发污染全局环境变量）
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
    #[allow(dead_code)] // 预留给后续真实模型 adapter 使用
    pub default_model: String,
    #[allow(dead_code)]
    pub allowed_models: Vec<String>,
    #[allow(dead_code)]
    pub allowed_purposes: Vec<String>,
}

/// 从实际环境变量解析配置
pub fn resolve_provider_config() -> ProviderConfig {
    let key = std::env::var("AGENT_SWARM_OPENAI_COMPAT_API_KEY").ok();
    let base_url = std::env::var("AGENT_SWARM_OPENAI_COMPAT_BASE_URL").ok();
    resolve(key.as_deref(), base_url.as_deref())
}

/// 纯函数版：可注入 key/base_url 用于测试，不会污染全局环境变量
pub(crate) fn resolve(key: Option<&str>, base_url: Option<&str>) -> ProviderConfig {
    let status = match (key, base_url) {
        (None, _) => ProviderConfigStatus::MissingKey,
        (_, None) => ProviderConfigStatus::MissingBaseUrl,
        (Some(_), Some(url)) if !is_valid_base_url(url) => ProviderConfigStatus::InvalidBaseUrl,
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
    if !lower.starts_with("https://") {
        return false;
    }
    if lower.contains("localhost") || lower.contains("127.0.0.1") {
        return false;
    }
    if lower.contains("192.168.") || lower.contains("10.") || lower.contains("172.16.") {
        return false;
    }
    true
}

// ---------------------------------------------------------------------------
// 测试（纯函数版 resolve，不碰全局 env）
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_key_when_not_set() {
        let config = resolve(None, None);
        assert_eq!(config.status, ProviderConfigStatus::MissingKey);
    }

    #[test]
    fn missing_base_url_when_only_key_set() {
        let config = resolve(Some("sk-test"), None);
        assert_eq!(config.status, ProviderConfigStatus::MissingBaseUrl);
    }

    #[test]
    fn rejects_non_https_base_url() {
        let config = resolve(Some("sk-test"), Some("http://example.com"));
        assert_eq!(config.status, ProviderConfigStatus::InvalidBaseUrl);
    }

    #[test]
    fn rejects_localhost_base_url() {
        let config = resolve(Some("sk-test"), Some("https://127.0.0.1:8080"));
        assert_eq!(config.status, ProviderConfigStatus::InvalidBaseUrl);
    }

    #[test]
    fn rejects_private_ip_base_url() {
        let config = resolve(Some("sk-test"), Some("https://192.168.1.1/api"));
        assert_eq!(config.status, ProviderConfigStatus::InvalidBaseUrl);
    }

    #[test]
    fn accepts_valid_https_base_url() {
        let config = resolve(Some("sk-test"), Some("https://api.openai.com"));
        assert_eq!(config.status, ProviderConfigStatus::Configured);
        assert_eq!(config.default_model, "gpt-5.4-mini");
        assert!(config.allowed_models.contains(&"gpt-5.4-mini".into()));
        assert!(config
            .allowed_purposes
            .contains(&"project_plan_generation".into()));
    }

    #[test]
    fn does_not_expose_key_value() {
        let config = resolve(Some("sk-secret-abc123"), Some("https://api.openai.com"));
        assert_eq!(config.status, ProviderConfigStatus::Configured);
        // ProviderConfig 不包含 key 字段，调用方拿不到 raw key
    }

    #[test]
    fn real_env_resolve_does_not_panic() {
        // 只验证不 panic，不依赖具体 env 值
        let _ = resolve_provider_config();
    }
}
