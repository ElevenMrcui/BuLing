# 不令 OPC · 架构决策记录（ADR）

本文记录 P0 立项时锁定的关键技术决策。每条决策格式：**决定 → 原因 → 备选与否决 → 影响**。

后续更改需**新增一条 ADR**（不覆盖旧条目），保留决策演进史。

---

## ADR-001 · 桌面框架：Tauri 2（不选 Electron）

**决定**：桌面应用外壳用 Tauri 2。

**原因**：
1. **包尺寸**：Tauri 3-5 MB vs Electron 100 MB+。私人 AI 应用要"下载即用"，Electron 太重。
2. **冷启动**：Tauri 用系统 WebView，冷启 < 1s；Electron 打包 Chromium，2-4s。
3. **内存**：空 App Tauri ~50 MB，Electron ~200 MB。OPC 要在用户 Mac 上长时间常驻。
4. **Rust 后端天然契合 Local-First**：文件系统 / 进程 / Git / SQLite / Keychain 走 Rust 又快又安全，比 Node 版少一大堆异步陷阱。
5. **安全**：Tauri 默认拒绝所有 IPC，明示才允许；Electron 默认全开，需手动收权。

**备选与否决**：
- **Electron**：能复用现有 TS 代码但打包成本 20 倍，长期维护成本更高。P0 后期发现有必须复用的旧 Node 依赖再回来评估。
- **Wails**（Go 后端）：Go 生态在 AI / SQLite / crypto 上不如 Rust 成熟。
- **纯 Web 应用**：违反 Local First 原则，用户无法长期驻留。

**影响**：
- `runtime/` 用 Rust，不用 TypeScript
- 前端 IPC 通过 Tauri `invoke()`，不是 HTTP
- 已有的 `apps/local-gateway`（Node/TS）会**并存**——短期它作为独立 daemon 继续跑；P0 后期把探测逻辑迁到 Rust in-process

---

## ADR-002 · 本地库：SQLite + sqlx（不选纯 rusqlite，不选完整 ORM）

**决定**：SQLite 存储 + sqlx crate（编译时校验 SQL）。

**原因**：
1. **sqlx 编译时校验 SQL**：写错字段名 / 类型不匹配在 `cargo build` 时就报错，运行时 crash 概率极低
2. **仍然 SQL-first**：所有查询就是 SQL，可读、可迁移、可用 sqlite3 CLI 直接看
3. **原生 async**：与 Tokio 完全契合
4. **有 migration 支持**：`sqlx migrate` 内置，不用另装工具
5. **零依赖 SQLite**：sqlx 打包 SQLite 到二进制，用户不需要装 sqlite3

**备选与否决**：
- **rusqlite** 直连：更轻，但每个查询都需要手写映射代码；错误直到运行时才发现
- **SeaORM**：完整 ORM，方便但**性能开销 + 学习成本**，SQLite 场景大炮打蚊子
- **Diesel**：老牌 ORM，async 支持后加的，不如 sqlx 原生

**影响**：
- 所有 SQL 集中在 `runtime/migrations/*.sql` 和 `runtime/crates/*/sql/*.sql`
- CI 里跑 `cargo sqlx prepare` 生成离线校验数据
- 团队成员本地必须至少能执行一次 migration 才能编译

---

## ADR-003 · 双层 SQLite：APP 库 + PROJECT 库

**决定**：不用一个大库存所有东西；APP 全局一个库（`~/.opc/db.sqlite`），每个项目一个独立库（`<project>/.opc/project.sqlite`）。

**原因**：
1. **项目是可迁移单元**：把项目文件夹拷到别的机器（USB / iCloud / Git）就能带走一切；跨项目公共数据不受影响
2. **权限隔离天然**：项目库文件权限就是项目文件夹权限，无需在应用层再做租户隔离
3. **备份 / 归档简单**：`.opc/project.sqlite` 跟着项目走
4. **性能**：单一大库随项目数增长会变慢；分库天然横向可扩

**备选与否决**：
- **单库**：简单但违反"项目可迁移"原则
- **单库 + tenant_id**：加了个字段却没换来可迁移性；坏处一样有

