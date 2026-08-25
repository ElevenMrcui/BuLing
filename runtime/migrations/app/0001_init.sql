-- ============================================================================
-- 不令 OPC · APP 级 SQLite Schema · 0001_init
--
-- 存放位置：~/.opc/db.sqlite（跨项目 · 全局 · 单用户）
--
-- 存什么：
--   · 全局用户资料
--   · Provider（AI 能力来源：CLI / API / Local）配置
--   · Agent 库（跨项目复用的岗位模板）
--   · 项目注册表（app 知道用户有哪些项目 + 各自路径）
--   · 应用设置
--   · 跨项目记忆（少量、可选）
--   · 全局执行日志（append-only 审计）
--
-- 不存什么（放 project.sqlite 里）：
--   · 单项目的 Team / Task / Artifact / Review / Workflow / 项目内 Memory
--
-- 硬约束：
--   · API Key 不落表 —— 一律走 OS Keychain（macOS / Windows / Linux）
--     此表 `providers.credential_ref` 只存 keychain 的引用 id
--   · execution_logs / audit-only 表用 CHECK 约束禁止 UPDATE（应用层执行）
-- ============================================================================

PRAGMA foreign_keys = ON;
PRAGMA journal_mode = WAL;

-- ----------------------------------------------------------------------------
-- 单用户资料（本机唯一一行，OPC 是 single-tenant 桌面应用）
-- ----------------------------------------------------------------------------
CREATE TABLE user_profile (
    id            INTEGER PRIMARY KEY CHECK (id = 1),  -- 强制只能一行
    display_name  TEXT NOT NULL,
    email         TEXT,                                -- 可选，仅用于文档署名
    avatar_path   TEXT,                                -- 本机文件路径
    created_at    TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at    TEXT NOT NULL DEFAULT (datetime('now'))
);

