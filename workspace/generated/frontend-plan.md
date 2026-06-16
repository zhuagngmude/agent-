```html
<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>项目计划 · 登录页面 · 第一版交互切片</title>
    <!-- 使用系统字体，干净现代 -->
    <style>
        * {
            margin: 0;
            padding: 0;
            box-sizing: border-box;
            font-family: system-ui, -apple-system, 'Segoe UI', Roboto, 'Helvetica Neue', sans-serif;
        }

        body {
            background: #f5f7fb;
            min-height: 100vh;
            display: flex;
            flex-direction: column;
            align-items: center;
            justify-content: center;
            padding: 2rem 1rem;
        }

        /* 主卡片 – 模拟审批后的UI流程容器 */
        .project-card {
            max-width: 1100px;
            width: 100%;
            background: white;
            border-radius: 32px;
            box-shadow: 0 20px 40px -12px rgba(0, 20, 30, 0.15);
            padding: 2.5rem 2rem;
            transition: all 0.2s ease;
        }

        /* 头部：项目计划状态 */
        .plan-header {
            display: flex;
            flex-wrap: wrap;
            align-items: center;
            justify-content: space-between;
            margin-bottom: 2.5rem;
            padding-bottom: 1.5rem;
            border-bottom: 1px solid #eef2f6;
        }

        .plan-header h1 {
            font-size: 1.8rem;
            font-weight: 600;
            letter-spacing: -0.02em;
            color: #0b1c2e;
            display: flex;
            align-items: center;
            gap: 0.75rem;
        }

        .plan-header h1 small {
            font-size: 0.9rem;
            font-weight: 400;
            background: #e6edf6;
            color: #1f4a7a;
            padding: 0.2rem 0.9rem;
            border-radius: 40px;
            letter-spacing: 0.3px;
        }

        .status-badge {
            background: #d1e7ff;
            color: #004e9e;
            font-weight: 500;
            font-size: 0.85rem;
            padding: 0.4rem 1.2rem;
            border-radius: 40px;
            display: inline-flex;
            align-items: center;
            gap: 6px;
            border: 1px solid #b8d6f5;
        }

        .status-badge::before {
            content: "✓";
            font-weight: 700;
            font-size: 1rem;
        }

        /* 流程步骤指示器 (UI流程) */
        .flow-steps {
            display: flex;
            flex-wrap: wrap;
            gap: 0.5rem 1.5rem;
            background: #f8faff;
            padding: 1rem 1.5rem;
            border-radius: 60px;
            margin-bottom: 2.5rem;
            border: 1px solid #e4ebf5;
            justify-content: center;
        }

        .step-item {
            display: flex;
            align-items: center;
            gap: 8px;
            font-size: 0.9rem;
            color: #2c3e50;
        }

        .step-item .step-num {
            background: white;
            border: 1px solid #cbdae9;
            width: 26px;
            height: 26px;
            border-radius: 30px;
            display: inline-flex;
            align-items: center;
            justify-content: center;
            font-weight: 600;
            font-size: 0.75rem;
            color: #1f4973;
        }

        .step-item.active .step-num {
            background: #1a5a9c;
            border-color: #1a5a9c;
            color: white;
        }

        .step-item.active {
            font-weight: 600;
            color: #0b2b4a;
        }

        .step-item.done .step-num {
            background: #1f8b4c;
            border-color: #1f8b4c;
            color: white;
        }

        .step-arrow {
            color: #b0c4db;
            font-weight: 300;
            font-size: 1.2rem;
            margin: 0 0.2rem;
        }

        /* 双栏布局：登录卡片 + 状态面板 */
        .split-panel {
            display: grid;
            grid-template-columns: 1fr 1fr;
            gap: 2rem;
            margin-top: 0.5rem;
        }

        /* 左侧：登录页面 UI (核心交互切片) */
        .login-ui {
            background: #ffffff;
            border-radius: 28px;
            padding: 2rem 1.8rem 2.2rem;
            box-shadow: 0 8px 24px rgba(0, 20, 40, 0.04);
            border: 1px solid #e9f0f8;
        }

        .login-ui .login-title {
            font-size: 1.6rem;
            font-weight: 600;
            color: #0b1f33;
            margin-bottom: 0.25rem;
        }

        .login-ui .login-sub {
            color: #5b6f86;
            font-size: 0.95rem;
            margin-bottom: 2rem;
            border-left: 3px solid #2b7be4;
            padding-left: 0.75rem;
        }

        .input-group {
            margin-bottom: 1.5rem;
        }

        .input-group label {
            display: block;
            font-size: 0.85rem;
            font-weight: 500;
            color: #1f3a57;
            margin-bottom: 0.4rem;
        }

        .input-group input {
            width: 100%;
            padding: 0.9rem 1rem;
            border: 1px solid #d7e0ea;
            border-radius: 18px;
            font-size: 1rem;
            background: #fafcff;
            transition: 0.15s;
            outline: none;
        }

        .input-group input:focus {
            border-color: #2b7be4;
            box-shadow: 0 0 0 3px rgba(43, 123, 228, 0.15);
            background: white;
        }

        .login-options {
            display: flex;
            justify-content: space-between;
            align-items: center;
            margin: 1.2rem 0 2rem;
            font-size: 0.9rem;
        }

        .login-options label {
            display: flex;
            align-items: center;
            gap: 6px;
            color: #2b4a6a;
        }

        .login-options a {
            color: #1a5a9c;
            text-decoration: none;
            font-weight: 500;
        }

        .login-options a:hover {
            text-decoration: underline;
        }

        .btn-primary {
            width: 100%;
            background: #1a5a9c;
            color: white;
            border: none;
            padding: 1rem;
            border-radius: 40px;
            font-size: 1.1rem;
            font-weight: 600;
            cursor: default;  /* 第一版静态切片，展示交互状态 */
            transition: 0.1s;
            box-shadow: 0 4px 8px rgba(26, 90, 156, 0.15);
            letter-spacing: 0.3px;
        }

        .btn-primary:active {
            transform: scale(0.98);
        }

        .login-footer-text {
            text-align: center;
            margin-top: 1.8rem;
            color: #5f748e;
            font-size: 0.9rem;
        }

        .login-footer-text a {
            color: #1a5a9c;
            font-weight: 500;
            text-decoration: none;
        }

        /* 右侧：状态展示面板 (审批后项目计划状态) */
        .status-panel {
            background: #f9fcff;
            border-radius: 28px;
            padding: 1.8rem 1.8rem 2rem;
            border: 1px solid #e2ecf9;
        }

        .status-panel h3 {
            font-size: 1.2rem;
            font-weight: 600;
            color: #0b1f33;
            display: flex;
            align-items: center;
            gap: 8px;
            margin-bottom: 1.5rem;
        }

        .status-panel h3 span {
            background: #dce8f5;
            font-size: 0.7rem;
            font-weight: 500;
            padding: 0.2rem 0.8rem;
            border-radius: 40px;
            color: #1f4a7a;
        }

        .status-item {
            display: flex;
            align-items: center;
            gap: 12px;
            padding: 0.9rem 0;
            border-bottom: 1px solid #e6eff9;
        }

        .status-item:last-child {
            border-bottom: none;
        }

        .status-icon {
            width: 32px;
            height: 32px;
            border-radius: 40px;
            background: white;
            border: 1px solid #d2e0f0;
            display: flex;
            align-items: center;
            justify-content: center;
            font-size: 1rem;
        }

        .status-icon.approved {
            background: #e1f5e8;
            border-color: #8fc9a8;
            color: #0f6a3a;
        }

        .status-icon.pending {
            background: #fff3d6;
            border-color: #f5d99b;
            color: #9e6f1a;
        }

        .status-content {
            flex: 1;
        }

        .status-content .title {
            font-weight: 600;
            color: #0b1f33;
            font-size: 0.95rem;
        }

        .status-content .desc {
            font-size: 0.8rem;
            color: #4f6a86;
            margin-top: 2px;
        }

        .status-tag {
            font-size: 0.7rem;
            font-weight: 500;
            padding: 0.2rem 0.8rem;
            border-radius: 40px;
            background: #eaf1fa;
            color: #1f4a7a;
        }

        .status-tag.approved {
            background: #d4edda;
            color: #0f5c2e;
        }

        .status-tag.in-progress {
            background: #d1e0f5;
            color: #1a4d7a;
        }

        /* 响应式 */
        @media (max-width: 780px) {
            .split-panel {
                grid-template-columns: 1fr;
                gap: 2rem;
            }
            .project-card {
                padding: 1.8rem 1.2rem;
            }
            .flow-steps {
                border-radius: 30px;
                padding: 0.8rem 1rem;
                gap: 0.3rem 1rem;
            }
        }

        /* 模拟审批后的小标记 */
        .approved-stamp {
            display: inline-block;
            background: #e5f4e9;
            color: #0f6a3a;
            border-radius: 40px;
            padding: 0.2rem 1rem;
            font-size: 0.7rem;
            font-weight: 500;
            border: 1px solid #b2d9be;
            margin-left: 0.5rem;
        }

        hr {
            border: none;
            border-top: 1px dashed #dce5f0;
            margin: 1rem 0;
        }
    </style>
</head>
<body>
    <div class="project-card">
        <!-- 头部：项目计划 + 审批状态 -->
        <div class="plan-header">
            <h1>
                📋 项目计划 · 登录页
                <small>v1.0 切片</small>
                <span class="approved-stamp">已审批</span>
            </h1>
            <div class="status-badge">审批通过 · 第一版</div>
        </div>

        <!-- UI 流程指示器 (前端交互流程) -->
        <div class="flow-steps">
            <span class="step-item done">
                <span class="step-num">1</span> 需求整理
            </span>
            <span class="step-arrow">→</span>
            <span class="step-item done">
                <span class="step-num">2</span> 设计评审
            </span>
            <span class="step-arrow">→</span>
            <span class="step-item active">
                <span class="step-num">3</span> 交互切片
            </span>
            <span class="step-arrow">→</span>
            <span class="step-item">
                <span class="step-num">4</span> 开发联调
            </span>
            <span class="step-arrow">→</span>
            <span class="step-item">
                <span class="step-num">5</span> 测试上线
            </span>
        </div>

        <!-- 核心双栏：登录页面 UI + 状态展示 -->
        <div class="split-panel">
            <!-- 左侧：登录页面 (第一版可用UI) -->
            <div class="login-ui">
                <div class="login-title">登录</div>
                <div class="login-sub">项目计划 · 用户入口</div>

                <div class="input-group">
                    <label for="email">邮箱 / 账号</label>
                    <input type="email" id="email" placeholder="name@example.com" value="admin@plan.local">
                </div>

                <div class="input-group">
                    <label for="password">密码</label>
                    <input type="password" id="password" placeholder="••••••••" value="plan2024">
                </div>

                <div class="login-options">
                    <label>
                        <input type="checkbox" checked disabled> 记住此设备
                    </label>
                    <a href="#">忘记密码？</a>
                </div>

                <!-- 主操作按钮 (静态切片，展示交互状态) -->
                <button class="btn-primary" disabled style="opacity:0.9; cursor:not-allowed;">登录 · 演示版本</button>
                <!-- 说明：第一版切片仅展示UI流程，按钮为静态预览，后续迭代激活 -->

                <div class="login-footer-text">
                    还没有账号？ <a href="#">申请项目权限</a>
                </div>

                <!-- 微交互提示 (状态) -->
                <div style="margin-top: 1.2rem; font-size:0.75rem; color:#6a7f9b; background:#f2f7ff; padding:0.4rem 1rem; border-radius:40px; text-align:center; border:1px solid #e2edfc;">
                    ⚡ 第一版交互切片 · 流程与状态展示 (审批后)
                </div>
            </div>

            <!-- 右侧：状态展示面板 (项目计划状态) -->
            <div class="status-panel">
                <h3>
                    📌 计划状态
                    <span>审批后</span>
                </h3>

                <div class="status-item">
                    <div class="status-icon approved">✓</div>
                    <div class="status-content">
                        <div class="title">登录页面 · 设计稿</div>
                        <div class="desc">UI 流程已定稿，交互切片 v1</div>
                    </div>
                    <span class="status-tag approved">已审批</span>
                </div>

                <div class="status-item">
                    <div class="status-icon approved">✓</div>
                    <div class="status-content">
                        <div class="title">用户流程映射</div>
                        <div class="desc">登录 → 仪表盘 (基础路径)</div>
                    </div>
                    <span class="status-tag approved">已通过</span>
                </div>

                <div class="status-item">
                    <div class="status-icon pending">⏳</div>
                    <div class="status-content">
                        <div class="title">表单验证逻辑</div>
                        <div class="desc">邮箱/密码非空校验 (切片展示)</div>
                    </div>
                    <span class="status-tag in-progress">进行中</span>
                </div>

                <div class="status-item">
                    <div class="status-icon pending">⏳</div>
                    <div class="status-content">
                        <div class="title">API 对接 / 登录请求</div>
                        <div class="desc">计划下一迭代接入</div>
                    </div>
                    <span class="status-tag">待开发</span>
                </div>

                <hr>

                <div style="display: flex; justify-content: space-between; font-size:0.85rem; color:#1f3a57; padding-top:0.5rem;">
                    <span>✅ 审批通过节点: 3/4</span>
                    <span>📅 迭代: 第一版</span>
                </div>
                <div style="margin-top: 0.8rem; background:#eaf2fc; border-radius: 40px; padding:0.3rem 1rem; font-size:0.75rem; color:#1f4a7a; text-align:center;">
                    状态展示基于审批后项目计划 · 全自动生成
                </div>
            </div>
        </div>

        <!-- 底部：最小执行记录 / 自动推进信息 -->
        <div style="margin-top: 2.5rem; border-top: 1px solid #e6eff9; padding-top: 1.2rem; display: flex; flex-wrap: wrap; justify-content: space-between; align-items: center; font-size:0.8rem; color:#4f6f8f;">
            <div>
                <span style="background:#eef4fa; padding:0.2rem 1rem; border-radius:40px;">📋 任务ID: task_project_plan_local_5348c2315f3d6ad2_frontend</span>
            </div>
            <div style="display: flex; gap: 1rem;">
                <span>⚙️ 生成方式: local_deterministic_template</span>
                <span>📌 最小执行记录 · 自动推进</span>
            </div>
        </div>
    </div>
</body>
</html>
```