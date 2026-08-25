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
│   ├── OPC-产品定义.md              ★ 产品全景 · 数据模型 · Runtime 架构 · MVP 清单
│   └── 本地网关.md                  local-gateway 协议 / 安全底线
│
├── apps/
│   └── local-gateway/               本机守护进程 · 只监听 127.0.0.1 · Token 鉴权
│                                    P0 之后会升级为 OPC Runtime 的 CLI 执行层
│
├── packages/
│   └── cli-registry/                AI 厂商 CLI 签名注册表（Claude Code / Codex / Gemini …）
│
└── legacy/                          旧「不令 · 企业级协作平台」骨架 · 归档参考 · 不构建
    ├── apps/api/                    NestJS + Prisma（AES 密钥加密 · 评审红线状态机可借鉴）
    ├── apps/web/                    Next.js 14
    ├── infra/                       docker-compose (Postgres + Redis)
    ├── prototype/                   原型 HTML · 视觉设计语言可借鉴
    └── docs/                        产品原型说明 · 协作机制设计 · 企业级架构与落地方案 · …
```

---

## 快速开始

**当前阶段仓库处在 pivot 后重启期**——旧「企业级协作平台」骨架已归档到 `legacy/`；OPC 新架构（Tauri 2 + React + Rust Runtime + SQLite）正在筹备 P0。

现存能跑的只有 `apps/local-gateway`（本机 AI CLI 扫描），它会作为 OPC Runtime 的 CLI 执行层继续沿用：

```bash
pnpm install
pnpm --filter @buling/local-gateway start   # 127.0.0.1:17817
```

详见 [`docs/本地网关.md`](docs/本地网关.md)。

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
