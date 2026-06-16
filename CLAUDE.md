# CLAUDE.md — agent-swarm

## 项目骨架

- **栈**：Tauri v2 (Rust) + React 19 + TypeScript 6 + Vite 8 + Ant Design 6
- **包结构**：`packages/ui` (前端) / `packages/shared` (类型) / `packages/agent-core` (业务逻辑)
- **路由**：`useState<PageKey>` 状态路由，非 React Router
- **样式**：CSS 自定义属性 `--as-*`，BEM 命名，lucide-react 图标
- **文案**：用户可见文案一律中文；英文技术词必须带中文解释

## Karpathy 四诫

1. **先想再写** — 动手前陈述假设、找到需要澄清的点、给出取舍方案。不猜需求。
2. **最简实现** — 最少代码、不提前抽象、不顺手加没要求的功能。删代码比加代码好。
3. **外科手术式改动** — 只碰任务要求的代码。不顺手重构、不改格式、不重命名、不删注释——除非任务明确要求。
4. **以目标驱动验证** — 先定义成功标准，再编码，循环直到验证通过。`tsc --noEmit` + `vite build` 必须零错误。

## 安全边界（不可逾越）

- **只改 `packages/ui/src`**，不得修改 `packages/shared`、Rust 后端、SQLite migration、Tauri command、Runner、模型调用、审批逻辑
- **第一版只读**：不创建任务、不保存配置、不触发模型调用、不执行 Runner
- 旧"项目计划"页必须保留且功能完整
- 破坏性操作（删文件、强制推送、改数据库）必须先中文说明后果再等确认

## 代码规则（踩过的坑）

### TypeScript
- **禁止**：多个异类型 fetch 用 `Promise.all` → 会导致 union type widening，8 个 `setXxx()` 全部报 TS2322
- **正确**：独立 `.then().catch().finally()` 链 + 共享 `done()` 计数器
- **禁止**：从 `labels.ts` 导入与页面 local 函数同名的函数（如 `statusLabel`、`boolLabel`）→ TS2440 冲突。先检查页面是否有同名 local。

### 错误处理
- **禁止**：`.catch(() => [])` 静默吞错 → 桌面模式数据加载失败用户完全看不到
- **正确**：收集失败项到 `failures[]`，展示"部分数据加载失败"横幅，逐条列出失败原因。成功数据仍正常展示。
- **禁止**：`messageApi.error(error.message)` 直接暴露后端 raw error
- **正确**：粗粒度中文错误映射（连接失败/对象不存在/模式不支持/稍后重试）

### 中文化
- **禁止**：在页面组件里散写 ad-hoc 映射对象（`{high:"高风险"}`）
- **正确**：所有映射走 `@/utils/labels` 统一函数。用户可见 JSX 文本不得出现英文枚举值。
- 新增英文技术词必须同步加到 `labels.ts`

## 提交流程

改完代码后必须跑（顺序固定）：
```bash
cd F:\projects\agent-swarm\packages\ui
npx tsc --noEmit    # 零错误才算过
npx vite build       # 成功才算过
```
然后用 rg 扫描确认无英文枚举残留：
```bash
rg -n "running|idle|queued|completed|failed|approved|pending|medium|high|low|project_plan|runner_preflight|file_write|git_checkpoint|agent_frontend|agent_backend|agent_qa|QA Agent|Backend Agent" packages/ui/src
```
扫描结果只允许出现在：`labels.ts` 映射表、TypeScript 类型定义、内部比较逻辑——不允许出现在用户默认可见文案。
