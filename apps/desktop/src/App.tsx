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

export default function App() {
  const [status, setStatus] = useState<OpcStatus | null>(null);
  const [statusError, setStatusError] = useState<string | null>(null);
  const [providers, setProviders] = useState<ProviderInfo[] | null>(null);
  const [providersError, setProvidersError] = useState<string | null>(null);
  const [agents, setAgents] = useState<AgentInfo[] | null>(null);
  const [agentsError, setAgentsError] = useState<string | null>(null);

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
  }, []);

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
        <span>P0.5 · Runtime + Storage + Provider + Agent 层已就绪 · 项目/任务/工作流待续</span>
      </footer>
    </main>
  );
}