**影响**：
- Runtime 需管理两种连接池（app pool + project pool 数组）
- Migration 也分两套（`migrations/app/` + `migrations/project/`）
- 跨库 JOIN 不能做——需要跨库聚合时应用层组装（少数场景，可接受）

---

## ADR-004 · API Key 存储：OS Keychain（不落数据库）

**决定**：所有 API Key（Anthropic / OpenAI / GLM / Qwen / DeepSeek …）走 OS Keychain：
- macOS: Keychain via `security` API
- Windows: Credential Manager via `wincred`
- Linux: Secret Service via D-Bus（`libsecret` / KWallet / GNOME Keyring）

Rust 端用 `keyring` crate 统一封装。

数据库 `providers` 表只存 keychain item 的 id 引用（如 `opc.provider.openai.default`）。

**原因**：
1. **不落表** = 备份 / 拷贝项目文件夹时不会误泄
2. 用户已经信任 OS 的密钥管理；再造轮子只会更弱
3. 三大 OS 都有成熟原生方案；`keyring` crate 统一接口

**备选与否决**：
- **AES-256-GCM 加密后落 SQLite**（旧 `apps/api/src/modules/providers/key-crypto.ts` 的做法）：作为**兜底选项保留**（Linux 无 D-Bus 场景 / 用户主动选择"文件加密而非 Keychain"），但不是默认
- **明文落表**：出局

**影响**：
- 每次调用 Provider 都要 keychain 读取（有系统弹窗风险 → 首次访问用户授权 always allow）
- CI / 无头环境不能读 keychain → 需要环境变量 fallback（`OPC_KEY_ANTHROPIC=...`）

---

## ADR-005 · Provider 层抽象：CLI / API / Local 三种（+ Hybrid 路由）

**决定**：Runtime 里一个 `Provider` trait，三种 impl：`CliProvider` / `ApiProvider` / `LocalProvider`。Agent 绑定 `provider_priority: [id1, id2, ...]` 做 fallback 路由。

**原因**：
1. **零额外账单是核心卖点**——用户已装 Claude Code / Codex，就用他登录态，不再收 Key
2. **端上私密可控**——本地模型 Provider 天然满足"数据不出机"
3. **Agent 与 Provider 解耦**——换模型不改 Agent；同 Agent 可 A/B 不同 Provider

**备选与否决**：
- **只做 API 直连**：需要用户额外配 Key，OPC 价值降一半
- **只做 CLI**：本地模型 / 无 CLI 的服务（GLM / Qwen）无法接
- **让 Agent 硬绑 Model**：换模型要改 Agent 定义，不灵活

**影响**：
- CLI Provider 要处理每家 CLI 的输出格式差异（Claude Code 有 `--output-format json`，Codex 不同，Gemini 又不同）
- 每加一家 Provider 就要写一个 adapter；`providers/` 目录规划见其 README

### ADR-005 附注 · `runtime/crates/opc-provider` 落地（P0）

首个可编译版本已落地，用了三个经典模式各解决一个变化点（避免"每加一家厂商就要改 Runtime 调用方代码"）：

| 变化点 | 模式 | 落地 |
|---|---|---|
| Runtime 不该认具体是哪家厂商 | **依赖倒置** | `Provider` trait（`execute` / `status` / `id` / `kind`），调用方只认这个接口 |
| API Provider 之间的差异只在"线协议" | **Strategy** | `WireFormat` trait，两个实现：`AnthropicMessagesWire`（Anthropic 官方 `/v1/messages`）· `OpenAiCompatibleWire`（OpenAI / GLM / Qwen / DeepSeek / Ollama / LM Studio 共用的 `/chat/completions` 公共子集）；`ApiProvider` 是持有 `Box<dyn WireFormat>` 的 context，不关心具体协议 |
| CLI Provider 之间的差异是"参数怎么拼 / 输出怎么解析" | **Adapter** | `CliAdapter` trait，当前只实现 `ClaudeCodeAdapter`（参数经真实 `claude --help` 核实）；`CliProvider` 是持有 `Box<dyn CliAdapter>` 的 context，只管进程管理（spawn / 超时 / stdout 采集） |
| 新增一家走已知协议的厂商不该碰 Rust 代码 | **Factory + 声明式 manifest** | `providers/*/manifest.toml` 描述厂商元数据（id / kind / wire_format / cli_adapter / base_url / credential_service / allowed_sensitivity），`ProviderRegistry::build()` 按 `kind` 路由到 `CliProvider::from_manifest` 或 `ApiProvider::from_manifest`；加一家 OpenAI 兼容协议的厂商只需要加一份 TOML |

