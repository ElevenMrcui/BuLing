# runtime/ · OPC 核心 Runtime（Rust · Tauri 后端）

不令 OPC 的执行发动机。跑在 Tauri App 里作为主进程后端，负责一切与"操作用户电脑"和"调 AI"有关的事。

**语言**：Rust
**构建**：Cargo workspace（多 crate）
**数据库**：SQLite（app 级 + project 级双库）+ sqlx（编译时校验 SQL）
**并发**：Tokio 异步
**前端接口**：Tauri IPC command

## 目录结构

```
runtime/
├── Cargo.toml                    ✅ Workspace 根
│
├── migrations/
│   ├── README.md
│   ├── app/                      APP 级 SQLite migration（跨项目 · 全局）
│   │   ├── 0001_init.sql         ✅ P0 完成
│   │   └── 0002_provider_wire_format.sql   ✅ 给 providers 加 wire_format 列
│   └── project/                  PROJECT 级 SQLite migration（每项目一份）
│       └── 0001_init.sql         ✅ P0 完成
│
├── crates/
│   ├── opc-storage/               ✅ SQLite + sqlx · migration runner · AppDb/ProjectDb（3 测试）
│   ├── opc-provider/               ✅ Provider trait · CLI/API/Local 抽象 · 11 家厂商 manifest（11 测试）
│   │                                见 §Provider 层 与 providers/README.md
│   ├── opc-privacy/                ✅ 隐私哨兵：Agent 敏感度 vs Provider 白名单硬拦（4 测试）见 §Privacy 层
│   ├── opc-agent/                 ✅ 加载 agents/*.yaml · 播种 app.sqlite · 驱动一次 Provider 执行（7 测试）
│   │                                见 §Agent 层
│   ├── opc-tool/                  ✅ 项目内文件写入（沙箱化）+ Artifact 登记（8 测试）见 §Tool 层
│   ├── opc-project/                ✅ 创建 / 打开 / 列出 project · 把预置 Agent 实例化进团队（8 测试）见 §Project 层
│   ├── opc-workflow/               ✅ 加载 templates/*.yaml · 实例化 DAG · 驱动 Agent 节点 · 人工 Gate · condition 求值（13 测试）见 §Workflow 层
│   ├── opc-task/                  ✅ manual/auto-claim 节点指派 · 能力匹配认领分 · 驱动已指派节点执行（10 测试）见 §Task 层
│   ├── opc-audit/                 ✅ project.sqlite execution_logs（append-only）写入（2 测试）见 §Audit 层
│   └── opc-mcp/                   MCP client（计划中）
```

（原规划里的 `opc-runtime` 总装 crate、`seeds/agents.rs` 独立脚本已被 `opc-agent` 的 `seed_agents()` 取代——不再需要单独一层，Tauri `src-tauri/src/lib.rs` 直接调用即可，见 §Agent 层。）

## Provider 层（`opc-provider`，已落地）

统一 CLI / API / Local 三种 AI 能力来源，见 `docs/OPC-架构决策.md` ADR-005 附注与 `providers/README.md`。核心设计：

- **依赖倒置**：`Provider` trait 是 Runtime 唯一认的接口
- **Strategy**：`WireFormat` trait 封装 API 线协议差异（`anthropic-messages` / `openai-compatible`）
- **Adapter**：`CliAdapter` trait 封装每家 CLI 的参数拼装 / 输出解析差异
- **Factory**：`ProviderRegistry` 从 `providers/*/manifest.toml` 声明式构建实例，加厂商不改 Rust 代码

职责边界：只做一次文本补全（system + 历史进，文本 + usage 出），不做工具调用循环——那是 `opc-tool` + `opc-workflow` 的事。

## Privacy 层（`opc-privacy`，已落地）

隐私哨兵——`AGENTS.md`"与外发有关的操作必须过隐私哨兵，敏感数据自动脱敏或直接拒绝"这条硬约束的"直接拒绝"那一半。纯逻辑、零依赖的小 crate：

