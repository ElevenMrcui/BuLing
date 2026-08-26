import { cn } from "@/lib/utils";
import { NAV_ITEMS, type ViewKey } from "./nav";
import type { ThemeChoice } from "./theme";

export function Sidebar({
  active,
  onSelect,
  counts,
  theme,
  onThemeChange,
}: {
  active: ViewKey;
  onSelect: (v: ViewKey) => void;
  counts: Partial<Record<ViewKey, number>>;
  theme: ThemeChoice;
  onThemeChange: (t: ThemeChoice) => void;
}) {
  const core = NAV_ITEMS.filter((n) => n.group === "core");
  const support = NAV_ITEMS.filter((n) => n.group === "support");

  const renderItem = (item: (typeof NAV_ITEMS)[number]) => {
    const count = counts[item.key];
    const isActive = active === item.key;
    const Icon = item.icon;
    return (
      <button
        key={item.key}
        type="button"
        onClick={() => onSelect(item.key)}
        className={cn(
          "flex w-full items-center gap-2.5 rounded-s px-3 py-2 text-left text-[13.5px] font-medium text-foreground/70 transition-colors",
          isActive ? "bg-primary text-primary-foreground shadow-sm" : "hover:bg-secondary hover:text-foreground",
        )}
      >
        <Icon className="h-[18px] w-[18px] shrink-0" strokeWidth={1.8} />
        <span className="truncate">{item.label}</span>
        {item.ready && count !== undefined ? (
          <span
            className={cn(
              "ml-auto rounded-pill px-1.5 py-px text-[11px] font-semibold",
              isActive ? "bg-white/25 text-white" : "bg-muted text-muted-foreground",
            )}
          >
            {count}
          </span>
        ) : null}
        {!item.ready ? (
          <span className={cn("ml-auto rounded-pill px-1.5 py-px text-[10px] font-bold", isActive ? "bg-white/25 text-white" : "bg-muted text-muted-foreground")}>
            待建
          </span>
        ) : null}
      </button>
    );
  };

  return (
    <nav aria-label="一级导航" className="flex h-screen w-[248px] shrink-0 flex-col gap-0.5 overflow-y-auto border-r border-border bg-card/70 p-3 backdrop-blur-2xl backdrop-saturate-150">
      <div className="flex items-center gap-2.5 px-2.5 pb-4 pt-1.5">
        <div className="flex h-[34px] w-[34px] shrink-0 items-center justify-center rounded-[10px] bg-gradient-to-br from-primary to-purple text-[17px] font-bold text-white shadow-sm">
          令
        </div>
        <div className="flex min-w-0 flex-col leading-tight">
          <b className="truncate text-[15px] font-bold tracking-tight">不令 OPC</b>
          <span className="truncate text-[11px] text-muted-foreground">其身正，不令而行。</span>
        </div>
      </div>

      {core.map(renderItem)}

      <div className="mb-0.5 mt-2.5 px-2.5 text-[10.5px] font-bold uppercase tracking-wider text-muted-foreground">支撑</div>
      {support.map(renderItem)}

      <div className="mt-auto border-t border-border pt-3.5">
        <div className="flex gap-1 rounded-s bg-muted p-[3px]">
          {(["light", "dark", "system"] as const).map((t) => (
            <button
              key={t}
              type="button"
              onClick={() => onThemeChange(t)}
              className={cn(
                "flex-1 rounded-[7px] py-1.5 text-[11.5px] transition-colors",
                theme === t ? "bg-card text-foreground shadow-sm" : "text-muted-foreground hover:text-foreground",
              )}
            >
              {t === "light" ? "浅色" : t === "dark" ? "深色" : "系统"}
            </button>
          ))}
        </div>
      </div>
    </nav>
  );
}
