import { Form, Input, Modal, Radio, Select, message } from "antd";
import type { AgentSummary, CreateTaskInput } from "../utils/desktopHost";
import { createTask, isTauriHost } from "../utils/desktopHost";

type CreateTaskModalProps = {
  open: boolean;
  agents: AgentSummary[];
  onClose: () => void;
  onCreated: () => void;
};

export function CreateTaskModal({ open, agents, onClose, onCreated }: CreateTaskModalProps) {
  const [form] = Form.useForm<CreateTaskInput>();

  const handleSubmit = async () => {
    try {
      const values = await form.validateFields();
      await createTask({
        title: values.title,
        description: values.description ?? null,
        priority: values.priority,
        assigned_agent_id: values.assigned_agent_id ?? null,
        risk_level: values.risk_level ?? null,
      });
      form.resetFields();
      message.success("任务已创建");
      onCreated();
    } catch (error) {
      showSubmitError(error);
    }
  };

  const handleOpen = () => {
    if (!isTauriHost()) {
      message.warning("浏览器预览模式不支持写入操作");
      return;
    }
  };

  return (
    <Modal
      title="新建任务"
      open={open}
      onOk={handleSubmit}
      onCancel={onClose}
      afterOpenChange={(visible) => {
        if (visible) handleOpen();
      }}
      okText="创建"
      cancelText="取消"
      destroyOnHidden
    >
      <Form form={form} layout="vertical" initialValues={{ priority: "medium" }}>
        <Form.Item
          name="title"
          label="任务标题"
          rules={[
            { required: true, message: "请输入任务标题" },
            { max: 120, message: "标题不超过 120 字符" },
          ]}
        >
          <Input placeholder="输入任务标题" maxLength={120} />
        </Form.Item>

        <Form.Item
          name="description"
          label="描述"
          rules={[{ max: 2000, message: "描述不超过 2000 字符" }]}
        >
          <Input.TextArea rows={3} placeholder="输入任务描述（选填）" maxLength={2000} />
        </Form.Item>

        <Form.Item
          name="priority"
          label="优先级"
          rules={[{ required: true, message: "请选择优先级" }]}
        >
          <Radio.Group>
            <Radio.Button value="low">低</Radio.Button>
            <Radio.Button value="medium">中</Radio.Button>
            <Radio.Button value="high">高</Radio.Button>
          </Radio.Group>
        </Form.Item>

        <Form.Item name="assigned_agent_id" label="指派 Agent">
          <Select
            allowClear
            placeholder="选择 Agent（选填）"
            options={agents.map((agent) => ({
              value: agent.id,
              label: agent.name,
            }))}
          />
        </Form.Item>

        <Form.Item name="risk_level" label="风险等级">
          <Radio.Group>
            <Radio.Button value="low">低</Radio.Button>
            <Radio.Button value="medium">中</Radio.Button>
            <Radio.Button value="high">高</Radio.Button>
          </Radio.Group>
        </Form.Item>
      </Form>
    </Modal>
  );
}

function showSubmitError(error: unknown): void {
  if (isFormValidationError(error)) {
    return;
  }

  const text = error instanceof Error ? error.message : String(error);
  if (text) {
    message.error(text);
  }
}

function isFormValidationError(error: unknown): boolean {
  return typeof error === "object" && error !== null && "errorFields" in error;
}
