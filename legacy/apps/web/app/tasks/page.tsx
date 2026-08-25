"use client";
import { useEffect, useState } from "react";
import { api } from "../../lib/api";
import type { TaskDTO, AgentDTO } from "@buling/shared";

/**
 * E2E 最小可跑链路：发布任务 → 自主认领 → 逐步完成 → 评审批准/打回。
 * 页面是一个原地增量刷新的操作面板，够验证后端骨架就行。
 * 完整体验以 prototype/index.html 为准；R2 迁入完整 UI 与设计令牌。
 */
export default function TasksPage() {
  const [tasks, setTasks] = useState<TaskDTO[]>([]);
  const [agents, setAgents] = useState<AgentDTO[]>([]);
  const [err, setErr] = useState<string | null>(null);
  const [title, setTitle] = useState("");
  const [skills, setSkills] = useState("市场调研, 数据整理");
  const [detail, setDetail] = useState<TaskDTO | null>(null);

  async function refresh() {
    try {
      setErr(null);
      const [t, a] = await Promise.all([api.tasks.list(), api.agents.list()]);
      setTasks(t);
      setAgents(a);
    } catch (e: any) {
      setErr(String(e?.message ?? e));
    }
  }
  useEffect(() => {
    refresh();
  }, []);

  async function publish() {
    if (!title.trim()) return;
    try {
      await api.tasks.publish({
        title,
        requiredSkills: skills.split(",").map((s) => s.trim()).filter(Boolean),
        assignMode: "auto",
        priority: "mid",
      });
      setTitle("");
      await refresh();
    } catch (e: any) { setErr(String(e?.message ?? e)); }
  }

  async function claim(taskId: string, agentId: string) {
    try { await api.tasks.claim(taskId, agentId); await refresh(); }
    catch (e: any) { setErr(String(e?.message ?? e)); }
  }

  async function completeStep(taskId: string, idx: number) {
    try { const t = await api.tasks.completeStep(taskId, idx); setDetail(t); await refresh(); }
    catch (e: any) { setErr(String(e?.message ?? e)); }
  }

  return (
    <>
      <h1>任务</h1>
      <div className="sub">R1 骨架 E2E · 完整体验请看 prototype/index.html</div>
      {err && <div className="err">{err}</div>}

      <h2>发布新任务</h2>
      <div className="card">
        <div style={{ display: "grid", gridTemplateColumns: "2fr 3fr auto", gap: 8 }}>
          <input className="input" placeholder="任务标题" value={title} onChange={(e) => setTitle(e.target.value)} />
          <input className="input" placeholder="所需技能，逗号分隔" value={skills} onChange={(e) => setSkills(e.target.value)} />
          <button className="btn primary" onClick={publish}>发布</button>
        </div>
      </div>

      <h2>任务看板</h2>
      {tasks.map((t) => (
        <TaskCard
          key={t.id}
          task={t}
          agents={agents}
          onClaim={(agentId) => claim(t.id, agentId)}
          onOpen={() => setDetail(t)}
        />
      ))}

      {detail && (
        <TaskDetail
          detail={detail}
          onClose={() => setDetail(null)}
          onCompleteStep={(idx) => completeStep(detail.id, idx)}
          onRefresh={refresh}
        />
      )}
    </>
  );
}

function TaskCard(props: { task: TaskDTO; agents: AgentDTO[]; onClaim: (agentId: string) => void; onOpen: () => void }) {
  const { task, agents, onClaim, onOpen } = props;
  return (
    <div className="card">
      <div className="row" style={{ justifyContent: "space-between" }}>
        <div>
          <b>{task.title}</b>{" "}
          <span className={`pill st-${task.status === "review" ? "review" : task.status === "done" ? "done" : "open"}`}>
            {task.status}
          </span>
          <div className="hint" style={{ marginTop: 4 }}>
            所需技能：{task.requiredSkills.join(" / ") || "—"} · 未认领 {task.openHours}h
          </div>
        </div>
        <div className="row">
          {task.status === "open" && (
            <select className="input" style={{ width: 160 }} onChange={(e) => e.target.value && onClaim(e.target.value)} defaultValue="">
              <option value="" disabled>选择 Agent 认领…</option>
              {agents.filter((a) => a.status !== "offline").map((a) => (
                <option key={a.id} value={a.id}>{a.name} · {a.role}</option>
              ))}
            </select>
          )}
          {task.status !== "open" && (
            <button className="btn" onClick={onOpen}>查看协作详情</button>
          )}
        </div>
      </div>
    </div>
  );
}

function TaskDetail(props: {
  detail: TaskDTO;
  onClose: () => void;
  onCompleteStep: (idx: number) => void;
  onRefresh: () => void;
}) {
  const { detail, onClose, onCompleteStep, onRefresh } = props;
  const [reviewErr, setReviewErr] = useState<string | null>(null);

  async function approve() {
    try {
      // 演示评审红线：docId 用种子的 d-security-1；R2 从任务的 docs[] 里取
      await api.review.approve("d-security-1");
      setReviewErr(null);
      onRefresh();
    } catch (e: any) { setReviewErr(String(e?.message ?? e)); }
  }
  async function reject() {
    try {
      await api.review.reject("d-security-1", "需要补充测试用例");
      setReviewErr(null);
      onRefresh();
    } catch (e: any) { setReviewErr(String(e?.message ?? e)); }
  }

  return (
    <div className="card" style={{ marginTop: 20 }}>
      <div className="row" style={{ justifyContent: "space-between" }}>
        <h2 style={{ margin: 0 }}>{detail.title}</h2>
        <button className="btn" onClick={onClose}>关闭</button>
      </div>
      <div className="hint" style={{ marginBottom: 12 }}>
        状态 {detail.status} · 进度 {detail.progressPct}% · 参与人 {detail.assignees.join(", ") || "—"}
      </div>

      <h2>步骤</h2>
      {detail.plan.map((s, i) => (
        <div className="row" key={i} style={{ padding: "6px 0", borderBottom: "1px solid var(--border)" }}>
          <span className="pill">{i + 1}</span>
          <span style={{ flex: 1 }}>{s.title}</span>
          <span className="hint">{s.status}</span>
          {s.status === "active" && <button className="btn primary" onClick={() => onCompleteStep(i)}>标记完成</button>}
        </div>
      ))}

      {detail.status === "review" && (
        <>
          <h2>评审（红线：只允许人工用户，服务端 ActorGuard 已限定）</h2>
          {reviewErr && <div className="err">{reviewErr}</div>}
          <div className="row">
            <button className="btn danger" onClick={reject}>打回</button>
            <button className="btn primary" onClick={approve}>批准</button>
            <span className="hint">
              演示提示：Web 客户端带 <code>x-actor-type=user, roles=reviewer</code>；把 roles 去掉即触发 403。
            </span>
          </div>
        </>
      )}
    </div>
  );
}
