import type { LucideIcon } from "lucide-react";
import { AlertTriangle, Loader2 } from "lucide-react";
import type { ReactNode } from "react";
import { cn } from "@/lib/utils";

export function EmptyState({ icon: Icon, title, hint }: { icon: LucideIcon; title: string; hint?: string }) {
  return (
    <div className="flex flex-col items-center gap-1.5 px-5 py-10 text-center text-muted-foreground">
      <Icon className="mb-1.5 h-8 w-8 opacity-40" strokeWidth={1.5} />
      <b className="text-[13.5px] font-semibold text-foreground/80">{title}</b>
      {hint ? <span className="max-w-[320px] text-xs">{hint}</span> : null}
    </div>
  );
}

const TINTS = {
  accent: "bg-primary/10 text-primary",
  purple: "bg-purple/10 text-purple",
  success: "bg-success/10 text-success",
  warning: "bg-warning/10 text-warning",
} as const;

export function StatCard({
  icon: Icon,
  tint,
  value,
  label,
}: {
  icon: LucideIcon;
  tint: keyof typeof TINTS;
  value: ReactNode;
  label: string;
}) {
  return (
    <div className="flex flex-col gap-2 rounded-lg border border-border bg-card p-4 shadow-sm">
      <span className={cn("flex h-8 w-8 items-center justify-center rounded-[10px]", TINTS[tint])}>
        <Icon className="h-4 w-4" strokeWidth={1.8} />
      </span>
      <span className="text-[28px] font-bold tracking-tight tabular-nums">{value}</span>
      <span className="text-xs text-muted-foreground">{label}</span>
    </div>
  );
}

/** 评审红线在界面上的固定视觉标记——出现在任何"需要人工点头"的地方，样式永远一致、永远显眼。 */
export function RedlineBanner({ children }: { children: ReactNode }) {
  return (
    <div className="flex items-start gap-2.5 rounded-m border border-destructive/20 bg-destructive/10 px-3.5 py-3 text-xs leading-relaxed text-foreground/80">
      <AlertTriangle className="mt-0.5 h-4 w-4 shrink-0 text-destructive" strokeWidth={1.8} />
      <span>
        <b className="text-destructive">评审红线：</b>
        {children}
      </span>
    </div>
  );
}

export function Spinner({ className }: { className?: string }) {
  return <Loader2 className={cn("h-3.5 w-3.5 animate-spin", className)} />;
}
