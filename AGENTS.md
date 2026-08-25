# AGENTS.md · 仓库开发规则（权威源）

本文是**不令 OPC** 仓库的开发规则权威源。首次接手请先读 [`CLAUDE.md`](CLAUDE.md)（分支政策 + 硬约束 + 命名 + 文档地图，精简指针）再读本文（详细规则）。

产品全景 → [`docs/OPC-产品定义.md`](docs/OPC-产品定义.md)
本地网关 → [`docs/本地网关.md`](docs/本地网关.md)

---

## 0. 项目速览

**不令 OPC**：Local-First 的私人 AI 公司桌面应用。用户提出目标，OPC 组织 AI 团队（产品经理 · 技术负责人 · 架构师 · 项目经理 · 设计师 · 前端 · 后端 · 测试 · 验收）完成完整项目交付。

**当前仓库处在 pivot 后重启期**——旧「不令 · 企业级协作平台」骨架已归档到 `legacy/`，OPC 新架构（Tauri 2 + React + Rust Runtime + SQLite）正在筹备 P0。

**现存可跑的东西**：
- `apps/local-gateway/`（Node/TS · Fastify · 只监听 127.0.0.1 · Token 鉴权 · 扫描本机 AI CLI）——未来会升级为 OPC Runtime 的 CLI 执行层
- `packages/cli-registry/`（10 家 AI 厂商 CLI 签名注册表）

---

## 1. 硬约束（不可违背）

沿用 [`CLAUDE.md`](CLAUDE.md) 里的清单，展开说明：

### 1.1 评审红线（品牌基因）
关键决策必须人工显式打回 / 批准，**永不自动化**：
- 需求评审 · 技术方案评审 · 最终验收
- 高风险命令（`git push` / `docker push` / `rm -rf` / `sudo`）
- 任何跨出项目工作目录的文件操作

### 1.2 Local First
- 默认本地 SQLite / 本地文件 / 本地 Git
- 默认调本机 CLI（Claude Code / Codex / Gemini CLI）
- 云 API 与本地模型都是**用户可选**，不是默认

### 1.3 Zero Server
- 核心功能不依赖不令官方云
- 用户下载安装 → 配置 AI → 开始工作

### 1.4 Provider Agnostic
- Agent 与 Provider 解耦
- 所有 CLI / API / Local 走统一 `Provider` 抽象
- Fallback 链路：优先 CLI → Local → API

### 1.5 Artifact Driven
- Agent 之间**主要靠标准化 Artifact 传递信息**，不主要靠聊天上下文
- 每个 Artifact 必须能反向追溯到用户最初的目标

### 1.6 最小权限 · 显式授权
Agent 操作电脑必须精细到六大类：
```
File · Command · Network · Git · Docker · MCP
```
高风险操作必须弹窗确认，且可选"允许一次 / 始终允许"。

### 1.7 不臆造
技术栈 / 后端 / 业务规则**未定的**，标"待确认"交用户裁决，**不猜**。

### 1.8 改动最小化
紧扣需求，禁止"顺手改一下"。

---

## 2. 分支政策

- **主开发分支：`claude/bulei-platform-prototype-dw7wxw`**（分支名沿用旧仓库，不改）。
- 直接在此分支开发、提交、推送。
- 未经用户明确许可，**不推送到其他分支**，**不创建 PR**。
- 每次接手第一动作：`git fetch origin claude/bulei-platform-prototype-dw7wxw && git status -sb`。

Sub-agent 用 worktree 隔离时可临时分支，交付后 `--ff-only` 合回并推，不留孤立分支。

---

## 3. 命名约定

### 3.1 产品名
- 正式：`不令 OPC`
- 简称：`OPC`
- 英文：`BuLing OPC` / `OPC by BuLing`

### 3.2 内部模块 / 一级菜单
统一 `XX 中心` 格式：
- 项目中心 / 团队中心 / 任务中心 / 工作流中心 / 产出中心 / 评审中心
- 技能中心 / 记忆中心 / 模型中心 / 权限中心 / 日志中心

### 3.3 AI 岗位
中文短名，**不加 "Agent" 后缀**：
- 产品经理 · 技术负责人 · 架构师 · 项目经理 · 设计师 · 前端 · 后端 · 测试 · 验收

### 3.4 系统文案主语
用岗位名，不用 "Agent"：
- ✅ "**产品经理** 正在整理需求…"
- ❌ "PM Agent is analyzing requirements…"

### 3.5 代码命名
- TypeScript：camelCase / PascalCase / UPPER_SNAKE 常量
- Rust：snake_case / PascalCase
- 文件：kebab-case（`agent-runner.ts` / `provider_router.rs`）
- 数据库表：snake_case（`agent_runs` / `task_reviews`）

---

## 4. Design-First 六步纪律

超过"单一自包含小改动"规模的工作，必须走完六步再动键盘：

