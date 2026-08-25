import { CheckCircle2, Cpu, FolderKanban, Users } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { EmptyState, StatCard } from "@/components/shared";
import type { ViewKey } from "../nav";
import type { AgentInfo, OpcStatus, ProjectInfo, ProviderInfo } from "../types";

export function ControlPanel({
  status,
  statusError,
  providers,
  agents,
  projects,
  onNavigate,
}: {
  status: OpcStatus | null;
  statusError: string | null;
  providers: ProviderInfo[] | null;
  agents: AgentInfo[] | null;
  projects: ProjectInfo[] | null;
  onNavigate: (v: ViewKey) => void;
}) {
  const availableProviders = providers?.filter((p) => p.available).length ?? 0;
  const recentProjects = (projects ?? []).slice(0, 5);

  return (
    <>
      <header className="flex items-start gap-4 px-8 pb-4 pt-6">
        <div>
          <h1 className="text-[22px] font-bold tracking-tight">控制台</h1>
          <p className="mt-0.5 text-[12.5px] text-muted-foreground">你出题，AI 团队开干——Local-First 的私人 AI 公司桌面应用。</p>
        </div>
        <div className="ml-auto shrink-0">
          <Button onClick={() => onNavigate("project-center")}>创建新项目</Button>
        </div>
      </header>

      <div className="flex flex-col gap-5 px-8 pb-10">
        {statusError ? (
          <Card>
            <CardContent className="py-5">
              <p className="text-[12.5px] text-destructive">存储层初始化失败：{statusError}</p>
            </CardContent>
          </Card>
        ) : (
          <div className="grid grid-cols-[repeat(auto-fit,minmax(190px,1fr))] gap-3.5">
            <StatCard icon={Users} tint="accent" value={agents?.length ?? "—"} label="预置 AI 岗位" />
            <StatCard
              icon={Cpu}
              tint="purple"
              value={providers ? `${availableProviders} / ${providers.length}` : "—"}
              label="可用引擎 / 已知引擎"
            />
            <StatCard icon={FolderKanban} tint="success" value={projects?.length ?? "—"} label="项目数" />
            <StatCard icon={CheckCircle2} tint="warning" value={status?.app_db_version ?? "—"} label="APP 库迁移版本" />
          </div>
        )}

        <Card>
          <CardHeader>
            <CardTitle>最近项目</CardTitle>
          </CardHeader>
          <CardContent className="pt-0">
            {recentProjects.length > 0 ? (
              <ul className="flex flex-col gap-2">
                {recentProjects.map((p) => (
                  <li key={p.id} className="flex flex-wrap items-center gap-3 rounded-s border border-border bg-muted/50 px-3 py-2.5">
                    <div className="flex min-w-0 flex-col gap-0.5">
                      <span className="text-[13px] font-semibold">{p.display_name}</span>
                      <span className="text-[11.5px] text-muted-foreground">
                        {p.slug} · {p.status}
                      </span>
                    </div>
                    <Button variant="ghost" size="sm" className="ml-auto" onClick={() => onNavigate("workflow-center")}>
                      打开工作流
                    </Button>
                  </li>
                ))}
              </ul>
            ) : (
              <EmptyState icon={FolderKanban} title="还没有项目" hint="去「项目中心」创建第一个，AI 团队会自动配齐。" />
            )}
          </CardContent>
        </Card>

        {status ? (
          <Card>
            <CardHeader>
              <CardTitle>存储层状态</CardTitle>
            </CardHeader>
            <CardContent className="pt-0">
              <div className="flex flex-wrap items-center gap-3 rounded-s border border-border bg-muted/50 px-3 py-2.5">
                <div className="flex min-w-0 flex-col gap-0.5">
                  <span className="text-[13px] font-semibold">APP 库路径</span>
                  <span className="break-all font-mono text-[11.5px] text-muted-foreground">{status.app_db_path}</span>
                </div>
                <Badge variant={status.ready ? "success" : "warning"} className="ml-auto">
                  {status.ready ? "就绪" : "初始化中"}
                </Badge>
              </div>
            </CardContent>
          </Card>
        ) : null}
      </div>
    </>
  );
}
