# 不令 OPC · 产品定义

> **不令 OPC** = 一个人 + 一台电脑 + 已有 AI 能力 = 一家公司的完整生产力
>
> 品牌沿用《论语》"其身正，不令而行" —— **你（CEO）出题，AI 团队各就其位不令而行**。

---

## 0. 一句话产品定义

**不令 OPC 是一款 Local-First 的私人 AI 公司桌面应用**：用户只需提出一个目标（例如"做一个健康管理 App"），OPC 就调用用户电脑上已有的 AI CLI / AI API / 本地模型，自动组织一支 AI 团队完成从需求、产品、技术、开发、测试到验收的完整项目交付。

- **中文名**：不令 OPC
- **英文名**：BuLing OPC · One Person Company
- **Slogan**：
  - 品牌层：**其身正，不令而行**
  - 产品层：**你出题，AI 团队开干**

---

## 1. 定位

### 1.1 它是什么

| 层 | 定位 |
|---|---|
| **不是** | 更好用的 ChatGPT / 传统 Multi-Agent SaaS / 云端协作平台 |
| **是** | **一个人的 AI 公司操作系统**——本地运行、私密可控、以项目交付为核心 |

### 1.2 与市面产品的区别

| 维度 | 传统 AI Chat | 云 Multi-Agent 平台 | **不令 OPC** |
|---|---|---|---|
| 部署 | 云 | 云 | **本地桌面 App** |
| 数据 | 上云 | 上云 | **不出机** |
| 计费 | 按 token | 平台订阅 + 模型 token | **零平台费**（复用你的 CLI/API 订阅）|
| 核心 | 聊天 | 工作流 | **项目交付** |
| 输入 | Prompt | Prompt / 表单 | **目标**（"做一个 App"）|
| 输出 | 文本 | 消息流 | **Artifact**（PRD / 代码 / 报告） |

---

## 2. 核心原则（硬约束，不打折）

1. **Local First**：默认本地项目 / 本地文件 / 本地 Git / 本地 AI CLI / 本地模型
2. **Zero Server**：核心功能不依赖不令官方云；下载即用
3. **Provider Agnostic**：不绑定任何 AI 厂商，用户已有的 CLI / API / 本地模型均可作为"AI 员工的大脑"
4. **Project Centric**：以项目为核心，不是以聊天为核心；聊天只是入口
5. **Artifact Driven**：Agent 之间通过标准化 Artifact 协作，不主要依赖聊天上下文传递信息
6. **Human in the Loop**：关键决策必须由用户显式确认（评审 / 高风险命令 / Git Push / 最终验收）
7. **评审红线**：质量把关与最终验收必须人工，永不自动化（沿用品牌基因）

---

## 3. 一级菜单

```
🏛  控制台          ← 每日入口 · 目标输入 · 项目总览
📁  项目中心         ← 所有项目工作区
👥  团队中心         ← AI 组织 · Agent 阵容
📋  任务中心
🧩  工作流中心
📦  产出中心         ← Artifact · 版本追溯
🔍  评审中心
──────────────
🧰  技能中心         ← Skill · Tool · MCP
🧠  记忆中心         ← Memory · Knowledge
🤖  模型中心         ← Provider · CLI · API · 本地模型
🔒  权限中心         ← 文件 / 命令 / 网络 / Git 授权
📊  日志中心         ← Execution Log · 可审计
⚙️  设置
```

---

## 4. AI 岗位（预置团队）

沿用中文岗位名，去掉 "Agent" 后缀，更符合中文语境：