- `is_allowed(agent_sensitivity, provider_allowed_sensitivity)` —— `AgentDefinition.sensitivity`（`agents/*.yaml`，如"验收"是 `high`）必须显式出现在候选 Provider 的 `allowed_sensitivity` 白名单（`providers/*/manifest.toml`）里，否则拒绝
- 接入点在 `opc_agent::select_provider()`：候选列表里被拦下的 Provider **压根不会被构建/调用**（不是"调用了但连不上"），如果全部候选都因为这条被拦，报独立的 `Error::PrivacyBlocked`（而不是笼统的 `NoProviderAvailable`），`opc-workflow::run_task_node()` 接住这个错误后会写一条 `kind=privacy.block` 的审计日志

**这一版的范围**（诚实标注）：只做"敏感度白名单硬拦截"，不做"自动脱敏"——把敏感内容从 Prompt 里洗掉再放行需要真正理解内容语义，这一版没有，也不该在没有明确脱敏规则的情况下臆造一套。

## Agent 层（`opc-agent`，已落地）

三件事：**加载** `agents/*.yaml` → **播种** `app.sqlite.agents`（幂等 upsert）→ **驱动**一次 Provider 执行（把 Agent 接到 `opc-provider` 上）。

- `load_agents_from_dir()` —— 解析 9 份预置 YAML 为 `AgentDefinition`
- `seed_agents()` —— upsert 进 `agents` 表；**只有 `system_prompt` 变化时才递增 `version`**（避免每次冷启动都无意义 +1）；`ON CONFLICT ... WHERE kind='preset'` 保证永远不覆盖用户 fork 过的同 id 行
- `select_provider()` —— 按 `agent.provider_priority` 顺序尝试，返回第一个 `status().available` 的 Provider（CLI 没装就走 API，都不行报错并列出试过哪些 id + 原因）
- `run_task()` —— `select_provider` + 组 `ProviderRequest`（`system` = Agent.system_prompt）+ `execute()`，返回 `AgentRunOutput { provider_id, response }`

Tauri App 每次冷启动都会重新播种（`src-tauri/src/lib.rs` 的 `get_app_db()` 里），保证仓库里改了 Agent YAML 后用户下次开 App 就同步。

职责边界：`run_task` 只是"一次文本补全"，不强制产出 Artifact / 不校验 Report.md——那些是 `opc-workflow` 的事。

## Tool 层（`opc-tool`，已落地）

把 `opc-agent::run_task` 吐出的文本真正落到项目目录，并登记进 `project.sqlite`。

- `fs_tool::write_text_file()` —— 沙箱化写入，对齐 `permission_defaults.file = "project-only"`：词法归一化路径后校验落在项目根以内，拒绝绝对路径 / `..` 穿越
- `artifact::write_and_register_artifact()` —— 一个事务内完成"写文件 + upsert `artifacts` + 追加 `artifact_versions`"；同一 `file_path` 再次写入 = 新版本，旧版本记录不删（append-only）

**关键约束**：`artifacts.producer_agent_id` 外键指向 `agent_instances(id)`（项目内实例），不是 `app.sqlite.agents` 的预置 id——一个 Agent 要先在某个项目里"实例化"，它的产出才能合法登记。生产路径见下方 §Project 层。

职责边界：只做"写 + 登记"这一件事，不做 shell/git/http 等其它 Tool，不做权限 prompt 弹窗确认——那些留给后续切片。

## Project 层（`opc-project`，已落地）

补上 Tool 层暴露的缺口：`create_project()` 建项目时，把传入的每个预置 `AgentDefinition` 都实例化进这个项目的默认团队（写一行 `agent_instances`），`opc-tool` 登记 Artifact 用的 `producer_agent_id` 才有真实、非手工种的来源。

