import type { ReactNode } from "react";
import { Bell, Database, GitBranch, LockKeyhole, RefreshCw, Settings, ShieldCheck } from "lucide-react";

import type { PageKey } from "../routes/mainNavItems";
import { mainNavItems } from "../routes/mainNavItems";
import { isTauriHost } from "../utils/desktopHost";

type AppShellProps = {
  activePage: PageKey;
  onNavigate: (page: PageKey) => void;
  children: ReactNode;
};

const commandPages: PageKey[] = ["overview", "workflow", "tasks"];
const swarmPages: PageKey[] = ["agents", "approvals", "agentRuns", "settings"];

export function AppShell({ activePage, onNavigate, children }: AppShellProps) {
  const isDesktop = isTauriHost();
  const activeNav = mainNavItems.find((item) => item.key === activePage);
  const commandItems = mainNavItems.filter((item) => commandPages.includes(item.key));
  const swarmItems = mainNavItems.filter((item) => swarmPages.includes(item.key));

  return (
    <div className="app-shell">
      <aside className="app-shell__sider">
        <div className="app-shell__brand">
          <span className="app-shell__brand-mark">蜂</span>
          <div>
            <strong>agent蜂群</strong>
            <span>本地 AI 协作指挥室</span>
          </div>
        </div>

        <section className="app-shell__project-card" aria-label="当前项目">
          <span>当前项目</span>
          <strong>agent蜂群桌面端</strong>
          <div className="app-shell__mini-progress" aria-hidden="true">
            <i />
          </div>
        </section>

        <nav className="app-shell__nav" aria-label="主导航">
          <div className="app-shell__nav-group">Command</div>
          {commandItems.map((item) => (
            <button
              key={item.key}
              type="button"
              className={activePage === item.key ? "app-shell__nav-item is-active" : "app-shell__nav-item"}
              onClick={() => onNavigate(item.key)}
            >
              <span className="app-shell__nav-icon">
                <item.icon size={15} aria-hidden="true" />
              </span>
              <span>{item.label}</span>
            </button>
          ))}

          <div className="app-shell__nav-group">Swarm</div>
          {swarmItems.map((item) => (
            <button
              key={item.key}
              type="button"
              className={activePage === item.key ? "app-shell__nav-item is-active" : "app-shell__nav-item"}
              onClick={() => onNavigate(item.key)}
            >
              <span className="app-shell__nav-icon">
                <item.icon size={15} aria-hidden="true" />
              </span>
              <span>{item.label}</span>
            </button>
          ))}
        </nav>

        <section className="app-shell__status-dock" aria-label="本地状态">
          <span>本地状态</span>
          <div>
            <Database size={14} aria-hidden="true" />
            <strong>{isDesktop ? "SQLite 已连接" : "只读预览数据"}</strong>
            <i />
          </div>
          <div>
            <ShieldCheck size={14} aria-hidden="true" />
            <strong>Runner 只读边界</strong>
            <i />
          </div>
          <div>
            <LockKeyhole size={14} aria-hidden="true" />
            <strong>模型目录受控</strong>
            <i />
          </div>
        </section>
      </aside>

      <main className="app-shell__workspace">
        <header className="app-shell__topbar">
          <div className="app-shell__crumb">
            <strong>{activeNav?.label ?? "主控台"}</strong>
            <span>/ 电脑端工作区 / {isDesktop ? "本地桌面模式" : "只读安全预览"}</span>
          </div>

          <div className="app-shell__actions">
            <span className="app-shell__pill">Runner 阶段34 · 二次确认</span>
            <span className="app-shell__pill">0 个高风险动作</span>
            <button
              type="button"
              className="app-shell__icon-button"
              aria-label="刷新界面"
              title="刷新界面"
              onClick={() => window.location.reload()}
            >
              <RefreshCw size={15} aria-hidden="true" />
            </button>
            <button type="button" className="app-shell__icon-button" disabled aria-label="Git 保存点">
              <GitBranch size={15} aria-hidden="true" />
            </button>
            <button type="button" className="app-shell__icon-button" disabled aria-label="通知">
              <Bell size={15} aria-hidden="true" />
            </button>
            <button type="button" className="app-shell__icon-button" aria-label="设置" onClick={() => onNavigate("settings")}>
              <Settings size={15} aria-hidden="true" />
            </button>
          </div>
        </header>

        <section className="app-shell__content">{children}</section>
      </main>
    </div>
  );
}
