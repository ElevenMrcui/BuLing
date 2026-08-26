---
kind: test-cases
version: 1
producer: 测试
inputs:
  - test-plan
  - acceptance
project: <auto-fill>
created_at: <auto-fill-iso>
---

# 测试用例 · Test Cases

<!--
测试产出。守则：
1. 每条用例 = 明确输入 · 明确期望 · 明确判定，任何人跑都得到同结果
2. Given/When/Then 结构直接映射到 Playwright / Jest 断言
3. 每条挂唯一 TC-ID · 关联 AC-ID
-->

## 1. 索引

| TC-ID | 关联 AC | 层级 | 一句话 |
|---|---|---|---|
| TC-001 | AC-001 | 单元 | [做什么] |

## 2. 用例详述

### TC-001 · [简短标题]

**关联 AC**：AC-001
**层级**：[单元 / 接口 / E2E]
**前置**：
- 环境：[干净数据库 · 已登录用户 X]
- 数据：[种子数据集 A]

**步骤**：
```
Given: [初始状态]
When:  [动作 1]
       [动作 2]
Then:  [预期结果 1]
And:   [预期结果 2]
```

**断言**（可直接翻译为代码）：
- expect(response.status).toBe(200)
- expect(response.body.field).toEqual(expectedValue)

**清理**：[如需]

---

### TC-002 · ...

## 3. 边界与异常用例

| TC-ID | 场景 | 期望 |
|---|---|---|
| TC-E01 | 输入为空 | 400 · error.code = INVALID_ARGUMENT |
| TC-E02 | 网络断开 | 客户端 Retry 1 次 · 失败 toast |
| TC-E03 | 权限不足 | 403 · 显示"无权访问" |
