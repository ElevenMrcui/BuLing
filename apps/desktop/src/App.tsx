import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface OpcStatus {
  app_db_version: number;
  app_db_path: string;
  ready: boolean;
  agents_seeded: number;
}

interface ProviderInfo {
  id: string;
  display_name: string;
  vendor: string;
  kind: "cli" | "api" | "local";
  wire_format: string | null;
  available: boolean;
  detail: string | null;
}

interface AgentInfo {
  id: string;
  kind: "preset" | "user";
  role: string;
  display_name: string;
  avatar: string | null;
  sensitivity: "low" | "medium" | "high";
  provider_priority: string[];
  version: number;
}

interface ProjectInfo {
  id: string;
  slug: string;
  display_name: string;
  root_path: string;
  status: string;
  starred: boolean;
  last_opened_at: string | null;
  active_workflow_id: string | null;
}

interface TaskInfo {
  node_key: string;
  kind: string;
  role: string | null;
  assignment_mode: string;
  status: string;
}

interface GateInfo {
  id: string;
  status: string;
}

export default function App() {
  const [status, setStatus] = useState<OpcStatus | null>(null);
  const [statusError, setStatusError] = useState<string | null>(null);
  const [providers, setProviders] = useState<ProviderInfo[] | null>(null);
  const [providersError, setProvidersError] = useState<string | null>(null);
  const [agents, setAgents] = useState<AgentInfo[] | null>(null);
  const [agentsError, setAgentsError] = useState<string | null>(null);
  const [projects, setProjects] = useState<ProjectInfo[] | null>(null);
  const [projectsError, setProjectsError] = useState<string | null>(null);
  const [newSlug, setNewSlug] = useState("");
  const [newDisplayName, setNewDisplayName] = useState("");
  const [newRootPath, setNewRootPath] = useState("");
  const [newGoal, setNewGoal] = useState("");
  const [withWorkflow, setWithWorkflow] = useState(true);
  const [creating, setCreating] = useState(false);
  const [createError, setCreateError] = useState<string | null>(null);

  // 「工作流中心」跟踪的是最近一次创建/选中的项目 + 它的活跃工作流。
  const [activeProjectId, setActiveProjectId] = useState<string | null>(null);
  const [activeWorkflowId, setActiveWorkflowId] = useState<string | null>(null);
  const [tasks, setTasks] = useState<TaskInfo[] | null>(null);
  const [readyNodeKeys, setReadyNodeKeys] = useState<Set<string>>(new Set());
  const [gates, setGates] = useState<GateInfo[] | null>(null);
  const [workflowError, setWorkflowError] = useState<string | null>(null);
  const [busyNodeKey, setBusyNodeKey] = useState<string | null>(null);

  const refreshProjects = () => {
    invoke<ProjectInfo[]>("opc_list_projects")
      .then(setProjects)
      .catch((e) => setProjectsError(String(e)));
  };

  const refreshWorkflow = (projectId: string, workflowId: string) => {
    invoke<TaskInfo[]>("opc_workflow_tasks", { projectId, workflowId })
      .then(setTasks)
      .catch((e) => setWorkflowError(String(e)));
    invoke<TaskInfo[]>("opc_workflow_ready_tasks", { projectId, workflowId })
      .then((ready) => setReadyNodeKeys(new Set(ready.map((t) => t.node_key))))
      .catch((e) => setWorkflowError(String(e)));
    invoke<GateInfo[]>("opc_workflow_gates", { projectId, workflowId })
      .then(setGates)
      .catch((e) => setWorkflowError(String(e)));
  };

  useEffect(() => {
    if (activeProjectId && activeWorkflowId) {
      refreshWorkflow(activeProjectId, activeWorkflowId);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [activeProjectId, activeWorkflowId]);

  const runNode = async (nodeKey: string) => {
    if (!activeProjectId || !activeWorkflowId) return;
    setBusyNodeKey(nodeKey);
    setWorkflowError(null);
    try {
      await invoke("opc_workflow_run_task", { projectId: activeProjectId, workflowId: activeWorkflowId, nodeKey });
      refreshWorkflow(activeProjectId, activeWorkflowId);
    } catch (err) {
      setWorkflowError(String(err));
    } finally {
      setBusyNodeKey(null);
    }
  };

  const decideGate = async (gateId: string, approve: boolean) => {
    if (!activeProjectId || !activeWorkflowId) return;
    setBusyNodeKey(gateId);
    setWorkflowError(null);
    try {
      const cmd = approve ? "opc_workflow_approve_gate" : "opc_workflow_reject_gate";
      await invoke(cmd, { projectId: activeProjectId, workflowId: activeWorkflowId, gateId, comment: null });
      refreshWorkflow(activeProjectId, activeWorkflowId);
    } catch (err) {
      setWorkflowError(String(err));
    } finally {
      setBusyNodeKey(null);
    }
  };

  useEffect(() => {
    invoke<OpcStatus>("opc_status")
      .then(setStatus)
      .catch((e) => setStatusError(String(e)));
    invoke<ProviderInfo[]>("opc_providers")
      .then(setProviders)
      .catch((e) => setProvidersError(String(e)));
    invoke<AgentInfo[]>("opc_agents")
      .then(setAgents)
      .catch((e) => setAgentsError(String(e)));
    refreshProjects();
  }, []);

  const handleCreateProject = async (e: React.FormEvent) => {
    e.preventDefault();
    setCreating(true);
    setCreateError(null);
    try {
      const created = await invoke<ProjectInfo>("opc_create_project", {
        slug: newSlug,
        displayName: newDisplayName,
        rootPath: newRootPath,
        goal: newGoal || null,
        templateId: withWorkflow ? "standard-software-delivery" : null,
      });
      setNewSlug("");
      setNewDisplayName("");
      setNewRootPath("");
      setNewGoal("");
      refreshProjects();
      if (created.active_workflow_id) {
        setActiveProjectId(created.id);
        setActiveWorkflowId(created.active_workflow_id);
      }
    } catch (err) {
      setCreateError(String(err));
    } finally {
      setCreating(false);
    }
  };

  return (
    <main className="app-shell">
      <header className="brand">
        <div className="brand-mark">令</div>
        <div className="brand-text">
          <b>不令 OPC</b>
          <span>其身正，不令而行。</span>
        </div>
      </header>

      <section className="hero">
        <h1>你出题，AI 团队开干。</h1>
        <p className="sub">Local-First 的私人 AI 公司桌面应用。</p>
      </section>

      <section className="status card">
        <h2>项目中心</h2>
        <form onSubmit={handleCreateProject} className="project-form">
          <input
            placeholder="slug（如 health-app）"
            value={newSlug}
            onChange={(e) => setNewSlug(e.target.value)}
            required
          />
          <input
            placeholder="项目名（如 健康管理 App）"
            value={newDisplayName}
            onChange={(e) => setNewDisplayName(e.target.value)}
            required
          />
          <input
            placeholder="本地目录（绝对路径）"
            value={newRootPath}
            onChange={(e) => setNewRootPath(e.target.value)}
            required
          />
          <input placeholder="目标（可选）" value={newGoal} onChange={(e) => setNewGoal(e.target.value)} />
          <label className="checkbox-row">
            <input type="checkbox" checked={withWorkflow} onChange={(e) => setWithWorkflow(e.target.checked)} />
            同时启动「标准软件交付流」工作流
          </label>
          <button type="submit" disabled={creating}>
            {creating ? "创建中…" : "创建项目"}
          </button>
        </form>
        {createError ? <p className="err">创建失败：{createError}</p> : null}

        {projects ? (
          projects.length > 0 ? (
            <ul className="provider-list">
              {projects.map((p) => (
                <li key={p.id} className="provider-row">
                  <span className="provider-name">{p.display_name}</span>
                  <span className="provider-meta">
                    {p.slug} · {p.status}
                    {p.starred ? " · ★" : ""}
                  </span>
                  <span className="provider-detail mono">{p.root_path}</span>
                </li>
              ))}
            </ul>
          ) : (
            <p className="hint">还没有项目，创建第一个吧。</p>
          )
        ) : projectsError ? (
          <p className="err">加载失败：{projectsError}</p>
        ) : (
          <p className="hint">正在加载项目列表…</p>
        )}
      </section>

      {activeProjectId && activeWorkflowId ? (
        <section className="status card" style={{ marginTop: 16 }}>
          <h2>工作流中心 · 标准软件交付流</h2>
          {workflowError ? <p className="err">{workflowError}</p> : null}

          <h3 className="subhead">评审红线（人工 Gate）</h3>
          {gates ? (
            <ul className="provider-list">
              {gates.map((g) => (
                <li key={g.id} className="provider-row">
                  <span className={`dot ${g.status === "passed" ? "dot-ok" : "dot-off"}`} />
                  <span className="provider-name">{g.id}</span>
                  <span className="provider-meta">{g.status}</span>
                  {g.status === "pending" ? (
                    <span className="gate-actions">
                      <button type="button" disabled={busyNodeKey === g.id} onClick={() => decideGate(g.id, true)}>
                        通过
                      </button>
                      <button type="button" disabled={busyNodeKey === g.id} onClick={() => decideGate(g.id, false)}>
                        打回
                      </button>
                    </span>
                  ) : null}
                </li>
              ))}
            </ul>
          ) : (
            <p className="hint">加载中…</p>
          )}

          <h3 className="subhead">任务节点</h3>
          {tasks ? (
            <ul className="provider-list">
              {tasks.map((t) => (
                <li key={t.node_key} className="provider-row">
                  <span className="provider-name">{t.node_key}</span>
                  <span className="provider-meta">
                    {t.kind} · {t.role ?? "—"} · {t.assignment_mode} · {t.status}
                  </span>
                  {readyNodeKeys.has(t.node_key) ? (
                    <button type="button" disabled={busyNodeKey === t.node_key} onClick={() => runNode(t.node_key)}>
                      {busyNodeKey === t.node_key ? "执行中…" : "跑这个节点"}
                    </button>
                  ) : null}
                </li>
              ))}
            </ul>
          ) : (
            <p className="hint">加载中…</p>
          )}
        </section>
      ) : null}

      <section className="status card" style={{ marginTop: 16 }}>
        <h2>存储层状态</h2>
        {status ? (
          <dl>
            <dt>APP 库版本</dt>
            <dd>{status.app_db_version}</dd>
            <dt>APP 库路径</dt>
            <dd className="mono">{status.app_db_path}</dd>
            <dt>就绪</dt>
            <dd>{status.ready ? "✓" : "—"}</dd>
            <dt>预置岗位</dt>
            <dd>{status.agents_seeded} 位</dd>
          </dl>
        ) : statusError ? (
          <p className="err">加载失败：{statusError}</p>
        ) : (
          <p className="hint">正在初始化 APP 库…</p>
        )}
      </section>

      <section className="status card" style={{ marginTop: 16 }}>
        <h2>团队中心 · 预置 AI 岗位</h2>
        {agents ? (
          <ul className="provider-list">
            {agents.map((a) => (
              <li key={a.id} className="provider-row">
                <span className="provider-name">
                  {a.avatar ? `${a.avatar} ` : ""}
                  {a.display_name}
                </span>
                <span className="provider-meta">
                  {a.role} · sensitivity={a.sensitivity} · v{a.version}
                </span>
                <span className="provider-detail">
                  provider_priority: {a.provider_priority.join(" → ") || "（未配置）"}
                </span>
              </li>
            ))}
          </ul>
        ) : agentsError ? (
          <p className="err">加载失败：{agentsError}</p>
        ) : (
          <p className="hint">正在加载预置岗位…</p>
        )}
      </section>

      <section className="status card" style={{ marginTop: 16 }}>
        <h2>模型中心 · Provider 发现</h2>
        {providers ? (
          <ul className="provider-list">
            {providers.map((p) => (
              <li key={p.id} className="provider-row">
                <span className={`dot ${p.available ? "dot-ok" : "dot-off"}`} />
                <span className="provider-name">{p.display_name}</span>
                <span className="provider-meta">
                  {p.vendor} · {p.kind}
                  {p.wire_format ? ` · ${p.wire_format}` : ""}
                </span>
                {p.detail ? <span className="provider-detail">{p.detail}</span> : null}
              </li>
            ))}
          </ul>
        ) : providersError ? (
          <p className="err">加载失败：{providersError}</p>
        ) : (
          <p className="hint">正在扫描 Provider…</p>
        )}
      </section>

      <footer>
        <span>P0.5 · Runtime + Storage + Provider + Agent + Project + Workflow 层已就绪 · Task 认领/自动派单待续</span>
      </footer>
    </main>
  );
}
