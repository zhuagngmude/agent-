```html
<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>项目计划 · 交互切片</title>
    <!-- 使用 Inter 字体，更现代 -->
    <link rel="preconnect" href="https://fonts.googleapis.com">
    <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
    <link href="https://fonts.googleapis.com/css2?family=Inter:opsz,wght@14..32,400;14..32,500;14..32,600;14..32,700&display=swap" rel="stylesheet">
    <!-- Font Awesome 6 (免费图标) -->
    <link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/font-awesome/6.0.0-beta3/css/all.min.css">
    <style>
        * {
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }

        body {
            background: #f4f6fa;
            font-family: 'Inter', -apple-system, BlinkMacSystemFont, sans-serif;
            padding: 2rem 1.5rem;
            color: #1e293b;
            display: flex;
            justify-content: center;
        }

        .app-container {
            max-width: 1280px;
            width: 100%;
        }

        /* ----- 头部 / 项目元信息 ----- */
        .project-header {
            display: flex;
            flex-wrap: wrap;
            justify-content: space-between;
            align-items: center;
            margin-bottom: 2rem;
        }

        .project-title-section {
            display: flex;
            align-items: center;
            gap: 0.75rem;
        }

        .project-badge {
            background: #eef2ff;
            color: #4f46e5;
            font-weight: 600;
            font-size: 0.75rem;
            padding: 0.25rem 0.75rem;
            border-radius: 20px;
            letter-spacing: 0.3px;
            border: 1px solid #c7d2fe;
        }

        .project-title {
            font-size: 1.8rem;
            font-weight: 700;
            letter-spacing: -0.3px;
            color: #0f172a;
        }

        .project-meta {
            display: flex;
            gap: 1.5rem;
            background: white;
            padding: 0.6rem 1.5rem;
            border-radius: 40px;
            box-shadow: 0 2px 6px rgba(0,0,0,0.02);
            border: 1px solid #e9edf2;
        }

        .meta-item {
            display: flex;
            align-items: center;
            gap: 0.4rem;
            font-size: 0.9rem;
            color: #475569;
        }

        .meta-item i {
            color: #64748b;
            width: 1rem;
        }

        .status-badge {
            background: #dcfce7;
            color: #166534;
            font-weight: 600;
            padding: 0.2rem 0.8rem;
            border-radius: 30px;
            font-size: 0.75rem;
            border: 1px solid #bbf7d0;
        }

        /* ----- 流程步骤 (水平时间线) ----- */
        .workflow-steps {
            display: flex;
            justify-content: space-between;
            align-items: flex-start;
            background: white;
            padding: 1.8rem 2rem;
            border-radius: 24px;
            box-shadow: 0 4px 12px rgba(0,0,0,0.02);
            border: 1px solid #eef2f6;
            margin-bottom: 2.5rem;
            flex-wrap: wrap;
            gap: 0.5rem 0;
        }

        .step-item {
            display: flex;
            flex-direction: column;
            align-items: center;
            flex: 1;
            min-width: 80px;
            position: relative;
        }

        .step-item:not(:last-child)::after {
            content: '';
            position: absolute;
            top: 20px;
            left: calc(50% + 28px);
            width: calc(100% - 56px);
            height: 2px;
            background: #e2e8f0;
            z-index: 0;
        }

        .step-item.active:not(:last-child)::after {
            background: linear-gradient(90deg, #4f46e5 0%, #818cf8 100%);
        }

        .step-circle {
            width: 40px;
            height: 40px;
            border-radius: 40px;
            background: #f1f5f9;
            display: flex;
            align-items: center;
            justify-content: center;
            font-weight: 700;
            font-size: 0.9rem;
            color: #64748b;
            border: 2px solid #e2e8f0;
            z-index: 2;
            background: white;
            transition: all 0.15s ease;
        }

        .step-item.active .step-circle {
            background: #4f46e5;
            border-color: #4f46e5;
            color: white;
            box-shadow: 0 6px 12px rgba(79, 70, 229, 0.2);
        }

        .step-item.completed .step-circle {
            background: #4f46e5;
            border-color: #4f46e5;
            color: white;
        }

        .step-label {
            margin-top: 0.6rem;
            font-size: 0.8rem;
            font-weight: 500;
            color: #475569;
            text-align: center;
            white-space: nowrap;
        }

        .step-item.active .step-label {
            color: #1e293b;
            font-weight: 600;
        }

        .step-item.completed .step-label {
            color: #1e293b;
        }

        .step-date {
            font-size: 0.65rem;
            color: #94a3b8;
            margin-top: 0.2rem;
        }

        /* ----- 卡片网格：状态展示 + 任务切片 ----- */
        .dashboard-grid {
            display: grid;
            grid-template-columns: 1fr 1fr;
            gap: 1.8rem;
            margin-bottom: 2rem;
        }

        .card {
            background: white;
            border-radius: 24px;
            border: 1px solid #eef2f6;
            box-shadow: 0 4px 10px rgba(0,0,0,0.02);
            padding: 1.5rem 1.8rem;
            transition: box-shadow 0.2s;
        }

        .card:hover {
            box-shadow: 0 8px 20px rgba(0,0,0,0.04);
        }

        .card-header {
            display: flex;
            justify-content: space-between;
            align-items: center;
            margin-bottom: 1.2rem;
        }

        .card-header h3 {
            font-weight: 600;
            font-size: 1.1rem;
            color: #0f172a;
            display: flex;
            align-items: center;
            gap: 0.5rem;
        }

        .card-header h3 i {
            color: #4f46e5;
            font-size: 1rem;
        }

        .card-action {
            color: #4f46e5;
            background: #f8fafc;
            border: 1px solid #e2e8f0;
            border-radius: 30px;
            padding: 0.3rem 1rem;
            font-size: 0.75rem;
            font-weight: 500;
            cursor: default;
        }

        /* 状态列表 (Kanban 风格) */
        .status-list {
            display: flex;
            flex-direction: column;
            gap: 0.8rem;
        }

        .status-item {
            display: flex;
            align-items: center;
            justify-content: space-between;
            padding: 0.6rem 0.2rem;
            border-bottom: 1px solid #f1f5f9;
        }

        .status-item:last-child {
            border-bottom: none;
        }

        .status-left {
            display: flex;
            align-items: center;
            gap: 0.8rem;
        }

        .status-dot {
            width: 10px;
            height: 10px;
            border-radius: 10px;
            background: #cbd5e1;
        }

        .status-dot.approved { background: #22c55e; }
        .status-dot.in-progress { background: #3b82f6; }
        .status-dot.pending { background: #f59e0b; }
        .status-dot.review { background: #a855f7; }

        .status-name {
            font-weight: 500;
            font-size: 0.95rem;
        }

        .status-count {
            background: #f1f5f9;
            padding: 0.15rem 0.7rem;
            border-radius: 30px;
            font-size: 0.75rem;
            font-weight: 600;
            color: #334155;
        }

        /* 任务切片列表 */
        .task-slice-list {
            display: flex;
            flex-direction: column;
            gap: 0.9rem;
        }

        .task-item {
            display: flex;
            align-items: center;
            gap: 0.8rem;
            padding: 0.5rem 0.2rem;
            border-bottom: 1px solid #f8fafc;
        }

        .task-item:last-child {
            border-bottom: none;
        }

        .task-check {
            width: 20px;
            height: 20px;
            border-radius: 6px;
            border: 2px solid #cbd5e1;
            background: white;
            display: flex;
            align-items: center;
            justify-content: center;
            color: white;
            font-size: 0.6rem;
            flex-shrink: 0;
        }

        .task-check.done {
            background: #4f46e5;
            border-color: #4f46e5;
        }

        .task-check.done i {
            color: white;
        }

        .task-content {
            flex: 1;
        }

        .task-title {
            font-weight: 500;
            font-size: 0.95rem;
        }

        .task-meta {
            display: flex;
            gap: 0.8rem;
            font-size: 0.7rem;
            color: #94a3b8;
            margin-top: 0.15rem;
        }

        .task-priority {
            font-size: 0.6rem;
            background: #f1f5f9;
            padding: 0.1rem 0.5rem;
            border-radius: 30px;
            font-weight: 600;
            color: #475569;
        }

        .task-priority.high {
            background: #fee2e2;
            color: #b91c1c;
        }

        .task-priority.medium {
            background: #fef9c3;
            color: #854d0e;
        }

        .task-assignee {
            display: flex;
            align-items: center;
            gap: 0.3rem;
        }

        .task-assignee i {
            font-size: 0.7rem;
            color: #94a3b8;
        }

        /* 底部执行记录 */
        .execution-log {
            background: white;
            border-radius: 24px;
            border: 1px solid #eef2f6;
            padding: 1.2rem 1.8rem;
            display: flex;
            flex-wrap: wrap;
            align-items: center;
            justify-content: space-between;
            gap: 0.8rem;
        }

        .log-left {
            display: flex;
            align-items: center;
            gap: 0.8rem;
        }

        .log-icon {
            background: #eef2ff;
            width: 32px;
            height: 32px;
            border-radius: 30px;
            display: flex;
            align-items: center;
            justify-content: center;
            color: #4f46e5;
        }

        .log-text {
            font-size: 0.85rem;
            color: #334155;
        }

        .log-text strong {
            font-weight: 600;
        }

        .log-badge {
            background: #f1f5f9;
            padding: 0.2rem 1rem;
            border-radius: 30px;
            font-size: 0.7rem;
            font-weight: 600;
            color: #1e293b;
            border: 1px solid #e2e8f0;
        }

        /* 响应式 */
        @media (max-width: 800px) {
            .dashboard-grid {
                grid-template-columns: 1fr;
            }
            .project-header {
                flex-direction: column;
                align-items: flex-start;
                gap: 1rem;
            }
            .workflow-steps {
                padding: 1.2rem;
                justify-content: flex-start;
                gap: 1rem 0.5rem;
            }
            .step-item {
                min-width: 60px;
                flex: none;
            }
            .step-item:not(:last-child)::after {
                display: none;
            }
        }
    </style>
</head>
<body>
<div class="app-container">

    <!-- 头部：项目计划名称 + 状态 -->
    <div class="project-header">
        <div class="project-title-section">
            <span class="project-badge"><i class="fas fa-clipboard-check" style="margin-right: 4px;"></i> 已审批</span>
            <h1 class="project-title">交互切片 · 第一版</h1>
        </div>
        <div class="project-meta">
            <span class="meta-item"><i class="far fa-calendar-alt"></i> 截止 2025-04-18</span>
            <span class="meta-item"><i class="far fa-user-circle"></i> 张一明</span>
            <span class="status-badge"><i class="fas fa-sync-alt fa-fw"></i> 进行中</span>
        </div>
    </div>

    <!-- 流程步骤 (水平时间线) 展示审批后切片流程 -->
    <div class="workflow-steps">
        <div class="step-item completed">
            <div class="step-circle"><i class="fas fa-check" style="font-size: 0.8rem;"></i></div>
            <span class="step-label">需求评审</span>
            <span class="step-date">04-02</span>
        </div>
        <div class="step-item completed">
            <div class="step-circle"><i class="fas fa-check" style="font-size: 0.8rem;"></i></div>
            <span class="step-label">设计定稿</span>
            <span class="step-date">04-05</span>
        </div>
        <div class="step-item active">
            <div class="step-circle">3</div>
            <span class="step-label">前端切片</span>
            <span class="step-date">04-08</span>
        </div>
        <div class="step-item">
            <div class="step-circle">4</div>
            <span class="step-label">交互联调</span>
            <span class="step-date">04-12</span>
        </div>
        <div class="step-item">
            <div class="step-circle">5</div>
            <span class="step-label">验收发布</span>
            <span class="step-date">04-18</span>
        </div>
    </div>

    <!-- 双栏：状态展示 + 任务切片 -->
    <div class="dashboard-grid">
        <!-- 左侧：UI 状态展示 (看板风格) -->
        <div class="card">
            <div class="card-header">
                <h3><i class="fas fa-th-list"></i> 状态展示</h3>
                <span class="card-action"><i class="far fa-edit"></i> 管理</span>
            </div>
            <div class="status-list">
                <div class="status-item">
                    <div class="status-left">
                        <span class="status-dot approved"></span>
                        <span class="status-name">已批准</span>
                    </div>
                    <span class="status-count">4</span>
                </div>
                <div class="status-item">
                    <div class="status-left">
                        <span class="status-dot in-progress"></span>
                        <span class="status-name">进行中</span>
                    </div>
                    <span class="status-count">3</span>
                </div>
                <div class="status-item">
                    <div class="status-left">
                        <span class="status-dot pending"></span>
                        <span class="status-name">待处理</span>
                    </div>
                    <span class="status-count">2</span>
                </div>
                <div class="status-item">
                    <div class="status-left">
                        <span class="status-dot review"></span>
                        <span class="status-name">审核中</span>
                    </div>
                    <span class="status-count">1</span>
                </div>
            </div>
            <!-- 微进度条 -->
            <div style="margin-top: 1rem; background: #f1f5f9; height: 6px; border-radius: 10px; overflow: hidden;">
                <div style="width: 68%; background: #4f46e5; height: 6px; border-radius: 10px;"></div>
            </div>
            <div style="display: flex; justify-content: space-between; margin-top: 0.4rem; font-size: 0.7rem; color: #64748b;">
                <span>整体进度 68%</span>
                <span>10 项任务</span>
            </div>
        </div>

        <!-- 右侧：任务切片 (UI 交互切片) -->
        <div class="card">
            <div class="card-header">
                <h3><i class="fas fa-puzzle-piece"></i> 交互切片</h3>
                <span class="card-action"><i class="fas fa-plus"></i> 切片</span>
            </div>
            <div class="task-slice-list">
                <div class="task-item">
                    <div class="task-check done"><i class="fas fa-check"></i></div>
                    <div class="task-content">
                        <div class="task-title">登录/注册流程</div>
                        <div class="task-meta">
                            <span class="task-priority high">高</span>
                            <span class="task-assignee"><i class="far fa-user"></i> 李思</span>
                        </div>
                    </div>
                </div>
                <div class="task-item">
                    <div class="task-check done"><i class="fas fa-check"></i></div>
                    <div class="task-content">
                        <div class="task-title">项目仪表盘</div>
                        <div class="task-meta">
                            <span class="task-priority high">高</span>
                            <span class="task-assignee"><i class="far fa-user"></i> 王柯</span>
                        </div>
                    </div>
                </div>
                <div class="task-item">
                    <div class="task-check"></div>
                    <div class="task-content">
                        <div class="task-title">任务看板拖拽</div>
                        <div class="task-meta">
                            <span class="task-priority medium">中</span>
                            <span class="task-assignee"><i class="far fa-user"></i> 赵一</span>
                        </div>
                    </div>
                </div>
                <div class="task-item">
                    <div class="task-check"></div>
                    <div class="task-content">
                        <div class="task-title">通知中心</div>
                        <div class="task-meta">
                            <span class="task-priority medium">中</span>
                            <span class="task-assignee"><i class="far fa-user"></i> 陈溪</span>
                        </div>
                    </div>
                </div>
                <div class="task-item">
                    <div class="task-check"></div>
                    <div class="task-content">
                        <div class="task-title">设置/偏好</div>
                        <div class="task-meta">
                            <span class="task-priority">低</span>
                            <span class="task-assignee"><i class="far fa-user"></i> 刘波</span>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    </div>

    <!-- 最小执行记录 (自动推进) -->
    <div class="execution-log">
        <div class="log-left">
            <div class="log-icon"><i class="fas fa-robot"></i></div>
            <div class="log-text">
                <strong>执行记录 #7</strong> · 前端交互切片 · 自动推进
                <span style="display: inline-block; margin-left: 0.6rem; color: #64748b; font-size: 0.75rem;">
                    <i class="far fa-clock"></i> 刚刚
                </span>
            </div>
        </div>
        <div class="log-badge">
            <i class="fas fa-check-circle" style="color: #22c55e;"></i> 最小记录已生成
        </div>
    </div>

    <!-- 隐式说明：全自动角色任务，执行单已就绪 -->
    <div style="margin-top: 1rem; font-size: 0.7rem; color: #94a3b8; text-align: right; border-top: 1px solid #eef2f6; padding-top: 0.8rem; display: flex; justify-content: space-between;">
        <span><i class="far fa-file-alt"></i> 角色: 前端开发 · 执行单: FE-SLICE-2401</span>
        <span>local_deterministic_template · 无真实写文件/命令</span>
    </div>
</div>
</body>
</html>
```