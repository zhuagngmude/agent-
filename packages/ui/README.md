# packages/ui

唯一 UI 源码包，桌面端和浏览器预览入口都应消费这里的页面、组件、布局和主题。

## 当前定位

- 技术栈：React + TypeScript + Vite + Ant Design。
- 职责范围：页面、布局、通用组件、主题、国际化和 UI 运行壳。
- 禁止范围：文件系统读写、SQLite 访问、Git 操作、Runner 调度、真实模型调用。

## 目录说明

```text
packages/ui/
  src/
    app/         # UI 应用根组件和运行壳
    components/  # 可复用通用组件
    pages/       # 页面级组件
    layouts/     # 全局布局
    theme/       # Ant Design 主题和设计 token
    i18n/        # 中文文案和后续国际化入口
    routes/      # 路由定义预留
    utils/       # 纯前端工具函数预留
```

## 本地预览

后续安装依赖后，可在本目录运行：

```powershell
npm install
npm run dev
```

当前骨架只提供可启动的 UI 壳，不接真实后端、不调用真实模型、不启动 Runner。
