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

### ADR-005 附注 6 · `runtime/crates/opc-task` 落地（P0.5）—— manual/auto-claim 指派

`opc-workflow` 只驱动 `assignment=template` 的节点；`manual`/`auto-claim` 节点在实例化时 `assigned_agent_id` 留空，没有任何生产路径能把它填上。`opc-task` 补的就是"谁来干"这一步，定下来之后复用 `opc_workflow::run_task_node` 执行，不重新实现一遍执行链。

- **认领打分**（`claim.rs`）：`compute_claim_scores()` 是个纯函数——节点声明的 `role`（如 `frontend_dev` 的 `role: frontend`）当作"这个节点需要什么能力"的提示，去查对应 `AgentDefinition.capabilities` 当需求集合，项目团队里每个 `agent_instance` 按自己模板的 `capabilities` 与需求集合的重合个数打分，`claim_task()` 选最高分（且 > 0）的中标，写 `assigned_agent_id` + `status='assigned'` + `claim_scores`（JSON 快照）。**诚实标注**：这不是真的"Agent 自主投标"，是一个能力匹配分；今天团队固定 9 个不重复预置岗位，这个算法几乎总选回 `role` 提示的那个岗位本身，等以后允许自定义/派生 Agent 参与认领才真正派上用场。
- **手动指派**（`manual.rs`）：`assign_task_manually()` 直接把 `agent_instance_id` 写进 `assigned_agent_id`——存不存在这个 id 交给 `tasks.assigned_agent_id` 的外键约束兜底，不重复校验。
- **就绪校验**：两个入口在写库前都会再查一遍 DAG 依赖是否满足（`readiness.rs`），不是只在 `list_claimable_tasks`/`list_manual_tasks` 这两个"给 UI 用的建议列表"里过滤——直接调底层函数的调用方不该绕过这层校验。
- **驱动**（`runner.rs`）：`run_assigned_task()` 是个薄封装，找到 `TaskRow` + `TemplateNode` 后直接转发给 `opc_workflow::run_task_node()`。

**开发时撞见的一个真实 bug**（不是 opc-task 自己的，是上一轮 opc-workflow 遗留的）：`frontend_dev`/`backend_dev` 的 output 是 `{ kind: source, path: frontend/** }`——`path` 解析后 `len() == 1`（是个裸字符串，不是列表），会被上一版 `run_task_node` 的"只支持单路径"校验错误地放行，实际把 LLM 吐出的文本写成一个字面叫 `frontend/**` 的文件——这是错的，`frontend/**` 是"整个目录"的 glob 契约，不是一个真实文件名。修法是在 `opc-workflow::runner::run_task_node` 里加一个 `is_glob_like()` 检查（路径含 `*`/`?`/`[` 就拒绝），跟"多路径"校验合并成一条"是否单个字面文件路径"的判断，`Error::UnsupportedOutputShape` 覆盖两种情况。**这个修复在 opc-workflow 里，但是靠给 opc-task 写测试才发现的**——`standard-software-delivery.yaml` 里全部三个 `manual`/`auto-claim` 节点（`frontend_dev`/`backend_dev`/`bug_fix`）的 output 都是这种目录级 glob，一次 LLM 调用产不出一整个目录的多份具名源码文件，这不是 `opc-task` 能解决的问题——是比它大得多的另一件事（"一次 Agent 产出多份具名文件"这种能力）。

**测试**：10 个——纯函数打分排序（含同分按 id 升序的确定性校验）、认领成功/依赖未满足/节点类型不对三种路径、`list_claimable_tasks`/`list_manual_tasks` 只返回就绪节点、手动指派成功/未知 agent_instance 被外键拒绝、`run_assigned_task` 对真实模板的 glob 输出正确拒绝而不是写出垃圾文件（并断言磁盘上确实什么都没写）、以及一条用合成模板（单文件输出的 `manual` 节点）证明"指派 → 执行 → 落盘登记 Artifact"全链路真的能跑通的端到端测试。workspace 累计 53 个测试全绿。

