# 不令 OPC · 架构决策记录（ADR）

本文记录 P0 立项时锁定的关键技术决策。每条决策格式：**决定 → 原因 → 备选与否决 → 影响**。

后续更改需**新增一条 ADR**（不覆盖旧条目），保留决策演进史。

---

## ADR-001 · 桌面框架：Tauri 2（不选 Electron）

**决定**：桌面应用外壳用 Tauri 2。

**原因**：
1. **包尺寸**：Tauri 3-5 MB vs Electron 100 MB+。私人 AI 应用要"下载即用"，Electron 太重。
2. **冷启动**：Tauri 用系统 WebView，冷启 < 1s；Electron 打包 Chromium，2-4s。
3. **内存**：空 App Tauri ~50 MB，Electron ~200 MB。OPC 要在用户 Mac 上长时间常驻。
4. **Rust 后端天然契合 Local-First**：文件系统 / 进程 / Git / SQLite / Keychain 走 Rust 又快又安全，比 Node 版少一大堆异步陷阱。
5. **安全**：Tauri 默认拒绝所有 IPC，明示才允许；Electron 默认全开，需手动收权。

**备选与否决**：
- **Electron**：能复用现有 TS 代码但打包成本 20 倍，长期维护成本更高。P0 后期发现有必须复用的旧 Node 依赖再回来评估。
- **Wails**（Go 后端）：Go 生态在 AI / SQLite / crypto 上不如 Rust 成熟。
- **纯 Web 应用**：违反 Local First 原则，用户无法长期驻留。

**影响**：
- `runtime/` 用 Rust，不用 TypeScript
- 前端 IPC 通过 Tauri `invoke()`，不是 HTTP
- 已有的 `apps/local-gateway`（Node/TS）会**并存**——短期它作为独立 daemon 继续跑；P0 后期把探测逻辑迁到 Rust in-process

---

## ADR-002 · 本地库：SQLite + sqlx（不选纯 rusqlite，不选完整 ORM）

**决定**：SQLite 存储 + sqlx crate（编译时校验 SQL）。

**原因**：
1. **sqlx 编译时校验 SQL**：写错字段名 / 类型不匹配在 `cargo build` 时就报错，运行时 crash 概率极低
2. **仍然 SQL-first**：所有查询就是 SQL，可读、可迁移、可用 sqlite3 CLI 直接看
3. **原生 async**：与 Tokio 完全契合
4. **有 migration 支持**：`sqlx migrate` 内置，不用另装工具
5. **零依赖 SQLite**：sqlx 打包 SQLite 到二进制，用户不需要装 sqlite3

**备选与否决**：
- **rusqlite** 直连：更轻，但每个查询都需要手写映射代码；错误直到运行时才发现
- **SeaORM**：完整 ORM，方便但**性能开销 + 学习成本**，SQLite 场景大炮打蚊子
- **Diesel**：老牌 ORM，async 支持后加的，不如 sqlx 原生

**影响**：
- 所有 SQL 集中在 `runtime/migrations/*.sql` 和 `runtime/crates/*/sql/*.sql`
- CI 里跑 `cargo sqlx prepare` 生成离线校验数据
- 团队成员本地必须至少能执行一次 migration 才能编译

---

## ADR-003 · 双层 SQLite：APP 库 + PROJECT 库

**决定**：不用一个大库存所有东西；APP 全局一个库（`~/.opc/db.sqlite`），每个项目一个独立库（`<project>/.opc/project.sqlite`）。

**原因**：
1. **项目是可迁移单元**：把项目文件夹拷到别的机器（USB / iCloud / Git）就能带走一切；跨项目公共数据不受影响
2. **权限隔离天然**：项目库文件权限就是项目文件夹权限，无需在应用层再做租户隔离
3. **备份 / 归档简单**：`.opc/project.sqlite` 跟着项目走
4. **性能**：单一大库随项目数增长会变慢；分库天然横向可扩

**备选与否决**：
- **单库**：简单但违反"项目可迁移"原则
- **单库 + tenant_id**：加了个字段却没换来可迁移性；坏处一样有

**影响**：
- Runtime 需管理两种连接池（app pool + project pool 数组）
- Migration 也分两套（`migrations/app/` + `migrations/project/`）
- 跨库 JOIN 不能做——需要跨库聚合时应用层组装（少数场景，可接受）

---

## ADR-004 · API Key 存储：OS Keychain（不落数据库）

**决定**：所有 API Key（Anthropic / OpenAI / GLM / Qwen / DeepSeek …）走 OS Keychain：
- macOS: Keychain via `security` API
- Windows: Credential Manager via `wincred`
- Linux: Secret Service via D-Bus（`libsecret` / KWallet / GNOME Keyring）

Rust 端用 `keyring` crate 统一封装。

数据库 `providers` 表只存 keychain item 的 id 引用（如 `opc.provider.openai.default`）。