| 岗位 | 职责 | 主要产出 |
|---|---|---|
| **产品经理** | 需求分析 / 用户画像 / 功能规划 / 验收标准 | `docs/product/PRD.md`, `Acceptance-Criteria.md` |
| **技术负责人** | 技术可行性 / 选型 / 约束 / 风险 | `docs/technical/Technology-Stack.md` |
| **架构师** | 系统架构 / 数据库 / API / 安全 / 部署 | `docs/technical/Architecture.md` |
| **项目经理** | 任务拆解 / WBS / 依赖 / 里程碑 / 风险 | `docs/project/Project-Plan.md` |
| **设计师** | UX / 页面结构 / 交互 / 视觉规范 | `docs/design/*.md` |
| **前端** | 前端开发 / 组件 / 单测 / 构建 | `frontend/**` |
| **后端** | 后端开发 / 数据库 / API / 单测 | `backend/**` |
| **测试** | 测试方案 / 用例 / 自动化 / 回归 / Bug | `docs/qa/*` |
| **验收** | Acceptance Criteria 检查 / 最终验收 | `docs/acceptance/Acceptance-Report.md` |

**系统文案示例**：
- "**产品经理** 正在整理需求…"
- "**测试** 发现 3 个 Bug，已推给 **后端**"
- "等你在 **评审中心** 通过技术方案"

---

## 5. Agent 定义（六件套）

每个 Agent 都是明确的"岗位"，包含以下六件：

```
Agent
├── Role              岗位名 + 职责
├── Skills            可用技能（组合复用单元）
├── Tools             可调工具（原子）
├── Memory            长期记忆（本机 SQLite）
├── Permissions       文件 / 命令 / 网络 / Git / Docker / MCP
└── Provider          绑定的 AI 能力来源（CLI / API / Local）
```

Agent 与 Provider **解耦**：换模型不改岗位。

---

## 6. Artifact 图（可追溯关系）

```
用户目标
    │
    ▼
   PRD ────► Acceptance Criteria
    │
    ▼
系统架构 ────► 数据库设计 · API 规范
    │
    ▼
项目计划 ────► 任务清单 · 里程碑
    │
    ▼
前端代码 · 后端代码
    │
    ▼
测试计划 · 用例 · 报告
    │
    ▼
Bug 报告 · 回归报告
    │
    ▼
验收报告
```

**每个 Artifact 都能反向追溯到用户最初的目标。**

---

## 7. Project Workflow（标准生命周期）

```
INIT → REQUIREMENT → REQUIREMENT_REVIEW → DESIGN → DESIGN_REVIEW
     → PLANNING → DEVELOPMENT → TESTING → BUG_FIX → REGRESSION
     → ACCEPTANCE → COMPLETED
```

**Gate（关键卡点）**：需求 Gate / 设计 Gate / 开发 Gate / QA Gate / 验收 Gate。任何 Gate 失败 → 返回责任 Agent 修改 Artifact → 重新 Review。

---

## 8. Provider 抽象（能力路由）

统一接口，四种来源：

| 类型 | 举例 | 隐私 | 用户成本 |
|---|---|---|---|
| **CLI Provider** | Claude Code / Codex / Gemini CLI / Aider | 走用户已有登录态 → 厂商（用户已有 ToS） | 零（用户已订阅） |
| **Local Provider** | Ollama / LM Studio | 不出机 | 零 |
| **API Provider** | OpenAI / Anthropic / GLM / Qwen / DeepSeek | 用户明示外发 | 用户按量 |
| **Hybrid** | 上面几种组合，按 Agent 策略路由 + Fallback | 按 Agent 敏感度设定 | 混合 |

**Agent 绑定 Provider 时的能力路由**：

```
Agent
  ↓
Provider Router  ←  优先级 [CLI / Local / API] · 敏感度过滤
  ↓
{ Claude Code / Codex / Ollama / OpenAI API / … }
```

某个 Provider 不可用（未安装 / 未登录 / 超时）自动 fallback 下一个。

---

## 9. 首次启动"一分钟组阁"