**职责边界（刻意收窄）**：Provider 层只做**一次文本补全**——system + 历史进，文本 + usage 出。工具调用循环、Artifact 落盘是 Runtime（`opc-tool` + `opc-workflow`，P0.5+）的职责，不塞进这一层，避免 Provider 变成"什么都干"的上帝对象。

**已接的厂商**（11 份 manifest，见 `providers/README.md`）：

| Provider id | kind | wire_format / cli_adapter | 状态 |
|---|---|---|---|
| claude-code | cli | claude-code | ✅ 参数已用 `claude --help` 核实；响应体字段未做真实调用核实（见下方"验证状态"） |
| codex-cli / gemini-cli / aider | cli | 未实现 | manifest 占位，`CliProvider::from_manifest` 对未知 adapter 显式报错（不猜参数） |
| anthropic-api | api | anthropic-messages | ✅ 字段已用 claude-api skill 权威参考核实 |
| openai-api / glm-api / qwen-api / deepseek-api | api | openai-compatible | ✅ 走各厂商官方文档声明的 OpenAI 兼容公共子集，不加任何单一厂商私有扩展字段 |
| ollama-local / lm-studio-local | local | openai-compatible | ✅ 同上；`allowed_sensitivity` 含 `high`（本地推理，数据不出机） |

**验证状态（诚实标注，不臆造）**：
- Claude Code CLI 的**命令行参数**（`-p`、`--output-format json`、`--model`、`--system-prompt`）已用 `claude --help` 的真实输出核实。
- Claude Code CLI **`--output-format json` 的响应体字段**（`result` / `is_error` / `usage.*`）基于官方文档记录的行为，本仓库开发过程中未做一次真实调用核实（用户当次会话明确拒绝了活体探测）。`parse_output` 按此假设实现，解析失败时返回携带原始 stdout 前 500 字的 `Error::Parse`，不会静默吞掉数据——上线前必须补一次真实调用核对。
- Anthropic Messages API 的请求/响应字段已用 `claude-api` skill 的权威参考（本仓库内置的 Anthropic 官方文档缓存）逐字段核实。
- OpenAI 兼容协议只实现了公共子集（`model` / `messages[].{role,content}` / `max_tokens` → `choices[0].message.content` / `usage.{prompt_tokens,completion_tokens}`），未对 GLM/Qwen/DeepSeek 做真实联调，因为这四家均在官方文档中声明兼容该协议，不属于臆造。

**API Key 解析顺序**（`credential.rs`，对齐 ADR-004）：环境变量 `OPC_KEY_<SERVICE>`（CI / 开发期兜底）→ OS Keychain `opc.provider.<service>/default`。`keyring` crate 本身不带任何后端，按平台在 `Cargo.toml` 用 `[target.'cfg(...)'.dependencies]` 显式开启 `apple-native` / `windows-native` / `sync-secret-service`。

**测试**：14 个测试全绿——`opc-storage` 3 个（migration + FTS5）+ `opc-provider` 11 个（5 个 CLI adapter 纯函数单测 + 6 个 wiremock 集成测试，覆盖两种 wire format 的成功/错误路径 + manifest 加载 + 敏感度约束校验 + 凭证缺失场景）。API 测试**不发起任何真实网络请求**（wiremock 起本机 mock server）；CLI 测试**不 spawn 真实二进制**（只测 build_args / parse_output 纯函数）。

**数据模型联动**：`app.sqlite.providers` 新增 `wire_format` 列（migration `0002_provider_wire_format.sql`），CLI 类型的行留 NULL。

**下一步（P0.5+）**：Codex / Gemini CLI / Aider 的 adapter（先跑一次真实 `--help` 核实参数）；Tauri IPC 已加 `opc_providers` 命令跑通"扫描全部 Provider 状态"的最小闭环；模型中心 UI 的"添加 Provider / 测试连接 / 保存 Key"表单待建。

