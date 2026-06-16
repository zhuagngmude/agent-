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
/// 顺序处理每种模式，每步操作在累积结果上做匹配和替换，
/// 避免多种模式交织时 reset 原文或跨模式索引错位。
pub fn redact_secrets(text: &str) -> String {
    let mut result = text.to_string();

    // 1. sk- 密钥脱敏（在累积结果上操作）
    result = redact_sk_keys(&result);

    // 2. Authorization: Bearer 脱敏
    result = redact_auth_bearer(&result);

    // 3. api_key= / token= / password= 脱敏
    result = redact_key_value_patterns(&result);

    result
}

fn redact_sk_keys(text: &str) -> String {
    let lower = text.to_lowercase();
    let mut result = text.to_string();
    let mut offset: isize = 0;

    let mut search_start = 0usize;
    while let Some(pos) = lower[search_start..].find("sk-") {
        let abs_pos = search_start + pos;
        let after = &text[abs_pos + 3..];
        let alnum_count = after
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric())
            .count();
        if alnum_count >= 20 {
            let start = (abs_pos as isize + offset) as usize;
            let end = (abs_pos as isize + offset + 3 + alnum_count as isize) as usize;
            let replacement = "[REDACTED_SECRET]";
            let old_len = end - start;
            result.replace_range(start..end, replacement);
            offset += replacement.len() as isize - old_len as isize;
        }
        search_start = abs_pos + 3;
    }
    result
}

fn redact_auth_bearer(text: &str) -> String {
    let mut result = String::with_capacity(text.len());

    for segment in text.split_inclusive('\n') {
        let (line, newline) = segment
            .strip_suffix('\n')
            .map_or((segment, ""), |line| (line, "\n"));
        let lower = line.to_lowercase();
        if lower.contains("authorization:") && lower.contains("bearer") {
            result.push_str("Authorization: Bearer [REDACTED_SECRET]");
            result.push_str(newline);
        } else {
            result.push_str(segment);
        }
    }

    result
}

fn redact_key_value_patterns(text: &str) -> String {
    let mut result = text.to_string();

    for pattern in &["api_key=", "token=", "password="] {
        // 每处理一种 pattern，基于当前 result 重新计算 lower 和偏移。
        // 避免跨 pattern 的 offset 污染后续 pattern 的索引。
        let lower = result.to_lowercase();
        let mut offset: isize = 0;
        let mut search_start = 0usize;
        while let Some(pos) = lower[search_start..].find(pattern) {
            let abs_pos = search_start + pos;
            let value_start = abs_pos + pattern.len();
            let value_end = lower[value_start..]
                .find(|c: char| c.is_whitespace())
                .map(|n| value_start + n)
                .unwrap_or(lower.len());
            if value_end > value_start {
                let start = (abs_pos as isize + offset) as usize;
                let end = (value_end as isize + offset) as usize;
                let replacement = format!("{pattern}[REDACTED_SECRET]");
                let old_len = end - start;
                result.replace_range(start..end, &replacement);
                offset += replacement.len() as isize - old_len as isize;
            }
            search_start = abs_pos + pattern.len();
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

    #[test]
    fn redaction_handles_multiple_patterns_without_leaking() {
        // 模型输出可能同时包含多种敏感模式
        let input = concat!(
            "key: sk-abcdefghijklmnopqrstuvwxyz123456\n",
            "Authorization: Bearer eyJhbGciOiJIUzI1NiJ9.xxx\n",
            "api_key=mysecret\n"
        );
        let output = redact_secrets(input);
        // 三种敏感内容都不应出现在输出中
        assert!(!output.contains("sk-abcdefghijklmnopqrstuvwxyz123456"));
        assert!(!output.contains("Bearer eyJhbGciOiJIUzI1NiJ9.xxx"));
        assert!(!output.contains("mysecret"));
        // 但 REDACTED_SECRET 应该出现（至少三次脱敏）
        let redacted_count = output.matches("[REDACTED_SECRET]").count();
        assert!(redacted_count >= 2, "应至少脱敏 sk 和 auth bearer 两种");
    }

    #[test]
    fn redaction_password_before_api_key() {
        let input = "password=hunter2 api_key=mysecret";
        let output = redact_secrets(input);
        assert!(!output.contains("hunter2"));
        assert!(!output.contains("mysecret"));
    }

    #[test]
    fn redaction_api_key_before_password() {
        let input = "api_key=mysecret password=hunter2";
        let output = redact_secrets(input);
        assert!(!output.contains("mysecret"));
        assert!(!output.contains("hunter2"));
    }

    #[test]
    fn redaction_multiple_tokens_interleaved() {
        let input = "token=abc token=def token=ghi";
        let output = redact_secrets(input);
        assert!(!output.contains("abc"));
        assert!(!output.contains("def"));
        assert!(!output.contains("ghi"));
    }

    #[test]
    fn redaction_handles_multiple_authorization_lines() {
        let input = concat!(
            "Authorization: Bearer first-token-12345\n",
            "Authorization: Bearer second-token-67890\n"
        );
        let output = redact_secrets(input);
        assert!(!output.contains("first-token-12345"));
        assert!(!output.contains("second-token-67890"));
        assert_eq!(output.matches("[REDACTED_SECRET]").count(), 2);
    }

    #[test]
    fn redaction_handles_token_and_password_together() {
        let input = "token=abc123.def456\npassword=hunter2";
        let output = redact_secrets(input);
        assert!(!output.contains("abc123.def456"));
        assert!(!output.contains("hunter2"));
    }
}
