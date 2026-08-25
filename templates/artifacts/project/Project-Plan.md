---
kind: project-plan
version: 1
producer: 项目经理
inputs:
  - prd
  - architecture
project: <auto-fill>
created_at: <auto-fill-iso>
---

# 项目计划 · Project Plan

<!--
项目经理产出。守则：
1. 排期以"节点计数"为主，绝对天数只在用户明确需要时给（AI 团队没有人工工时概念）
2. 里程碑不超过 5 个，每个对应用户可感知的可交付物
3. 依赖关系必须显式（谁挡谁）；平行的任务显式标出可并行
-->

## 1. 总体规划

**目标**（引用 PRD §1）：[一句话]

**规划分阶段**：

| 阶段 | 目标交付 | 关键节点 | 依赖 |
|---|---|---|---|
| Phase 1 · 需求 | PRD + AC | prd_review 通过 | 用户目标 |
| Phase 2 · 技术 | 架构 + API + DB | design_review 通过 | PRD |
| Phase 3 · 开发 | frontend + backend 通编译 | 联调通过 | 架构 + 排期 |
| Phase 4 · 测试 | 测试报告 + Bug 修复 | qa_gate 通过 | 开发完 |
| Phase 5 · 验收 | 验收报告 + CEO 确认 | 最终验收 | 测试通过 |

## 2. 里程碑

| M | 名称 | 定义（可验证的产物） | 关联 Gate |
|---|---|---|---|
| M1 | 需求锁定 | PRD + AC 全部通过评审 | requirement-gate |
| M2 | 技术方案锁定 | Architecture + API + DB 全部通过 | design-gate |
| M3 | 开发交付 | frontend/ + backend/ 通编译 · 联调 OK | development-gate |
| M4 | 测试通过 | Test-Report 关键用例全绿 + Bug P0 = 0 | qa-gate |
| M5 | 交付 | Acceptance-Report + CEO 批准 | acceptance-gate |

## 3. 依赖图

```
prd ─▶ tech ─▶ arch ─▶ ┬─▶ frontend ─┐
                       └─▶ backend ──┴─▶ qa ─▶ acceptance
```

## 4. 任务清单（引用 Task-List.md）

详见 [`Task-List.md`](Task-List.md)。

## 5. 风险登记（引用 Risk.md）

详见 [`Risk.md`](Risk.md)。

## 6. 变更控制

- 用户改需求 → 回到 Phase 1 起，可能推翻下游
- 技术方案变更 → 回到 Phase 2 起，需要重新评审
- 计划变更 = 新增一版本，不覆盖旧版
