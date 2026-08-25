# agents/ · 预置 AI 岗位模板

本目录是不令 OPC **预置的 9 位 AI 员工**的定义源。每个 YAML 文件对应一个岗位；应用首次启动 / 用户创建新项目时，从这里读定义并写入 `app.sqlite` 的 `agents` 表（`kind='preset'`）。

用户可以在项目里对某个 Agent **fork + 覆盖**（改 prompt / 换 provider / 加权限），fork 实例保存在 `<project>/.opc/project.sqlite` 的 `agent_instances` 表。

## 岗位清单

| 文件 | 中文岗位 | 主要产出 | 敏感度 |
|---|---|---|---|
| `product-manager.yaml` | 产品经理 | PRD · User Stories · Acceptance Criteria | medium |
| `tech-lead.yaml` | 技术负责人 | Technology-Stack · Technical-Risk | medium |
| `architect.yaml` | 架构师 | Architecture · Database-Design · API-Spec · Security · Deployment | medium |
| `project-manager.yaml` | 项目经理 | Project-Plan · Task-List · Milestone · Risk | low |
| `designer.yaml` | 设计师 | UX-Overview · Page-Structure · Interaction · Visual-Tokens | low |
| `frontend.yaml` | 前端 | `frontend/**` 源码 | medium |
| `backend.yaml` | 后端 | `backend/**` 源码 | medium |
| `qa.yaml` | 测试 | Test-Plan · Cases · Report · Bug-Report · Regression | medium |
| `acceptance.yaml` | 验收 | Acceptance-Report | **high** |

## YAML 字段规范

严格对齐 `runtime/migrations/app/0001_init.sql` 中 `agents` 表的字段。字段说明：

| 字段 | 类型 | 说明 |
|---|---|---|
| `id` | string | 稳定 id，预置岗位用 kebab-case 词 |
| `kind` | `preset \| user` | 是内置岗位还是用户自建 |
| `role` | string | 中文短名（1-4 字），系统文案主语用它 |
| `display_name` | string | 展示名，一般同 role |
| `avatar` | emoji or path | 头像 |
| `sensitivity` | `low \| medium \| high` | 敏感度：high 只允许本地 provider（隐私哨兵硬拦）|
| `system_prompt` | string (multiline) | 岗位人格 / 责任 / 输出格式约束 |
| `responsibilities` | list | 职责清单 |
| `expected_outputs` | list of `{path, kind}` | 期望产出的 Artifact 契约 |
| `capabilities` | list | 能力标签（用于自主认领的意愿分匹配 + provider 路由）|
| `skills` | list | 复用技能 |
| `tools` | list | 可用工具（原子）|
| `mcp_servers` | list | 可挂 MCP 服务白名单 |
| `provider_priority` | list | Provider 路由链，按序尝试 |
| `permission_defaults` | object | 六大类权限默认（file/command/network/git/docker/mcp）|

## 修改守则

1. **改预置岗位**：任何改动都要在 commit message 说明**为什么**——预置岗位是全应用共享，改一次影响所有用户的默认体验
2. **加新预置岗位**：`kind: preset` 的 id 加进 `runtime/seeds/agents.rs`（app 首次启动会 seed 这些）
3. **不要写死具体 provider**：`provider_priority` 是**优先级列表**而不是硬绑定，用户机上可能装了不同的 CLI
4. **权限最小化**：`permission_defaults` 里的 command / network 只列**这个岗位真的需要的**，其余走 prompt 兜底

## 关联文档

- `docs/OPC-产品定义.md § 4/5` —— 岗位设计原则
- `runtime/migrations/app/0001_init.sql` —— `agents` 表 schema
- `templates/standard-software-delivery.yaml` —— 标准工作流引用哪些岗位
