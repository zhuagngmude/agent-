import React from "react";
import { createRoot } from "react-dom/client";
import { App as AntdApp, ConfigProvider } from "antd";
import zhCN from "antd/locale/zh_CN";

import { App } from "./app/App";
import { uiTheme } from "./theme/uiTheme";
import "./theme/global.css";

const rootElement = document.getElementById("root");

if (!rootElement) {
  throw new Error("缺少 #root 挂载节点");
}

createRoot(rootElement).render(
  <React.StrictMode>
    <ConfigProvider locale={zhCN} theme={uiTheme}>
      <AntdApp>
        <App />
      </AntdApp>
    </ConfigProvider>
  </React.StrictMode>,
);