### ADR-005 附注 2 · `runtime/crates/opc-agent` 落地（P0.5）—— Agent 接到 Provider 上

在 `opc-provider` 之上加了一层薄的 Agent 层，把"岗位定义"和"真的跑一次"接起来，闭环从 YAML 文件走到一次真实 Provider 调用：

- **加载**：`load_agents_from_dir()` 把 `agents/*.yaml` 解析成 `AgentDefinition`（字段与 `agents` 表逐一对应）
- **播种**：`seed_agents()` upsert 进 `app.sqlite.agents`。两条设计取舍：
  1. `ON CONFLICT(id) DO UPDATE ... WHERE agents.kind = 'preset'` —— 用户 fork 过某个预置 id 的场景（理论不该发生，但 SQL 层面必须防）不会被重新播种覆盖
  2. `version` 只在 `system_prompt` 真的变化时才 `+1`（`CASE WHEN ... THEN +1 ELSE` 表达式），不是每次冷启动播种都无脑递增——`version` 字段要留给未来 UI 做"这个岗位改过几次"的有意义信号
- **驱动**：`select_provider()` 按 `agent.provider_priority` 顺序尝试，跳过不可用的（CLI 未装 / adapter 未实现 / API 无凭证），返回第一个 `status().available` 的实例；`run_task()` 在此基础上组 `ProviderRequest`（`system` = Agent 人格）并 `execute()`

**Tauri 集成**：`get_app_db()` 每次冷启动都会重新播种（幂等，见上），新增 IPC `opc_agents` 列出全部岗位。顺手修了 `opc_providers` 的一个真实 bug——原实现在 for 循环里对 `registry.build()` 用 `?`，导致任何一个 manifest 构建失败（比如 codex-cli/gemini-cli/aider 这类 adapter 未实现的）会让**整个** Provider 列表请求失败；改为单个失败只把该 Provider 标 `available:false` 并附错误详情，不拖累其余条目。

**测试**：6 个，覆盖加载（真实解析 9 份 YAML）、播种幂等性、version 只在 prompt 变化时递增、不覆盖用户行、`select_provider` 的 fallback 与全失败报错路径（后两个用临时 manifest 目录 + wiremock，不依赖沙箱机器装了什么 CLI，避免测试结果随机器环境漂移）。

**未做的事**（刻意留给 `opc-workflow`）：工具调用循环、Artifact 落盘、强制产出 Report.md、认领意愿分——`run_task` 现在只是"喂一句话，吐一段文本回来"。

### ADR-005 附注 3 · `runtime/crates/opc-tool` 落地（P0.5）—— 产出真正落盘

`opc-agent::run_task` 吐出的文本到这一层才算真正"交付"：写进项目目录的文件，并登记进 `project.sqlite`。P0 最小切片只做一件事到位，不铺开做 shell/git/http 等其它 Tool。

- **`fs_tool`**：沙箱化写入，对齐 `permission_defaults.file = "project-only"`——词法归一化路径（不要求父目录已存在，因为写入场景经常还没建目录）后校验落在项目根以内，拒绝绝对路径和 `..` 穿越，不看真实文件系统状态（不 `canonicalize`，因为目标文件可能是首次创建）
- **`artifact`**：`write_and_register_artifact()` 一个事务内做完"写文件 + upsert `artifacts` + 追加 `artifact_versions`"——避免"文件写了但库没记"或反过来的半成品状态。同一 `file_path` 再次写入 = 新版本（`latest_version` 递增），旧版本记录不删（`artifact_versions` 是 append-only，见 `docs/OPC-数据模型.md` §7）

**踩到的一个 schema 细节**：`artifacts.producer_agent_id` 外键指向 `agent_instances(id)`（**项目内实例**，不是 `app.sqlite.agents` 的预置 id）——测试第一次跑直接传 `agents/*.yaml` 里的 `product-manager` 当 producer_agent_id 会 FK 报错，因为这个 id 在项目库里根本不存在对应的 `agent_instances` 行。修法是先種一条最小 `teams` + `agent_instances`，而不是绕过外键约束——这提前暴露了一个后续 `opc-project`/`opc-agent` 必须补的环节：**Agent 要在某个项目里"实例化"（fork 出 `agent_instances` 行）之后，它的产出才能合法地登记为 Artifact**。

