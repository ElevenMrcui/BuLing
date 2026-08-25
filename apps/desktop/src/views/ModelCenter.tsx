import { Cpu } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { EmptyState } from "@/components/shared";
import { cn } from "@/lib/utils";
import type { ProviderInfo } from "../types";

const KIND_LABEL: Record<ProviderInfo["kind"], string> = { cli: "CLI", api: "API", local: "本地模型" };

export function ModelCenter({ providers, providersError }: { providers: ProviderInfo[] | null; providersError: string | null }) {
  const available = providers?.filter((p) => p.available) ?? [];

  return (
    <>
      <header className="px-8 pb-4 pt-6">
        <h1 className="text-[22px] font-bold tracking-tight">模型中心</h1>
        <p className="mt-0.5 text-[12.5px] text-muted-foreground">
          只发现不执行——扫描本机已装 CLI / 配置的 API Key / 本地模型服务，不发起任何真实推理调用。
        </p>
      </header>

      <div className="flex flex-col gap-5 px-8 pb-10">
        <Card>
          <CardHeader>
            <CardTitle>Provider 发现{providers ? `（${available.length} / ${providers.length} 可用）` : ""}</CardTitle>
          </CardHeader>
          <CardContent className="pt-0">
            {providers ? (
              providers.length > 0 ? (
                <ul className="flex flex-col gap-2">
                  {providers.map((p) => (
                    <li key={p.id} className="flex flex-wrap items-center gap-3 rounded-s border border-border bg-muted/50 px-3 py-2.5">
                      <span className={cn("h-2 w-2 shrink-0 rounded-full", p.available ? "bg-success" : "bg-muted-foreground/40")} />
                      <div className="flex min-w-0 flex-col gap-0.5">
                        <span className="text-[13px] font-semibold">{p.display_name}</span>
                        <span className="text-[11.5px] text-muted-foreground">
                          {p.vendor} · {KIND_LABEL[p.kind]}
                          {p.wire_format ? ` · ${p.wire_format}` : ""}
                        </span>
                      </div>
                      <Badge variant={p.available ? "success" : "secondary"} className="ml-auto">
                        {p.available ? "可用" : "不可用"}
                      </Badge>
                      {p.detail ? <span className="basis-full text-[11.5px] text-muted-foreground">{p.detail}</span> : null}
                    </li>
                  ))}
                </ul>
              ) : (
                <EmptyState icon={Cpu} title="没有发现任何 Provider" />
              )
            ) : providersError ? (
              <p className="text-[12.5px] text-destructive">加载失败：{providersError}</p>
            ) : (
              <p className="text-[12.5px] text-muted-foreground">正在扫描 Provider…</p>
            )}
          </CardContent>
        </Card>
      </div>
    </>
  );
}