-- ----------------------------------------------------------------------------
-- 应用设置（Key-Value；主题 / 语言 / 隐私哨兵默认策略 / 默认工作目录）
-- ----------------------------------------------------------------------------
CREATE TABLE settings (
    key         TEXT PRIMARY KEY,
    value       TEXT NOT NULL,             -- JSON 或简单字符串
    updated_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

-- ----------------------------------------------------------------------------
-- Provider · AI 能力来源
-- 四种：cli / api / local / hybrid
-- ----------------------------------------------------------------------------
CREATE TABLE providers (
    id              TEXT PRIMARY KEY,                       -- 稳定 id, e.g. "claude-code"
    display_name    TEXT NOT NULL,                          -- "Claude Code"
    vendor          TEXT NOT NULL,                          -- "Anthropic"
    kind            TEXT NOT NULL CHECK (kind IN ('cli','api','local','hybrid')),
    status          TEXT NOT NULL DEFAULT 'unknown'
                    CHECK (status IN ('connected','untested','error','unknown','not-installed')),

    -- CLI Provider 用（本地网关发现结果）
    cli_binary      TEXT,                                   -- 'claude' / 'codex'
    cli_path        TEXT,                                   -- 探测到的绝对路径
    cli_version     TEXT,                                   -- '2.1.241'

    -- API Provider 用
    api_base_url    TEXT,                                   -- 'https://api.anthropic.com'
    api_credential_ref TEXT,                                -- Keychain item id, e.g. 'opc.provider.openai.default'

    -- Local Provider 用
    local_base_url  TEXT,                                   -- 'http://localhost:11434/v1'

    -- 通用元数据
    capabilities    TEXT,                                   -- JSON: 支持的能力标签 ['long-context','vision','tool-use',...]
    default_model   TEXT,                                   -- 该 provider 默认用哪个 model
    models          TEXT,                                   -- JSON 数组：可用 model 列表

    -- 敏感度约束：能挂哪些 sensitivity 的 Agent
    allowed_sensitivity TEXT NOT NULL DEFAULT 'low,medium,high'
                        CHECK (allowed_sensitivity IN ('low','low,medium','low,medium,high')),

    -- 隐私声明（面向用户展示的一句话）
    privacy_note    TEXT,

    last_tested_at  TEXT,
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX providers_kind_idx ON providers (kind);
CREATE INDEX providers_status_idx ON providers (status);

-- ----------------------------------------------------------------------------
-- Agent 库 · 跨项目复用的岗位模板
-- 具体项目里挂载的 Agent 实例（可对模板 fork + 调 prompt）在 project.sqlite
-- ----------------------------------------------------------------------------
CREATE TABLE agents (
    id              TEXT PRIMARY KEY,                       -- 'product-manager' 等 (预置) 或 uuid (用户自建)
    kind            TEXT NOT NULL CHECK (kind IN ('preset','user')),
    role            TEXT NOT NULL,                          -- '产品经理' (中文短名)
    display_name    TEXT NOT NULL,                          -- 展示名，可与 role 同或起花名
    avatar          TEXT,                                   -- emoji 或 file path
    system_prompt   TEXT NOT NULL,                          -- 岗位人格 / 责任 / 输出格式约束
    responsibilities TEXT NOT NULL,                         -- JSON 数组：职责清单
    expected_outputs TEXT NOT NULL,                         -- JSON: [{ path: 'docs/product/PRD.md', kind: 'markdown' }, ...]

    -- 能力标签（用于自主认领的意愿分匹配 + Provider 路由）
    capabilities    TEXT NOT NULL DEFAULT '[]',              -- JSON 数组

    -- 敏感度：high 只能挂 local provider（隐私哨兵硬拦）
    sensitivity     TEXT NOT NULL DEFAULT 'medium'
                    CHECK (sensitivity IN ('low','medium','high')),

    -- Skills / Tools / MCP —— P0 阶段先内嵌 JSON，P1 拆表
    skills          TEXT NOT NULL DEFAULT '[]',              -- JSON: ['coding','doc-writing',...]
    tools           TEXT NOT NULL DEFAULT '[]',              -- JSON: ['fs','shell','git','http',...]
    mcp_servers     TEXT NOT NULL DEFAULT '[]',              -- JSON: ['github','notion',...]

    -- Provider 路由：按优先级尝试，第一个可用命中
    provider_priority TEXT NOT NULL DEFAULT '[]',           -- JSON: ['claude-code','codex','anthropic-api']

    -- 默认权限位（细粒度用 permissions 表覆盖）
    permission_defaults TEXT NOT NULL DEFAULT '{}',         -- JSON: { file:'project-only', git:['status','diff','commit'], ... }

    version         INTEGER NOT NULL DEFAULT 1,             -- 用户改了 prompt 递增
    is_active       INTEGER NOT NULL DEFAULT 1,

    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX agents_kind_idx ON agents (kind);

-- ----------------------------------------------------------------------------
-- 项目注册表 · APP 知道用户有哪些项目 + 每个项目工作目录在哪
-- ----------------------------------------------------------------------------
CREATE TABLE projects (
    id              TEXT PRIMARY KEY,                       -- uuid
    slug            TEXT NOT NULL UNIQUE,                   -- 'health-app'
    display_name    TEXT NOT NULL,                          -- '健康管理 App'
    root_path       TEXT NOT NULL UNIQUE,                   -- '/Users/foo/Projects/health-app'
    template_id     TEXT,                                   -- 用哪个 workflow template 起的
    goal            TEXT,                                   -- 用户最初输入的目标
    status          TEXT NOT NULL DEFAULT 'active'
                    CHECK (status IN ('active','archived','deleted')),
    starred         INTEGER NOT NULL DEFAULT 0,
    last_opened_at  TEXT,
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX projects_status_idx ON projects (status);
CREATE INDEX projects_last_opened_idx ON projects (last_opened_at DESC);

-- ----------------------------------------------------------------------------
-- 跨项目记忆（可选 · 少量） · e.g. 用户偏好、跨项目的技术偏好、命名习惯
-- 项目内记忆放 project.sqlite 的 memory_entries
-- ----------------------------------------------------------------------------
CREATE TABLE cross_project_memory (
    id           TEXT PRIMARY KEY,
    scope        TEXT NOT NULL,                             -- 'user','agent:product-manager'
    key          TEXT NOT NULL,
    value        TEXT NOT NULL,                             -- JSON 或纯文本
    created_at   TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at   TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE (scope, key)
);

-- ----------------------------------------------------------------------------
-- 全局执行日志 · append-only 审计线
-- （项目级更细的 log 在 project.sqlite）
-- ----------------------------------------------------------------------------
CREATE TABLE execution_logs (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    ts           TEXT NOT NULL DEFAULT (datetime('now')),
    project_id   TEXT,                                       -- 可能为 null（app 级操作）
    actor        TEXT NOT NULL,                              -- 'user' / 'agent:product-manager' / 'system'
    action       TEXT NOT NULL,                              -- 'provider.test' / 'project.create' / 'privacy.block'
    target       TEXT,                                       -- 被作用对象
    input        TEXT,                                       -- JSON: 简要输入
    output       TEXT,                                       -- JSON: 简要输出
    result       TEXT NOT NULL CHECK (result IN ('ok','error','blocked','confirmed','denied')),
    error        TEXT,                                       -- 失败原因
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE SET NULL
);

CREATE INDEX exec_log_ts_idx ON execution_logs (ts DESC);
CREATE INDEX exec_log_project_idx ON execution_logs (project_id);
CREATE INDEX exec_log_actor_idx ON execution_logs (actor);

-- ----------------------------------------------------------------------------
-- 迁移追踪
-- ----------------------------------------------------------------------------
CREATE TABLE _migrations (
    version     INTEGER PRIMARY KEY,
    name        TEXT NOT NULL,
    applied_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

INSERT INTO _migrations (version, name) VALUES (1, '0001_init');

-- ============================================================================
-- 内置种子数据（app 首次启动时）
-- ============================================================================

-- 单用户占位（首次启动会被 onboarding 覆盖）
INSERT INTO user_profile (id, display_name) VALUES (1, 'CEO');

-- 默认设置
INSERT INTO settings (key, value) VALUES
    ('theme', 'system'),
    ('language', 'zh-CN'),
    ('default_workspace_root', ''),
    ('privacy_sentinel.default_policy', '{"api":"prompt","local":"allow"}'),
    ('onboarding.completed', 'false');