**测试**：8 个（4 单测覆盖路径穿越校验的边界情况 + 4 集成测试覆盖首次写入 / 重写版本递增+历史保留 / 越界拒绝 / 端到端胶水——真跑一个 Agent（wiremock）→ 把它的产出写进项目并登记，证明 `opc-agent` 与 `opc-tool` 能拼起来用）。workspace 累计 28 个测试全绿。

**下一步（P0.5+）**：`opc-project` 把"创建项目 → 实例化 Team/Agent"这条链补上，这样 `producer_agent_id` 才有一个真实来源，而不是测试里手工种的行；`opc-workflow` 把 `run_task` + `write_and_register_artifact` 接进 DAG 节点，让"一个岗位跑完 → 自动落盘 → 触发下游"整条链自动化。

### ADR-005 附注 4 · `runtime/crates/opc-project` 落地（P0.5）—— 补上"实例化"缺口

正是附注 3 里发现的缺口：Artifact 的 `producer_agent_id` 外键指向 `agent_instances(id)`，但在这个 crate 之前，没有任何 runtime 代码会创建这一行——只能靠测试手工种。`opc-project` 补的就是这一步。

四个函数：

- **`create_project()`**：建 `<root>/.opc/` 目录 + 打开（跑迁移）`project.sqlite` → 检查 `slug` 唯一 → 在 `app.sqlite.projects` 注册 → 在 `project.sqlite.project_meta` 写自描述 → 建一个默认 `teams` 行 → **把传入的每个 `AgentDefinition`（通常是 `load_agents_from_dir()` 读到的全部 9 个预置岗位）实例化成一行 `agent_instances`**。全程一个项目对应一次调用，不是分步的多次 IPC 往返，避免中间状态。
- **`open_project()`**：按 `project_id` 查 `root_path`，刷新 `last_opened_at`，重新打开 `ProjectDb`。
- **`list_projects()`**：「项目中心」列表源，按 `last_opened_at DESC NULLS LAST, created_at DESC` 排序——最近打开的在最前面。
- **`find_agent_instance_id()`**：按 `template_agent_id`（如 `"product-manager"`）查这个项目团队里对应的 `agent_instances.id`——`opc-tool` 登记 Artifact 时该传这个 id 当 `producer_agent_id`，不是 `agents/*.yaml` 里的预置 id。

**Tauri 集成**：新增 IPC `opc_create_project` / `opc_list_projects`，桌面壳的"项目中心"卡片现在能真的建项目、列项目（表单直接填本地绝对路径当 `root_path`——P0 阶段还没接原生目录选择器）。

**测试**：8 个集成测试——创建项目校验 app.sqlite/project_meta/团队/全部 Agent 实例化四件事都做到、重复 slug 拒绝、打开已存在/不存在项目、按最近打开排序、按模板 id 查实例、真实加载 9 份 `agents/*.yaml` 全部实例化、以及一条**闭环端到端测试**：`create_project()` 建项目 → `find_agent_instance_id()` 取到真实（非手工种的）`agent_instances.id` → `run_task()` 真跑一次 Agent → `write_and_register_artifact()` 用这个真实 id 登记 Artifact，FK 约束正常放行。workspace 累计 36 个测试全绿。

**下一步（P0.5+）**：`opc-workflow` 把 `create_project` 产出的 `agent_instances` 接进工作流模板（`templates/*.yaml`）实例化出的 DAG 节点，让"选模板 → 建项目 → 团队自动配齐 → 任务自动派发"整条链跑起来。

### ADR-005 附注 5 · `runtime/crates/opc-workflow` 落地（P0.5）—— DAG 加载 + 实例化 + 驱动 + 评审红线唯一入口

把附注 4 留的缺口接上：`templates/*.yaml` 怎么变成一次真的能跑的项目交付链。四件事：

