import { Users } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { EmptyState } from "@/components/shared";
import type { AgentInfo } from "../types";

const SENSITIVITY_LABEL: Record<AgentInfo["sensitivity"], string> = {
  low: "低敏感度",
  medium: "中敏感度",
  high: "高敏感度 · 只走本地模型",
};

export function TeamCenter({ agents, agentsError }: { agents: AgentInfo[] | null; agentsError: string | null }) {
  return (
    <>
      <header className="px-8 pb-4 pt-6">
        <h1 className="text-[22px] font-bold tracking-tight">团队中心</h1>
        <p className="mt-0.5 text-[12.5px] text-muted-foreground">
          9 位预置岗位——产品经理 / 技术负责人 / 架构师 / 项目经理 / 设计师 / 前端 / 后端 / 测试 / 验收。
        </p>
      </header>

      <div className="flex flex-col gap-5 px-8 pb-10">
        <Card>
          <CardHeader>
            <CardTitle>预置 AI 岗位</CardTitle>
          </CardHeader>
          <CardContent className="pt-0">
            {agents ? (
              agents.length > 0 ? (
                <ul className="flex flex-col gap-2">
                  {agents.map((a) => (
                    <li key={a.id} className="flex flex-wrap items-center gap-3 rounded-s border border-border bg-muted/50 px-3 py-2.5">
                      <div className="flex h-[30px] w-[30px] shrink-0 items-center justify-center rounded-[9px] bg-gradient-to-br from-primary to-purple text-sm font-bold text-white">
                        {a.avatar ?? a.display_name.slice(0, 1)}
                      </div>
                      <div className="flex min-w-0 flex-col gap-0.5">
                        <span className="text-[13px] font-semibold">{a.display_name}</span>
                        <span className="text-[11.5px] text-muted-foreground">
                          {a.role} · v{a.version} · {SENSITIVITY_LABEL[a.sensitivity]}
                        </span>
                      </div>
                      <Badge variant={a.kind === "preset" ? "secondary" : "default"} className="ml-auto">
                        {a.kind === "preset" ? "预置" : "自建"}
                      </Badge>
                      <span className="basis-full text-[11.5px] text-muted-foreground">
                        引擎优先级：{a.provider_priority.join(" → ") || "（未配置）"}
                      </span>
                    </li>
                  ))}
                </ul>
              ) : (
                <EmptyState icon={Users} title="还没有岗位" />
              )
            ) : agentsError ? (
              <p className="text-[12.5px] text-destructive">加载失败：{agentsError}</p>
            ) : (
              <p className="text-[12.5px] text-muted-foreground">正在加载预置岗位…</p>
            )}
          </CardContent>
        </Card>
      </div>
    </>
  );
}
