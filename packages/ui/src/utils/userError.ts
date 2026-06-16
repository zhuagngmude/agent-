export function userErrorLabel(error: unknown, fallback = "操作失败，请稍后重试"): string {
  const text = error instanceof Error ? error.message : String(error ?? "");
  const lower = text.toLowerCase();

  if (!text) return fallback;
  if (lower.includes("connection") || lower.includes("refused") || lower.includes("failed to fetch")) {
    return "连接失败，请检查桌面宿主是否已启动";
  }
  if (lower.includes("not found") || lower.includes("no such")) {
    return "操作对象不存在，请刷新后重试";
  }
  if (lower.includes("permission") || lower.includes("denied") || lower.includes("forbidden")) {
    return "当前权限不足，无法完成操作";
  }
  if (lower.includes("feature_disabled")) {
    return "功能开关未开启";
  }
  if (lower.includes("provider_config") || lower.includes("api key") || lower.includes("base url")) {
    return "模型服务配置异常";
  }
  if (lower.includes("provider") || lower.includes("network") || lower.includes("timeout")) {
    return "模型服务调用失败，请稍后重试";
  }
  if (lower.includes("audit_write_failed")) {
    return "审计写入失败，操作未完成";
  }
  if (lower.includes("invalid_request") || lower.includes("invalid input")) {
    return "请求内容无效，请检查后重试";
  }
  if (lower.includes("invalid_state")) {
    return "当前状态不允许此操作，请刷新后重试";
  }

  return fallback;
}
