# CLAUDE.md

本文件是精简指针层。**详细规则的权威源是 [`AGENTS.md`](AGENTS.md)**；新 Agent 首次接手的统一入口是 [`docs/项目开发须知.md`](docs/项目开发须知.md)。

---

## 项目一句话

**不令（BuLing）**：去中心化多智能体协作平台。仓库同时承载两个阶段：
- **产品原型**：`prototype/index.html`（单文件零依赖），产品 / 评审的高保真交互演示
- **企业级 R1 骨架**：`apps/api` (NestJS+Fastify+Prisma) + `apps/web` (Next.js 14) + `packages/shared` + `prisma` + `infra`；技术选型 / 架构 / 数据模型 / API 契约 / 分期路线图详见 [`docs/企业级架构与落地方案.md`](docs/企业级架构与落地方案.md)，跑起来看 [`docs/骨架启动指南.md`](docs/骨架启动指南.md)

---

## 分支政策

- **主开发分支：`claude/bulei-platform-prototype-dw7wxw`**。直接在此分支开发、提交、推送。
- 未经用户明确许可，**不推送到其他分支**，**不创建 PR**。
- 每次接手第一动作：`git fetch origin claude/bulei-platform-prototype-dw7wxw && git status -sb`。

## Design-First Flow

超过"单一自包含小改动"规模的工作，先设计后编码：

1. 读 [`docs/产品原型说明.md`](docs/产品原型说明.md) 确认现状；
2. 读 [`docs/协作机制设计.md`](docs/协作机制设计.md) 核对不变量（**评审红线不可自动化**）；
3. 写清改动方案（改什么 / 为什么 / 影响哪些既有交互）再动手；
4. 完成后回填相关文档。

## 硬约束（不可违背）

- **评审红线**：质量把关与评审必须人工显式打回 / 批准，永不自动化。
- **术语**：界面与代码注释里不出现蚁群 / 蜂群 / 狼群，统一用功能名（自主认领机制 / 临时协同机制 / 协作痕迹机制）。
- **主题合规**：颜色走设计令牌，不把颜色只定义在 `@media` / `[data-theme]` 块里。
- **不臆造**：技术栈 / 后端 / 业务规则未定的，标"待确认"交用户裁决，不猜。
- **改动最小化**：紧扣需求，禁止"顺手改一下"。

## 文档地图

| 层级 | 文档 |
|---|---|
| 项目规则 | `CLAUDE.md`（本文）+ `AGENTS.md` |
| 上手主入口 | `docs/项目开发须知.md` |
| 原型规格 | `docs/产品原型说明.md` |
| 机制不变量 | `docs/协作机制设计.md` |
| 交付物 · 原型 | `prototype/index.html` |
| 交付物 · 骨架 | `apps/api` · `apps/web` · `packages/shared` · `prisma` · `infra` |
| 企业级架构 | `docs/企业级架构与落地方案.md` · `docs/骨架启动指南.md` |

## 每次改动后

浏览器打开 `prototype/index.html` 自查无报错（或跑 Playwright 冒烟）→ 更新相关文档 → Conventional Commits 小步提交 → 推送。
