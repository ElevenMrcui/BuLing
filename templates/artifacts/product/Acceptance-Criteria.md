---
kind: acceptance
version: 1
producer: 产品经理
inputs:
  - prd
project: <auto-fill>
created_at: <auto-fill-iso>
---

# 验收标准 · Acceptance Criteria

<!--
每条 AC 严格 Given/When/Then 格式，且必须**可自动化验证**（QA 会拿来生成用例）。
禁止"用户体验流畅"这类主观描述。
每条 AC 有唯一 id (AC-NNN)，PRD 与 Test-Cases 会引用它。
-->

## 1. 索引

| AC-ID | 关联功能 | 描述（一行） | 优先级 |
|---|---|---|---|
| AC-001 | F001 | [一句话] | P0 |

## 2. 详细验收标准

### AC-001 · [简短标题]

**关联功能**：F001

**前置条件**：
- [用户已完成 XX 步骤]
- [系统处于 XX 状态]

**Given / When / Then**：

```
Given [初始状态]
When  [用户动作]
Then  [系统响应]
And   [附加断言]
```

**验证方式**：[单元测试 / 集成测试 / 手动测试 / 混合]

**边界与异常**：
- 输入为空：[期望行为]
- 网络断开：[期望行为]
- 权限不足：[期望行为]

---

### AC-002 · [简短标题]

...

## 3. 非功能性验收

<!-- 性能 / 无障碍 / 兼容性 / 隐私 -->

| 类别 | 要求 | 验证方式 |
|---|---|---|
| 性能 | 首屏 < 2s | Lighthouse |
| 无障碍 | WCAG AA 对比度 | axe-core |
| 隐私 | 无外发用户敏感字段 | 隐私哨兵审计 |
