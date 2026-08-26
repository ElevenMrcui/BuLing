# legacy/ · 旧「不令 · 企业级协作平台」骨架

本目录是 pivot 到 **不令 OPC（Personal AI Company）** 之前的旧仓库交付物存档。

**不构建，不引用，不删除**——仅作参考。

## 内容

| 目录 | 曾是什么 | 可借鉴的硬资产 |
|---|---|---|
| `apps/api/` | NestJS 10 + Fastify + Prisma + PostgreSQL R1 骨架 | **AES-256-GCM 密钥加密**（`src/modules/providers/key-crypto.ts`）· **评审红线三层强约束**（Guard + 状态机 + 审计） · **Actor / RBAC 装饰器**模式 |
| `apps/web/` | Next.js 14 App Router · E2E 演示 UI | 前后端契约调用约定 |
| `infra/` | docker-compose (Postgres + Redis) | 无（OPC 单机不用） |
| `prototype/` | 单文件 HTML 高保真原型 | **视觉设计语言**（配色 / 图标 / 卡片布局 / 深浅色 token）可复用到 OPC UI |
| `docs/` | 5 份产品与架构文档 | 产品原型说明 · 协作机制设计 · 企业级架构与落地方案 · 骨架启动指南 · 项目开发须知 |
| `packages-shared/` | 前后端共享 DTO / 枚举 | 类型体系与命名可参考 |
| `.env.example` | Postgres / Redis / JWT / Provider Master Key | 加密主密钥的组织方式可借鉴 |

## 明令禁止

- **不要 `import`** `legacy/` 里的任何东西到活跃代码
- **不要 `pnpm add`** `legacy/` 下的包
- 迁移代码时**手抄一份**到新位置 + 改造，不做符号链接 / workspace 引用

## 为什么归档而不删除

1. 部分工程实现（加密算法 / 状态机 / RBAC 装饰器）在 OPC 里仍是硬需求
2. 视觉设计语言（原型 UI 的 token / 布局）可复用到 OPC 的 Tauri UI
3. 历史文档留存产品演进脉络（旧「去中心化多智能体协作平台」→ 新「一人公司 AI 操作系统」）

产品定义现以 [`/docs/OPC-产品定义.md`](../docs/OPC-产品定义.md) 为唯一权威源。
