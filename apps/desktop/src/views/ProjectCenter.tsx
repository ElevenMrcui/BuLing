import { FolderKanban } from "lucide-react";
import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { EmptyState } from "@/components/shared";
import { errorMessage, ipc } from "../ipc";
import type { ProjectInfo } from "../types";

export function ProjectCenter({
  projects,
  projectsError,
  onCreated,
  onOpenWorkflow,
}: {
  projects: ProjectInfo[] | null;
  projectsError: string | null;
  onCreated: (project: ProjectInfo) => void;
  onOpenWorkflow: (projectId: string) => void;
}) {
  const [slug, setSlug] = useState("");
  const [displayName, setDisplayName] = useState("");
  const [rootPath, setRootPath] = useState("");
  const [goal, setGoal] = useState("");
  const [withWorkflow, setWithWorkflow] = useState(true);
  const [creating, setCreating] = useState(false);
  const [createError, setCreateError] = useState<string | null>(null);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setCreating(true);
    setCreateError(null);
    try {
      const created = await ipc.createProject({
        slug,
        displayName,
        rootPath,
        goal: goal || null,
        templateId: withWorkflow ? "standard-software-delivery" : null,
      });
      setSlug("");
      setDisplayName("");
      setRootPath("");
      setGoal("");
      onCreated(created);
    } catch (err) {
      setCreateError(errorMessage(err));
    } finally {
      setCreating(false);
    }
  };

  return (
    <>
      <header className="px-8 pb-4 pt-6">
        <h1 className="text-[22px] font-bold tracking-tight">项目中心</h1>
        <p className="mt-0.5 text-[12.5px] text-muted-foreground">一个目标 → 一个项目 → 一支自动配齐的 AI 团队。</p>
      </header>

      <div className="flex flex-col gap-5 px-8 pb-10">
        <Card>
          <CardHeader>
            <CardTitle>创建项目</CardTitle>
          </CardHeader>
          <CardContent className="pt-0">
            <form onSubmit={handleSubmit} className="flex flex-col gap-3.5">
              <div className="grid gap-1.5">
                <Label htmlFor="pc-slug">slug</Label>
                <Input id="pc-slug" placeholder="如 health-app" value={slug} onChange={(e) => setSlug(e.target.value)} required />
              </div>
              <div className="grid gap-1.5">
                <Label htmlFor="pc-name">项目名</Label>
                <Input id="pc-name" placeholder="如 健康管理 App" value={displayName} onChange={(e) => setDisplayName(e.target.value)} required />
              </div>
              <div className="grid gap-1.5">
                <Label htmlFor="pc-root">本地目录（绝对路径）</Label>
                <Input
                  id="pc-root"
                  placeholder="/Users/you/opc-projects/health-app"
                  value={rootPath}
                  onChange={(e) => setRootPath(e.target.value)}
                  required
                />
              </div>
              <div className="grid gap-1.5">
                <Label htmlFor="pc-goal">目标（可选）</Label>
                <Input id="pc-goal" placeholder="做一个健康管理 App" value={goal} onChange={(e) => setGoal(e.target.value)} />
              </div>
              <label className="flex items-center gap-2 text-[13px] text-foreground/80">
                <Checkbox checked={withWorkflow} onCheckedChange={(c) => setWithWorkflow(c === true)} />
                同时启动「标准软件交付流」工作流
              </label>
              {createError ? <p className="text-[12.5px] text-destructive">创建失败：{createError}</p> : null}
              <div>
                <Button type="submit" disabled={creating}>
                  {creating ? "创建中…" : "创建项目"}
                </Button>
              </div>
            </form>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>全部项目</CardTitle>
          </CardHeader>
          <CardContent className="pt-0">
            {projects ? (
              projects.length > 0 ? (
                <ul className="flex flex-col gap-2">
                  {projects.map((p) => (
                    <li key={p.id} className="flex flex-wrap items-center gap-3 rounded-s border border-border bg-muted/50 px-3 py-2.5">
                      <div className="flex min-w-0 flex-col gap-0.5">
                        <span className="text-[13px] font-semibold">{p.display_name}</span>
                        <span className="text-[11.5px] text-muted-foreground">
                          {p.slug} · {p.status}
                          {p.starred ? " · ★" : ""}
                        </span>
                      </div>
                      <Button variant="ghost" size="sm" className="ml-auto" onClick={() => onOpenWorkflow(p.id)}>
                        打开工作流
                      </Button>
                      <span className="basis-full break-all font-mono text-[11.5px] text-muted-foreground">{p.root_path}</span>
                    </li>
                  ))}
                </ul>
              ) : (
                <EmptyState icon={FolderKanban} title="还没有项目" hint="用上面的表单创建第一个吧。" />
              )
            ) : projectsError ? (
              <p className="text-[12.5px] text-destructive">加载失败：{projectsError}</p>
            ) : (
              <p className="text-[12.5px] text-muted-foreground">正在加载项目列表…</p>
            )}
          </CardContent>
        </Card>
      </div>
    </>
  );
}