```
第一次打开不令 OPC
        │
        ▼
本地网关扫描 → "在你机器上发现："
   ✓ Claude Code   (登录态)
   ✓ Codex CLI     (登录态)
   ○ Gemini CLI    (未登录，需 `gemini auth`)
   ✓ Ollama        (运行中，qwen2.5 / llama3 已下载)
        │
        ▼
"根据你装了什么，为你推荐 9 位 AI 员工的默认引擎："
   产品经理   → Claude Code
   技术负责人 → Claude Code
   架构师     → Claude Code
   项目经理   → Codex CLI
   设计师     → Claude Code
   前端       → Claude Code
   后端       → Codex CLI
   测试       → Codex CLI
   验收       → 本地 Qwen        (涉及验收敏感度高)
        │
        ▼
   [全部接受]  [逐个自定义]
        │
        ▼
1 分钟内一整套 AI 团队上线，零额外账单
```

---

## 10. 项目工作区（本地目录结构）

```
~/Projects/health-app/
│
├── .opc/                          ← OPC 元数据（AI 团队看得懂的形状）
│   ├── project.json               项目配置
│   ├── agents/                    本项目实例化的 Agent
│   ├── tasks/                     任务清单
│   ├── workflows/                 工作流实例 · 状态机
│   ├── artifacts/                 产物索引 + 版本
│   ├── memory/                    项目级记忆
│   ├── reviews/                   评审记录 · Gate 结果
│   ├── evidence/                  执行证据（外发拦截日志 · 命令日志）
│   └── logs/                      Execution Log
│
├── docs/
│   ├── product/                   PRD · User Stories · AC
│   ├── technical/                 架构 · DB · API · 安全 · 部署
│   ├── project/                   排期 · 任务 · 里程碑 · 风险
│   ├── design/                    UX · 交互 · 视觉规范
│   ├── qa/                        测试计划 · 用例 · 报告 · Bug
│   └── acceptance/                验收报告
│
├── frontend/                      前端产出
├── backend/                       后端产出
└── .git/                          Git 强制启用 · 关键阶段自动 commit
```

---

## 11. 权限系统（Agent 可以操作用户电脑，权限是核心）

**六大类粒度**：

```
Agent Permission
├── File        当前项目目录 / Home / 其他绝对路径
├── Command     npm · git · docker · sudo · shell 白名单
├── Network     AI API only · 特定 host · 全网
├── Git         status · diff · commit · push
├── Docker      run · build · push · exec
└── MCP         指定 MCP Server 白名单
```

**高风险操作必须用户显式确认**（模态弹窗）：

```
┌──────────────────────────────────┐
│  ⚠️  后端 请求执行                │
│                                  │
│  git push origin main            │
│                                  │
│  [拒绝]  [允许一次]  [始终允许]   │
└──────────────────────────────────┘
```

**四条安全原则**：最小权限 → 显式授权 → 操作可审计 → 操作可回滚。

---

## 12. 单机存储

**P0 硬约束：不引入 PostgreSQL / Redis / Kafka / K8s**。

```
数据面：
  SQLite                业务数据（Project / Agent / Task / TaskRun / Review / Provider / Settings / Memory / Usage）
  SQLite FTS5           全文检索
  Local File System     源代码 / Markdown / 图片 / 报告
  Git                   代码 / 文档版本
  OS Keychain           API Key（不落 SQLite）
                        · macOS Keychain
                        · Windows Credential Manager
                        · Linux Secret Service

后续（P1+）：
  sqlite-vec            语义检索
  Docker                容器化工具（可选）
```

---

## 13. 推荐技术架构

```
                    ┌───────────────────┐
                    │  OPC Desktop UI   │
                    │ React + shadcn/ui │
                    │ Monaco + ReactFlow│
                    └────────┬──────────┘
                             │  Tauri IPC
                    ┌────────┴──────────┐
                    │   OPC Runtime     │
                    │       Rust        │
                    └────────┬──────────┘
                             │
       ┌─────────────────────┼─────────────────────┐
       ▼                     ▼                     ▼
  Agent Runtime         Project Runtime       Tool Runtime
       │                     │                     │
       │                Artifact/Files        Git / Docker
       │                Memory / Reviews      Terminal / MCP
       │
       ▼
 Provider Router
       │
   ┌───┼───────────────────────────────┐
   ▼   ▼        ▼            ▼        ▼
Claude Codex  Gemini      Ollama    API
 Code  CLI     CLI         Local  Providers
```

