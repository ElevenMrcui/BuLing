-- ============================================================================
-- 不令 OPC · PROJECT 级 SQLite Schema · 0001_init
--
-- 存放位置：<project-root>/.opc/project.sqlite（每个项目一份）
--
-- 存什么（项目局部数据）：
--   · 本项目实例化的 Agent Team（可以从 app 级 agents 表 fork + 定制）
--   · 工作流实例 · 任务 · 任务执行 · 节点执行
--   · Artifact 元数据 + 版本 + 追溯链（文件本体在 <project-root>/docs 等目录）
--   · 评审 / Gate
--   · 项目内 Memory / Knowledge chunks（P0 简化，P1 拆丰）
--   · 项目级 ExecutionLog
--   · 项目级权限 grant
--
-- 硬约束：
--   · artifacts / artifact_versions / reviews / gate_checks / execution_logs
--     append-only；应用层不允许 UPDATE，只能 INSERT 新版本
-- ============================================================================

PRAGMA foreign_keys = ON;
PRAGMA journal_mode = WAL;

-- ----------------------------------------------------------------------------
-- 项目元数据（单行 · 冗余 app.projects 一份，用于项目独立时也能自我描述）
-- ----------------------------------------------------------------------------
CREATE TABLE project_meta (
    id            INTEGER PRIMARY KEY CHECK (id = 1),
    project_id    TEXT NOT NULL UNIQUE,                     -- 与 app.projects.id 对齐
    slug          TEXT NOT NULL,
    display_name  TEXT NOT NULL,
    goal          TEXT,
    template_id   TEXT,
    created_at    TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at    TEXT NOT NULL DEFAULT (datetime('now'))
);

-- ----------------------------------------------------------------------------
-- Team · 本项目的 AI 组织（一个项目一个团队）
-- ----------------------------------------------------------------------------
CREATE TABLE teams (
    id           TEXT PRIMARY KEY,
    name         TEXT NOT NULL DEFAULT '默认团队',
    created_at   TEXT NOT NULL DEFAULT (datetime('now'))
);

-- ----------------------------------------------------------------------------
-- Agent 实例 · 从 app 级 agents 表 fork 而来，可本项目定制 prompt/provider
-- ----------------------------------------------------------------------------
CREATE TABLE agent_instances (
    id                  TEXT PRIMARY KEY,                   -- uuid
    team_id             TEXT NOT NULL,
    template_agent_id   TEXT NOT NULL,                      -- 引用 app.agents.id
    role                TEXT NOT NULL,                      -- 冗余：'产品经理'
    display_name        TEXT NOT NULL,
    avatar              TEXT,

    -- Fork 自模板但可局部覆盖
    system_prompt_override  TEXT,
    provider_priority_override TEXT,                        -- JSON 数组
    permission_override     TEXT,                           -- JSON

    status              TEXT NOT NULL DEFAULT 'idle'
                        CHECK (status IN ('idle','working','waiting','offline','error')),

    -- 统计
    tasks_done          INTEGER NOT NULL DEFAULT 0,
    tasks_rejected      INTEGER NOT NULL DEFAULT 0,
    last_active_at      TEXT,

    created_at          TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at          TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (team_id) REFERENCES teams(id) ON DELETE CASCADE
);

CREATE INDEX agent_instances_team_idx ON agent_instances (team_id);
CREATE INDEX agent_instances_status_idx ON agent_instances (status);

-- ----------------------------------------------------------------------------
-- 权限 grant · Agent 级细粒度覆盖 default
-- 六大类：file / command / network / git / docker / mcp
-- ----------------------------------------------------------------------------
CREATE TABLE permissions (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_id        TEXT NOT NULL,
    kind            TEXT NOT NULL CHECK (kind IN ('file','command','network','git','docker','mcp')),
    scope           TEXT NOT NULL,                          -- 具体值：路径 / 命令名 / 域名 / git 动词
    mode            TEXT NOT NULL CHECK (mode IN ('allow','deny','prompt')),
    granted_at      TEXT NOT NULL DEFAULT (datetime('now')),
    granted_until   TEXT,                                   -- 时效性，null = 永久
    granted_by      TEXT NOT NULL DEFAULT 'user',
    FOREIGN KEY (agent_id) REFERENCES agent_instances(id) ON DELETE CASCADE
);

