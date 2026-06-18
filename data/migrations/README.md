# data/migrations

数据库迁移预留目录。

后续接 SQLite / PostgreSQL 时再使用。

第一版设计见：

```text
../../docs/sqlite-seed-plan.md
```

约定：

- migration SQL 可以提交。
- 本地 SQLite 数据库文件不能提交。
- 第一版 schema 应保持 PostgreSQL / Supabase 迁移友好。
- `017_add_agent_config_core.sql` 开始承载 AI 员工、执行器、Skill 和边界检查核心配置；密钥仍不得进入 SQLite。
