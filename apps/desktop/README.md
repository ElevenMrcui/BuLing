# apps/desktop/ · 不令 OPC 桌面应用

Tauri 2 桌面壳 · React + TypeScript 前端 · Rust 后端（复用 `runtime/`）。

**当前状态：骨架已落地并跑通 E2E。** `cargo build`（`apps/desktop/src-tauri`）+ `pnpm --filter @buling/desktop build` 均通过；已联通十六个 IPC 命令：
- `opc_status` —— APP 库路径 / 迁移版本 / 已播种的预置岗位数
- `opc_providers` —— 扫描 `providers/*/manifest.toml`，返回每个 Provider 的可用性（只发现不执行；单个 Provider 构建失败不拖垮整个列表）
- `opc_agents` —— 列出 `app.sqlite.agents` 里的全部岗位；每次冷启动会先用 `agents/*.yaml` 重新播种（幂等，不覆盖用户 fork 过的行）
- `opc_create_project` —— 建项目（目录 + project.sqlite + app.sqlite 注册 + 默认团队 + 全部预置 Agent 实例化）；传 `template_id` 会顺带实例化对应工作流
- `opc_list_projects` —— 列出「项目中心」，最近打开的排最前
- `opc_workflow_tasks` / `opc_workflow_ready_tasks` —— 列出一个工作流的全部/当前可执行任务节点
- `opc_workflow_run_task` —— 真的跑一次可执行的 Agent 节点（选 Provider → 执行 → 落盘登记 Artifact）
- `opc_workflow_gates` / `opc_workflow_approve_gate` / `opc_workflow_reject_gate` —— 评审红线：Gate 列表 + 人工通过/打回（永远由用户在界面上点，Runtime 不会自己调）
- `opc_task_claimable_tasks` / `opc_task_manual_tasks` —— 列出当前可认领（`auto-claim`）/ 待手动指派（`manual`）的节点
- `opc_task_claim` —— 真的执行一次认领（能力匹配打分，选最高分中标）
- `opc_task_assign_manually` —— 把一个 `manual` 节点指派给指定岗位
- `opc_task_run` —— 跑一次已经指派/认领好的节点（复用 `opc_workflow_run_task` 背后同一条执行链）

前端首页展示项目中心 + 工作流中心 + 三块状态，作为"Runtime + Storage + Provider + Agent + Project + Workflow + Task 七层打通"的最小可视证据。

**窗口级冒烟已验证**（Xvfb 虚拟显示 + release 二进制，非真机物理屏幕）：`cargo build --release` 编译出的二进制在 `DISPLAY=:99`（Xvfb）下真的启动、渲染出了完整 UI（项目中心表单 + 存储层状态卡片，`APP 库版本=2`/`就绪=✓`/`预置岗位=9 位`，证明后端真的初始化了 SQLite 并跑完迁移+播种），截图确认非空白帧。过程中发现并修了一个真实崩溃：`icons/icon.png` 是个 8×8 的占位文件，被 `tao`/`gdk-pixbuf` 解析窗口图标时直接 panic（`data.len() must fit the width, height, and row_stride`）——用 `pnpm exec tauri icon` 从品牌色块 + "令" 字重新生成了整套桌面图标（`icons/*.png`/`.ico`/`.icns`），修完就能正常起窗口了。

**打包也验证过**：`pnpm exec tauri build -b deb -c '{"bundle":{"active":true}}'`（一次性覆盖 `tauri.conf.json` 里默认关闭的 `bundle.active`，没有改动提交的配置）成功产出 `不令 OPC_0.1.0_amd64.deb`（~7MB，`dpkg-deb --info` 校验元数据正常，依赖声明 `libwebkit2gtk-4.1-0`/`libgtk-3-0`）。要长期打开打包，把 `tauri.conf.json` 的 `bundle.active` 改成 `true` 即可，这是个有意的构建行为开关，不是 bug，本次没有改动它。