- `create_project()` —— 建 `<root>/.opc/` 目录 + 打开（跑迁移）`project.sqlite` → 检查 slug 唯一 → 注册进 `app.sqlite.projects` → 写 `project.sqlite.project_meta` → 建默认 `teams` 行 → 把全部预置 Agent 实例化进 `agent_instances`
- `open_project()` —— 按 `project_id` 查 `root_path`，刷新 `last_opened_at`，重新打开 `ProjectDb`
- `list_projects()` —— 「项目中心」列表源，最近打开的排最前
- `find_agent_instance_id()` —— 按模板 Agent id（如 `"product-manager"`）查这个项目里对应的 `agent_instances.id`

Tauri IPC `opc_create_project` / `opc_list_projects` 已联通，桌面壳「项目中心」卡片能真的建项目、列项目。

职责边界：只管"项目本体"的创建/打开/列出/团队搭建，不管 Task/Workflow 编排——那是 `opc-workflow` + `opc-task` 的事。

## Workflow 层（`opc-workflow`，已落地）

把 `templates/*.yaml` 变成一次真的能跑的项目交付链。五件事：

- `template::load_templates_from_dir()` —— 解析模板成 `WorkflowTemplate`；`parallel` 分组节点在加载时就地拍平成独立节点，组级 `depends_on` 并入每个子节点；节点依赖不只看显式 `depends_on`，还会从 `inputs`（`templates/README.md` 里"只声明依赖，Runtime 自动注入"那句话字面意思）和 `human` 节点的 `subject` 推导补全
- `instantiate::instantiate_workflow()` —— 建 `workflows` 行（`dag` 存整份模板 JSON 快照）+ 逐节点建 `tasks` 行（`assignment=template` 的 Agent 节点顺带用 `opc_project::find_agent_instance_id()` 解析出 `assigned_agent_id`）+ 逐 Gate 建 `gates` 行
- `runner::list_ready_agent_tasks()` / `run_task_node()` —— 只驱动 `kind=agent · assignment=template` 且依赖已满足（`load_resolved_node_keys()`：`completed` 或 `cancelled`）的节点：解析 `node.inputs` 把上游节点真实落盘的 Artifact 内容（`"<node_id>"` 注入整节点全部 output，`"<node_id>.output.<kind>"` 只注入一个；`__goal__` 从 `project_meta.goal` 取用户最初的目标）拼进 Prompt → 调 `opc_agent::run_task()` 拿文本 → 对节点声明的每个 output 调 `opc_tool::write_and_register_artifact()` 落盘登记 → 写 `task_runs` → 标记完成 → 跑一次 `condition::advance_condition_nodes()`。每次 LLM 调用/文件落盘都追加一条 `opc_audit::record()`
- `gate::approve_gate()` / `reject_gate()` —— **评审红线的唯一入口**，`runner` 永远不会自动把 `kind=human` 节点标完成。`reject_gate` 把 `on_reject.goto` 指向的节点**连同它们的全部下游**（`depends_on` 正向展开）一起重置回 `pending`——不只是直接点名的那几个，`docs/OPC-架构决策.md` ADR-005 附注 8 有完整设计说明
- `condition::advance_condition_nodes()` —— `kind=condition` 节点（如 `qa_gate`）的表达式求值器：`<node>.output.<kind>.<field> <op> <literal>`，`<field>` 从目标 Artifact 正文第一个 ` ```yaml ` 围栏代码块（"Runtime 契约字段"，`templates/artifacts/qa/Regression-Report.md` 是范例）里取；判断得出结果就把没选中的分支目标标 `cancelled`（不会永远堵住下游），判断不出来就留在 `pending`，不瞎猜

**这一版没做的事**（诚实标注）：`human` 节点不自动推进（评审红线要求）；`manual`/`auto-claim` 节点不解析执行；一次 Provider 调用的同一段文本原样写进节点声明的每一个 output（不会拆成几份不同内容的文件）；`node.inputs` 引用到 `frontend/**` 这类目录 glob 契约的 output 时读不到内容会静默跳过；`kind=agent` 节点 `on_complete` 上挂的条件分支（`regression` 节点那种形状）不求值，只有独立的 `kind=condition` 节点求值；`reject_gate` 级联重置不清空 `manual`/`auto-claim` 节点已写的 `assigned_agent_id`。见 `docs/OPC-架构决策.md` ADR-005 附注 5、7、8。

Tauri IPC `opc_create_project`（加了 `template_id` 参数）+ `opc_workflow_tasks` / `opc_workflow_ready_tasks` / `opc_workflow_run_task` / `opc_workflow_gates` / `opc_workflow_approve_gate` / `opc_workflow_reject_gate` 已联通，桌面壳新增「工作流中心」卡片。

## Task 层（`opc-task`，已落地）

补 `opc-workflow` 只驱动 `assignment=template` 节点留下的缺口：`manual`/`auto-claim` 节点谁来干、怎么定下来。定下来之后复用 `opc_workflow::run_task_node` 执行，不重新实现一遍执行链。

- `claim::compute_claim_scores()` —— 纯函数：节点 `role` 提示对应的 `AgentDefinition.capabilities` 当需求集合，项目团队里每个 `agent_instance` 按自己 `capabilities` 与需求集合的重合个数打分；`claim_task()` 选最高分（且 > 0）的中标，写 `assigned_agent_id` + `status='assigned'` + `claim_scores`
- `manual::assign_task_manually()` —— 直接把 `agent_instance_id` 写进 `assigned_agent_id`；id 是否存在交给外键约束兜底
- `readiness.rs` —— 两个入口写库前都会再查一遍 DAG 依赖是否满足，不只是 `list_claimable_tasks`/`list_manual_tasks` 这两个"给 UI 用的建议列表"里过滤
- `runner::run_assigned_task()` —— 薄封装，找到 `TaskRow` + `TemplateNode` 后直接转发给 `opc_workflow::run_task_node()`

**开发时在 opc-workflow 里发现并修的一个真实 bug**：`frontend_dev`/`backend_dev` 的 output（`{ path: frontend/** }`）解析后是单个字符串、不是列表，被旧版"只支持单路径"校验错误放行，会把 LLM 输出字面写进一个叫 `frontend/**` 的文件——`frontend/**` 是整个目录的 glob 契约，不是真实文件名。已在 `opc-workflow::runner::run_task_node` 加 glob 路径校验修掉。

**诚实标注**：`standard-software-delivery.yaml` 里全部 `manual`/`auto-claim` 节点的 output 都是这种目录级 glob，`claim_task`/`assign_task_manually` 本身能正常工作，但 `run_assigned_task` 对这几个节点会正确报错拒绝（不是遗漏，是"一次 Agent 产出一整个目录的多份具名文件"这种能力目前还不存在，比这个 crate 大得多的另一件事）；这不是能力匹配算法真的在模拟"Agent 自主投标"，是确定性的能力重合打分——见 `docs/OPC-架构决策.md` ADR-005 附注 6。

Tauri IPC `opc_task_claimable_tasks` / `opc_task_manual_tasks` / `opc_task_claim` / `opc_task_assign_manually` / `opc_task_run` 已联通，「工作流中心」卡片新增认领/指派入口。

## Audit 层（`opc-audit`，已落地）

只做一件事：把一次操作追加写进 `project.sqlite` 的 `execution_logs`（append-only，见 `docs/OPC-数据模型.md` §3.12）。

- `record(db, LogEvent { kind, result, .. })` —— 一个 `INSERT`，`kind`/`result` 取值对齐 schema 注释里已经枚举过的常量（`opc_audit::kind::*` / `opc_audit::outcome::*`），不用调用方手敲字符串
- 目前的写入点：`opc-workflow::runner::run_task_node()` 每次 `opc_agent::run_task()`（`llm.call`，成功/失败/被隐私哨兵拦下分别记 `ok`/`error`/`blocked`）和每次 `write_and_register_artifact()`（`tool.file.write`）；`opc-workflow::gate::approve_gate()`/`reject_gate()`（`review.decision`，`confirmed`/`denied`）

**这一版的范围**（诚实标注）：只接了 PROJECT 级 `execution_logs`（项目内操作）；APP 级 `execution_logs`（provider 测试/项目创建这类全局操作）还没有调用方接进来。不做日志查询/聚合——那是「日志中心」UI 直接对 `execution_logs` 建索引查询就够的事。

## 双层 SQLite 布局

**为什么两层？** 为了让"项目"是**独立可迁移单元**——把整个项目文件夹拷贝到别的机器还能用；不需要跨项目的公共设置。

| 层 | 位置 | 存什么 |
|---|---|---|
| **APP** | `~/.opc/db.sqlite` | 用户资料 · Provider · Agent 库 · 项目注册表 · 设置 · 全局日志 |
| **PROJECT** | `<project-root>/.opc/project.sqlite` | Team · Task · Artifact · Review · Workflow · 项目内 Memory · 项目日志 |

`~/.opc/` 目录布局：
```
~/.opc/
├── db.sqlite                     APP 级主库
├── db.sqlite-wal                 WAL 副本（sqlx 用）
└── logs/                         系统日志
```

`<project>/.opc/` 目录布局：
```
<project>/.opc/
├── project.sqlite                项目主库
├── project.json                  项目元数据（人可读，冗余 project_meta 表）
├── agents/                       fork 出的 agent 实例的定制 prompt（大文本）
├── artifacts/                    Artifact 版本二进制（小文件）
└── logs/                         项目日志
```

## Migration 执行策略

- APP 级：Tauri App 启动时用 sqlx migrate 跑 `migrations/app/*.sql`
- PROJECT 级：`opc-project` 里 `create_project()` / `open_project()` 时跑 `migrations/project/*.sql`（已落地）
- 版本追踪在各库的 `_migrations` 表；已应用的跳过

## API Key 安全

**绝不落表**。密钥统一走 OS Keychain：

- macOS: `security add-generic-password`（`keyring` crate）
- Windows: Credential Manager（`keyring` crate）
- Linux: Secret Service via D-Bus（`keyring` crate）

`providers.api_credential_ref` 只存 keychain 里那个 item 的 id（比如 `opc.provider.openai.default`），实际密钥每次用时从 keychain 现取。

## P0 落地顺序

1. ✅ `opc-storage` —— 连库 / 跑 migration / AppDb + ProjectDb
2. ✅ `opc-provider` —— Provider trait + CliProvider(Claude Code) + ApiProvider(11 家厂商) + ProviderRegistry；Tauri IPC `opc_providers` 已联通
3. ✅ `opc-agent` —— 加载 `agents/*.yaml` seed 到 app.sqlite + select_provider/run_task 闭环；Tauri IPC `opc_agents` 已联通，冷启动自动重新播种
4. ✅ `opc-tool` —— fs.write（沙箱化）+ Artifact 登记；`run_task` 的产出能真正落盘
5. ✅ `opc-project` —— 创建 project 目录 + project.sqlite + 把预置 Agent"实例化"进 `agent_instances`；Tauri IPC `opc_create_project`/`opc_list_projects` 已联通，桌面壳「项目中心」可用
6. ✅ `opc-workflow` —— 加载 `templates/*.yaml` + DAG 实例化，把 `run_task` + `write_and_register_artifact` 接进节点，`approve_gate`/`reject_gate` 是评审红线唯一入口；Tauri IPC 六个工作流命令已联通，桌面壳「工作流中心」可用
7. ✅ `opc-task` —— `manual`/`auto-claim` 节点的指派/认领逻辑（能力匹配打分）；Tauri IPC 五个命令已联通
8. ✅ `opc-privacy` + `opc-audit` —— Provider 选型前过敏感度白名单硬拦 + 每次 LLM 调用/文件落盘/评审决策都写 `execution_logs`；`condition` 节点（`qa_gate`）表达式求值器 + `reject_gate` 下游级联失效
9. "一次 Agent 产出一整个目录的多份具名文件"这个新的执行模型（会让 `frontend_dev`/`backend_dev`/`bug_fix` 真正跑起来）；APP 级 `execution_logs`（全局操作）接入；`opc-privacy` 的"自动脱敏"半句（目前只做"直接拒绝"）
