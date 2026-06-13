# 共享 UI 方案确认

日期：2026-06-13

## 一、确认结论

这个项目只保留一套 UI 源码：

- `packages/ui` 是唯一 UI 源码
- `apps/desktop` 只负责桌面主入口和本地能力接入
- `apps/web` 只负责浏览器预览入口

桌面端和 Web 端共用同一套页面、组件、布局和主题，不再各写一套 UI。

## 二、UI 职责边界

`packages/ui` 负责：

- 页面
- 布局
- 通用组件
- 主题和设计 Token
- 路由和导航壳

`apps/desktop` 和 `apps/web` 负责：

- 启动入口
- 运行时配置
- 宿主能力接入
- 加载同一套 UI 源码

## 三、建议目录

```text
packages/ui/
  index.html
  package.json
  tsconfig.json
  vite.config.ts
  src/
    app/
    pages/
    layouts/
    components/
    theme/
    i18n/
    routes/
    utils/
```

当前 `packages/ui` 已按上述结构建立最小可运行 Vite 工程骨架。该骨架只验证 UI 包能启动、类型检查和构建，不接真实模型、不接 Runner、不接本地文件或数据库能力。

## 四、组件边界

### 适合放进 `packages/ui`

- App Shell
- Sidebar
- PageHeader
- SectionHeader
- DataTable
- FilterBar
- StatusBadge
- ConfirmDialog
- Timeline
- EmptyState

### 不适合放进 `packages/ui`

- 文件系统读写
- SQLite 访问
- Git 操作
- Runner 调度
- 本地权限判断

这些能力应该由桌面宿主或共享业务层提供，UI 只负责展示和触发。

## 五、共享规则

1. 同一种页面结构只保留一份实现。
2. 重复出现的 UI 片段优先抽成通用组件。
3. 所有样式统一走主题和 Token。
4. 页面逻辑和领域规则尽量放到 `packages/agent-core` 和 `packages/shared`。
5. 入口层只做桥接，不再复制 UI。

## 六、下一步

1. 确认桌面宿主的本地能力边界。
2. 确认 `packages/agent-core` 的共享规则范围。
3. 确认旧原型归档清单。
4. 继续确认桌面宿主和数据库工程初始化边界。
