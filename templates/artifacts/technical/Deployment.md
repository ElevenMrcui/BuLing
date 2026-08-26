---
kind: deployment
version: 1
producer: 架构师
inputs:
  - architecture
  - tech-stack
project: <auto-fill>
created_at: <auto-fill-iso>
---

# 部署方案 · Deployment

<!--
架构师产出。硬约束：
1. 必须给出至少一条"最小成本可跑"的路径（免费 / 单机）
2. 花哨方案放"可选升级"，不作为默认
3. 环境变量清单必须列全，前后端能照抄跑通
-->

## 1. 目标环境

| 环境 | 用途 | 域名 / URL | 备注 |
|---|---|---|---|
| local | 本机开发 | http://localhost:PORT | - |
| staging | 预发 | [url] | - |
| production | 正式 | [url] | - |

## 2. 最小可跑路径（推荐 P0）

<!-- 单机 · 免费 / 极低成本 · 5 分钟起 -->

**架构**：
```
用户 → 单机 Docker Compose → { frontend, backend, db }
```

**步骤**：
```bash
git clone <repo>
cp .env.example .env      # 按 §4 填 env
docker compose up -d
open http://localhost:<port>
```

## 3. 可选升级 · 云端部署

| 层 | 推荐 | 备选 | 月成本 |
|---|---|---|---|
| 前端 | [Vercel / Cloudflare Pages] | Netlify | 免费起 |
| 后端 | [Fly.io / Railway] | Render, Heroku | ~$5 起 |
| 数据库 | [Supabase / Neon] | PlanetScale | 免费起 |
| 存储 | [R2 / S3] | GCS | 按量 |

## 4. 环境变量

**前端**（`frontend/.env.production`）：

| 变量 | 示例 | 说明 |
|---|---|---|
| `VITE_API_BASE_URL` | https://api.example.com | 后端 API 前缀 |

**后端**（`backend/.env`）：

| 变量 | 示例 | 说明 | 敏感 |
|---|---|---|---|
| `DATABASE_URL` | postgres://... | 数据库连接 | ✓ |
| `JWT_SECRET` | 32B+ 随机 | JWT 签名 | ✓ |

**注意**：敏感变量在生产由 KMS / Vault / OS Keychain 注入；**不要**写死到镜像。

## 5. 构建 & 发布流程

```
git push main
   ↓
CI: lint + typecheck + test
   ↓
Build 前端 + 后端镜像
   ↓
自动部署 staging
   ↓
手动 approve
   ↓
部署 production
   ↓
Smoke test
```

## 6. 回滚

- 前端：静态资源版本化，指向前一个 commit
- 后端：容器镜像标签保留 [N] 个，一键切
- 数据库：迁移只加不删；破坏性变更走前向兼容窗口期

## 7. 观测

- 日志：[stdout → CloudWatch / Loki]
- 指标：[/metrics → Prometheus]
- 追踪：[OTEL]（可选）
- 告警：[Slack / 邮件]