- **加载**（`template.rs`）：解析 `templates/*.yaml` 成 `WorkflowTemplate`。**不是全字段忠实解析**——`on_approve`/`on_complete`（含条件分支）/`on_true`/`on_false` 这些字段这一版不认识（serde 默认忽略未知字段，不报错但也不保留）。`parallel` 分组节点在加载时**就地拍平**成独立节点，组级 `depends_on` 并入每个子节点。
- **依赖推导踩到的一个真实坑**：`templates/README.md` 写明"`inputs`：前置节点的 Artifact 引用……只声明依赖，Runtime 自动注入"，但模板文件里不少 `agent` 节点（`regression` / `acceptance_prep`）压根没写 `depends_on`，真正的前置关系只体现在 `inputs` 里。第一版加载器只认 `depends_on` + `human` 节点的 `subject` 兜底，跑真实模板测试时"应该只有入口节点 ready"却测出 3 个——`regression`/`acceptance_prep` 被误判为无依赖直接可跑。修法是从 `inputs`（取第一个 `.` 之前的 token）里再补一遍依赖，`condition` 节点同理从 `condition` 表达式里补，跟显式 `depends_on`/`subject` 做并集，不是二选一。
- **实例化**（`instantiate.rs`）：建 `workflows` 行（`dag` 存整份模板的 JSON 快照）→ 逐节点建 `tasks` 行——`assignment=template` 的 `agent` 节点顺带调 `opc_project::find_agent_instance_id()` 把 `assigned_agent_id` 解析好 → 逐 Gate 建 `gates` 行。
- **驱动**（`runner.rs`）：`ready_node_keys()` 是个纯函数（DAG + 已完成节点集合 → 依赖满足的节点 id），`list_ready_agent_tasks()` 在此基础上过滤出 `kind=agent · assignment=template · assigned_agent_id 已解析` 的可执行任务；`run_task_node()` 真的调 `opc_agent::run_task()` 拿文本，再对节点声明的**每一个** output 调 `opc_tool::write_and_register_artifact()` 落盘登记，写 `task_runs`，标记 `tasks.status='completed'`。
- **人工 Gate**（`gate.rs`）：`approve_gate()` / `reject_gate()` 是评审红线的**唯一入口**，`runner` 模块自己永远不会把 `kind=human` 的节点标记完成——这不是"没做完"，是红线硬约束本身要求的行为。`reject_gate()` 会把节点上 `on_reject.goto` 指向的任务重置回 `pending`，让它们重新出现在可执行集合里，操作化"打回重做"。**已知局限**：只重置 `goto` 直接点名的节点，不做下游级联失效。

**Gate 的两个 key 空间要分清**：`gates` 表按模板顶层 `gates:` 声明的 Gate id（如 `requirement-gate`）建行；`tasks` 表按节点 id（如 `prd_review`）建行；两者靠节点的 `gate:` 字段关联。`approve_gate`/`reject_gate` 都吃 Gate id，内部从 DAG 快照里找到对应的评审任务节点——这是实现时踩的一个坑：最初直接拿任务节点 id 去查 `gates` 表，查不到。

**这一版没做的事**（诚实标注，不是遗漏）：
- `human` 节点不自动推进——评审红线要求，不是缺口
- `condition` 节点（如 `qa_gate` 的表达式判断）没有求值器，会一直停在 `pending`
- `manual` / `auto-claim` 节点不解析 `assigned_agent_id`，不会被驱动（`frontend_dev`/`backend_dev`/`bug_fix` 这几个节点目前没有生产路径）
- 不把上游节点的 Artifact 内容注入 Prompt，只给一句提到节点名和期望产出的通用指令——"读上游 PRD 写架构"这种真正的上下文传递是下一版的事
- `reject_gate` 打回不做下游级联失效

**Tauri 集成**：`opc_create_project` 加了 `template_id` 参数，传了就顺带 `instantiate_workflow`，`active_workflow_id` 带回前端；新增 `opc_workflow_tasks` / `opc_workflow_ready_tasks` / `opc_workflow_run_task` / `opc_workflow_gates` / `opc_workflow_approve_gate` / `opc_workflow_reject_gate` 六个 IPC。桌面壳新增「工作流中心」卡片：任务节点列表 + 可执行节点的"跑这个节点"按钮 + Gate 列表 + "通过"/"打回"按钮（`approve_gate`/`reject_gate` 只能由用户在界面上点，Runtime 不会自己调）。项目库 sqlx pool 现在按 `project_id` 缓存在 `OpcState.open_projects`（`Mutex<HashMap>`），避免每个工作流 IPC 调用都重新开一个 pool。

