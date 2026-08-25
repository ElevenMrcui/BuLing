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
│   ├── opc-agent/                 ✅ 加载 agents/*.yaml · 播种 app.sqlite · 驱动一次 Provider 执行（6 测试）
│   │                                见 §Agent 层
│   ├── opc-tool/                  ✅ 项目内文件写入（沙箱化）+ Artifact 登记（8 测试）见 §Tool 层
│   ├── opc-project/                ✅ 创建 / 打开 / 列出 project · 把预置 Agent 实例化进团队（8 测试）见 §Project 层
│   ├── opc-task/                  Task 生命周期 · TaskRun 调度 · 认领意愿分
│   ├── opc-workflow/               DAG 引擎 · 节点触发 · Gate 处理 · 循环 / 条件
│   ├── opc-mcp/                   MCP client
│   ├── opc-privacy/                隐私哨兵：外发拦截 / 数据脱敏
│   └── opc-audit/                 ExecutionLog append-only 写入
```

（原规划里的 `opc-runtime` 总装 crate、`seeds/agents.rs` 独立脚本已被 `opc-agent` 的 `seed_agents()` 取代——不再需要单独一层，Tauri `src-tauri/src/lib.rs` 直接调用即可，见 §Agent 层。）

## Provider 层（`opc-provider`，已落地）

统一 CLI / API / Local 三种 AI 能力来源，见 `docs/OPC-架构决策.md` ADR-005 附注与 `providers/README.md`。核心设计：

- **依赖倒置**：`Provider` trait 是 Runtime 唯一认的接口
- **Strategy**：`WireFormat` trait 封装 API 线协议差异（`anthropic-messages` / `openai-compatible`）
- **Adapter**：`CliAdapter` trait 封装每家 CLI 的参数拼装 / 输出解析差异
- **Factory**：`ProviderRegistry` 从 `providers/*/manifest.toml` 声明式构建实例，加厂商不改 Rust 代码

职责边界：只做一次文本补全（system + 历史进，文本 + usage 出），不做工具调用循环——那是 `opc-tool` + `opc-workflow` 的事。

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
6. `opc-workflow` —— 加载 `templates/*.yaml` + DAG 实例化，把 `run_task` + `write_and_register_artifact` 接进节点
7. `opc-task` —— Task 生命周期 · TaskRun 调度 · 认领意愿分
8. `opc-privacy` + `opc-audit` —— 每次外发和 tool 调用都写日志