**下一步（P0.5+）**：`qa_gate` 这类 `condition` 节点的表达式求值器；"一次 Agent 产出一整个目录的多份具名文件"这个新的执行模型（会让 `frontend_dev`/`backend_dev`/`bug_fix` 真正跑起来）；`opc-privacy`/`opc-audit` 补齐 execution_logs 的写入路径。

### ADR-005 附注 7 · 上游 Artifact 内容真的注入 Prompt 了（P0.5）

补附注 5 留的最大一个诚实缺口：`run_task_node()` 以前只给 Agent 一句"请完成工作流节点「xxx」，产出：yyy"的通用指令，`node.inputs` 声明的依赖只用来推导 `depends_on`，从没真正读进 Prompt——"读上游 PRD 写架构"这种上下文传递是假的。

- **`runner.rs` 新增 `build_prompt()`**：解析 `node.inputs` 的每个 token，按 `templates/README.md` 的语义分两种：`"<node_id>"`（裸节点 id）——注入该节点全部 output；`"<node_id>.output.<kind>"`（如 `prd.output.acceptance`）——只注入这一个具体 output。解析出目标节点声明的 `output.path` 后，直接从磁盘按这个路径 `tokio::fs::read_to_string`——不查 `artifacts` 表，因为 `write_and_register_artifact()` 本来就是原样写到这个路径，磁盘内容即最新版本，不需要多一层间接查询。`"__goal__"` 单独处理，从 `project_meta.goal`（单行表）里取用户最初的目标文本。
- **读不到就跳过，不报错**：引用到 `frontend/**` 这类目录 glob 契约的 output（`assignment=manual`/`auto-claim` 节点常见）没法当单文件读，静默跳过；文件因为某种原因还没落盘也一样跳过。这是有意的宽松——`depends_on`（含 `augment_depends_on_from_inputs` 补的隐式依赖）已经保证"引用的节点跑完了才轮到当前节点"，这里出意外不该炸掉整个节点执行，只应该降级成"这部分上下文拿不到"。
- **签名变了**：`run_task_node()` 新增 `dag: &WorkflowTemplate` 参数（原来的调用方 `opc-task::run_assigned_task()`、桌面壳 `opc_workflow_run_task` IPC 早就为了拿 `TemplateNode` 而加载过一份 DAG，顺手传进来即可，不用重新查库）。
- **测试怎么证明不是摆设**：新增两条集成测试，用 wiremock 的 `body_string_contains` 卡两个互斥条件——一条 mock 只认"节点「prd」"，另一条必须同时看到"节点「tech_selection」"和 `prd` 落盘的真实文本才应答，且两条 mock 的应答内容完全不同。落盘的 `tech_selection` 产出内容能对上"要求带上游真实内容"那条 mock 的应答，就证明 Provider 确实收到了注入后的 Prompt，不是巧合命中。另一条测试同理验证 `__goal__` 能从 `project_meta.goal` 读出来。workspace 累计 55 个测试全绿。

**这一版仍然没做的事**：一次 Provider 调用的同一段文本仍然原样写进节点声明的每一个 output 路径（附注 5 的简化 1 还在）；`output.path` 是目录 glob 契约的节点仍然不驱动（附注 5 的简化 2 还在）——这两条跟"读上游内容"是两件独立的事，这次没有顺手动它们。

### ADR-005 附注 8 · `condition` 节点表达式求值器 + `reject_gate` 下游级联失效（P0.5）

补附注 5 留的另外两个诚实缺口：`condition` 节点（如 `qa_gate`）一直停在 `pending` 不会自动推进；`reject_gate` 打回只重置 `on_reject.goto` 直接点名的节点，不管下游已经跑完的那些。

