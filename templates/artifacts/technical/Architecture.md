---
kind: architecture
version: 1
producer: 架构师
inputs:
  - prd
  - tech-stack
project: <auto-fill>
created_at: <auto-fill-iso>
---

# 系统架构 · Architecture

<!--
架构师产出。硬约束：
1. 所有内容必须能让前端 / 后端**直接照做**，禁止"TBD"
2. 组件边界 · 数据流 · 依赖关系写清；ASCII 图或 mermaid 都可以
3. 与 PRD / 技术选型冲突时，回退到那两份，不擅自改
-->

## 1. 架构总图

<!-- 用 ASCII 或 mermaid 画组件级视图。至少标出：前端 · 后端 · 数据库 · 外部依赖 -->

```
┌──────────────┐     HTTPS      ┌──────────────┐
│  前端 App    │ ─────────────▶ │  后端 API    │
│  (React)     │                │  (Node/22)   │
└──────────────┘                └──────┬───────┘
                                       │
                                       ▼
                                ┌──────────────┐
                                │  Database    │
                                │  (SQLite)    │
                                └──────────────┘
```

## 2. 组件清单

| 组件 | 职责 | 技术栈 | 端口 / 入口 |
|---|---|---|---|
| [Component A] | [一句话职责] | [栈] | [:port / path] |

## 3. 数据流

<!-- 关键几条：用户请求怎么走完全链路；异步任务怎么触发；错误如何回溯 -->

**流 1：[名称]**

```
Actor → Component A → Component B → Storage → 响应
```

**流 2：[名称]**

...

## 4. 组件间契约

<!-- 谁调用谁 · 通过什么协议 · 依赖方向 -->

| 主 | 依赖 | 协议 | 契约位置 |
|---|---|---|---|
| 前端 | 后端 | HTTPS/JSON | API-Spec.md |
| 后端 | 数据库 | SQL over TCP | Database-Design.md |

## 5. 部署拓扑（引用）

详见 [`Deployment.md`](Deployment.md)。

## 6. 安全边界（引用）

详见 [`Security.md`](Security.md)。

## 7. 关键决定

<!-- 架构上的取舍：为什么这么切模块、为什么用同步/异步、为什么单库/多库 -->

- **[决定 1]**：[结论] · 理由：[一句话] · 备选：[否决理由]

## 8. 待用户确认

- [ ] [跨栈的关键取舍]
