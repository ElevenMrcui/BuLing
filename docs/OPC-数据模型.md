# 不令 OPC · 数据模型（P0 定稿）

本文是 OPC 数据模型的**唯一权威源**。**任何字段级信息以本文与 `runtime/migrations/*.sql` 为准**——冲突时以 SQL 为准（可编译执行的才是真理）。

- APP 库 schema：`runtime/migrations/app/0001_init.sql` + `0002_provider_wire_format.sql`
- PROJECT 库 schema：`runtime/migrations/project/0001_init.sql`

设计前置读物：[`OPC-架构决策.md § ADR-003`](OPC-架构决策.md)（为什么双层库）· [`OPC-架构决策.md § ADR-002`](OPC-架构决策.md)（为什么 SQLite + sqlx）· [`OPC-架构决策.md § ADR-004`](OPC-架构决策.md)（为什么 Key 走 Keychain）。

---

## 1. 双层布局全景

```
~/.opc/db.sqlite           APP 级 · 全局 · 跨项目
├── user_profile           单用户资料
├── settings               应用设置（KV）
├── providers              AI 能力来源（CLI/API/Local）
├── agents                 Agent 库（预置 + 用户自建）
├── projects               项目注册表
├── cross_project_memory   跨项目记忆（少量）
├── execution_logs         全局审计（append-only）
└── _migrations            迁移追踪

<project>/.opc/project.sqlite    PROJECT 级 · 每项目一份
├── project_meta           项目元数据（冗余 app.projects 一份）
├── teams                  本项目 AI 团队
├── agent_instances        Agent 实例（fork 自 app.agents）
├── permissions            Agent 权限 grant
├── workflows              工作流实例
├── tasks                  任务节点
├── task_runs              任务执行尝试
├── artifacts              产出元数据
├── artifact_versions      版本（append-only）
├── artifact_refs          追溯图（DAG 边）
├── gates                  卡点
├── reviews                评审动作（append-only）
├── memory_entries         项目内记忆（+ FTS5 全文索引）
├── execution_logs         项目审计（append-only）
└── _migrations            迁移追踪
```

## 2. 核心实体关系

```
                                 APP 库
   ┌─────────────────────────────────────────────────────────┐
   │                                                          │
   │  user_profile                                            │
   │       │                                                  │
   │       ▼                                                  │
   │   projects ────▶ 项目注册表（root_path 指向磁盘目录）    │
   │       │                                                  │
   │       │ 引用（同 id）                                     │
   │       ▼                                                  │
   └─────────────┬───────────────────────────────────────────┘
                 │
                 │ 独立文件夹 + 独立 SQLite
                 ▼
   ┌─────────────────────────────────────────────────────────┐
   │                            PROJECT 库                     │
   │                                                           │
   │  project_meta ──▶ teams ──▶ agent_instances               │
   │                                    │  (fork 自 app.agents)│
   │                                    │                      │
   │  workflows ──▶ tasks ──▶ task_runs (每 agent 每次执行)    │
   │                  │           │                            │
   │                  │           ▼                            │
   │                  │       artifacts ──▶ artifact_versions  │
   │                  │           │                            │
   │                  │           │  DAG 追溯                  │
   │                  │           ▼                            │
   │                  │       artifact_refs (parent-child)     │
   │                  │                                        │
   │                  ▼                                        │
   │              gates ──▶ reviews (人工评审, append-only)    │
   │                                                           │
   │  permissions (per-agent 授权 grant)                        │
   │  memory_entries (+ memory_fts 全文索引)                    │
   │  execution_logs (append-only 审计)                         │
   └───────────────────────────────────────────────────────────┘
```

## 3. 字段设计说明（关键决策）

### 3.1 `providers.credential_ref` = Keychain ID · 不落 Key
见 [`ADR-004`](OPC-架构决策.md)。表里只存 `opc.provider.openai.default` 这种 id，实际 Key 从 OS Keychain 现取。

### 3.2 `providers.allowed_sensitivity` = 敏感度白名单
`low` / `low,medium` / `low,medium,high` 三档。**Agent.sensitivity=high 的岗位（如"验收"）只能绑到 `allowed_sensitivity` 包含 `high` 的 Provider**——一般只有本地模型 Provider（Ollama / LM Studio）满足。这是隐私哨兵的硬拦。

### 3.2.1 `providers.wire_format`（migration `0002_provider_wire_format.sql`）
`kind='api'/'local'` 的行标注走哪种 HTTP 线协议：`anthropic-messages`（Anthropic 官方）或 `openai-compatible`（OpenAI / GLM / Qwen / DeepSeek / Ollama / LM Studio 共用的公共子集）。`kind='cli'` 的行留 `NULL`（走 `cli_binary` + Rust 侧对应的 `CliAdapter`，不经 HTTP）。与 `runtime/crates/opc-provider/src/manifest.rs` 的 `ProviderManifest.wire_format` 字段一一对应，见 `docs/OPC-架构决策.md` ADR-005 附注。

### 3.3 `agents.provider_priority` = 优先级列表 · 不是单值
```json
["claude-code", "anthropic-api", "openai-api"]
```
Provider Router 按序尝试；第一个 `status=connected` 的命中。方便"没装 CLI 就走 API，都不行再本地兜底"。

