---
kind: regression-report
version: 1
producer: 测试
inputs:
  - test-cases
  - bug-report
  - test-report
project: <auto-fill>
created_at: <auto-fill-iso>
---

# 回归测试报告 · Regression Report

<!--
测试产出。守则：
1. **Bug 修复后必须重跑完整用例集**，不允许"只跑改动那一处"
2. 新引入 Bug（regression）必须标出来，比旧 Bug 更严重
3. 明确判定 `all_passed` 布尔（Runtime 靠这个决定回环 / 前进）
-->

## 1. 结论

**Runtime 契约字段**：

```yaml
all_passed: true              # ← 影响工作流走向
                              # true  → 进 acceptance
                              # false → 回 bug_fix 循环
critical_count: 0             # 剩余 P0 Bug 数
new_regressions: 0            # 本轮新引入的问题数
```

**推荐动作**：
- [ ] all_passed=true → 进入验收
- [ ] all_passed=false → 回 bug_fix

## 2. 上轮 Bug 修复验证

<!-- 上轮 Bug-Report 里所有 Bug 逐条验证 -->

| BUG-ID | 上轮严重度 | 本轮状态 | 验证方式 |
|---|---|---|---|
| BUG-001 | P0 | ✅ 已修复 | TC-002 通过 |
| BUG-003 | P1 | ❌ 未修复 | TC-005 仍失败（新证据附上）|

## 3. 全量用例结果

| TC-ID | 上轮结果 | 本轮结果 | 变化 |
|---|---|---|---|
| TC-001 | ✅ | ✅ | 稳定 |
| TC-002 | ❌ | ✅ | 修好 |
| TC-050 | ✅ | ❌ | ⚠️ 回归（新问题）|

## 4. 新引入的问题（Regressions）

**这一节最重要**——修 Bug 引入的新 Bug 常常比原 Bug 严重。

### REG-01 · [标题]

- **上轮状态**：TC-050 通过
- **本轮状态**：TC-050 失败
- **可能触发变更**：BUG-001 的修复引入
- **严重度**：P0（比原 Bug 更严重）
- **重现**：[步骤]
- **证据**：[链接]

## 5. 关键性能对比

| 指标 | 上轮 | 本轮 | 变化 |
|---|---|---|---|
| API P95 | 178ms | 195ms | +17ms |
| 首屏 TTI | 1.8s | 1.9s | +100ms |

## 6. 环境信息

- 前端版本：[commit-hash（本轮）] ← 上轮 [commit-hash]
- 后端版本：[commit-hash（本轮）] ← 上轮 [commit-hash]