**原因**：
1. **不落表** = 备份 / 拷贝项目文件夹时不会误泄
2. 用户已经信任 OS 的密钥管理；再造轮子只会更弱
3. 三大 OS 都有成熟原生方案；`keyring` crate 统一接口

**备选与否决**：
- **AES-256-GCM 加密后落 SQLite**（旧 `apps/api/src/modules/providers/key-crypto.ts` 的做法）：作为**兜底选项保留**（Linux 无 D-Bus 场景 / 用户主动选择"文件加密而非 Keychain"），但不是默认
- **明文落表**：出局

**影响**：
- 每次调用 Provider 都要 keychain 读取（有系统弹窗风险 → 首次访问用户授权 always allow）
- CI / 无头环境不能读 keychain → 需要环境变量 fallback（`OPC_KEY_ANTHROPIC=...`）

---

## ADR-005 · Provider 层抽象：CLI / API / Local 三种（+ Hybrid 路由）

**决定**：Runtime 里一个 `Provider` trait，三种 impl：`CliProvider` / `ApiProvider` / `LocalProvider`。Agent 绑定 `provider_priority: [id1, id2, ...]` 做 fallback 路由。

**原因**：
1. **零额外账单是核心卖点**——用户已装 Claude Code / Codex，就用他登录态，不再收 Key
2. **端上私密可控**——本地模型 Provider 天然满足"数据不出机"
3. **Agent 与 Provider 解耦**——换模型不改 Agent；同 Agent 可 A/B 不同 Provider

**备选与否决**：
- **只做 API 直连**：需要用户额外配 Key，OPC 价值降一半
- **只做 CLI**：本地模型 / 无 CLI 的服务（GLM / Qwen）无法接
- **让 Agent 硬绑 Model**：换模型要改 Agent 定义，不灵活

**影响**：
- CLI Provider 要处理每家 CLI 的输出格式差异（Claude Code 有 `--output-format json`，Codex 不同，Gemini 又不同）
- 每加一家 Provider 就要写一个 adapter；`providers/` 目录规划见其 README

---

## ADR-006 · 依赖沿用：`apps/local-gateway` 与 `packages/cli-registry` 短期保留

**决定**：新架构 P0 阶段**不迁**这两个组件，它们继续按当前形态（Node/TS · Fastify · 127.0.0.1）运行。

**原因**：
1. `packages/cli-registry`（10 家 AI CLI 签名注册表）**数据本身**是资产，Rust 那边写一个 loader 直接读同一份 YAML/JSON 即可，不需要重造
2. `apps/local-gateway` 已经通过 CORS + Token 走通了本地网关协议，Tauri App 起步阶段可以直接调这个 daemon 拿探测结果
3. 时机成熟（P0 后期）再把探测逻辑用 Rust 重写融入 `runtime/crates/opc-provider`

**影响**：
- 短期仓库有两种语言：主体 Rust + 小旧 Node（local-gateway + cli-registry）
- 用户如果只跑 Tauri App，local-gateway 会被 App 内嵌启动（而不是让用户手动 `make gateway`）——P0 后期实现

---

## ADR-007 · 前端：React + shadcn/ui + Tailwind

**决定**：Tauri 前端用 React 18 + shadcn/ui + Tailwind CSS。

**原因**：
1. React 生态最成熟；组件资源最多
2. shadcn/ui 是"复制到你项目里"式的，不是 npm 依赖，完全可控 · 不受升级绑架
3. Tailwind 让主题 tokens（沿用旧原型的 `--bg` / `--ink` / `--accent` 那套）非常自然
4. TypeScript strict + noUncheckedIndexedAccess 保持类型健壮

**备选与否决**：
- **Svelte / SolidJS**：更小更快但社区组件少，招 AI 时也不如 React 熟
- **Vue**：也可以，但目前团队 React 更熟
- **HTMX + Alpine**：太保守，Tauri IPC 复杂交互撑不住

---

## ADR-008 · 命名：主品牌沿用「不令」，产品线定位为 OPC

**决定**：
- 主品牌：**不令**（保留《论语》"其身正，不令而行"的品牌哲学）
- 产品线：**OPC**（One Person Company）
- 完整名：**不令 OPC** / 简称 `OPC` / 英文 `BuLing OPC`

**原因**：
1. "不令" 二字**天然对应"AI 团队不令而行"**的产品机制，是白捡的品牌资产
2. 但"不令"过于抽象，普通用户理解成本高；用 "OPC" 做副定位一句话把功能讲清楚
3. 主 + 副的组合可以走"品牌沉淀 + 定位清晰"两不误

**备选与否决**：
- 完全放弃"不令"，纯用 "OPC"：丢掉品牌沉淀，字母缩写辨识度低
- 用纯中文如"分身 / 搭子 / 一府"：品牌较薄，无国际化空间

**影响**：
- 官网 / 商店 / 文档标题用 `不令 OPC`
- 应用内简称 `OPC`
- 一级菜单命名统一 `XX 中心`；AI 岗位命名统一中文短名（不加 `Agent` 后缀）
