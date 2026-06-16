# 数据建模切片文档

## 1. 数据模型

### 1.1 用户表 (users)

| 字段名 | 类型 | 约束 | 说明 |
|--------|------|------|------|
| id | BIGINT | PRIMARY KEY, AUTO_INCREMENT | 用户唯一标识 |
| username | VARCHAR(50) | UNIQUE, NOT NULL | 用户名 |
| email | VARCHAR(100) | UNIQUE, NOT NULL | 邮箱 |
| password_hash | VARCHAR(255) | NOT NULL | 密码哈希值 |
| salt | VARCHAR(64) | NOT NULL | 密码盐值 |
| status | TINYINT | DEFAULT 1 | 状态：1-正常，0-禁用 |
| last_login_at | DATETIME | NULL | 最后登录时间 |
| created_at | DATETIME | NOT NULL, DEFAULT CURRENT_TIMESTAMP | 创建时间 |
| updated_at | DATETIME | NOT NULL, DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP | 更新时间 |

### 1.2 登录日志表 (login_logs)

| 字段名 | 类型 | 约束 | 说明 |
|--------|------|------|------|
| id | BIGINT | PRIMARY KEY, AUTO_INCREMENT | 日志唯一标识 |
| user_id | BIGINT | NOT NULL, FOREIGN KEY (users.id) | 用户ID |
| ip_address | VARCHAR(45) | NOT NULL | 登录IP地址 |
| user_agent | VARCHAR(500) | NULL | 浏览器UA |
| login_result | TINYINT | NOT NULL | 结果：1-成功，0-失败 |
| fail_reason | VARCHAR(200) | NULL | 失败原因 |
| login_at | DATETIME | NOT NULL, DEFAULT CURRENT_TIMESTAMP | 登录时间 |

### 1.3 会话表 (sessions)

| 字段名 | 类型 | 约束 | 说明 |
|--------|------|------|------|
| id | BIGINT | PRIMARY KEY, AUTO_INCREMENT | 会话唯一标识 |
| user_id | BIGINT | NOT NULL, FOREIGN KEY (users.id) | 用户ID |
| token | VARCHAR(255) | UNIQUE, NOT NULL | 会话令牌 |
| expires_at | DATETIME | NOT NULL | 过期时间 |
| created_at | DATETIME | NOT NULL, DEFAULT CURRENT_TIMESTAMP | 创建时间 |

## 2. 迁移策略

### 2.1 初始迁移 (V1__create_users_table.sql)

```sql
CREATE TABLE users (
    id BIGINT AUTO_INCREMENT PRIMARY KEY,
    username VARCHAR(50) NOT NULL UNIQUE,
    email VARCHAR(100) NOT NULL UNIQUE,
    password_hash VARCHAR(255) NOT NULL,
    salt VARCHAR(64) NOT NULL,
    status TINYINT DEFAULT 1,
    last_login_at DATETIME,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    INDEX idx_username (username),
    INDEX idx_email (email),
    INDEX idx_status (status)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
```

### 2.2 第二次迁移 (V2__create_login_logs_table.sql)

```sql
CREATE TABLE login_logs (
    id BIGINT AUTO_INCREMENT PRIMARY KEY,
    user_id BIGINT NOT NULL,
    ip_address VARCHAR(45) NOT NULL,
    user_agent VARCHAR(500),
    login_result TINYINT NOT NULL,
    fail_reason VARCHAR(200),
    login_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    INDEX idx_user_id (user_id),
    INDEX idx_login_at (login_at),
    INDEX idx_login_result (login_result)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
```

### 2.3 第三次迁移 (V3__create_sessions_table.sql)

```sql
CREATE TABLE sessions (
    id BIGINT AUTO_INCREMENT PRIMARY KEY,
    user_id BIGINT NOT NULL,
    token VARCHAR(255) NOT NULL UNIQUE,
    expires_at DATETIME NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    INDEX idx_token (token),
    INDEX idx_user_id (user_id),
    INDEX idx_expires_at (expires_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
```

## 3. 查询边界

### 3.1 用户认证查询

```sql
-- 根据用户名或邮箱查找用户（登录验证）
SELECT id, username, email, password_hash, salt, status
FROM users
WHERE (username = :login_name OR email = :login_name)
  AND status = 1
LIMIT 1;

-- 更新最后登录时间
UPDATE users
SET last_login_at = NOW()
WHERE id = :user_id;
```

### 3.2 登录日志查询

```sql
-- 记录登录日志
INSERT INTO login_logs (user_id, ip_address, user_agent, login_result, fail_reason)
VALUES (:user_id, :ip_address, :user_agent, :login_result, :fail_reason);

-- 查询用户最近登录记录
SELECT ip_address, login_result, fail_reason, login_at
FROM login_logs
WHERE user_id = :user_id
ORDER BY login_at DESC
LIMIT 10;

-- 统计登录失败次数（用于防暴力破解）
SELECT COUNT(*) AS fail_count
FROM login_logs
WHERE user_id = :user_id
  AND login_result = 0
  AND login_at > DATE_SUB(NOW(), INTERVAL 30 MINUTE);
```

### 3.3 会话管理查询

```sql
-- 创建会话
INSERT INTO sessions (user_id, token, expires_at)
VALUES (:user_id, :token, :expires_at);

-- 验证会话有效性
SELECT s.user_id, u.username, u.email
FROM sessions s
JOIN users u ON s.user_id = u.id
WHERE s.token = :token
  AND s.expires_at > NOW()
  AND u.status = 1
LIMIT 1;

-- 删除过期会话（定时清理）
DELETE FROM sessions
WHERE expires_at < NOW();

-- 用户登出（删除会话）
DELETE FROM sessions
WHERE token = :token;
```

### 3.4 用户注册查询

```sql
-- 检查用户名是否已存在
SELECT COUNT(*) AS count
FROM users
WHERE username = :username;

-- 检查邮箱是否已存在
SELECT COUNT(*) AS count
FROM users
WHERE email = :email;

-- 创建新用户
INSERT INTO users (username, email, password_hash, salt)
VALUES (:username, :email, :password_hash, :salt);
```

## 4. 索引优化建议

1. **users表**：已对username、email、status建立索引，覆盖登录查询场景
2. **login_logs表**：对user_id、login_at、login_result建立索引，支持快速查询登录历史和失败统计
3. **sessions表**：对token建立唯一索引，支持快速会话验证；对expires_at建立索引，支持高效清理过期会话

## 5. 安全边界

1. 密码存储使用bcrypt或PBKDF2算法，配合随机盐值
2. 登录失败次数限制：30分钟内失败5次则锁定账号30分钟
3. 会话令牌使用加密安全的随机字符串（至少32字节）
4. 所有SQL查询使用参数化查询防止SQL注入
5. 敏感字段（密码哈希、盐值）不在API响应中返回