CREATE INDEX permissions_agent_idx ON permissions (agent_id);
CREATE UNIQUE INDEX permissions_uniq_idx ON permissions (agent_id, kind, scope);

-- ----------------------------------------------------------------------------
-- Workflow · 项目里在跑的工作流实例
-- 定义（YAML 模板）在 templates/ 目录，实例是这里的一行
-- ----------------------------------------------------------------------------
CREATE TABLE workflows (
    id           TEXT PRIMARY KEY,                          -- uuid
    template_id  TEXT NOT NULL,                             -- 'standard-software-delivery'
    name         TEXT NOT NULL,
    dag          TEXT NOT NULL,                             -- JSON: 节点 + 依赖（实例化时快照）
    status       TEXT NOT NULL DEFAULT 'draft'
                 CHECK (status IN ('draft','running','paused','completed','failed','cancelled')),
    started_at   TEXT,
    finished_at  TEXT,
    created_at   TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at   TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX workflows_status_idx ON workflows (status);

-- ----------------------------------------------------------------------------
-- Task · 工作流里的节点或独立任务
-- ----------------------------------------------------------------------------
CREATE TABLE tasks (
    id              TEXT PRIMARY KEY,
    workflow_id     TEXT,                                   -- null = 独立任务
    node_key        TEXT,                                   -- 对应模板里的 node id, e.g. 'prd'
    parent_task_id  TEXT,                                   -- 子任务
    kind            TEXT NOT NULL CHECK (kind IN ('agent','human','condition','parallel','sub-workflow')),

    title           TEXT NOT NULL,
    description     TEXT,
    role            TEXT,                                   -- 期望岗位 '产品经理'
    assignment_mode TEXT NOT NULL DEFAULT 'template'
                    CHECK (assignment_mode IN ('template','manual','auto-claim')),
    assigned_agent_id TEXT,                                 -- 具体 agent_instance；null = 未分配

    priority        TEXT NOT NULL DEFAULT 'mid' CHECK (priority IN ('low','mid','high')),
    status          TEXT NOT NULL DEFAULT 'pending'
                    CHECK (status IN ('pending','claimable','assigned','running','waiting-review','review-passed','review-rejected','completed','failed','cancelled','blocked')),

    inputs          TEXT NOT NULL DEFAULT '[]',             -- JSON: 引用哪些 artifact
    expected_outputs TEXT NOT NULL DEFAULT '[]',            -- JSON: 期望产出的 artifact 契约

    -- 认领相关
    claim_deadline  TEXT,
    claim_scores    TEXT,                                   -- JSON: { agent_id: 意愿分 }

    -- 时间
    started_at      TEXT,
    finished_at     TEXT,
    deadline        TEXT,

    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at      TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (workflow_id) REFERENCES workflows(id) ON DELETE CASCADE,
    FOREIGN KEY (parent_task_id) REFERENCES tasks(id) ON DELETE CASCADE,
    FOREIGN KEY (assigned_agent_id) REFERENCES agent_instances(id) ON DELETE SET NULL
);

CREATE INDEX tasks_workflow_idx ON tasks (workflow_id);
CREATE INDEX tasks_status_idx ON tasks (status);
CREATE INDEX tasks_agent_idx ON tasks (assigned_agent_id);

-- ----------------------------------------------------------------------------
-- TaskRun · 一次任务执行尝试（同一 task 可以有多次 run · 失败重试 / 重跑）
-- ----------------------------------------------------------------------------
CREATE TABLE task_runs (
    id           TEXT PRIMARY KEY,
    task_id      TEXT NOT NULL,
    agent_id     TEXT NOT NULL,                             -- 本次是谁跑
    provider_id  TEXT NOT NULL,                             -- 实际路由到哪个 Provider
    status       TEXT NOT NULL DEFAULT 'running'
                 CHECK (status IN ('running','succeeded','failed','cancelled','superseded')),

    -- 输入输出快照
    input_snapshot   TEXT,                                  -- JSON: 本次收到的 context
    output_summary   TEXT,                                  -- 摘要
    report_artifact_id TEXT,                                -- 强制产出的 Report.md 的 artifact id
    error            TEXT,

    -- 计量
    tokens_input     INTEGER,
    tokens_output    INTEGER,
    duration_ms      INTEGER,

    started_at       TEXT NOT NULL DEFAULT (datetime('now')),
    finished_at      TEXT,
    FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE
);

CREATE INDEX task_runs_task_idx ON task_runs (task_id);
CREATE INDEX task_runs_agent_idx ON task_runs (agent_id);

-- ----------------------------------------------------------------------------
-- Artifact · 产出物元数据（本体是磁盘文件）
-- ----------------------------------------------------------------------------
CREATE TABLE artifacts (
    id            TEXT PRIMARY KEY,
    kind          TEXT NOT NULL,                            -- 'prd','architecture','code','test-report','acceptance','report','other'
    name          TEXT NOT NULL,                            -- 'PRD.md'
    file_path     TEXT NOT NULL,                            -- 项目根相对路径：'docs/product/PRD.md'
    mime          TEXT,
    latest_version INTEGER NOT NULL DEFAULT 1,

    producer_agent_id TEXT,
    producer_task_id  TEXT,

    created_at    TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at    TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (producer_agent_id) REFERENCES agent_instances(id) ON DELETE SET NULL,
    FOREIGN KEY (producer_task_id) REFERENCES tasks(id) ON DELETE SET NULL,
    UNIQUE (file_path)
);

CREATE INDEX artifacts_kind_idx ON artifacts (kind);
CREATE INDEX artifacts_producer_agent_idx ON artifacts (producer_agent_id);

-- ----------------------------------------------------------------------------
-- ArtifactVersion · append-only 版本
-- ----------------------------------------------------------------------------
CREATE TABLE artifact_versions (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    artifact_id   TEXT NOT NULL,
    version       INTEGER NOT NULL,
    file_hash     TEXT NOT NULL,                            -- sha256
    file_bytes    INTEGER NOT NULL,
    author_agent_id TEXT,
    task_run_id   TEXT,
    change_note   TEXT,
    created_at    TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (artifact_id) REFERENCES artifacts(id) ON DELETE CASCADE,
    UNIQUE (artifact_id, version)
);

-- ----------------------------------------------------------------------------
-- ArtifactRef · 追溯图 · 一个 artifact 引用了哪些前置 artifact
-- ----------------------------------------------------------------------------
CREATE TABLE artifact_refs (
    child_artifact_id  TEXT NOT NULL,
    parent_artifact_id TEXT NOT NULL,
    relation           TEXT NOT NULL DEFAULT 'derives-from',  -- 'derives-from','refines','tests','fixes'
    created_at         TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (child_artifact_id, parent_artifact_id, relation),
    FOREIGN KEY (child_artifact_id) REFERENCES artifacts(id) ON DELETE CASCADE,
    FOREIGN KEY (parent_artifact_id) REFERENCES artifacts(id) ON DELETE CASCADE
);

-- ----------------------------------------------------------------------------
-- Gate · 关键卡点 · 需要人工评审通过才能进入下一阶段
-- ----------------------------------------------------------------------------
CREATE TABLE gates (
    id           TEXT PRIMARY KEY,
    workflow_id  TEXT NOT NULL,
    node_key     TEXT NOT NULL,                             -- 'requirement-gate' / 'design-gate' / 'acceptance-gate'
    kind         TEXT NOT NULL,                             -- 语义分类
    status       TEXT NOT NULL DEFAULT 'pending'
                 CHECK (status IN ('pending','passed','failed','waived')),
    reviewer     TEXT,                                      -- 'user' 或多评审场景下的具体 id
    passed_at    TEXT,
    FOREIGN KEY (workflow_id) REFERENCES workflows(id) ON DELETE CASCADE
);

CREATE INDEX gates_workflow_idx ON gates (workflow_id);
CREATE INDEX gates_status_idx ON gates (status);

-- ----------------------------------------------------------------------------
-- Review · 一次评审动作（对 artifact 或 task_run）
-- append-only
-- ----------------------------------------------------------------------------
CREATE TABLE reviews (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    subject_kind      TEXT NOT NULL CHECK (subject_kind IN ('artifact','task_run','gate')),
    subject_id        TEXT NOT NULL,
    reviewer          TEXT NOT NULL DEFAULT 'user',
    decision          TEXT NOT NULL CHECK (decision IN ('approved','rejected','changes-requested')),
    comment           TEXT,
    evidence          TEXT,                                 -- JSON: 引用文档 / 截图路径
    created_at        TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX reviews_subject_idx ON reviews (subject_kind, subject_id);

-- ----------------------------------------------------------------------------
-- 项目内 Memory（P0 简化：kv 形式）
-- ----------------------------------------------------------------------------
CREATE TABLE memory_entries (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    scope        TEXT NOT NULL,                             -- 'project','team','agent:xxx','task:xxx'
    key          TEXT NOT NULL,
    value        TEXT NOT NULL,                             -- 短文本 / JSON
    tags         TEXT NOT NULL DEFAULT '[]',                -- JSON 数组
    embedded_at  TEXT,                                      -- P1: 有 embedding 后填时间戳
    created_at   TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at   TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX memory_scope_idx ON memory_entries (scope);
CREATE INDEX memory_key_idx ON memory_entries (key);

-- FTS5 全文索引（P0 就上，P1 加向量）
CREATE VIRTUAL TABLE memory_fts USING fts5 (
    scope UNINDEXED,
    key,
    value,
    tags,
    content='memory_entries',
    content_rowid='id'
);

CREATE TRIGGER memory_fts_ai AFTER INSERT ON memory_entries BEGIN
    INSERT INTO memory_fts(rowid, scope, key, value, tags)
    VALUES (new.id, new.scope, new.key, new.value, new.tags);
END;
CREATE TRIGGER memory_fts_ad AFTER DELETE ON memory_entries BEGIN
    INSERT INTO memory_fts(memory_fts, rowid, scope, key, value, tags)
    VALUES ('delete', old.id, old.scope, old.key, old.value, old.tags);
END;
CREATE TRIGGER memory_fts_au AFTER UPDATE ON memory_entries BEGIN
    INSERT INTO memory_fts(memory_fts, rowid, scope, key, value, tags)
    VALUES ('delete', old.id, old.scope, old.key, old.value, old.tags);
    INSERT INTO memory_fts(rowid, scope, key, value, tags)
    VALUES (new.id, new.scope, new.key, new.value, new.tags);
END;

-- ----------------------------------------------------------------------------
-- 项目级 ExecutionLog · 更细的操作审计
-- ----------------------------------------------------------------------------
CREATE TABLE execution_logs (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    ts           TEXT NOT NULL DEFAULT (datetime('now')),
    task_id      TEXT,
    task_run_id  TEXT,
    agent_id     TEXT,
    provider_id  TEXT,

    kind         TEXT NOT NULL,                             -- 'llm.call','tool.file.write','tool.git.commit','privacy.block','review.decision','permission.grant'
    tool         TEXT,                                      -- 'shell:npm test' / 'git:commit'
    input        TEXT,                                      -- JSON
    output       TEXT,                                      -- JSON
    result       TEXT NOT NULL CHECK (result IN ('ok','error','blocked','confirmed','denied','superseded')),
    error        TEXT,

    -- 计量
    duration_ms  INTEGER,
    bytes_in     INTEGER,
    bytes_out    INTEGER,

    FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE SET NULL,
    FOREIGN KEY (task_run_id) REFERENCES task_runs(id) ON DELETE SET NULL
);

CREATE INDEX proj_exec_log_ts_idx ON execution_logs (ts DESC);
CREATE INDEX proj_exec_log_task_idx ON execution_logs (task_id);
CREATE INDEX proj_exec_log_agent_idx ON execution_logs (agent_id);

-- ----------------------------------------------------------------------------
-- 迁移追踪
-- ----------------------------------------------------------------------------
CREATE TABLE _migrations (
    version     INTEGER PRIMARY KEY,
    name        TEXT NOT NULL,
    applied_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

INSERT INTO _migrations (version, name) VALUES (1, '0001_init');
