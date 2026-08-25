# 不令 OPC · BuLing · One Person Company

> 其身正，不令而行。
>
> **你出题，AI 团队开干。**

**不令 OPC** 是一款 Local-First 的私人 AI 公司桌面应用。用户只需提出一个目标（例如"帮我做一个健康管理 App"），OPC 就调用用户电脑上**已有的** AI CLI（Claude Code / Codex / Gemini CLI …）/ AI API（OpenAI / Anthropic / GLM / Qwen / DeepSeek）/ 本地模型（Ollama / LM Studio），自动组织一支 AI 团队完成从**需求 → 产品 → 技术 → 开发 → 测试 → 验收**的完整项目交付。

品牌沿用《论语·子路》"其身正，不令而行"——**你（CEO）出题，AI 团队各就其位不令而行**。产品线定位 **OPC = One Person Company**：**一个人 + 一台电脑 + 已有 AI 能力 = 一家公司的完整生产力**。

---

## 核心主张

| 原则 | 一句话 |
|---|---|
| **Local First** | 默认本地项目 / 本地文件 / 本地 Git / 本地 CLI / 本地模型；数据默认不出机 |
| **Zero Server** | 核心功能不依赖不令官方云；下载即用 |
| **Provider Agnostic** | 不绑定任何 AI 厂商；复用你已有的 CLI / API / 本地模型订阅 |
| **Project Centric** | 以项目为核心，不是以聊天为核心 |
| **Artifact Driven** | Agent 之间通过标准化 Artifact 协作（PRD / 架构 / 代码 / 测试报告 / 验收报告）|
| **Human in the Loop** | 关键决策（需求评审 / 技术评审 / 高风险命令 / 最终验收）必须人工显式确认 |
| **评审红线** | 质量把关与验收永不自动化——**品牌基因，硬约束** |

---

## AI 团队（9 位预置岗位）

**产品经理** · **技术负责人** · **架构师** · **项目经理** · **设计师** · **前端** · **后端** · **测试** · **验收**

用户首次启动时，OPC 扫描本机已装 AI CLI，一分钟内自动完成"9 位员工 × 合适引擎"的组阁，零额外账单。

---

## 一级菜单

```
🏛  控制台          每日入口 · 目标输入 · 项目总览
📁  项目中心         所有项目工作区
👥  团队中心         AI 组织 · Agent 阵容
📋  任务中心
🧩  工作流中心
📦  产出中心         Artifact · 版本追溯
🔍  评审中心
──────────────
🧰  技能中心         Skill · Tool · MCP
🧠  记忆中心         Memory · Knowledge
🤖  模型中心         Provider · CLI · API · 本地模型
🔒  权限中心         文件 / 命令 / 网络 / Git 授权
📊  日志中心         Execution Log · 可审计
⚙️  设置
```

---

## 目录结构（当前状态）

