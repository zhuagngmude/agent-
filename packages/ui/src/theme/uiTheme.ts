import type { ThemeConfig } from "antd";

export const uiTheme: ThemeConfig = {
  token: {
    colorPrimary: "#7367f0",
    colorInfo: "#7367f0",
    borderRadius: 6,
    fontFamily:
      "\"Microsoft YaHei UI\", \"Microsoft YaHei\", \"Segoe UI\", ui-sans-serif, system-ui, sans-serif",
  },
  components: {
    Layout: {
      bodyBg: "#f7f7fb",
      headerBg: "#ffffff",
      siderBg: "#071a34",
    },
    Card: {
      borderRadiusLG: 8,
    },
    Button: {
      borderRadius: 6,
    },
  },
};