### 3.4 `agents.kind` = `preset` / `user`
- `preset` 从 `agents/*.yaml` seed 而来，用户不能删（只能 fork 后改）
- `user` 用户自建 / fork 而来的

### 3.5 `agent_instances` vs `agents`
- APP 库 `agents` = **岗位模板**（跨项目复用）
- PROJECT 库 `agent_instances` = **本项目里挂的具体实例**（可以对模板 prompt / provider / permission 局部覆盖）

覆盖字段：`system_prompt_override` / `provider_priority_override` / `permission_override`——`NULL` 表示继承模板，否则用覆盖值。

### 3.6 `tasks.assignment_mode`
三种协作策略（见 [`templates/README.md § Assignment 三种策略`](../templates/README.md)）：
- `template` = 模板默认岗位，平台推荐 Provider
- `manual` = 用户手动指派
- `auto-claim` = 广播给能力池，Agent 意愿分认领

`claim_scores` 存本次广播的意愿分快照，用于 UI 展示"谁想干这个任务，为什么"。

**Rust 实现**：`runtime/crates/opc-task`——`claim_task()` 算能力匹配分（节点 `role` 提示对应 `AgentDefinition.capabilities` 当需求集合，团队每个 `agent_instance` 按自己 `capabilities` 与需求集合的重合个数打分，最高分中标）并写 `claim_scores`；`assign_task_manually()` 直接写 `assigned_agent_id`（存在性交给外键约束兜底）。**这不是真的"Agent 自主投标"**，P0 是一个确定性的能力匹配分，见 `docs/OPC-架构决策.md` ADR-005 附注 6。

### 3.7 `task_runs` = 每次执行尝试
同一 task 可有多个 task_run：失败重试 / 用户重新触发 / 打回后 rerun。旧 run 状态置 `superseded`，不删。

`report_artifact_id` 指向本次强制产出的 `Report.md`——**没产 Report 就不算完成**（Runtime 侧的硬校验，P0 尚未接，见下）。

**Rust 实现**：`runtime/crates/opc-workflow`——`instantiate_workflow()` 把 `templates/*.yaml` 解析出的 `WorkflowTemplate`（`dag` 字段存整份 JSON 快照）落成一行 `workflows` + 逐节点一行 `tasks` + 逐 Gate 一行 `gates`；`run_task_node()` 驱动 `kind=agent` 的节点真正执行并落盘 Artifact（`opc-task::run_assigned_task()` 对 `manual`/`auto-claim` 节点复用同一个函数，不重复实现）。`kind=human` 节点（评审红线）永远不会被自动推进，只能通过 `approve_gate()`/`reject_gate()` 显式人工触发。独立的 `kind=condition` 节点（如 `qa_gate`）会在依赖满足后自动求值推进；"一次 Agent 产出一整个目录的多份具名文件"（`frontend_dev`/`backend_dev`/`bug_fix` 这几个节点需要的能力）、`kind=agent` 节点 `on_complete` 上挂的条件分支（`regression` 节点那种形状）、Report.md 强制产出这几件事这一版还没接，见 `docs/OPC-架构决策.md` ADR-005 附注 5/6/8。

### 3.8 `artifacts` + `artifact_versions` + `artifact_refs`
三张表拼出"追溯图"：
- `artifacts` = 一条产出的元信息（文件名 · 路径 · 类型）
- `artifact_versions` = append-only 版本流
- `artifact_refs` = 边（child derives-from parent）

反向追溯就是 `artifact_refs` 上的 BFS/DFS：一段代码 → 引用的 API 规范 → 引用的架构 → 引用的 PRD → 引用的用户目标。

**为什么不做 Git 化的 diff？** P0 简化，用 file_hash 判断有无变化 + 每版单独存 hash。P1 引入内容 diff（可选）。

**Rust 实现**：`runtime/crates/opc-tool::artifact::write_and_register_artifact()`——写文件 + upsert `artifacts` + 追加 `artifact_versions` 在一个 sqlx 事务内完成。**`producer_agent_id` 外键指向 `agent_instances(id)`，不是 `app.sqlite.agents` 的预置 id**——写 Artifact 前，产出它的 Agent 必须已经在这个项目里"实例化"（有一行 `agent_instances`），否则 FK 直接拒绝写入。见 `docs/OPC-架构决策.md` ADR-005 附注 3。

`agent_instances` 行的真实来源是 `runtime/crates/opc-project::create_project()`——建项目时把传入的每个预置 `AgentDefinition` 都实例化进这个项目的默认团队；`find_agent_instance_id()` 按模板 id 查回实例 id，供 `write_and_register_artifact()` 的 `producer_agent_id` 使用。见 `docs/OPC-架构决策.md` ADR-005 附注 4。

### 3.9 `gates`
每个 workflow 里的关键卡点。`kind='acceptance-gate'` 由品牌硬约束——`required=true` 永远不可跳过。

