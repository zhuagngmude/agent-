import type { ReactNode } from "react";
import { Button, Layout, Space, Tag, Typography } from "antd";
import { Bell, GitBranch, Settings } from "lucide-react";

import type { PageKey } from "../routes/mainNavItems";
import { mainNavItems } from "../routes/mainNavItems";

const { Header, Sider, Content } = Layout;
const { Text, Title } = Typography;

type AppShellProps = {
  activePage: PageKey;
  onNavigate: (page: PageKey) => void;
  children: ReactNode;
};

export function AppShell({ activePage, onNavigate, children }: AppShellProps) {
  return (
    <Layout className="app-shell">
      <Sider width={248} className="app-shell__sider">
        <div className="app-shell__brand">
          <span className="app-shell__brand-mark">蜂</span>
          <div>
            <Title level={4}>agent蜂群</Title>
            <Text type="secondary">本地 AI 协作控制台</Text>
          </div>
        </div>

        <nav className="app-shell__nav" aria-label="主导航">
          {mainNavItems.map((item) => (
            <button
              key={item.key}
              type="button"
              className={
                activePage === item.key
                  ? "app-shell__nav-item is-active"
                  : "app-shell__nav-item"
              }
              onClick={() => onNavigate(item.key)}
            >
              <item.icon size={17} aria-hidden="true" />
              <span>{item.label}</span>
            </button>
          ))}
        </nav>
      </Sider>

      <Layout>
        <Header className="app-shell__header">
          <div>
            <Text type="secondary">桌面主入口</Text>
            <Title level={3}>共享 UI 工程骨架</Title>
          </div>
          <Space size={12}>
            <Tag color="blue">Tauri 预留</Tag>
            <Tag color="default">Runner 关闭</Tag>
            <Button icon={<GitBranch size={16} />} aria-label="Git 保存点" disabled />
            <Button icon={<Bell size={16} />} aria-label="通知" disabled />
            <Button icon={<Settings size={16} />} aria-label="设置" disabled />
          </Space>
        </Header>

        <Content className="app-shell__content">{children}</Content>
      </Layout>
    </Layout>
  );
}