```
.
├── README.md
├── AGENTS.md                        仓库开发规则
├── CLAUDE.md                        分支政策 · design-first · 硬约束
├── docs/
│   ├── OPC-产品定义.md              ★ 产品全景 · 一分钟组阁 · Runtime 架构 · MVP 清单
│   ├── OPC-架构决策.md              ADR：Tauri 2 / sqlx / 双层 SQLite / Keychain / 三种 Provider
│   ├── OPC-数据模型.md              双层 SQLite schema 全景 · 实体关系 · 关键字段决策
│   └── 本地网关.md                  local-gateway 协议 / 安全底线
│
├── runtime/                         ★ Rust · Tauri 后端 · 核心执行引擎
│   ├── Cargo.toml                   workspace 根
│   ├── README.md                    workspace 布局 + P0 落地顺序
│   ├── migrations/                  SQLite migration（双层）
│   │   ├── README.md
│   │   ├── app/0001_init.sql        APP 库：user/settings/providers/agents/projects/logs
│   │   ├── app/0002_provider_wire_format.sql   加 providers.wire_format 列
│   │   └── project/0001_init.sql    PROJECT 库：teams/tasks/artifacts/reviews/workflows/memory+FTS5
│   └── crates/
│       ├── opc-storage/             ✅ sqlx + SQLite · migrator · AppDb + ProjectDb（3 测试）
│       ├── opc-provider/            ✅ Provider trait · CLI/API/Local 抽象 · 11 家厂商（11 测试）
│       │                            见 providers/README.md
│       ├── opc-agent/               ✅ 加载 agents/*.yaml · 播种 app.sqlite · 驱动一次 Provider 执行（6 测试）
│       ├── opc-tool/                ✅ 项目内文件写入（沙箱化）+ Artifact 登记（8 测试）
│       ├── opc-project/             ✅ 创建/打开/列出项目 · 把预置 Agent 实例化进团队（8 测试）
│       ├── opc-workflow/            ✅ 加载模板 · 实例化 DAG · 驱动 Agent 节点 · 人工 Gate（7 测试）
│       └── opc-task/                ✅ manual/auto-claim 节点指派 · 能力匹配认领分（10 测试）
│
├── providers/                       AI 能力源 manifest（CLI / API / Local · 11 家）
│   ├── README.md                    manifest 格式 · Provider trait · 加厂商步骤
│   ├── claude-code/ · codex/ · gemini/ · aider/         （CLI）
│   ├── anthropic/ · openai/ · glm/ · qwen/ · deepseek/  （API）
│   └── ollama/ · lm-studio/                             （Local）
│
├── agents/                          9 位预置 AI 岗位模板
│   ├── README.md
│   ├── product-manager.yaml         产品经理
│   ├── tech-lead.yaml               技术负责人
│   ├── architect.yaml               架构师
│   ├── project-manager.yaml         项目经理
│   ├── designer.yaml                设计师
│   ├── frontend.yaml                前端
│   ├── backend.yaml                 后端
│   ├── qa.yaml                      测试
│   └── acceptance.yaml              验收（敏感度 high · 默认本地 Provider）
│
├── templates/                       工作流模板 + Artifact 骨架
│   ├── README.md
│   ├── standard-software-delivery.yaml   标准软件交付流（Goal → PRD → 架构 → 开发 → 测试 → 验收）
│   └── artifacts/                   24 份 Markdown 骨架（PRD / 架构 / 测试报告 / 验收报告 …）
│       ├── README.md
│       ├── common/Report.md
│       ├── product/ · technical/ · project/ · design/ · qa/ · acceptance/
│
├── apps/
│   ├── desktop/                     ✅ Tauri 2 桌面壳（骨架已落地 · 可 cargo build）
│   │   ├── package.json             Vite + React + @tauri-apps/api
│   │   ├── src/                     React 前端（项目中心 + 存储层状态 + 预置岗位 + Provider 发现列表）
│   │   └── src-tauri/               Rust 后端（薄壳 · 引用 opc-storage + opc-provider + opc-agent + opc-project + opc-workflow + opc-task）
│   │                                IPC: opc_status · opc_providers · opc_agents · opc_create_project · opc_list_projects
│   │                                     · opc_workflow_tasks · opc_workflow_ready_tasks · opc_workflow_run_task
│   │                                     · opc_workflow_gates · opc_workflow_approve_gate · opc_workflow_reject_gate
│   │                                     · opc_task_claimable_tasks · opc_task_manual_tasks · opc_task_claim
│   │                                     · opc_task_assign_manually · opc_task_run
│   └── local-gateway/               本机守护进程 · 127.0.0.1 + Token · 短期沿用
│                                    P0 后期融入 runtime/crates/opc-provider
│
├── packages/
│   └── cli-registry/                AI 厂商 CLI 签名注册表（10 家）
│
└── legacy/                          旧「不令 · 企业级协作平台」骨架 · 归档参考 · 不构建
    ├── README.md                    禁引规则 + 可借鉴资产清单
    ├── apps/api/                    NestJS + Prisma（AES 密钥加密 · 评审红线状态机可借鉴）
    ├── apps/web/                    Next.js 14
    ├── infra/                       docker-compose (Postgres + Redis)
    ├── prototype/                   原型 HTML · 视觉设计语言可借鉴
    └── docs/                        产品原型说明 · 协作机制设计 · 企业级架构与落地方案 · …
```

---

## 快速开始

P0 已跑通：**Rust runtime + Tauri 桌面壳**能编译，SQLite 存储层集成测试全绿。