---

## 14. 技术栈选型

| 模块 | 技术 |
|---|---|
| Desktop 壳 | **Tauri 2**（包小、冷启快、Rust 后端） |
| 前端 | React + TypeScript |
| UI 库 | shadcn/ui（配 Tailwind） |
| 图 / 流程 | React Flow |
| 编辑器 | Monaco Editor |
| Runtime | Rust（Tauri 后端） |
| 本地库 | SQLite + FTS5 |
| 存储 | Local File System |
| 版控 | Git（深度集成） |
| Key 存储 | OS Keychain / Credential Manager / Secret Service |
| AI 协议 | MCP |
| AI CLI | Claude Code / Codex / Gemini CLI（本地网关发现） |
| AI API | OpenAI / Anthropic / GLM / Qwen / DeepSeek |
| 本地模型 | Ollama / LM Studio |
| 可观测 | Local Execution Log |

---

## 15. 目录结构（新仓库）

```
opc/
│
├── apps/
│   └── desktop/               Tauri App
│       ├── src/               前端（React）
│       └── src-tauri/         后端（Rust）
│
├── runtime/                   Rust 核心 runtime（可独立编译）
│   ├── agent/
│   ├── project/
│   ├── task/
│   ├── workflow/
│   ├── provider/
│   ├── tool/
│   ├── mcp/
│   ├── git/
│   ├── filesystem/
│   ├── terminal/
│   └── security/
│
├── providers/                 Provider 适配器（每家 CLI/API 一个）
│   ├── claude-code/
│   ├── codex/
│   ├── gemini/
│   ├── openai/
│   ├── anthropic/
│   ├── glm/
│   ├── qwen/
│   └── deepseek/
│
├── skills/                    可复用 Skill 定义
├── agents/                    预置 9 位 Agent 定义
├── templates/                 项目模板（标准软件交付 / 内容创作 / 数据分析）
├── packages/
│   ├── cli-registry/          ✅ 沿用旧仓库（CLI 签名注册表）
│   └── local-gateway/         ⚠️ 从旧 `apps/local-gateway/` 迁入 · 升级为 OPC Runtime 的 CLI 层
├── docs/
│   ├── OPC-产品定义.md         （本文）
│   ├── 本地网关.md
│   └── ...
├── legacy/                    旧「不令 · 企业级协作平台」骨架 · 参考不构建
└── README.md
```

---

## 16. 核心数据模型（SQLite）

```
User
 └── Workspace
       └── Project
             ├── Team
             │    └── Agent
             │          ├── Role · Skill · Memory · Tool · Permission
             │          └── Provider (CLI / API / Local)
             ├── Task
             │    └── TaskRun
             ├── Artifact
             │    └── ArtifactVersion
             ├── Review · Gate
             ├── Workflow
             ├── Memory · Knowledge · Evidence
             └── ExecutionLog
```

---

## 17. MVP（P0）功能清单

**目标：一个用户，一个项目，端到端跑通"目标 → AI 团队 → 交付"。**

```
✓ Desktop App (Tauri 壳)
✓ Project 创建 + 工作区初始化 + .opc/ 目录
✓ Workspace 目录选择
✓ 9 位预置 Agent（产品 / 技术 / 架构 / 项目 / 设计 / 前端 / 后端 / 测试 / 验收）
✓ Team 组装向导（自动推荐 Provider 绑定 + 用户可换）
✓ Task 拆解 + 分配
✓ Artifact 产出 + 版本
✓ Workflow 标准生命周期 + Gate
✓ Review 面板（人工评审红线）
✓ Acceptance 报告
✓ Local Runtime：File / Terminal / Git
✓ Claude Code Adapter（走本地网关）
✓ Codex Adapter
✓ OpenAI API Adapter
✓ Ollama Adapter（本地模型）
✓ SQLite 存储
✓ 权限系统（六大类 + 显式确认模态）
✓ Execution Log
✓ API Key 走 OS Keychain
```

