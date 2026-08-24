# 不令 · BuLing

> 其身正，不令而行。

**不令** 是一个去中心化的多智能体（Multi-Agent）协作平台。名字化用《论语·子路》"其身正，不令而行"——不需要发号施令，事情照样成了。它精准对应本平台的核心主张：平台不做中心化的任务分发者，Agent 靠技能契约自主认领、靠协作痕迹彼此感知、靠临时聚阵完成复杂任务，只有质量评审这一环保留人工兜底。

仓库同时承载两个阶段的交付物：

1. **产品原型**：`prototype/index.html` —— 高保真、可交互、单文件零依赖，用于产品/内部评审
2. **企业级骨架（R1）**：`apps/api` + `apps/web` + `packages/shared` + `prisma` + `infra` —— NestJS + Next.js + PostgreSQL + Redis，已落地评审红线的三层强约束、意愿分算法、Provider 密钥加密。详见 [`docs/企业级架构与落地方案.md`](docs/企业级架构与落地方案.md) 与 [`docs/骨架启动指南.md`](docs/骨架启动指南.md)

---

## 这是什么

一个可自定义 Agent 身份、技能、可挂载 Skill 模块的协作平台。核心能力分三层：

| 层 | 机制 | 一句话 |
|---|---|---|
| 发现与认领 | **自主认领机制** | 任务按技能契约匹配，多个 Agent 同屏展示"认领意愿"，谁命中谁领，平台不指派 |
| 执行中协同 | **临时协同机制** | 领队随当前推进节点流转（临时领队），卡壳可"换人接棒"，瓶颈处临时组"攻坚小组"，事后自动解散 |
| 系统底层 | **协作痕迹机制** | 匹配权重按历史完成痕迹自我强化 / 久未接触自然衰减；反复被打回的 Agent 临时降权（非永久拉黑），产出回升后自动恢复 |

**唯一红线**：质量把关与评审必须由人工显式打回 / 批准，永不自动化。这条贯穿三轮设计讨论，是不可被任何自动化逻辑接管的不变量。

> 三层机制在内部设计推演时借助了蚁群 / 蜂群 / 狼群三种自然协作模型做类比，但**产品与界面里只保留功能性命名**，不对外暴露生物学比喻。设计取舍的完整矩阵见 [`docs/协作机制设计.md`](docs/协作机制设计.md)。

---

## 原型覆盖的页面

| 页面 | 作用 |
|---|---|
| 工作台概览 | 状态摘要（统计卡 / 最近动态 / Agent 状态）；点进具体任务原地展开为**协作详情**（规划 / 时间线 / 产出 / 评审 / 团队同屏） |
| Agent 中心 | Agent 网格；"创建新 Agent"卡片置顶；五步创建 / 编辑向导（身份 → 人格与模型 → 技能与模块 → 协作与权限 → 确认） |
| 任务 | 任务看板（待认领 / 规划中 / 执行中 / 待评审 / 已完成）；待认领卡显示紧急度累积条与认领意愿 |
| 发布任务 | 独立表单页（侧边栏直达）：标题 / 描述 / 所需技能 / 优先级 / 截止 / 分配方式 |
| 模型接入 | 配置 AI API 提供方（Anthropic / OpenAI / OpenAI 兼容）：Base URL / API Key（脱敏）/ 测试连接 / 模型发现；已连接服务的模型即时成为 Agent 可选的"底层模型" |
| 文档产出库 | Agent 产出物汇总，按项目分组或按类型筛选，可预览、显示被引用次数 |

产品原型的完整规格（数据模型、交互、哪些是真交互 / 哪些是 mock）见 [`docs/产品原型说明.md`](docs/产品原型说明.md)。

---

## 快速开始

### 方式 A：只看原型（0 依赖）

```bash
# 直接浏览器打开
open prototype/index.html            # macOS
xdg-open prototype/index.html        # Linux
# 或起个静态服务器
cd prototype && python3 -m http.server 8080  # http://localhost:8080
```

### 方式 B：跑真实后端骨架（Node 22 + Docker）

```bash
cp .env.example .env
make install
make db                              # 起 Postgres + Redis
make generate && make migrate        # Prisma
pnpm --filter @buling/api prisma:seed
make dev                             # API :13500 + Web :13000
make gateway                         # 另开一个终端：本地网关（Mac CLI 扫描）
```

详细步骤与 E2E 演示见 [`docs/骨架启动指南.md`](docs/骨架启动指南.md)；本地网关的安全底线与协议见 [`docs/本地网关.md`](docs/本地网关.md)。

---

## 目录结构

```
.
├── README.md
├── AGENTS.md                       仓库开发规则（权威源）
├── CLAUDE.md                       分支政策 · design-first flow · 文档地图（精简指针）
├── Makefile
├── package.json / pnpm-workspace.yaml / tsconfig.base.json / .env.example
├── prototype/
│   └── index.html                  高保真可交互原型（单文件、零依赖）
├── apps/
│   ├── api/                        NestJS + Fastify + Prisma（R1 骨架）
│   ├── web/                        Next.js 14 App Router（R1 E2E 演示）
│   └── local-gateway/              用户本机守护进程；127.0.0.1 + X-Gateway-Token
│                                   `make gateway` 起，供「本机发现」扫描 AI 厂商 CLI
├── packages/
│   ├── shared/                     前后端契约唯一真源（DTO / 枚举）
│   └── cli-registry/               AI 厂商 CLI 签名注册表（Claude Code / Codex / Gemini / Ollama …）
├── infra/
│   └── docker-compose.yml          Postgres + Redis
└── docs/
    ├── 项目开发须知.md              新 Agent 主入口（~15 分钟统一上手）
    ├── 产品原型说明.md              原型规格
    ├── 协作机制设计.md              三层机制矩阵 · 评审红线 · 设计取舍
    ├── 企业级架构与落地方案.md      技术选型 / 架构图 / 模块拆分 / 数据模型 / API / R1-R3 路线图
    ├── 骨架启动指南.md              R1 骨架三分钟跑起来
    └── 本地网关.md                  local-gateway 协议 / 安全底线 / 集成契约
```

---

## 文档从哪读起

- **产品 / 评审视角**：本 README → [`docs/产品原型说明.md`](docs/产品原型说明.md) → [`docs/协作机制设计.md`](docs/协作机制设计.md)
- **开发 / 接手视角**：[`docs/项目开发须知.md`](docs/项目开发须知.md)（主入口）→ [`AGENTS.md`](AGENTS.md)（详细规则）→ [`CLAUDE.md`](CLAUDE.md)

---

## 命名由来

"不令"不是从英文概念词翻译来的，是从中文语料里长出来的：字面意思"不需要发号施令（事情照样成了）"，原句讲为政者以身作则、无需强制命令别人就自然跟从，拿来命名一个去中心化协作平台恰好对上"平台不指派、Agent 自组织"的结论。别的协作产品不会撞名。