```bash
# 一次性装依赖
pnpm install

# —— Runtime（Rust · SQLite 存储） ——
make runtime-check        # cargo check
make runtime-test         # 53 个测试：opc-storage 3 个（app/project db + FTS5）
                          #           + opc-provider 11 个（CLI adapter 单测 + wire format 集成测试）
                          #           + opc-agent 6 个（YAML 加载 + 播种幂等性 + Provider fallback）
                          #           + opc-tool 8 个（沙箱路径校验 + Artifact 版本化 + 端到端胶水）
                          #           + opc-project 8 个（创建/打开/列出项目 + Agent 实例化 + 端到端胶水）
                          #           + opc-workflow 7 个（模板加载 + DAG 实例化 + 驱动执行 + Gate 通过/打回）
                          #           + opc-task 10 个（能力匹配认领打分 + 手动指派 + 驱动已指派节点执行）

# —— 桌面 App（Tauri 2） ——
make desktop-check        # 无窗口 · 前端 typecheck+build + Rust cargo check
make desktop-dev          # 起真实窗口（需要图形环境 · macOS/Linux+X11/Wayland）
make desktop-build        # 打包生产版

# —— 本地 AI CLI 扫描 daemon（沿用旧仓库形态，未来融入 Runtime） ——
make gateway              # http://127.0.0.1:17817
```

详见：
- [`docs/OPC-产品定义.md`](docs/OPC-产品定义.md) —— 产品全景
- [`docs/OPC-架构决策.md`](docs/OPC-架构决策.md) —— ADR（Tauri / sqlx / 双层 SQLite / Keychain / Provider 抽象）
- [`docs/OPC-数据模型.md`](docs/OPC-数据模型.md) —— 双层 SQLite schema 全景
- [`docs/本地网关.md`](docs/本地网关.md) —— local-gateway 协议

---

## 使用教程

> **当前进度提醒**：Runtime 已跑通 Storage / Provider / Agent / Tool / Project / Workflow / Task 七层，桌面壳能真的**创建项目、把 9 位预置岗位实例化进项目团队、发现本机可用的 AI 引擎、跑通"选模板 → Agent 接力产出 → 评审红线通过/打回 → 认领/指派下一阶段"这条链的前半段**。止步的地方是诚实的边界：前后端开发这几个节点需要"一次 Agent 产出一整个目录的多份源码文件"，这是比现有 Artifact 模型大得多的另一件事，还没做——下面教程会讲到具体停在哪。

### 1. 准备环境

```bash
# 需要：Node.js 18+ / pnpm / Rust stable 工具链（cargo）
pnpm install
```

想让「模型中心」发现真实可用的 AI 引擎，任选其一（都不装也能跑，只是 Provider 全部显示"不可用"）：

- **本机已装 AI CLI**：Claude Code（`claude`）等会被自动发现，无需任何配置
- **API 厂商 Key**：设置对应环境变量（Key 只在内存里用一次，正式版会走 OS Keychain 而不是环境变量），例如：
  ```bash
  export OPC_KEY_ANTHROPIC=sk-...   # providers/anthropic
  export OPC_KEY_OPENAI=sk-...      # providers/openai
  export OPC_KEY_GLM=...            # providers/glm
  export OPC_KEY_QWEN=...           # providers/qwen
  export OPC_KEY_DEEPSEEK=...       # providers/deepseek
  ```
- **本地模型**：装好 Ollama / LM Studio 并保持在默认端口运行即可，无需 Key

### 2. 起桌面 App

```bash
make desktop-dev   # 需要图形环境（macOS / Linux X11·Wayland）；无图形环境用 make desktop-check 验证能编译
```

首次启动会自动：建 `~/.opc/db.sqlite`（APP 级库）→ 跑 migration → 把 `agents/*.yaml` 的 9 位预置岗位播种进库。

### 3. 建第一个项目

窗口顶部「项目中心」卡片：
1. **slug**：项目短标识，如 `health-app`（同一 slug 不能重复建）
2. **项目名**：如 `健康管理 App`
3. **本地目录**：一个本地绝对路径，如 `/Users/you/opc-projects/health-app`（目录不存在会自动创建）
4. **目标**（可选）：一句话描述，如 `做一个健康管理 App`
5. 勾选「同时启动『标准软件交付流』工作流」（默认勾选）
6. 点「创建项目」

