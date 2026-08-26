# agents/ · 预置 AI 岗位模板

本目录是不令 OPC **预置的 9 位 AI 员工**的定义源。每个 YAML 文件对应一个岗位；由 `runtime/crates/opc-agent` 的 `load_agents_from_dir()` 解析、`seed_agents()` 写入 `app.sqlite` 的 `agents` 表（`kind='preset'`）——Tauri App **每次冷启动都会重新播种**（`src-tauri/src/lib.rs` 的 `get_app_db()`），改了这里的 YAML，用户下次开 App 就同步。

播种是幂等 upsert：内容没变不会让 `version` 无意义 +1（只在 `system_prompt` 变化时才递增）；且**永远不会覆盖用户 fork 过的同 id 行**（`ON CONFLICT ... WHERE kind='preset'` 守住）。

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
2. **加新预置岗位**：直接在本目录加一份 `<id>.yaml`（`kind: preset`）——`load_agents_from_dir()` 扫描整个目录，不需要额外注册
3. **不要写死具体 provider**：`provider_priority` 是**优先级列表**而不是硬绑定，用户机上可能装了不同的 CLI
4. **权限最小化**：`permission_defaults` 里的 command / network 只列**这个岗位真的需要的**，其余走 prompt 兜底

## 加载 / 播种 / 驱动（`runtime/crates/opc-agent`）

```rust
let defs = opc_agent::load_agents_from_dir("agents/")?;
opc_agent::seed_agents(&app_db, &defs).await?;                 // 幂等 upsert

// 真正跑一次（按 provider_priority 做 fallback）：
let agent = defs.iter().find(|d| d.id == "product-manager").unwrap();
let out = opc_agent::run_task(&registry, agent, "帮我写个健康 App 的 PRD", 4000).await?;
println!("{} 用了 {}: {}", agent.role, out.provider_id, out.response.text);
```

`run_task` 只做一次文本补全；不写 Artifact、不强制 Report.md 格式——那是 `opc-workflow`（P0.5+）接手时的事。

## 关联文档

- `docs/OPC-产品定义.md § 4/5` —— 岗位设计原则
- `runtime/migrations/app/0001_init.sql` —— `agents` 表 schema
- `runtime/README.md` §Agent 层 —— 加载/播种/驱动的设计说明
- `templates/standard-software-delivery.yaml` —— 标准工作流引用哪些岗位
