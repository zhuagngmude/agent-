// ---------------------------------------------------------------------------
// project_plan 请求验证（阶段 22 helper-only，只验证输入，不发请求）
// ---------------------------------------------------------------------------

/// 验证输入的 idea 和 constraints 字段
pub fn validate_input(idea: &str, constraints: &Option<String>) -> Result<(), String> {
    if idea.trim().is_empty() {
        return Err("项目想法不能为空".into());
    }

    if idea.len() > 5000 {
        return Err("项目想法不能超过 5000 个字符".into());
    }

    if let Some(c) = constraints {
        if c.len() > 2000 {
            return Err("约束条件不能超过 2000 个字符".into());
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// 测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_rejects_empty_idea() {
        let result = validate_input("   ", &None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("不能为空"));
    }

    #[test]
    fn validate_rejects_too_long_idea() {
        let idea = "a".repeat(5001);
        let result = validate_input(&idea, &None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("5000"));
    }

    #[test]
    fn validate_rejects_too_long_constraints() {
        let constraints = "a".repeat(2001);
        let result = validate_input("valid idea", &Some(constraints));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("2000"));
    }

    #[test]
    fn validate_accepts_valid_input() {
        let result = validate_input("valid idea", &Some("some constraints".into()));
        assert!(result.is_ok());
    }
}
