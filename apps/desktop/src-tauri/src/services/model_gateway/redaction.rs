// ---------------------------------------------------------------------------
// 输入值安全检查（禁止在 idea/constraints 中夹带密钥格式字符串）
// 纯字符串匹配，不依赖 regex crate
// ---------------------------------------------------------------------------

/// 检查文本中是否包含禁止的值模式（密钥、token 等）
pub fn check_forbidden_value_patterns(text: &str) -> Result<(), String> {
    let lower = text.to_lowercase();

    // "sk-" 开头 + 20 位以上字母数字
    if let Some(pos) = lower.find("sk-") {
        let after = &text[pos + 3..];
        let alnum_count = after
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric())
            .count();
        if alnum_count >= 20 {
            return Err("输入包含疑似 API key 的值，已被拒绝".into());
        }
    }

    // api_key= 格式
    if lower.contains("api_key=") {
        return Err("输入包含疑似 API key 的值，已被拒绝".into());
    }

    // Authorization: Bearer 格式
    if lower.contains("authorization:") && lower.contains("bearer") {
        let after_bearer = lower.split("bearer").nth(1).unwrap_or("");
        if after_bearer.trim().len() > 0 {
            return Err("输入包含疑似密钥的值，已被拒绝".into());
        }
    }

    // token= 格式
    if lower.contains("token=") {
        return Err("输入包含疑似 token 的值，已被拒绝".into());
    }

    // password= 格式
    if lower.contains("password=") {
        return Err("输入包含疑似密码的值，已被拒绝".into());
    }

    Ok(())
}

/// 截断摘要到指定最大长度（字节边界对齐）
pub fn truncate_summary(text: &str, max_len: usize) -> String {
    if text.len() <= max_len {
        text.to_string()
    } else {
        // 在 max_len 处截断，向后找到合法的 char 边界
        let mut end = max_len;
        while end > 0 && !text.is_char_boundary(end) {
            end -= 1;
        }
        let mut result = text[..end].to_string();
        result.push('…');
        result
    }
}

/// 脱敏函数（阶段 22 helper-only，阶段 25.1 用于返回前脱敏）
pub fn redact_secrets(text: &str) -> String {
    let mut result = text.to_string();
    let lower = text.to_lowercase();

    // sk- 密钥脱敏
    if let Some(pos) = lower.find("sk-") {
        let after = &text[pos + 3..];
        let alnum_count = after
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric())
            .count();
        if alnum_count >= 20 {
            let start = pos;
            let end = pos + 3 + alnum_count;
            result.replace_range(start..end, "[REDACTED_SECRET]");
        }
    }

    // Authorization: Bearer 脱敏
    if lower.contains("authorization:") && lower.contains("bearer") {
        result = text.to_string(); // 简化：完整替换行
        let start = lower.find("authorization:").unwrap_or(0);
        result.replace_range(start..text.len(), "Authorization: Bearer [REDACTED_SECRET]");
    }

    // api_key= / token= / password= 脱敏
    for pattern in &["api_key=", "token=", "password="] {
        if lower.contains(pattern) {
            result = result.replace(
                &result[lower.find(pattern).unwrap_or(0)..],
                &format!("{pattern}[REDACTED_SECRET]"),
            );
        }
    }

    result
}

// ---------------------------------------------------------------------------
// 测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forbids_openai_style_key() {
        let result = check_forbidden_value_patterns("sk-abcdefghijklmnopqrstuvwxyz123456");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("API key"));
    }

    #[test]
    fn forbids_api_key_eq_format() {
        let result = check_forbidden_value_patterns("api_key=mysecretkey123");
        assert!(result.is_err());
    }

    #[test]
    fn forbids_authorization_bearer_format() {
        let result = check_forbidden_value_patterns("Authorization: Bearer eyJhbGciOiJIUzI1NiJ9");
        assert!(result.is_err());
    }

    #[test]
    fn forbids_token_eq_format() {
        let result = check_forbidden_value_patterns("token=abc123");
        assert!(result.is_err());
    }

    #[test]
    fn forbids_password_eq_format() {
        let result = check_forbidden_value_patterns("password=hunter2");
        assert!(result.is_err());
    }

    #[test]
    fn allows_normal_text() {
        let result = check_forbidden_value_patterns("我想做一个本地客户线索管理工具");
        assert!(result.is_ok());
    }

    #[test]
    fn allows_short_sk_prefix_without_enough_chars() {
        let result = check_forbidden_value_patterns("sk-short");
        assert!(result.is_ok());
    }

    #[test]
    fn truncate_within_limit_returns_unchanged() {
        let text = "短文本";
        let result = truncate_summary(text, 200);
        assert_eq!(result, "短文本");
    }

    #[test]
    fn truncate_exceeds_limit_adds_ellipsis() {
        let text = "a".repeat(100);
        let result = truncate_summary(&text, 50);
        // "…" (U+2026) 为 3 字节 UTF-8 字符，50 个 "a" + "…" = 53 字节
        assert!(result.len() <= 53);
        assert!(result.ends_with('…'));
        // 不应包含完整原文
        assert!(!result.contains(&"a".repeat(100)));
    }

    #[test]
    fn truncate_respects_char_boundary() {
        let text = "你好世界".repeat(50); // 200 字节
        let result = truncate_summary(&text, 15);
        // 不应 panic，即使截断点落在多字节字符中间
        assert!(result.ends_with('…'));
    }

    #[test]
    fn redaction_replaces_secrets() {
        let input = "我用的是 sk-abcdefghijklmnopqrstuvwxyz123456 这个 key";
        let output = redact_secrets(input);
        assert!(!output.contains("sk-abcdefghijklmnopqrstuvwxyz123456"));
        assert!(output.contains("[REDACTED_SECRET]"));
    }
}
