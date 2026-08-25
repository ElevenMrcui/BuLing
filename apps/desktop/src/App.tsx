import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface OpcStatus {
  app_db_version: number;
  app_db_path: string;
  ready: boolean;
}

export default function App() {
  const [status, setStatus] = useState<OpcStatus | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    invoke<OpcStatus>("opc_status")
      .then(setStatus)
      .catch((e) => setError(String(e)));
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
          </dl>
        ) : error ? (
          <p className="err">加载失败：{error}</p>
        ) : (
          <p className="hint">正在初始化 APP 库…</p>
        )}
      </section>

      <footer>
        <span>P0 · Runtime + Storage 已就绪 · 项目/团队/任务/工作流待续</span>
      </footer>
    </main>
  );
}
