# CLAUDE.md

本文件是精简指针层。**详细规则的权威源是 [`AGENTS.md`](AGENTS.md)**；产品定义是 [`docs/OPC-产品定义.md`](docs/OPC-产品定义.md)。

---

## 项目一句话

**不令 OPC（BuLing · One Person Company）**：Local-First 的私人 AI 公司桌面应用。

用户提出一个目标（"做一个健康管理 App"），OPC 调用用户电脑上已有的 AI CLI / API / 本地模型，自动组织一支 AI 团队（产品经理 · 技术负责人 · 架构师 · 项目经理 · 设计师 · 前端 · 后端 · 测试 · 验收）完成从需求、产品、技术、开发、测试到验收的完整项目交付。

品牌沿用《论语》"其身正，不令而行"——**你（CEO）出题，AI 团队各就其位不令而行**。

---

## 分支政策

- **主开发分支：`claude/bulei-platform-prototype-dw7wxw`**（分支名沿用，不改）。直接在此分支开发、提交、推送。
- 未经用户明确许可，**不推送到其他分支**，**不创建 PR**。
- 每次接手第一动作：`git fetch origin claude/bulei-platform-prototype-dw7wxw && git status -sb`。

---

## Design-First Flow

超过"单一自包含小改动"规模的工作，先设计后编码：

1. 读 [`docs/OPC-产品定义.md`](docs/OPC-产品定义.md) 确认现状与不变量；
2. 写清改动方案（改什么 / 为什么 / 影响哪些既有能力）再动手；
3. 完成后回填相关文档。

---

## 硬约束（不可违背）

- **评审红线**：关键决策（需求评审 / 技术方案评审 / 最终验收 / 高风险命令）必须人工显式打回 / 批准，永不自动化。
- **Local First**：默认本地项目 / 本地文件 / 本地 Git / 本地 CLI / 本地模型；数据默认不出机。
- **Zero Server**：核心功能不依赖不令官方云；用户下载即用。
- **Provider Agnostic**：不绑定任何 AI 厂商；所有 CLI / API / Local 走统一 Provider 抽象。
- **Artifact Driven**：Agent 之间通过标准化 Artifact 协作，不主要依赖聊天上下文传递信息。
- **最小权限 · 显式授权**：Agent 操作电脑要精细到 File / Command / Network / Git / Docker / MCP 六大类；高风险操作必须弹窗确认。
- **不臆造**：技术栈 / 后端 / 业务规则未定的，标"待确认"交用户裁决，不猜。
- **改动最小化**：紧扣需求，禁止"顺手改一下"。

---

## 文档地图

| 层级 | 文档 |
|---|---|
| 项目规则 | `CLAUDE.md`（本文）+ `AGENTS.md` |
| 产品定义 | [`docs/OPC-产品定义.md`](docs/OPC-产品定义.md) |
| 本地网关 | [`docs/本地网关.md`](docs/本地网关.md) |
| 现存代码 | `packages/cli-registry/` · `apps/local-gateway/` |
| 旧骨架参考 | `legacy/`（不构建，仅供借鉴：AES 密钥加密 · 评审红线状态机 · 原型 UI 语言） |

---

## 命名约定（写文案 / 写系统提示时用）

- **产品名**：`不令 OPC`（正式）· `OPC`（简称）· `BuLing OPC`（英文）
- **一级菜单**：统一 `XX 中心` 格式（项目中心 / 团队中心 / 任务中心 / 工作流中心 / 产出中心 / 评审中心 / 技能中心 / 记忆中心 / 模型中心 / 权限中心 / 日志中心）
- **AI 岗位**：中文短名，**不加 "Agent" 后缀**（产品经理 / 技术负责人 / 架构师 / 项目经理 / 设计师 / 前端 / 后端 / 测试 / 验收）
- **系统文案**：主语用岗位名，不用 "Agent" —— 例如："**产品经理** 正在整理需求…"

---

## 每次改动后

- 有代码变更：`pnpm -r typecheck` 通过；相关模块跑单测 / 手动冒烟。
- 有文档变更：确认 `docs/OPC-产品定义.md` 与代码一致，`CLAUDE.md` / `README.md` 的引用不悬空。
- Conventional Commits 小步提交，推 `claude/bulei-platform-prototype-dw7wxw`。
