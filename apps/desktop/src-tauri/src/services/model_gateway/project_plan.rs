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

/// 验证二次确认（阶段 25.1：feature flag 开启后强制要求）
/// second_confirm 必须为 true，confirm_text 必须等于 "我确认发起真实模型调用"
pub fn validate_second_confirm(
    second_confirm: bool,
    confirm_text: &Option<String>,
) -> Result<(), String> {
    if !second_confirm {
        return Err("必须勾选二次确认".into());
    }
    let text = confirm_text.as_deref().unwrap_or("");
    if text != "我确认发起真实模型调用" {
        return Err("确认文本不正确".into());
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

    #[test]
    fn second_confirm_rejects_false() {
        let result = validate_second_confirm(false, &None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("二次确认"));
    }

    #[test]
    fn second_confirm_rejects_wrong_text() {
        let result = validate_second_confirm(true, &Some("随便写的".into()));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("确认文本不正确"));
    }

    #[test]
    fn second_confirm_rejects_empty_text() {
        let result = validate_second_confirm(true, &None);
        assert!(result.is_err());
    }

    #[test]
    fn second_confirm_accepts_correct_input() {
        let result = validate_second_confirm(true, &Some("我确认发起真实模型调用".into()));
        assert!(result.is_ok());
    }
}
