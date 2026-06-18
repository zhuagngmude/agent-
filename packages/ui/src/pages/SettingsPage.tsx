import { useEffect, useState } from "react";
import { Alert, Button, Card, Descriptions, Form, Input, Space, Typography, message } from "antd";

import type { ProjectSummary, RuntimeModelProviderStatus } from "@agent-swarm/shared";
import {
  getRuntimeModelProviderStatus,
  testRuntimeModelProvider,
  updateRuntimeModelProvider,
} from "../utils/desktopHost";
import { statusLabel } from "../utils/labels";

type SettingsPageProps = {
  project: ProjectSummary;
};

type ModelProviderForm = {
  apiKey: string;
  baseUrl: string;
  modelId: string;
};

export function SettingsPage({ project }: SettingsPageProps) {
  const [form] = Form.useForm<ModelProviderForm>();
  const [providerStatus, setProviderStatus] = useState<RuntimeModelProviderStatus | null>(null);
  const [loadingStatus, setLoadingStatus] = useState(false);
  const [savingProvider, setSavingProvider] = useState(false);
  const [testingProvider, setTestingProvider] = useState(false);
  const [testResult, setTestResult] = useState<{ ok: boolean; message: string } | null>(null);

  const refreshProviderStatus = async () => {
    setLoadingStatus(true);
    try {
      const status = await getRuntimeModelProviderStatus();
      setProviderStatus(status);
      form.setFieldsValue({
        apiKey: "",
        baseUrl: status.base_url ?? "https://api.deepseek.com",
        modelId: status.model_id || "deepseek-chat",
      });
    } catch (error) {
      message.error(error instanceof Error ? error.message : "读取模型服务状态失败");
    } finally {
      setLoadingStatus(false);
    }
  };

  useEffect(() => {
    void refreshProviderStatus();
  }, []);

  const handleSaveProvider = async () => {
    const values = await form.validateFields();
    setSavingProvider(true);
    setTestResult(null);
    try {
      const status = await updateRuntimeModelProvider({
        api_key: values.apiKey,
        base_url: values.baseUrl,
        model_id: values.modelId,
      });
      setProviderStatus(status);
      form.setFieldValue("apiKey", "");
      message.success("模型服务已切换，当前桌面进程立即生效");
    } catch (error) {
      message.error(error instanceof Error ? error.message : "保存模型服务失败");
    } finally {
      setSavingProvider(false);
    }
  };

  const handleTestProvider = async () => {
    setTestingProvider(true);
    try {
      const result = await testRuntimeModelProvider();
      setTestResult({ ok: result.ok, message: result.message });
      if (result.ok) {
        message.success(result.message);
      } else {
        message.warning(result.message);
      }
    } catch (error) {
      const text = error instanceof Error ? error.message : "测试模型服务失败";
      setTestResult({ ok: false, message: text });
      message.error(text);
    } finally {
      setTestingProvider(false);
    }
  };

  return (
    <Space orientation="vertical" size={16} className="page-stack">
      <div className="page-heading">
        <Typography.Title level={2}>系统设置</Typography.Title>
        <Typography.Text type="secondary">项目状态、模型服务和 Runner 执行能力</Typography.Text>
      </div>

      <Card title="项目信息">
        <Descriptions column={1} bordered>
          <Descriptions.Item label="项目名称">{project.name}</Descriptions.Item>
          <Descriptions.Item label="项目状态">{statusLabel(project.status)}</Descriptions.Item>
          <Descriptions.Item label="当前阶段">{project.phase}</Descriptions.Item>
        </Descriptions>
      </Card>

      <Card title="模型服务 / 智能体大脑">
        <Space orientation="vertical" size={12} style={{ width: "100%" }}>
          <Alert
            type={providerStatus?.has_api_key ? "success" : "warning"}
            showIcon
            message={
              providerStatus?.has_api_key
                ? `当前已配置 Key：${providerStatus.api_key_hint ?? "****"}`
                : "当前没有可用 API Key"
            }
            description={`Base URL：${providerStatus?.base_url ?? "未配置"}；模型：${providerStatus?.model_id ?? "deepseek-chat"}`}
          />

          <Form form={form} layout="vertical" disabled={loadingStatus}>
            <Form.Item
              label="API Key"
              name="apiKey"
              extra="只写入当前桌面进程环境变量，不保存到数据库；保存后输入框会清空。"
              rules={[{ required: true, message: "请输入新的 API Key" }]}
            >
              <Input.Password placeholder="sk-..." autoComplete="off" />
            </Form.Item>

            <Form.Item
              label="Base URL"
              name="baseUrl"
              rules={[{ required: true, message: "请输入 Base URL" }]}
            >
              <Input placeholder="https://api.deepseek.com" />
            </Form.Item>

            <Form.Item
              label="模型 ID"
              name="modelId"
              rules={[{ required: true, message: "请输入模型 ID" }]}
            >
              <Input placeholder="deepseek-chat" />
            </Form.Item>

            <Space>
              <Button type="primary" loading={savingProvider} onClick={handleSaveProvider}>
                保存并切换
              </Button>
              <Button loading={testingProvider} onClick={handleTestProvider}>
                测试连接
              </Button>
              <Button onClick={refreshProviderStatus}>刷新状态</Button>
            </Space>
          </Form>

          {testResult && (
            <Alert
              type={testResult.ok ? "success" : "error"}
              showIcon
              message={testResult.message}
            />
          )}
        </Space>
      </Card>

      <Card title="Runner 执行引擎">
        <Alert
          type="info"
          showIcon
          message="Runner 已按全自动模式开放"
          description="全自动流程会自动完成预检、放行、试跑、范围锁定和最小执行；输出写入 workspace/generated 下的任务文件夹。"
        />
      </Card>
    </Space>
  );
}
