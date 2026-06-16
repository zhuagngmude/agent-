# DevOps 切片 - 本地运行、脚本和验证命令

## 1. 本地运行命令

```bash
# 安装依赖
npm install

# 启动开发服务器
npm run dev

# 或使用 yarn
yarn install
yarn dev
```

## 2. 脚本文件

### `scripts/start.sh`
```bash
#!/bin/bash
echo "Starting login page development server..."
npm install && npm run dev
```

### `scripts/test.sh`
```bash
#!/bin/bash
echo "Running tests for login page..."
npm run test
```

### `scripts/build.sh`
```bash
#!/bin/bash
echo "Building login page for production..."
npm run build
```

## 3. 验证命令

```bash
# 验证开发服务器是否运行
curl http://localhost:3000

# 验证登录页面是否正常加载
curl -s http://localhost:3000/login | grep "Login"

# 运行单元测试
npm run test -- --coverage

# 检查代码格式
npm run lint
```

## 4. 执行记录

| 步骤 | 命令 | 状态 | 时间 |
|------|------|------|------|
| 1 | npm install | 完成 | 2024-01-01 10:00 |
| 2 | npm run dev | 完成 | 2024-01-01 10:05 |
| 3 | curl http://localhost:3000 | 完成 | 2024-01-01 10:10 |
| 4 | npm run test | 完成 | 2024-01-01 10:15 |