`gates.node_key` 和 `tasks.node_key` 是**两个不同的 key 空间**：前者是模板顶层 `gates:` 声明的 Gate id（如 `requirement-gate`），后者是节点 id（如 `prd_review`）——两者靠 `human` 节点的 `gate:` 字段关联。`opc_workflow::approve_gate()`/`reject_gate()` 都吃 Gate id，内部从 `workflows.dag` 快照里找到对应的评审任务节点再去更新 `tasks`。

### 3.10 `reviews` = append-only
一次评审动作 = 一行记录。历史所有决策都留着；某次"批准"后又发现问题，写一条新的"changes-requested"覆盖，前一条不删。

**Rust 实现**：`opc_workflow::approve_gate()` 写 `decision='approved'`，`reject_gate()` 写 `decision='changes-requested'` 并把模板 `on_reject.goto` 指向的节点**连同它们的全部下游**（`depends_on` 正向展开）一起重置回 `pending`，见 `docs/OPC-架构决策.md` ADR-005 附注 8。

### 3.11 `memory_entries` + `memory_fts`
- 主表存 KV
- FTS5 虚拟表做全文检索
- 三个触发器（ai/ad/au）保持 FTS 与主表同步

**P0 只做关键词检索**；P1 加 `embedded_at` 字段和 `sqlite-vec` 语义检索。

### 3.12 `execution_logs`（两库都有）
- APP 库的 log = 全局操作（provider 测试 / 项目创建 / 隐私拦截）
- PROJECT 库的 log = 项目内操作（LLM 调用 / Tool 调用 / Git 操作 / 评审决策）

**都是 append-only**（应用层禁止 UPDATE / DELETE，DBA 手工除外）。

## 4. 未在 P0 实现的实体（P1 计划）

以下概念在产品定义里存在但 P0 暂用 JSON 内嵌处理：

| 未拆表 | P0 存哪 | P1 独立表原因 |
|---|---|---|
| `Role` | `agents.role`（字段） | P1 加权限模板复用 |
| `Skill` | `agents.skills`（JSON 数组） | P1 支持技能市场 · 跨 Agent 复用 |
| `Tool` | `agents.tools`（JSON 数组） | P1 加沙箱声明 · 用量计费 |
| `MCPServer` | `agents.mcp_servers`（JSON） | P1 加安装 / 认证 / 生命周期 |
| `Knowledge` / `KnowledgeChunk` | 尚未 | P1 引入 RAG 时新建 |
| `Evidence` | `execution_logs.output` JSON | P1 独立表 · 加签 / 加密 |

**P0 的判断**：先跑通"一句目标 → AI 团队接力 → 交付"，Skill / Knowledge / Memory 这些高级能力先靠 JSON 内嵌 + 简单实现顶住，P1 拆表再上。

## 5. Schema 变更纪律

改 schema 是全仓最贵的操作之一。守则：

1. **不允许** `ALTER TABLE ... DROP COLUMN`；只能加列，用不到的列打注释
2. 加列必须 `DEFAULT` 或 `NULL`，避免旧行迁移逻辑
3. 加索引 CONCURRENTLY 不需要（SQLite 单线程写，`CREATE INDEX` 阻塞短）
4. 破坏性改动（改列语义 / 拆表）走 [Design-First 六步](../AGENTS.md#4-design-first-六步纪律)：产品定义 → 数据模型 → migration → runtime 改造 → UI 联动 → 验证
5. 每份 migration 必须**幂等安全**，具体见 [`runtime/migrations/README.md`](../runtime/migrations/README.md)

## 6. 常见查询示例

### 6.1 "健康 App 项目里，产品经理最近产出了什么"

```sql
-- 在 <project>/.opc/project.sqlite 上跑
SELECT a.id, a.name, a.file_path, a.updated_at
FROM artifacts a
JOIN agent_instances ai ON a.producer_agent_id = ai.id
WHERE ai.role = '产品经理'
ORDER BY a.updated_at DESC
LIMIT 20;
```

### 6.2 "PRD 的每一个 Acceptance Criterion 有没有对应测试用例覆盖"

```sql
-- 通过 artifact_refs 反查
SELECT p.name AS parent, c.name AS child, c.kind
FROM artifacts p
JOIN artifact_refs r ON r.parent_artifact_id = p.id
JOIN artifacts c ON c.id = r.child_artifact_id
WHERE p.kind = 'acceptance' AND c.kind IN ('test-cases','test-report');
```

### 6.3 "过去 24 小时哪些外发请求被隐私哨兵拦了"

```sql
-- APP 库
SELECT ts, actor, target, error
FROM execution_logs
WHERE result = 'blocked'
  AND ts > datetime('now', '-1 day');
```

## 7. 关联文档

- [`OPC-产品定义.md`](OPC-产品定义.md) —— 产品全景与业务概念
- [`OPC-架构决策.md`](OPC-架构决策.md) —— 为什么这么选（ADR）
- [`runtime/README.md`](../runtime/README.md) —— Runtime crate 布局
- [`runtime/migrations/README.md`](../runtime/migrations/README.md) —— Migration 规范
- [`agents/README.md`](../agents/README.md) —— 预置 Agent 定义源
- [`templates/README.md`](../templates/README.md) —— 工作流模板定义源
