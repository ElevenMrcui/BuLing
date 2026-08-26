---
kind: acceptance-report
version: 1
producer: 验收
inputs:
  - acceptance             # PRD 里的 Acceptance-Criteria
  - test-report
  - regression-report
project: <auto-fill>
created_at: <auto-fill-iso>
---

# 验收报告 · Acceptance Report

<!--
验收产出。硬约束：
1. **验收只呈现事实，不做决定**——通过与否由 CEO 在评审中心显式点通过（品牌红线）
2. 每项 Acceptance Criterion 必须给证据链：引用哪份 Test Report / 哪次 Run / 哪张截图
3. 未覆盖的 AC 明确列出，不写"应该没问题"这种主观判断
4. 敏感度 high：默认只允许本地 Provider 生成本文（隐私哨兵硬拦）
-->

## 1. 整体结论（呈现事实 · 不做决定）

**AC 覆盖统计**：

| 类别 | 数量 |
|---|---|
| AC 总数 | [N] |
| ✅ 通过 | [N]（[%]）|
| ❌ 未通过 | [N] |
| ⏭ 未覆盖 | [N] |

**Runtime 契约字段**：

```yaml
all_ac_passed: true|false     # 所有 P0 AC 是否全部通过
uncovered_p0: 0               # 未覆盖的 P0 AC 数
critical_issues: 0            # 剩余 P0 Bug 数
```

**建议行动（供 CEO 参考）**：
- [ ] 建议通过（一切绿）
- [ ] 建议延期（列具体阻塞项）
- [ ] 建议接受局部风险后通过（列具体风险 + 兜底）

**⚠️ 最终决定由 CEO 在评审中心显式点通过 —— 本报告不代替评审。**

## 2. AC 逐项核对

<!-- 每条 AC 一行 · 有证据链 -->

| AC-ID | 关联功能 | 优先级 | 结果 | 证据 |
|---|---|---|---|---|
| AC-001 | F001 | P0 | ✅ 通过 | Test-Report §2 TC-001 · [screenshot](../qa/screenshots/TC-001.png) |
| AC-002 | F001 | P0 | ❌ 未通过 | Regression-Report §4 REG-01 |
| AC-003 | F002 | P1 | ⏭ 未覆盖 | Test-Cases 未包含 · 见 §4 |

## 3. 未通过项详情

### AC-002 · [标题]

- **期望**（引用 Acceptance-Criteria）：[原文]
- **实际**：[事实描述 · 不主观评价]
- **证据**：[log 路径 / 截图 / 具体运行]
- **相关 Bug**：BUG-XXX（当前状态：未修复）
- **对交付的影响**：[事实性描述]

## 4. 未覆盖项详情

### AC-003 · [标题]

- **原因**：[具体理由 · 不是"没时间"]
- **风险**：[如果不测，最坏情况是什么]
- **建议**：[延后覆盖 / 手工验一次即可 / 必须补]

## 5. 关键运行验证

<!-- 除了 AC，验收还要看"真的能跑通端到端"的证据 -->

- **前端启动**：[命令 · 结果 · 截图]
- **后端启动**：[命令 · 结果 · 日志片段]
- **核心动线** 1：[步骤 · 是否成功]
- **核心动线** 2：[...]

## 6. 附证据清单

| 类型 | 位置 |
|---|---|
| 测试报告 | [`docs/qa/Test-Report.md`](../qa/Test-Report.md) |
| 回归报告 | [`docs/qa/Regression-Report.md`](../qa/Regression-Report.md) |
| Bug 报告 | [`docs/qa/Bug-Report.md`](../qa/Bug-Report.md) |
| 运行日志 | `docs/acceptance/logs/*.log` |
| 截图 | `docs/acceptance/screenshots/*.png` |
