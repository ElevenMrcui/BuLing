# apps/desktop/ · 不令 OPC 桌面应用

Tauri 2 桌面壳 · React + TypeScript 前端 · Rust 后端（复用 `runtime/`）。

**当前状态：骨架已落地并跑通 E2E。** `cargo build`（`apps/desktop/src-tauri`）+ `pnpm --filter @buling/desktop build` 均通过；已联通十七个 IPC 命令：
- `opc_status` —— APP 库路径 / 迁移版本 / 已播种的预置岗位数
- `opc_providers` —— 扫描 `providers/*/manifest.toml`，返回每个 Provider 的可用性（只发现不执行；单个 Provider 构建失败不拖垮整个列表）
- `opc_agents` —— 列出 `app.sqlite.agents` 里的全部岗位；每次冷启动会先用 `agents/*.yaml` 重新播种（幂等，不覆盖用户 fork 过的行）
- `opc_create_project` —— 建项目（目录 + project.sqlite + app.sqlite 注册 + 默认团队 + 全部预置 Agent 实例化）；传 `template_id` 会顺带实例化对应工作流
- `opc_list_projects` —— 列出「项目中心」，最近打开的排最前
- `opc_project_workflow` —— 查一个项目最近一条工作流（不限于刚建的那次）——「工作流中心」作为独立页面打开任意已有项目时靠它找到工作流 id
- `opc_workflow_tasks` / `opc_workflow_ready_tasks` —— 列出一个工作流的全部/当前可执行任务节点
- `opc_workflow_run_task` —— 真的跑一次可执行的 Agent 节点（选 Provider → 执行 → 落盘登记 Artifact）
- `opc_workflow_gates` / `opc_workflow_approve_gate` / `opc_workflow_reject_gate` —— 评审红线：Gate 列表 + 人工通过/打回（永远由用户在界面上点，Runtime 不会自己调）
- `opc_task_claimable_tasks` / `opc_task_manual_tasks` —— 列出当前可认领（`auto-claim`）/ 待手动指派（`manual`）的节点
- `opc_task_claim` —— 真的执行一次认领（能力匹配打分，选最高分中标）
- `opc_task_assign_manually` —— 把一个 `manual` 节点指派给指定岗位
- `opc_task_run` —— 跑一次已经指派/认领好的节点（复用 `opc_workflow_run_task` 背后同一条执行链）

**前端已经是真实的多页应用**，不是单页堆卡片：固定侧边栏（13 个一级菜单，沿用产品定义的信息架构）+ 顶栏 + 内容区，6 个页面接了真实后端（控制台/项目中心/团队中心/工作流中心/模型中心 + 认领指派），其余 7 个（任务中心/产出中心/评审中心/技能中心/记忆中心/权限中心/日志中心/设置）用统一的"待建"占位页诚实占住导航位置——不假装功能存在，点进去会说清楚"这块还没接什么"，能接的地方给一个跳到"工作流中心"的按钮。见 §设计系统。

**窗口级冒烟 + 端到端真实功能都已验证**（Xvfb 虚拟显示 + release 二进制，非真机物理屏幕）：`cargo build --release` 编译出的二进制在 `DISPLAY=:99`（Xvfb）下真的启动、渲染出完整 UI，用 `xdotool` 模拟点击走了一遍真实流程——「项目中心」建项目（表单校验、`同时启动工作流` 勾选）→ 自动跳到「工作流中心」（项目已预选）→ 点「跑这个节点」→ **真的调用了本机的 `claude` CLI**（`opc-provider` 的 CLI adapter，进程可见）→ 收到响应后落盘、`prd` 节点状态变成"已完成"，全程没有 mock。深浅色主题切换、「团队中心」/「模型中心」列表渲染、「产出中心」等"待建"占位页也都截图确认过，非空白帧。过程中发现并修了一个真实崩溃：`icons/icon.png` 是个 8×8 的占位文件，被 `tao`/`gdk-pixbuf` 解析窗口图标时直接 panic（`data.len() must fit the width, height, and row_stride`）——用 `pnpm exec tauri icon` 从品牌色块 + "令" 字重新生成了整套桌面图标（`icons/*.png`/`.ico`/`.icns`），修完就能正常起窗口了。前端后来从手写 CSS 重做成 Tailwind + shadcn/ui（见 §设计系统 · ADR-007 附注）之后，用同一套 Xvfb 截图流程重新过了一遍：Radix Select 下拉、Radix Checkbox、深浅色 class 切换、`opc_project_workflow` 对"项目目录已被删除"这种边界情况的优雅降级（不崩溃，正确显示"这个项目还没有工作流"）都截图确认过。