创建成功后可以在这个目录下看到：
```
health-app/
└── .opc/
    └── project.sqlite     ← 项目级 SQLite：已经有一条「默认团队」+ 9 位 Agent 实例
                              + 一条正在跑的工作流（16 个任务节点 + 4 个 Gate）
```
列表会立刻刷新，显示这个新项目（按最近打开排序）；如果勾了工作流，页面上会多出一块「工作流中心」卡片。

### 4. 跑工作流第一步 · 体验评审红线

「工作流中心」卡片分两块：

- **任务节点**：16 个节点（PRD → 技术方案 → 架构 → 排期/设计 → 前后端开发 → 测试 → 验收），当前能跑的节点（依赖已满足 · 岗位已就绪）旁边会出现「跑这个节点」按钮。项目刚建好时只有 `prd` 是可跑的——点一下，产品经理岗位会真的调一次 AI 引擎，产出 PRD/验收标准/用户故事三份文档，直接写进项目目录并在任务列表里标记为 `completed`。
- **评审红线（人工 Gate）**：`prd` 跑完后，下一个节点 `prd_review` 是人工评审节点——**不会自动放行**，「评审红线」卡片里会出现「通过」/「打回」两个按钮。点「通过」，`tech_selection`（技术负责人）才会出现在任务节点里变成可跑；点「打回」，`prd` 会被重置回待办，重新出现「跑这个节点」按钮。

这就是"评审红线永不自动化"这条硬约束在界面上的样子：AI 团队能一路把活干到评审节点前，但过 Gate 这一步永远要你自己点。

### 5. 前后端开发节点 · 认领 / 指派能跑通，执行会诚实卡住

一路「通过」到「排期」/「设计」阶段完成后，「前后端开发」（`frontend_dev`/`backend_dev`）会出现「认领（能力匹配）」按钮——点一下，Runtime 真的会按能力标签重合度给团队每个成员打分，把任务指派给分最高的（这里几乎总是前端/后端本人）。指派型节点（如后面 QA 报告分类后的「Bug 修复」）同理会出现岗位下拉框 + 「指派」按钮，机制相同，只是这一版还走不到那一步（见下）。

**认领/指派机制本身是真的**，但指派完之后点「跑这个节点」会报错——因为这几个节点的产出是"整个 `frontend/`/`backend/` 目录的源码"，不是一份 Markdown。现有的 Artifact 模型只知道怎么落一份**具名单文件**，"一次 AI 调用产出一整个目录的多份文件"是完全不同的另一件事，这一版没做。工作流会诚实地停在这里，不会静默产出一个叫 `frontend/**` 的假文件——见 [`runtime/README.md` § P0 落地顺序](runtime/README.md#p0-落地顺序)。

### 6. 命令行验证（不想开图形界面时）

```bash
make runtime-test   # 跑全部 53 个 Rust 集成/单元测试，验证 Storage/Provider/Agent/Tool/Project/Workflow/Task 七层逻辑
make desktop-check  # 无窗口验证前端 + Rust 后端都能编译
```

---

## 从哪读起

- **产品全景**：本 README → [`docs/OPC-产品定义.md`](docs/OPC-产品定义.md)
- **开发接手**：[`CLAUDE.md`](CLAUDE.md)（硬约束 · 命名 · 分支政策）→ [`AGENTS.md`](AGENTS.md)（详细规则）
- **本地网关**：[`docs/本地网关.md`](docs/本地网关.md)
- **旧协作平台参考**：`legacy/docs/*`（不再是权威源，只作历史文档留存）

---

## 命名由来

**不令 = 主品牌**。"不令"化用《论语·子路》"其身正，不令而行"——不需要发号施令，事情照样成了。产品哲学：CEO 自身立得住，AI 团队自然会围绕目标动起来。

**OPC = 产品线（One Person Company）**。表达对用户的承诺：让**一个人** 就能拥有一家公司的完整生产力。

**完整名** `不令 OPC` 兼具品牌哲学与产品定位——一句话讲清楚"我是谁 · 我要解决什么"。