**测试**：7 个集成测试——真实模板加载（拍平校验 + 依赖推导校验）、纯函数 `ready_node_keys` 的两种依赖满足状态、实例化校验 tasks/gates 数量与 `assigned_agent_id` 解析对/不对、`list_ready_agent_tasks` 只返回入口节点、端到端闭环（`prd` 真跑 wiremock Provider → 3 个 output 落盘 → `prd_review` 不被自动推进 → `approve_gate` 之后 `tech_selection` 才进入可执行集合）、打回重置。workspace 累计 43 个测试全绿。

**下一步（P0.5+）**：`opc-task` 把 `manual`/`auto-claim` 节点的指派/认领逻辑接上；`qa_gate` 这类 `condition` 节点需要一个表达式求值器；上游 Artifact 内容注入 Prompt 是让 Agent 产出真正有意义的下一件大事。

---

## ADR-006 · 依赖沿用：`apps/local-gateway` 与 `packages/cli-registry` 短期保留

**决定**：新架构 P0 阶段**不迁**这两个组件，它们继续按当前形态（Node/TS · Fastify · 127.0.0.1）运行。

**原因**：
1. `packages/cli-registry`（10 家 AI CLI 签名注册表）**数据本身**是资产，Rust 那边写一个 loader 直接读同一份 YAML/JSON 即可，不需要重造
2. `apps/local-gateway` 已经通过 CORS + Token 走通了本地网关协议，Tauri App 起步阶段可以直接调这个 daemon 拿探测结果
3. 时机成熟（P0 后期）再把探测逻辑用 Rust 重写融入 `runtime/crates/opc-provider`

**影响**：
- 短期仓库有两种语言：主体 Rust + 小旧 Node（local-gateway + cli-registry）
- 用户如果只跑 Tauri App，local-gateway 会被 App 内嵌启动（而不是让用户手动 `make gateway`）——P0 后期实现

---

## ADR-007 · 前端：React + shadcn/ui + Tailwind

**决定**：Tauri 前端用 React 18 + shadcn/ui + Tailwind CSS。

**原因**：
1. React 生态最成熟；组件资源最多
2. shadcn/ui 是"复制到你项目里"式的，不是 npm 依赖，完全可控 · 不受升级绑架
3. Tailwind 让主题 tokens（沿用旧原型的 `--bg` / `--ink` / `--accent` 那套）非常自然
4. TypeScript strict + noUncheckedIndexedAccess 保持类型健壮

**备选与否决**：
- **Svelte / SolidJS**：更小更快但社区组件少，招 AI 时也不如 React 熟
- **Vue**：也可以，但目前团队 React 更熟
- **HTMX + Alpine**：太保守，Tauri IPC 复杂交互撑不住

---

## ADR-008 · 命名：主品牌沿用「不令」，产品线定位为 OPC

**决定**：
- 主品牌：**不令**（保留《论语》"其身正，不令而行"的品牌哲学）
- 产品线：**OPC**（One Person Company）
- 完整名：**不令 OPC** / 简称 `OPC` / 英文 `BuLing OPC`

**原因**：
1. "不令" 二字**天然对应"AI 团队不令而行"**的产品机制，是白捡的品牌资产
2. 但"不令"过于抽象，普通用户理解成本高；用 "OPC" 做副定位一句话把功能讲清楚
3. 主 + 副的组合可以走"品牌沉淀 + 定位清晰"两不误

**备选与否决**：
- 完全放弃"不令"，纯用 "OPC"：丢掉品牌沉淀，字母缩写辨识度低
- 用纯中文如"分身 / 搭子 / 一府"：品牌较薄，无国际化空间

**影响**：
- 官网 / 商店 / 文档标题用 `不令 OPC`
- 应用内简称 `OPC`
- 一级菜单命名统一 `XX 中心`；AI 岗位命名统一中文短名（不加 `Agent` 后缀）
