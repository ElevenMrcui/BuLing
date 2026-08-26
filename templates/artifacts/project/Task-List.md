---
kind: task-list
version: 1
producer: 项目经理
inputs:
  - prd
  - architecture
project: <auto-fill>
created_at: <auto-fill-iso>
---

# 任务清单 · Task List

<!--
项目经理产出。守则：
1. **单任务颗粒度**：一个 Agent 一次 run 可完成。超过就再拆。
2. 每个任务显式写"前置产出"（inputs），Runtime 靠这个自动注入 Context
3. 每个任务写"期望产出"（outputs），Runtime 校验产出是否达标
4. `assignment` 三选一：template（默认岗位）· manual（用户指派）· auto-claim（能力池认领）
-->

## 1. 索引

| T-ID | Phase | 岗位 | 一句话 | assignment | 依赖 | 前置产出 |
|---|---|---|---|---|---|---|
| T-001 | 需求 | 产品经理 | 写 PRD | template | - | goal |
| T-002 | 需求 | 产品经理 | 写 Acceptance Criteria | template | T-001 | prd |
| T-101 | 技术 | 技术负责人 | 出技术选型 | template | T-002 | prd |
| T-102 | 技术 | 架构师 | 出架构 + DB + API | template | T-101 | prd, tech-stack |
| T-201 | 排期 | 项目经理 | 出排期 + Milestone + Risk | template | T-102 | prd, arch |
| T-202 | 设计 | 设计师 | 出 UX + Page + Interaction | template | T-102 | prd, arch |
| T-301 | 开发 | 前端 | 前端主页 + [核心页] | auto-claim | T-201, T-202 | prd, arch, design |
| T-302 | 开发 | 后端 | 后端 [核心模块] | auto-claim | T-201 | prd, arch |
| T-401 | 测试 | 测试 | 完整测试用例 + 报告 | template | T-301, T-302 | prd, arch, 代码 |
| T-501 | 验收 | 验收 | Acceptance 报告草稿 | template | T-401 | prd, test-report |

## 2. 详细任务

<!-- 每个任务一节。字段与 Task 表对齐 -->

### T-001 · 写 PRD

- **岗位**：产品经理
- **assignment**：template
- **前置产出**：`__goal__`
- **期望产出**：`docs/product/PRD.md`
- **完成定义**：PRD 结构完整（8 章节）· "待用户确认"清单存在
- **预估节点数**：1

### T-002 · 写 Acceptance Criteria

- **岗位**：产品经理
- **前置产出**：`prd`
- **期望产出**：`docs/product/Acceptance-Criteria.md`
- **完成定义**：每个 PRD 里的 P0 功能都有至少一条可自动化验证的 AC

### T-101 · 技术选型

...

---

## 3. 变更政策

- 加任务：允许，写清依赖插入位置
- 改任务范围：等价于新增子任务，别在原任务上扩容（防止追不出因果）
- 删任务：需评审，评估是否会孤立下游