---

## 18. P1 / P2 / P3 展望

**P1（丰富能力）**：
- Gemini CLI / GLM / Qwen / DeepSeek Adapters
- MCP 深度集成
- Skill 复用库
- Memory / Knowledge
- Browser Tool / Docker Tool / GitHub Tool

**P2（生态）**：
- 多项目管理
- Agent Templates / Team Templates / Workflow Templates
- Skill Marketplace / Agent Marketplace
- 云同步（E2E 加密可选）
- 语音 / Computer Use
- 移动端配套（只看 · 只审阅 · 不做重活）

**P3（升维）**：
- Autonomous Project Execution
- 跨项目记忆
- 智能模型路由 + AI Cost Optimization
- AI Employee Marketplace
- AI Company Templates（行业级模板池）

---

## 19. 从旧仓库继承什么

**能保留的硬资产**（不重造）：

| 旧代码 | 处置 | 用于 |
|---|---|---|
| `packages/cli-registry/` | ✅ 直接沿用 | CLI 签名注册表（Provider Adapter 用） |
| `apps/local-gateway/` | ⚠️ 迁入新仓库 `packages/local-gateway/` · 升级 | 从"只发现 CLI"升级为"OPC Runtime 的 CLI 执行层" |
| Provider AES-256-GCM Key 加密逻辑 | 🟡 借鉴迁移 | `ApiProvider` 场景仍需（OS Keychain 是主，加密文件是兜底） |
| 评审红线三层强约束（Guard + 状态机 + 审计） | ✅ 迁移到 Rust Runtime | 核心品质承诺 |
| CLI probe 白名单 `shellQuote` | ✅ 沿用 | 安全底座 |
| `docs/本地网关.md` | ✅ 沿用 · 微调 | 本地网关协议依然有效 |

**归档到 `legacy/`（不构建、不删除、纯参考）**：

- `legacy/apps/api/`（NestJS + Prisma 骨架 · 旧「企业级协作平台」）
- `legacy/apps/web/`（Next.js · 旧协作平台前端）
- `legacy/infra/`（docker-compose · Postgres/Redis）
- `legacy/prototype/`（原型 HTML · 旧协作平台设计语言）
- `legacy/docs/`（旧产品原型说明 / 协作机制设计 / 企业级架构与落地方案 / 骨架启动指南 / 项目开发须知）

---

## 20. 待你拍板的三个关键选择

在 P0 动手前请拍板：

1. **桌面框架**：Tauri 2（推荐，Rust 后端 + React 前端，包小 3-5 MB）vs Electron（Node 后端可复用现有 TS 代码，包 100 MB+）
2. **本地库首选**：SQLite 直连（推荐）vs 走 ORM（Diesel / SeaORM）—— 影响后续迁移成本
3. **业务实体字段口径**：Project / Agent / Task / Artifact / Review 的字段草图是否要在 P1 前**先写出来给你审**（推荐要）

---

## 附录 · 品牌语料库（供文案 / 官网 / 界面文案复用）

**品牌一句话（对外）**：
> 不令 OPC · 一个人的 AI 公司操作系统。

**产品价值主张（对内）**：
> 你只做只有你能做的判断，其他事让 AI 团队不令而行。

**Slogan 池**：
- 其身正，不令而行。（品牌层）
- 你出题，AI 团队开干。（产品层）
- 一个人，一支团队，一家公司。
- 本地跑，私密守，产出准。

**产品叫法（不同场景对应）**：
- 正式：`不令 OPC`（宣传物料 / 官网 / 应用商店）
- 简称：`OPC`（用户日常交流）
- 英文：`BuLing OPC` / `OPC by BuLing`
