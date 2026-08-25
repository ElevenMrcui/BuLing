---
kind: test-plan
version: 1
producer: 测试
inputs:
  - prd
  - acceptance
  - architecture
project: <auto-fill>
created_at: <auto-fill-iso>
---

# 测试计划 · Test Plan

<!--
测试产出。守则：
1. 每个 AC 必须映射到至少一条测试用例；未覆盖必须显式列
2. 覆盖类型分层：单元 / 接口 / E2E / 性能 / 安全 / 兼容
3. 通过标准量化（禁止"基本通过"这种）
-->

## 1. 测试范围

**在范围内**（对齐 PRD 的 P0）：
- [模块 A · 功能 X]

**不在范围内**（P1 之后）：
- [列点]

## 2. 测试层级与工具

| 层级 | 目标 | 工具 | 覆盖率目标 |
|---|---|---|---|
| 单元 | 核心逻辑 · 边界 | jest / vitest / pytest | ≥ 70% 行 |
| 接口 | API 端点契约 | supertest / httpx | 100% endpoint |
| E2E | 关键动线 | playwright / cypress | 3-5 条主流程 |
| 性能 | 首屏 / 关键接口 | Lighthouse / k6 | 达标见 §5 |
| 安全 | XSS / SQLi / 越权 | 手工 + zap-baseline | 无 P0 高危 |

## 3. AC 覆盖矩阵

| AC-ID | 测试层级 | 用例 ID | 备注 |
|---|---|---|---|
| AC-001 | 单元 + E2E | TC-001, TC-101 | - |
| AC-002 | 接口 | TC-050 | - |

**未覆盖**：
- [AC-XXX：为什么未覆盖 + 是否需要延后覆盖]

## 4. 测试数据

- **种子数据**：`docs/qa/seed/*.sql`
- **测试用户**：test-user-{1..N}
- **敏感数据处理**：mock；禁止真实 PII

## 5. 通过标准（Exit Criteria）

- P0 Bug 数 = 0
- P1 Bug 数 ≤ [N]
- 单元测试通过率 100%
- 接口测试通过率 100%
- E2E 关键流程通过
- 性能：首屏 < 2s · API P95 < 300ms

## 6. 执行计划

- 全量：M3 完成后一次
- 回归：Bug 修复后
- 冒烟：每次前端 / 后端 push 后自动跑

## 7. 报告

- 详细用例 → [`Test-Cases.md`](Test-Cases.md)
- 执行结果 → [`Test-Report.md`](Test-Report.md)
- Bug 列表 → [`Bug-Report.md`](Bug-Report.md)
- 回归结果 → [`Regression-Report.md`](Regression-Report.md)