1. **明确目标**：这个改动要解决什么用户/系统问题？
2. **读现状**：`docs/OPC-产品定义.md` + 相关模块代码 → 确认当前形状
3. **核对不变量**：本文 §1 硬约束是否被本次改动挑战
4. **写方案**：改什么 / 为什么 / 影响哪些既有能力 / 迁移风险
5. **拍板**：不确定的技术栈 / 数据字段 / 交互，走 AskUserQuestion 或标"待确认"，不猜
6. **落地**：小步提交 · Conventional Commits · 每步能独立通过 typecheck

---

## 5. 数据模型改动纪律

数据是 OPC 的地基，改起来最贵：

- SQLite schema 每次改动都要写 migration，不允许"手工改 schema"
- Artifact / Review / ExecutionLog 三张表**只允许 append**，不允许 update / delete（审计要求）
- API Key **永远走 OS Keychain**（macOS Keychain / Windows Credential Manager / Linux Secret Service），不允许落 SQLite
- 加密算法沿用旧 `legacy/apps/api/src/modules/providers/key-crypto.ts` 的 AES-256-GCM，作为文件加密兜底

---

## 6. 权限与安全纪律

任何新增 Agent 可调用的 Tool，必须：

1. 声明所属的权限类（File / Command / Network / Git / Docker / MCP）
2. 声明**最小权限集合**（不是"启用整个 shell"，而是"只允许 `npm test`"）
3. 高风险操作必须走弹窗确认组件，不允许静默执行
4. 所有执行必须写 `ExecutionLog`（Who / What / When / Where / Provider / Command / Input / Output / Result）
5. 与外发有关的操作必须过隐私哨兵（Privacy Sentinel），敏感数据自动脱敏或直接拒绝

---

## 7. Commit / PR 规范

**Conventional Commits**：
```
feat(runtime): 新增 Provider Router
fix(gateway): 修复 CORS null origin 被拒绝
docs(opc): 更新产品定义文档
refactor(agents): Agent Skill 层与 Tool 层拆分
chore: 归档旧 apps/api 到 legacy/
```

**尾行**：所有 commit 尾行加 `Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>` + `Claude-Session: ...`（见 Bash 工具规范）。

**PR**：未经用户明确许可**不创建 PR**。用户明确要求时才建，并检查仓库有无 `.github/PULL_REQUEST_TEMPLATE.md`。

---

## 8. 测试规范

- **Runtime 层**（Rust）：`cargo test` 每 crate 有覆盖
- **Provider Adapter**：每家 CLI / API 至少一个"发现 + 空跑一次" 的集成测试
- **前端**（React）：关键交互组件（Team 组阁向导 / 权限确认弹窗 / 评审面板）走 Playwright 无头浏览器冒烟
- **本地网关**：POST /discover 通过 401 → 200 → 结构断言三段式

---

## 9. 文档纪律

**文档即代码**：
- 改了产品定义 → 同步 `docs/OPC-产品定义.md`
- 改了本地网关协议 → 同步 `docs/本地网关.md`
- 改了硬约束 → 同步 [`CLAUDE.md`](CLAUDE.md) 与本文
- **没更新相关文档，切片就没做完**

---

## 10. 从旧仓库继承的资产

**能沿用（已保留）**：
- `packages/cli-registry/` — CLI 签名注册表（10 家）
- `apps/local-gateway/` — 本机守护进程 · 127.0.0.1 + Token
- `docs/本地网关.md` — 协议 / 安全底线

**归档到 `legacy/`（不构建 · 参考不引用）**：
- `legacy/apps/api/` — NestJS + Prisma（AES 加密 · 评审红线状态机可借鉴）
- `legacy/apps/web/` — Next.js 14
- `legacy/infra/` — docker-compose (Postgres + Redis)
- `legacy/prototype/` — 原型 HTML（视觉设计语言可借鉴）
- `legacy/docs/*` — 旧「不令 · 企业级协作平台」全套文档

**明令禁止**：
- 不要 `import` `legacy/` 里的任何东西
- 不要 `pnpm add` `legacy/` 下的包
- 迁移代码时**手抄一份到新位置** + 改造，不做符号链接 / workspace 引用

---

## 11. 第一天上手清单

新接手 Agent 请按此清单一次走完：

- [ ] 读 [`CLAUDE.md`](CLAUDE.md)（3 分钟）
- [ ] 读本文 §0-3 §10（5 分钟）
- [ ] 读 [`docs/OPC-产品定义.md`](docs/OPC-产品定义.md) §1 §2 §3 §5 §17（10 分钟）
- [ ] `git fetch origin claude/bulei-platform-prototype-dw7wxw && git status -sb`
- [ ] `pnpm install` · `pnpm -r typecheck` → 全绿
- [ ] `make gateway` → 本地网关起来 · 用 curl 试 `/health` 和 `/discover` → 200
- [ ] 开工

---

## 12. 常见陷阱

- **误引 legacy/**：新代码不要 `import` `legacy/` 下的东西，会污染 workspace
- **在 SQLite 存 API Key**：一律走 OS Keychain
- **静默调用高风险命令**：任何 Tool 调用前先过权限系统
- **忘同步文档**：改了产品行为不改 `OPC-产品定义.md` 就是没做完
- **发号施令的文案**：主语用岗位（"产品经理"），不要写 "Agent" / "系统"
