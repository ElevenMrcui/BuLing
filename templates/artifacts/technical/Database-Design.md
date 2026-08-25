---
kind: db-design
version: 1
producer: 架构师
inputs:
  - prd
  - architecture
project: <auto-fill>
created_at: <auto-fill-iso>
---

# 数据库设计 · Database Design

<!--
架构师产出。硬约束：
1. 每张表都要列全字段 · 类型 · 约束 · 索引，禁止"TBD"
2. 关键 JOIN 用示例 SQL 展示
3. 禁忌：不允许在业务表存 API Key（走 OS Keychain）
-->

## 1. 库全景

| 库 | 存储引擎 | 用途 |
|---|---|---|
| [main] | [SQLite/PostgreSQL/MySQL] | [业务数据] |

## 2. 表清单

| 表 | 用途 | 主键 | 大概行量级 | 特殊性质 |
|---|---|---|---|---|
| `users` | 用户 | id | ~10^4 | - |
| `<table>` | [用途] | [pk] | [级别] | append-only / FTS / ... |

## 3. 表结构

<!-- 每表一节。用 SQL DDL 直接可执行 -->

### 3.1 `<table_name>`

**用途**：[一句话]

```sql
CREATE TABLE <table_name> (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    <field>      <type> NOT NULL,
    created_at   TEXT NOT NULL DEFAULT (datetime('now')),
    ...
);

CREATE INDEX <table>_<field>_idx ON <table_name> (<field>);
```

**字段说明**：

| 字段 | 类型 | 约束 | 说明 |
|---|---|---|---|
| id | INTEGER | PK | 自增 |
| <field> | <type> | NOT NULL | [业务含义] |

**关键索引**：
- `<table>_<field>_idx` — 支撑 [某查询场景]

**关联关系**：
- FK → `<other_table>.<field>`（ON DELETE CASCADE / SET NULL / RESTRICT）

---

### 3.2 `<next_table>`

...

## 4. 关键查询示例

<!-- 每个高频 / 复杂查询给一个可执行 SQL 示例 -->

**查询 1：[名称]**

```sql
SELECT ...
FROM ...
WHERE ...;
```

## 5. 迁移策略

- 加列必须 `DEFAULT` 或 `NULL`（避免旧行迁移逻辑）
- 破坏性改动走 Design-First 六步
- 每份 migration 幂等安全

## 6. 敏感字段清单

<!-- 明示哪些字段是敏感的，供隐私哨兵参考 -->

| 表 | 字段 | 敏感度 | 处理 |
|---|---|---|---|
| users | email | mid | 界面脱敏显示 |
| providers | api_credential_ref | high | 只存 keychain 引用 |
