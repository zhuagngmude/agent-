# 后端状态切片文档

## 1. 本地命令定义

### 1.1 用户相关命令

| 命令 | 参数 | 描述 |
|------|------|------|
| `user:register` | `username`, `password`, `email` | 注册新用户 |
| `user:login` | `username`, `password` | 用户登录 |
| `user:logout` | - | 用户登出 |
| `user:get-profile` | `userId` | 获取用户信息 |
| `user:update-profile` | `userId`, `fields` | 更新用户信息 |

### 1.2 会话相关命令

| 命令 | 参数 | 描述 |
|------|------|------|
| `session:create` | `userId` | 创建新会话 |
| `session:validate` | `sessionId` | 验证会话有效性 |
| `session:destroy` | `sessionId` | 销毁会话 |

## 2. 状态流转图

### 2.1 用户状态流转

```
[未注册] --user:register--> [已注册/未登录]
[已注册/未登录] --user:login--> [已登录]
[已登录] --user:logout--> [已注册/未登录]
[已登录] --session:expire--> [已注册/未登录]
```

### 2.2 会话状态流转

```
[无会话] --session:create--> [活跃会话]
[活跃会话] --session:validate(成功)--> [活跃会话]
[活跃会话] --session:validate(失败)--> [过期会话]
[活跃会话] --session:destroy--> [无会话]
[过期会话] --session:destroy--> [无会话]
```

## 3. SQLite 持久化边界

### 3.1 数据库表结构

```sql
-- 用户表
CREATE TABLE IF NOT EXISTS users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    username TEXT UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    email TEXT UNIQUE NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 会话表
CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    user_id INTEGER NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    expires_at TIMESTAMP NOT NULL,
    is_active BOOLEAN DEFAULT 1,
    FOREIGN KEY (user_id) REFERENCES users(id)
);

-- 登录日志表
CREATE TABLE IF NOT EXISTS login_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER,
    username TEXT,
    login_time TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    success BOOLEAN NOT NULL,
    ip_address TEXT,
    FOREIGN KEY (user_id) REFERENCES users(id)
);
```

### 3.2 持久化操作边界

| 操作 | 读取表 | 写入表 | 说明 |
|------|--------|--------|------|
| 用户注册 | - | users | 插入新用户记录 |
| 用户登录 | users, sessions | sessions, login_logs | 验证用户，创建会话，记录日志 |
| 用户登出 | sessions | sessions | 标记会话为无效 |
| 会话验证 | sessions | - | 检查会话是否存在且未过期 |
| 获取用户信息 | users | - | 查询用户数据 |

### 3.3 数据访问层接口

```python
# 用户数据访问
class UserRepository:
    def create_user(username, password_hash, email) -> User
    def get_user_by_id(user_id) -> User | None
    def get_user_by_username(username) -> User | None
    def update_user(user_id, fields) -> bool

# 会话数据访问
class SessionRepository:
    def create_session(user_id, expires_at) -> Session
    def get_session(session_id) -> Session | None
    def deactivate_session(session_id) -> bool
    def clean_expired_sessions() -> int

# 日志数据访问
class LoginLogRepository:
    def create_log(user_id, username, success, ip_address) -> LoginLog
    def get_logs_by_user(user_id, limit) -> List[LoginLog]
```

## 4. 状态管理边界

### 4.1 内存状态

```python
# 当前活跃会话缓存（可选，用于性能优化）
active_sessions_cache: Dict[str, Session] = {}

# 登录失败计数（用于防暴力破解）
login_attempts: Dict[str, int] = {}
```

### 4.2 状态一致性规则

1. 所有用户数据以 SQLite 为准
2. 会话缓存仅在读取时使用，写入时同步更新数据库
3. 登录失败计数在成功登录或达到阈值后重置
4. 定期清理过期会话（可通过定时任务或懒清理）

## 5. 错误处理边界

| 错误场景 | 错误码 | 处理方式 |
|----------|--------|----------|
| 用户名已存在 | USER_EXISTS | 返回注册失败 |
| 用户不存在 | USER_NOT_FOUND | 返回登录失败 |
| 密码错误 | INVALID_PASSWORD | 增加失败计数，返回登录失败 |
| 会话已过期 | SESSION_EXPIRED | 返回需要重新登录 |
| 数据库错误 | DB_ERROR | 记录日志，返回系统错误 |