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
    // 白名单：只允许 api.cheng.pink（无 path 或 /v1）
    // 见 dev-docs/Model Gateway正式入口设计.md 第 217 行
    let lower = url.to_lowercase();
    let Some(stripped) = lower.strip_prefix("https://") else {
        return false;
    };
    // 拒绝 userinfo、query、fragment
    if stripped.contains('@') || stripped.contains('?') || stripped.contains('#') {
        return false;
    }
    // 只允许 api.cheng.pink 域名
    let host_and_path: Vec<&str> = stripped.splitn(2, '/').collect();
    if host_and_path[0] != "api.cheng.pink" {
        return false;
    }
    // path 只允许空、/、/v1、/v1/
    let path = host_and_path
        .get(1)
        .map(|p| p.trim_end_matches('/'))
        .unwrap_or("");
    path.is_empty() || path == "v1"
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
    fn accepts_whitelisted_no_path() {
        let config = resolve(Some("sk-test"), Some("https://api.cheng.pink"));
        assert_eq!(config.status, ProviderConfigStatus::Configured);
        assert_eq!(config.default_model, "gpt-5.4-mini");
        assert!(config.allowed_models.contains(&"gpt-5.4-mini".into()));
        assert!(config
            .allowed_purposes
            .contains(&"project_plan_generation".into()));
    }

    #[test]
    fn accepts_whitelisted_trailing_slash() {
        let config = resolve(Some("sk-test"), Some("https://api.cheng.pink/"));
        assert_eq!(config.status, ProviderConfigStatus::Configured);
    }

    #[test]
    fn accepts_whitelisted_v1() {
        let config = resolve(Some("sk-test"), Some("https://api.cheng.pink/v1"));
        assert_eq!(config.status, ProviderConfigStatus::Configured);
    }

    #[test]
    fn accepts_whitelisted_v1_trailing_slash() {
        let config = resolve(Some("sk-test"), Some("https://api.cheng.pink/v1/"));
        assert_eq!(config.status, ProviderConfigStatus::Configured);
    }

    #[test]
    fn rejects_missing_https_scheme() {
        let config = resolve(Some("sk-test"), Some("api.cheng.pink"));
        assert_eq!(config.status, ProviderConfigStatus::InvalidBaseUrl);
    }

    #[test]
    fn rejects_non_whitelisted_domain() {
        let config = resolve(Some("sk-test"), Some("https://api.openai.com"));
        assert_eq!(config.status, ProviderConfigStatus::InvalidBaseUrl);
    }

    #[test]
    fn rejects_non_v1_path() {
        let config = resolve(Some("sk-test"), Some("https://api.cheng.pink/anything"));
        assert_eq!(config.status, ProviderConfigStatus::InvalidBaseUrl);
    }

    #[test]
    fn does_not_expose_key_value() {
        let config = resolve(Some("sk-secret-abc123"), Some("https://api.cheng.pink/v1"));
        assert_eq!(config.status, ProviderConfigStatus::Configured);
        // ProviderConfig 不包含 key 字段，调用方拿不到 raw key
    }

    #[test]
    fn real_env_resolve_does_not_panic() {
        // 只验证不 panic，不依赖具体 env 值
        let _ = resolve_provider_config();
    }
}