**打包也验证过**：`pnpm exec tauri build -b deb -c '{"bundle":{"active":true}}'`（一次性覆盖 `tauri.conf.json` 里默认关闭的 `bundle.active`，没有改动提交的配置）成功产出 `不令 OPC_0.1.0_amd64.deb`（~7MB，`dpkg-deb --info` 校验元数据正常，依赖声明 `libwebkit2gtk-4.1-0`/`libgtk-3-0`）。要长期打开打包，把 `tauri.conf.json` 的 `bundle.active` 改成 `true` 即可，这是个有意的构建行为开关，不是 bug，本次没有改动它。

**仍未做**：真机（非虚拟显示）上的窗口验证；macOS/Windows 平台打包（这个 sandbox 是 Linux，只验证了 `.deb`；`.dmg`/`.msi` 需要对应平台环境）。

## 目录结构（现状）

```
apps/desktop/
├── README.md
├── package.json                    前端 npm 包（React 18 + TS + Vite + Tailwind + shadcn/ui 模式）
├── vite.config.ts                  Vite dev server + `@/` → `src/` 路径别名
├── tailwind.config.ts              主题 token 映射（HSL CSS 变量，见下）
├── postcss.config.js
├── index.html
├── tsconfig.json
│
├── src/                            React 源码
│   ├── main.tsx                    入口，挂载 <App/> + 引入 index.css
│   ├── index.css                   @tailwind 指令 + 设计令牌（HSL 变量，浅/深两套）
│   ├── App.tsx                     壳：Sidebar + 顶层数据拉取（status/providers/agents/projects）+ 视图分发
│   ├── Sidebar.tsx                 侧边栏导航（13 个一级菜单 + 深浅色三态切换）
│   ├── nav.ts                      NAV_ITEMS（导航项定义，图标是 lucide-react 组件）+ COMING_SOON_COPY
│   ├── types.ts                    跟 Tauri IPC 返回值一一对应的 TS 接口
│   ├── ipc.ts                      IPC 命令的类型化封装（`ipc.createProject()` 等）+ `errorMessage()`
│   ├── theme.ts                    深浅色三态 hook（class 策略 + matchMedia 解"系统"）
│   ├── lib/utils.ts                `cn()`（clsx + tailwind-merge，shadcn 标配 helper）
│   ├── components/
│   │   ├── shared.tsx              跨页面共用：EmptyState / StatCard / RedlineBanner / Spinner
│   │   └── ui/                     shadcn/ui 组件源码（不是 npm 依赖，复制进仓库这份）
│   │       ├── button.tsx · input.tsx · select.tsx · checkbox.tsx · label.tsx · card.tsx · badge.tsx
│   └── views/                      对应一级菜单的页面组件
│       ├── ControlPanel.tsx        🏛 控制台——统计卡片 + 最近项目 + 存储层状态
│       ├── ProjectCenter.tsx       📁 项目中心——建项目表单 + 项目列表
│       ├── TeamCenter.tsx          👥 团队中心——9 位预置岗位列表
│       ├── WorkflowCenter.tsx      🧩 工作流中心——选项目 → Gate 通过/打回 + 任务节点跑/认领/指派
│       ├── ModelCenter.tsx         🤖 模型中心——Provider 发现列表
│       └── ComingSoon.tsx          其余 7 个菜单项共用的诚实占位页（任务/产出/评审/技能/记忆/权限/日志中心 + 设置）
│
└── src-tauri/                      Tauri 后端（Rust，薄壳，见 src/lib.rs）
    ├── Cargo.toml                  引用 runtime/crates/{opc-storage,opc-provider,opc-agent,opc-project,opc-workflow,opc-task}
    ├── tauri.conf.json             bundle.active 默认 false（见上文"打包也验证过"）
    ├── icons/                      桌面图标（品牌色块 + "令" 字，pnpm exec tauri icon 生成）
    └── src/lib.rs                  全部 IPC command（薄壳，转发到 runtime crate，业务逻辑不落这层）
```

