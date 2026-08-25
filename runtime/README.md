# runtime/ · OPC 核心 Runtime（Rust · Tauri 后端）

不令 OPC 的执行发动机。跑在 Tauri App 里作为主进程后端，负责一切与"操作用户电脑"和"调 AI"有关的事。

**语言**：Rust
**构建**：Cargo workspace（多 crate）
**数据库**：SQLite（app 级 + project 级双库）+ sqlx（编译时校验 SQL）
**并发**：Tokio 异步
**前端接口**：Tauri IPC command

## 目录结构

```
runtime/
├── Cargo.toml                    Workspace 根（P0 后创建）
│
├── migrations/
│   ├── README.md
│   ├── app/                      APP 级 SQLite migration（跨项目 · 全局）
│   │   └── 0001_init.sql         ✅ P0 完成
│   └── project/                  PROJECT 级 SQLite migration（每项目一份）
│       └── 0001_init.sql         ✅ P0 完成
│
├── crates/                       各模块 crate（P0 逐个补）
│   ├── opc-storage/              SQLite + sqlx · migration runner · repos
│   ├── opc-agent/                Agent 定义加载 / 实例化 / 状态
│   ├── opc-project/              Project 创建 / 打开 / 归档 · .opc/ 目录布局
│   ├── opc-task/                 Task 生命周期 · TaskRun 调度 · 认领意愿分
│   ├── opc-workflow/             DAG 引擎 · 节点触发 · Gate 处理 · 循环 / 条件
│   ├── opc-provider/             Provider Router · CLI / API / Local 抽象
│   ├── opc-tool/                 Tool 层（fs / shell / git / http / db）+ 权限系统
│   ├── opc-mcp/                  MCP client
│   ├── opc-privacy/              隐私哨兵：外发拦截 / 数据脱敏
│   ├── opc-audit/                ExecutionLog append-only 写入
│   └── opc-runtime/              总装：把上面所有 crate 编成一个 lib，供 Tauri 调
│
└── seeds/                        首次启动播种数据
    └── agents.rs                 从 agents/*.yaml 读并写入 app.sqlite
```

## 双层 SQLite 布局

**为什么两层？** 为了让"项目"是**独立可迁移单元**——把整个项目文件夹拷贝到别的机器还能用；不需要跨项目的公共设置。

| 层 | 位置 | 存什么 |
|---|---|---|
| **APP** | `~/.opc/db.sqlite` | 用户资料 · Provider · Agent 库 · 项目注册表 · 设置 · 全局日志 |
| **PROJECT** | `<project-root>/.opc/project.sqlite` | Team · Task · Artifact · Review · Workflow · 项目内 Memory · 项目日志 |

`~/.opc/` 目录布局：
```
~/.opc/
├── db.sqlite                     APP 级主库
├── db.sqlite-wal                 WAL 副本（sqlx 用）
└── logs/                         系统日志
```

`<project>/.opc/` 目录布局：
```
<project>/.opc/
├── project.sqlite                项目主库
├── project.json                  项目元数据（人可读，冗余 project_meta 表）
├── agents/                       fork 出的 agent 实例的定制 prompt（大文本）
├── artifacts/                    Artifact 版本二进制（小文件）
└── logs/                         项目日志
```

## Migration 执行策略

- APP 级：Tauri App 启动时用 sqlx migrate 跑 `migrations/app/*.sql`
- PROJECT 级：`opc-project` 里 `create_project()` / `open_project()` 时跑 `migrations/project/*.sql`
- 版本追踪在各库的 `_migrations` 表；已应用的跳过

## API Key 安全

**绝不落表**。密钥统一走 OS Keychain：

- macOS: `security add-generic-password`（`keyring` crate）
- Windows: Credential Manager（`keyring` crate）
- Linux: Secret Service via D-Bus（`keyring` crate）

`providers.api_credential_ref` 只存 keychain 里那个 item 的 id（比如 `opc.provider.openai.default`），实际密钥每次用时从 keychain 现取。

## P0 落地顺序（等桌面壳就位后）

1. `opc-storage` —— 连库 / 跑 migration / 基础 repos
2. `opc-provider` —— 迁入现有 `apps/local-gateway` 的 CLI 探测逻辑
3. `opc-agent` —— 加载 `agents/*.yaml` seed 到 app.sqlite
4. `opc-project` —— 创建 project 目录 + project.sqlite
5. `opc-workflow` —— 加载 `templates/*.yaml` + DAG 实例化
6. `opc-task` + `opc-runtime` —— 让一个节点跑起来（Agent + Provider + Tool）
7. `opc-tool` —— 至少接通 fs.write / shell.run
8. `opc-privacy` + `opc-audit` —— 每次外发和 tool 调用都写日志
