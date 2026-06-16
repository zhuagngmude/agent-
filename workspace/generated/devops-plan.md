# DevOps 切片 - 本地运行、脚本和验证命令

## 1. 本地运行命令

### 启动开发服务器
```bash
# 安装依赖
npm install

# 启动开发模式
npm run dev

# 或使用 yarn
yarn install
yarn dev
```

### 构建生产版本
```bash
npm run build
# 或
yarn build
```

## 2. 脚本命令

### package.json 脚本配置
```json
{
  "scripts": {
    "dev": "vite",
    "build": "vite build",
    "preview": "vite preview",
    "lint": "eslint . --ext .js,.jsx,.ts,.tsx",
    "format": "prettier --write .",
    "type-check": "tsc --noEmit",
    "test": "vitest",
    "test:run": "vitest run",
    "test:coverage": "vitest run --coverage"
  }
}
```

### 常用脚本命令
```bash
# 代码检查
npm run lint

# 代码格式化
npm run format

# TypeScript 类型检查
npm run type-check

# 运行测试
npm run test
npm run test:run
npm run test:coverage
```

## 3. 验证命令

### 功能验证
```bash
# 启动开发服务器后，访问 http://localhost:5173
# 验证 UI 界面是否正常渲染
# 验证所有交互功能是否正常
```

### 构建验证
```bash
# 构建生产版本
npm run build

# 预览构建结果
npm run preview

# 访问 http://localhost:4173 验证生产版本
```

### 代码质量验证
```bash
# 运行所有检查
npm run lint && npm run type-check && npm run test:run

# 或使用组合命令
npm run verify
```

### 组合验证脚本
```json
{
  "scripts": {
    "verify": "npm run lint && npm run type-check && npm run test:run",
    "ci": "npm run build && npm run verify"
  }
}
```

## 4. 完整工作流

```bash
# 1. 安装依赖
npm install

# 2. 开发
npm run dev

# 3. 代码检查
npm run lint
npm run format

# 4. 类型检查
npm run type-check

# 5. 测试
npm run test:run

# 6. 构建
npm run build

# 7. 预览
npm run preview

# 8. 完整 CI 流程
npm run ci
```

## 5. 环境要求

- Node.js >= 18.0.0
- npm >= 9.0.0 或 yarn >= 1.22.0
- 现代浏览器（Chrome, Firefox, Edge, Safari）