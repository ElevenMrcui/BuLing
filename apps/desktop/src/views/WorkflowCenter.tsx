import { CheckCircle2, Workflow } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { EmptyState, RedlineBanner, Spinner } from "@/components/shared";
import { cn } from "@/lib/utils";
import { errorMessage, ipc } from "../ipc";
import type { AgentInfo, ClaimScore, GateInfo, ProjectInfo, TaskInfo, WorkflowSummary } from "../types";

const STATUS_LABEL: Record<string, string> = {
  pending: "待办",
  assigned: "已指派",
  running: "执行中",
  completed: "已完成",
  failed: "失败",
};

export function WorkflowCenter({
  projects,
  agents,
  requestedProjectId,
}: {
  projects: ProjectInfo[] | null;
  agents: AgentInfo[] | null;
  requestedProjectId: string | null;
}) {
  const [selectedProjectId, setSelectedProjectId] = useState("");
  const [workflow, setWorkflow] = useState<WorkflowSummary | null>(null);
  const [workflowLoading, setWorkflowLoading] = useState(false);
  const [tasks, setTasks] = useState<TaskInfo[] | null>(null);
  const [readyKeys, setReadyKeys] = useState<Set<string>>(new Set());
  const [claimableKeys, setClaimableKeys] = useState<Set<string>>(new Set());
  const [manualKeys, setManualKeys] = useState<Set<string>>(new Set());
  const [gates, setGates] = useState<GateInfo[] | null>(null);
  const [manualPick, setManualPick] = useState<Record<string, string>>({});
  const [busyKey, setBusyKey] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [noticeIsError, setNoticeIsError] = useState(false);

  // 从别的页面点「打开工作流」跳过来时，把请求的项目选中。
  useEffect(() => {
    if (requestedProjectId) setSelectedProjectId(requestedProjectId);
  }, [requestedProjectId]);

  useEffect(() => {
    if (!selectedProjectId) {
      setWorkflow(null);
      return;
    }
    setWorkflowLoading(true);
    setNotice(null);
    ipc
      .projectWorkflow(selectedProjectId)
      .then(setWorkflow)
      .catch((e) => {
        setNotice(errorMessage(e));
        setNoticeIsError(true);
      })
      .finally(() => setWorkflowLoading(false));
  }, [selectedProjectId]);

  const refresh = useCallback(() => {
    if (!selectedProjectId || !workflow) return;
    ipc.workflowTasks(selectedProjectId, workflow.id).then(setTasks).catch((e) => setNotice(errorMessage(e)));
    ipc
      .workflowReadyTasks(selectedProjectId, workflow.id)
      .then((r) => setReadyKeys(new Set(r.map((t) => t.node_key))))
      .catch((e) => setNotice(errorMessage(e)));
    ipc
      .taskClaimableTasks(selectedProjectId, workflow.id)
      .then((r) => setClaimableKeys(new Set(r.map((t) => t.node_key))))
      .catch((e) => setNotice(errorMessage(e)));
    ipc
      .taskManualTasks(selectedProjectId, workflow.id)
      .then((r) => setManualKeys(new Set(r.map((t) => t.node_key))))
      .catch((e) => setNotice(errorMessage(e)));
    ipc.workflowGates(selectedProjectId, workflow.id).then(setGates).catch((e) => setNotice(errorMessage(e)));
  }, [selectedProjectId, workflow]);

  useEffect(refresh, [refresh]);

  const withBusy = async (key: string, action: () => Promise<unknown>, okMessage?: string) => {
    setBusyKey(key);
    setNotice(null);
    try {
      await action();
      if (okMessage) {
        setNotice(okMessage);
        setNoticeIsError(false);
      }
      refresh();
    } catch (err) {
      setNotice(errorMessage(err));
      setNoticeIsError(true);
    } finally {
      setBusyKey(null);
    }
  };

  const runNode = (nodeKey: string) => withBusy(nodeKey, () => ipc.workflowRunTask(selectedProjectId, workflow!.id, nodeKey));

  const runAssignedNode = (nodeKey: string) => withBusy(nodeKey, () => ipc.taskRun(selectedProjectId, workflow!.id, nodeKey));

  const claimNode = (nodeKey: string) =>
    withBusy(nodeKey, async () => {
      const winner: ClaimScore = await ipc.taskClaim(selectedProjectId, workflow!.id, nodeKey);
      setNotice(`「${nodeKey}」认领给了 ${winner.template_agent_id}（能力匹配分 ${winner.score}）`);
      setNoticeIsError(false);
    });

  const assignManually = (nodeKey: string) => {
    const templateAgentId = manualPick[nodeKey];
    if (!templateAgentId) {
      setNotice("先选一个岗位再指派");
      setNoticeIsError(true);
      return;
    }
    return withBusy(nodeKey, () => ipc.taskAssignManually(selectedProjectId, workflow!.id, nodeKey, templateAgentId));
  };

  const decideGate = (gateId: string, approve: boolean) =>
    withBusy(gateId, () =>
      approve
        ? ipc.workflowApproveGate(selectedProjectId, workflow!.id, gateId, null)
        : ipc.workflowRejectGate(selectedProjectId, workflow!.id, gateId, null),
    );

  return (
    <>
      <header className="flex items-start gap-4 px-8 pb-4 pt-6">
        <div>
          <h1 className="text-[22px] font-bold tracking-tight">工作流中心</h1>
          <p className="mt-0.5 text-[12.5px] text-muted-foreground">选一个项目，跟着它的工作流走——从产品经理的 PRD 到验收，AI 团队接力，评审红线永远人工点头。</p>
        </div>
        <div className="ml-auto shrink-0">
          <Select value={selectedProjectId} onValueChange={setSelectedProjectId}>
            <SelectTrigger className="w-[240px]">
              <SelectValue placeholder="选一个项目…" />
            </SelectTrigger>
            <SelectContent>
              {(projects ?? []).map((p) => (
                <SelectItem key={p.id} value={p.id}>
                  {p.display_name}（{p.slug}）
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
      </header>

      <div className="flex flex-col gap-5 px-8 pb-10">
        {!selectedProjectId ? (
          <Card>
            <CardContent className="py-10">
              <EmptyState icon={Workflow} title="先选一个项目" hint="右上角下拉框选项目，或者从「项目中心」点「打开工作流」过来。" />
            </CardContent>
          </Card>
        ) : workflowLoading ? (
          <Card>
            <CardContent className="py-10">
              <p className="text-center text-[12.5px] text-muted-foreground">加载中…</p>
            </CardContent>
          </Card>
        ) : !workflow ? (
          <Card>
            <CardContent className="py-10">
              <EmptyState
                icon={Workflow}
                title="这个项目还没有工作流"
                hint="只有创建时勾选了「同时启动标准软件交付流工作流」的项目才有——这一版还不支持给已有项目补建工作流。"
              />
            </CardContent>
          </Card>
        ) : (
          <>
            {notice ? <p className={cn("text-[12.5px]", noticeIsError ? "text-destructive" : "text-muted-foreground")}>{notice}</p> : null}

            <RedlineBanner>Gate 只能靠下面「通过」/「打回」按钮由你亲自点，Runtime 永远不会替你按下去。</RedlineBanner>

            <Card>
              <CardHeader>
                <CardTitle>评审红线（人工 Gate）</CardTitle>
              </CardHeader>
              <CardContent className="pt-0">
                {gates && gates.length > 0 ? (
                  <ul className="flex flex-col gap-2">
                    {gates.map((g) => (
                      <li key={g.id} className="flex flex-wrap items-center gap-3 rounded-s border border-border bg-muted/50 px-3 py-2.5">
                        <span className={cn("h-2 w-2 shrink-0 rounded-full", g.status === "passed" ? "bg-success" : "bg-muted-foreground/40")} />
                        <div className="flex min-w-0 flex-col gap-0.5">
                          <span className="text-[13px] font-semibold">{g.id}</span>
                          <span className="text-[11.5px] text-muted-foreground">{STATUS_LABEL[g.status] ?? g.status}</span>
                        </div>
                        {g.status === "pending" ? (
                          <div className="ml-auto flex gap-1.5">
                            <Button size="sm" disabled={busyKey === g.id} onClick={() => decideGate(g.id, true)}>
                              {busyKey === g.id ? <Spinner /> : null}通过
                            </Button>
                            <Button variant="destructive" size="sm" disabled={busyKey === g.id} onClick={() => decideGate(g.id, false)}>
                              打回
                            </Button>
                          </div>
                        ) : null}
                      </li>
                    ))}
                  </ul>
                ) : (
                  <EmptyState icon={CheckCircle2} title="没有 Gate" />
                )}
              </CardContent>
            </Card>

            <Card>
              <CardHeader>
                <CardTitle>任务节点</CardTitle>
              </CardHeader>
              <CardContent className="pt-0">
                {tasks && tasks.length > 0 ? (
                  <ul className="flex flex-col gap-2">
                    {tasks.map((t) => (
                      <li key={t.node_key} className="flex flex-wrap items-center gap-3 rounded-s border border-border bg-muted/50 px-3 py-2.5">
                        <div className="flex min-w-0 flex-col gap-0.5">
                          <span className="text-[13px] font-semibold">{t.node_key}</span>
                          <span className="text-[11.5px] text-muted-foreground">
                            {t.kind} · {t.role ?? "—"} · {t.assignment_mode} · {STATUS_LABEL[t.status] ?? t.status}
                          </span>
                        </div>
                        <div className="ml-auto flex flex-wrap items-center gap-1.5">
                          {readyKeys.has(t.node_key) ? (
                            <Button size="sm" disabled={busyKey === t.node_key} onClick={() => runNode(t.node_key)}>
                              {busyKey === t.node_key ? <Spinner /> : null}跑这个节点
                            </Button>
                          ) : null}
                          {claimableKeys.has(t.node_key) ? (
                            <Button variant="secondary" size="sm" disabled={busyKey === t.node_key} onClick={() => claimNode(t.node_key)}>
                              {busyKey === t.node_key ? <Spinner /> : null}认领（能力匹配）
                            </Button>
                          ) : null}
                          {manualKeys.has(t.node_key) ? (
                            <>
                              <Select value={manualPick[t.node_key] ?? ""} onValueChange={(v) => setManualPick((m) => ({ ...m, [t.node_key]: v }))}>
                                <SelectTrigger className="h-7 w-[140px] text-xs">
                                  <SelectValue placeholder="选岗位…" />
                                </SelectTrigger>
                                <SelectContent>
                                  {(agents ?? []).map((a) => (
                                    <SelectItem key={a.id} value={a.id}>
                                      {a.display_name}
                                    </SelectItem>
                                  ))}
                                </SelectContent>
                              </Select>
                              <Button variant="secondary" size="sm" disabled={busyKey === t.node_key} onClick={() => assignManually(t.node_key)}>
                                {busyKey === t.node_key ? <Spinner /> : null}指派
                              </Button>
                            </>
                          ) : null}
                          {t.status === "assigned" && t.assignment_mode !== "template" ? (
                            <Button size="sm" disabled={busyKey === t.node_key} onClick={() => runAssignedNode(t.node_key)}>
                              {busyKey === t.node_key ? <Spinner /> : null}跑这个节点
                            </Button>
                          ) : null}
                        </div>
                      </li>
                    ))}
                  </ul>
                ) : (
                  <EmptyState icon={Workflow} title="没有任务节点" />
                )}
              </CardContent>
            </Card>
          </>
        )}
      </div>
    </>
  );
}
