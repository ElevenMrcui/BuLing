---
kind: milestones
version: 1
producer: 项目经理
inputs:
  - project-plan
project: <auto-fill>
created_at: <auto-fill-iso>
---

# 里程碑 · Milestones

<!--
项目经理产出。守则：
1. 不超过 5 个里程碑（多了 CEO 记不住）
2. 每个都对应一个"用户可感知的可交付物"
3. 完成定义必须可自动化验证 / 或有明确评审动作
-->

## 1. 里程碑一览

| M | 名称 | 完成定义（可验证）| 关联 Gate | 关联任务 |
|---|---|---|---|---|
| M1 | 需求锁定 | AC 全部通过评审 | requirement-gate | T-001, T-002 |
| M2 | 技术方案锁定 | Arch + API + DB 通过评审 | design-gate | T-101, T-102 |
| M3 | 开发交付 | frontend/backend 通编译 + 联调 OK | development-gate | T-301, T-302 |
| M4 | 测试通过 | P0 Bug = 0 · 关键用例全绿 | qa-gate | T-401 |
| M5 | 交付 | Acceptance-Report 生成 + CEO 批准 | acceptance-gate | T-501 |

## 2. 详细定义

### M1 · 需求锁定

- **完成产物**：`docs/product/PRD.md` + `docs/product/Acceptance-Criteria.md`
- **验证方式**：requirement-gate 上 CEO 显式批准
- **失败回退**：回到 T-001（产品经理重做）

### M2 · 技术方案锁定

- **完成产物**：`docs/technical/Architecture.md` + `API-Spec.md` + `Database-Design.md`
- **验证方式**：design-gate 上 CEO 显式批准
- **失败回退**：回到 T-102（架构师重做）

...

## 3. 完成度看板（Runtime 自动更新）

<!-- Runtime 每步节点完成时更新这里 -->

- [ ] M1 · 需求锁定
- [ ] M2 · 技术方案锁定
- [ ] M3 · 开发交付
- [ ] M4 · 测试通过
- [ ] M5 · 交付
