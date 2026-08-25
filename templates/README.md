# templates/ · 项目工作流模板

模板 = **一个项目怎么从"目标"跑到"交付"的蓝图**。用户在"新建项目"时选一份模板，OPC 就按模板实例化 Team + Workflow 开始跑。

## 模板清单

| 文件 | 名称 | 覆盖场景 |
|---|---|---|
| `standard-software-delivery.yaml` | 标准软件交付流 | 从"一句目标"到"可交付软件产品"的完整链路（PRD → 架构 → 开发 → 测试 → 验收） |

**P1 计划补的模板**：
- `content-creation.yaml` —— 内容创作（选题 → 调研 → 大纲 → 撰稿 → 编辑 → 发布）
- `data-analysis.yaml` —— 数据分析（问题定义 → 数据准备 → 分析 → 可视化 → 报告）
- `marketing-campaign.yaml` —— 营销活动（定位 → 内容矩阵 → 投放 → 复盘）

## 模板文件格式

顶层字段：

| 字段 | 说明 |
|---|---|
| `id` | 稳定 id, kebab-case |
| `name` | 中文名 |
| `description` | 一段说明 |
| `version` | 版本号 |
| `required_roles` | 需要哪些 Agent 岗位（引用 `agents/*.yaml` 的 id） |
| `nodes` | 工作流节点列表（DAG） |
| `gates` | 关键卡点声明 |

## 节点类型

每个节点必有 `kind`：

| kind | 说明 |
|---|---|
| `agent` | 由某个 Agent 执行 |
| `human` | 人工评审 / 决策（评审红线走这个） |
| `condition` | 条件分支 |
| `parallel` | 并行子节点 |
| `sub-workflow` | 嵌套子工作流（P1） |

## Assignment 三种策略

`agent` 节点必须声明 `assignment`：

| 策略 | 语义 |
|---|---|
| `template` | 用模板默认岗位 + 平台推荐 Provider |
| `manual` | 项目负责人（用户）手动指派 |
| `auto-claim` | 广播给能力池，Agent 自主认领（意愿分算法） |

## Inputs / Outputs 契约

- `inputs`: 前置节点的 Artifact 引用（例如 `prd.output.acceptance`）——**只声明依赖，Runtime 自动注入**
- `outputs`: 期望产出的 Artifact 契约（kind + path），Runtime 会检查节点执行完是否真的产出了这些

**特殊输入**：
- `__goal__` —— 用户最初输入的目标（只在入口节点用）

**特殊输出目标**：
- `__completed__` —— 工作流成功结束
- `__failed__` —— 工作流失败结束

## Gate（人工卡点）

`kind: human` 的节点自动作为 Gate。Gate 通过前，DAG 后续节点**不能启动**。

**品牌硬约束**：`acceptance-gate` `required: true` 永远不可跳过——评审红线不给关。

## 关联文档

- `docs/OPC-产品定义.md § 7` —— Project Workflow 生命周期
- `agents/README.md` —— 可用岗位清单
- `runtime/migrations/project/0001_init.sql` —— `workflows` / `tasks` / `gates` 表 schema