**仍未做**：真机（非虚拟显示）上的窗口验证；macOS/Windows 平台打包（这个 sandbox 是 Linux，只验证了 `.deb`；`.dmg`/`.msi` 需要对应平台环境）。

## 规划的目录

```
apps/desktop/
├── README.md
├── package.json                    前端 npm 包
├── vite.config.ts                  Vite dev server
├── index.html
├── tsconfig.json
│
├── src/                            React 源码
│   ├── main.tsx                    入口
│   ├── App.tsx
│   ├── pages/                      对应一级菜单
│   │   ├── ControlPanel.tsx        🏛 控制台
│   │   ├── ProjectCenter.tsx       📁 项目中心
│   │   ├── TeamCenter.tsx          👥 团队中心
│   │   ├── TaskCenter.tsx          📋 任务中心
│   │   ├── WorkflowCenter.tsx      🧩 工作流中心
│   │   ├── ArtifactCenter.tsx      📦 产出中心
│   │   ├── ReviewCenter.tsx        🔍 评审中心
│   │   ├── SkillCenter.tsx         🧰 技能中心
│   │   ├── MemoryCenter.tsx        🧠 记忆中心
│   │   ├── ModelCenter.tsx         🤖 模型中心
│   │   ├── PermissionCenter.tsx    🔒 权限中心
│   │   ├── LogCenter.tsx           📊 日志中心
│   │   └── Settings.tsx            ⚙️ 设置
│   ├── components/                 通用组件（shadcn/ui 二次封装）
│   ├── ipc/                        Tauri command 客户端封装
│   ├── stores/                     Zustand / Jotai 状态
│   └── styles/                     Tailwind 配置
│
└── src-tauri/                      Tauri 后端（Rust）
    ├── Cargo.toml                  引用 workspace = "../../runtime"
    ├── tauri.conf.json
    ├── build.rs
    └── src/
        ├── main.rs                 Tauri App 入口
        └── commands/               IPC command 实现（薄壳，转发到 runtime crates）
            ├── projects.rs
            ├── agents.rs
            ├── providers.rs
            ├── tasks.rs
            └── workflows.rs
```

## 技术栈

| 模块 | 选型 |
|---|---|
| 桌面壳 | Tauri 2 |
| 前端框架 | React 18 |
| 语言 | TypeScript strict |
| UI 库 | shadcn/ui + Tailwind CSS |
| 状态管理 | Zustand（简单）· 或 TanStack Query（服务端状态） |
| 编辑器 | Monaco Editor（PRD / 代码 / 配置查看） |
| 图 / 流程 | React Flow（工作流可视化 · P1） |
| 主题 | 浅 / 深 / 跟随系统三态（沿用旧原型 CSS token 语言）|

## 与 runtime/ 的关系

Tauri App 后端 = `src-tauri/` 的 Rust 代码 + `runtime/crates/opc-runtime` 打包成的 lib。所有真正干活的逻辑都在 `runtime/`；`src-tauri/` 只做 IPC command 薄壳和 Tauri 事件订阅。

**为什么这样切？** 让 `runtime/` 可以脱离 Tauri 单独用（比如未来做 CLI 版 / 无头模式 / 服务器版）。桌面壳换 Electron 或换 Web 都不影响 runtime。

## 已决定的 UI 原则

1. **深浅色 tri-state**：`data-theme="light"` / `data-theme="dark"` / 无属性跟随系统（沿用旧原型三态 token）
2. **中文优先，简洁 tone**：菜单和文案全中文；主标题小；密度中等
3. **主语用岗位**：所有系统文案主语用"产品经理"/"测试" 而非"Agent"
4. **红线可见**：评审中心永远有明显的"必须人工"标签，不给"一键全部通过"的按钮
5. **权限即视觉**：Agent 卡片上永远可见"我现在能干什么"（file / command / network / git / docker / mcp 六个小灯）

旧原型 `legacy/prototype/index.html` 的视觉语言（配色 / 卡片 / 图标 / token）作为参考——**不复制代码，但复用审美**。