## 设计系统

前端按 [`docs/OPC-架构决策.md` ADR-007](../../docs/OPC-架构决策.md) 落地：**React 18 + Tailwind CSS 3 + shadcn/ui 模式**（`@radix-ui/react-*` 无障碍原语 + `class-variance-authority` 变体 + `tailwind-merge`/`clsx`），组件源码复制进 `src/components/ui/`，不是 npm 依赖，改起来完全可控。图标用 `lucide-react`。视觉语言沿用 [`legacy/prototype/index.html`](../../legacy/prototype/index.html) 已经验证过的一套 Apple-HIG 风格 token 系统（配色 / 圆角 / 阴影分级 / 毛玻璃侧边栏），换算成 shadcn 惯例的 HSL CSS 变量放进 `src/index.css`——**复用的是审美和数值，不是代码**；原型里"认领意愿强度条""蚁群/蜂群机制说明"这类描述旧版"去中心化协作平台"的产品隐喻没有搬过来，因为新 OPC 的 `opc-task` 认领机制是确定性的能力匹配打分，不是那套叙事。深浅色三态（浅色/深色/跟随系统）用 `class` 策略实现——`theme.ts` 里 `matchMedia` 现解一遍"系统"该是浅是深，是 `next-themes` 一类库的标准做法。

**信息架构**：侧边栏 13 个一级菜单严格对应 `CLAUDE.md` 里定义的产品菜单结构，`nav.ts` 的 `NAV_ITEMS` 是唯一权威源。每一项标 `ready: true/false`——`false` 的项目在导航上挂"待建"标签，点进去是 `ComingSoonView`（诚实说明"这块还没接什么"+ 有真实功能时给一个跳转按钮），不是空白页或假装能用的死链接。「任务中心」「评审中心」目前都指向「工作流中心」——认领/指派、Gate 通过/打回这些功能是真的，只是这一版还按"项目 + 工作流"维度组织，没有独立出跨项目的汇总视图。

**硬约束在界面上落地**：评审红线用 `RedlineBanner` 组件固定渲染在「工作流中心」的 Gate 区块顶部（红色背景、警示图标，不能配置隐藏）；Gate 只有"通过"/"打回"两个按钮，没有任何"全部通过"或自动化选项。

**已知的能力边界会诚实展示**：`frontend_dev`/`backend_dev` 这类节点认领后点"跑这个节点"会报错（后端 `Error::UnsupportedOutputShape`——一次 AI 调用产不出一整个目录的多份源码文件），错误信息原样展示在页面上（`hint`/`err-text` 文案区），不会白屏或崩溃。

## 与 runtime/ 的关系

Tauri App 后端 = `src-tauri/src/lib.rs` 的 IPC command 薄壳 + `runtime/crates/*` 打包成的 lib。所有真正干活的逻辑都在 `runtime/`；`src-tauri/` 不写业务逻辑。

**为什么这样切？** 让 `runtime/` 可以脱离 Tauri 单独用（比如未来做 CLI 版 / 无头模式 / 服务器版）。桌面壳换 Electron 或换 Web 都不影响 runtime。