**`condition` 节点求值**（新增 `runtime/crates/opc-workflow/src/condition.rs`）：
- **表达式语法**：只认 `templates/README.md` 里已经出现过的形状——`<node_id>.output.<kind>.<field> <op> <literal>`，恰好 3 个空格分隔的 token（`qa_test.output.bug-report.critical_count == 0`）。`<field>` 从目标 Artifact 正文第一个 ` ```yaml ` 围栏代码块里取（"Runtime 契约字段"，`templates/artifacts/qa/Regression-Report.md` 已经是这个格式的范例）。**踩到一个真实的模板内部不一致**：`templates/artifacts/qa/Bug-Report.md` 原本只在 prose 里写"Runtime 契约：`critical_count = P0 数`"，没给围栏代码块，跟 `Regression-Report.md` 已有的约定对不上——这次一并把围栏代码块补齐，不是凭空发明新格式，是让模板自己内部一致。
- **分支怎么"取消"，不是靠改 `depends_on`**：真实模板里 `bug_fix`/`acceptance_prep` 这些分支目标的 `depends_on` 并不包含 `qa_gate`（只包含 `qa_test`），所以`condition` 节点求出结果后，**没选中的那个分支目标**会被标 `cancelled`（`tasks.status` 新增的语义用法，schema 本来就有这个取值，不用改表）；`load_resolved_node_keys()`（原来查 `status='completed'` 的三处重复查询，这次统一收成这一个函数）把 `completed ∪ cancelled` 都当"已解决，下游可以继续"——一个被跳过的分支不该永远堵住依赖它的下游。判断不出结果（Artifact 没落盘/没写契约字段）就留在 `pending`，不瞎猜。
- **范围**：只求值独立的 `kind=condition` 节点。**不**求值 `kind=agent` 节点 `on_complete` 上挂的条件分支（`regression` 节点那种形状）——那是附着在一个已经在跑的 Agent 节点后面的另一种分支，还牵扯"跳过的分支要不要级联跳过它自己的下游"这类没有先例可循的设计判断，这版不猜，留给下一版专门设计。
- **触发点**：`run_task_node()` 成功跑完一个 `kind=agent` 节点之后调用一次——这版 `condition` 节点的唯一真实先例（`qa_gate`）依赖的正是一个 `kind=agent` 节点（`qa_test`）。

**`reject_gate` 下游级联失效**（`gate.rs` 新增 `cascade_downstream()`）：
- 用 `depends_on` 建一张正向邻接表（谁依赖谁），从 `on_reject.goto` 指名的节点出发 BFS，找出全部下游（不管间接多少层）一起重置回 `pending`。级联集合里如果包含别的 `kind=human` 评审节点，连它对应的 `gates` 行也一起重置——包括这次被打回的 Gate 自己对应的评审节点，只要它结构上依赖某个 goto 目标（比如打回 `prd_review` 时，`prd_review` 自己就依赖 `prd`，会被自己的级联扫回去，这是对的：一个曾经 `passed` 又被重新打回的 Gate 就该回到 `pending`）。
- **已知局限**：级联只重置 `status`/`started_at`/`finished_at`，不清空 `manual`/`auto-claim` 节点已经写好的 `assigned_agent_id`——重跑会沿用原来的认领/指派，不会变回"待认领"。

**测试**：4 条新集成测试——合成的最小 condition 模板（`check`→`gate`→`happy_path`/`fix_path`）分别验证 true 分支取消 `fix_path`、false 分支取消 `happy_path`、契约字段缺失时两个分支都不动；真实模板上验证打回一个已经 `passed`、下游已经真的跑完的 Gate，`prd`/`prd_review`/`tech_selection` 和 `requirement-gate` 都级联回 `pending`。workspace 累计 70 个测试全绿。

### ADR-005 附注 9 · `opc-privacy` + `opc-audit` 落地（P0.5）

补 `runtime/README.md` 规划里一直空着的两个 crate。

**`opc-privacy`**（纯逻辑、零依赖）：`is_allowed(agent_sensitivity, provider_allowed_sensitivity)` 一个函数——`AgentDefinition.sensitivity` 必须显式出现在候选 Provider 的 `allowed_sensitivity` 白名单里，这是 `AGENTS.md`"敏感数据自动脱敏或直接拒绝"的"直接拒绝"那一半。接入点在 `opc_agent::select_provider()`：被拦下的候选**压根不会被构建/调用**（不是"调用了但连不上"），如果全部候选都因为这条被拦，报独立的 `Error::PrivacyBlocked` 而不是笼统的 `NoProviderAvailable`——两种失败对用户的意义完全不同。**这一版明确不做**"自动脱敏"——把敏感内容从 Prompt 里洗掉再放行需要真正理解内容语义，没有明确规则的情况下不该臆造一套。

**`opc-audit`**：`record(db, LogEvent { kind, result, .. })`，一个 `INSERT` 进 `project.sqlite.execution_logs`（append-only）。`kind`/`result` 取值对齐 schema 注释里已经枚举过的常量，导出成 `opc_audit::kind::*`/`opc_audit::outcome::*` 避免调用方手敲字符串拼错。接入点：`opc-workflow::runner::run_task_node()` 每次 `opc_agent::run_task()`（`llm.call`，成功/失败/被隐私哨兵拦下分别记 `ok`/`error`/`blocked`）和每次 `write_and_register_artifact()`（`tool.file.write`）；`gate::approve_gate()`/`reject_gate()`（`review.decision`，`confirmed`/`denied`——这两个取值本来就在 schema 的 CHECK 约束里，不是这次新加的，只是第一次真的用上）。**这一版只接了 PROJECT 级** `execution_logs`；APP 级（provider 测试/项目创建这类全局操作）还没有调用方接进来。

**测试**：`opc-privacy` 4 个纯函数测试；`opc-audit` 2 个真实写库/读回测试；`opc-agent` 新增 1 条测试证明 `sensitivity=high` 的 Agent 面对只有云 Provider 的候选列表时，**从没真的往对方服务器发过请求**（用 wiremock 的 `received_requests()` 断言请求数为 0）就被拦下。

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

### ADR-007 附注 · 落地（P0.5）—— 桌面壳从单页手写 CSS 重做成 shadcn/Tailwind 多页应用

`apps/desktop` 早期几轮迭代（验证 IPC 链路阶段）图快，直接在空白 Vite+React 脚手架上写了一版手写 CSS 的单页——先把"后端真的通不通"跑明白，样式先放一边。这条路径实际上**偏离了本 ADR 的决定**，且直到用户追问"为什么跟原型不一致"才发现、被追认修正。补的东西：

- **Tailwind CSS 3.4 + PostCSS**：`tailwind.config.ts` 用 shadcn 惯例的 HSL CSS 变量做主题（`--background`/`--primary`/`--destructive`/… 在 `src/index.css` 定义），数值沿用 `legacy/prototype/index.html` 已验证过的配色/圆角/阴影 token（`--accent: #0071e3` 换算成 HSL 分量这类），复用的是审美和数值，不是抄代码
- **shadcn/ui 模式**（不是 npm 依赖，是"复制进仓库"的组件源码）：`src/components/ui/` 下手写了 `button.tsx`/`input.tsx`/`select.tsx`/`checkbox.tsx`/`label.tsx`/`card.tsx`/`badge.tsx`，基于 `@radix-ui/react-*` 无障碍原语 + `class-variance-authority` 变体 + `tailwind-merge`/`clsx` 的 `cn()` helper——这就是 shadcn 生成器本来会产出的那套东西，只是没跑 CLI（CLI 需要额外的注册表交互，手写等价物更可控）
- **深浅色三态改成 class 策略**：Tailwind 的 `darkMode:["class"]` 本身不认"跟随系统"，`theme.ts` 用 `matchMedia` 现解一遍 OS 偏好，"系统"选项额外监听 OS 主题变化——这是 `next-themes` 一类库的标准做法，不是发明新轮子
- **图标从手写 SVG 换成 `lucide-react`**：shadcn 生态的标配图标库，比手绘一遍更省、更全

**页面结构没变**：`Sidebar` + `views/*`（控制台/项目中心/团队中心/工作流中心/模型中心 + 通用占位页）这套信息架构和状态管理逻辑原样保留，只是把渲染层从手写 CSS class 换成 Tailwind utility + shadcn 组件——`cargo build --release` + Xvfb 截图重新验证过一遍（含 Radix Select 下拉、Radix Checkbox、深浅色切换、真实项目数据渲染），视觉和交互都对得上，见 `apps/desktop/README.md` § 设计系统。

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
