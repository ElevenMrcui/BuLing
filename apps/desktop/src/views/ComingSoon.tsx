import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { COMING_SOON_COPY, NAV_ITEMS, type ViewKey } from "../nav";

export function ComingSoonView({ view, onNavigate }: { view: ViewKey; onNavigate: (v: ViewKey) => void }) {
  const item = NAV_ITEMS.find((n) => n.key === view)!;
  const copy = COMING_SOON_COPY[view];
  const Icon = item.icon;

  return (
    <>
      <header className="flex items-start gap-4 px-8 pb-4 pt-6">
        <div>
          <h1 className="text-[22px] font-bold tracking-tight">{item.label}</h1>
          <p className="mt-0.5 text-[12.5px] text-muted-foreground">这块还没接后端能力</p>
        </div>
      </header>
      <div className="flex flex-col gap-5 px-8 pb-10">
        <Card>
          <CardContent className="flex flex-col items-center gap-2.5 py-16 text-center">
            <Icon className="h-10 w-10 text-purple" strokeWidth={1.5} />
            <b className="text-base font-bold">{copy?.title ?? `${item.label}即将推出`}</b>
            <p className="max-w-[420px] text-[13px] text-muted-foreground">
              {copy?.body ?? "这一版 Runtime 还没有对应的能力，先占个导航位置，不假装它已经能用。"}
            </p>
            {copy?.goTo ? (
              <Button className="mt-1" onClick={() => onNavigate(copy.goTo!)}>
                去{NAV_ITEMS.find((n) => n.key === copy.goTo)?.label}
              </Button>
            ) : null}
          </CardContent>
        </Card>
      </div>
    </>
  );
}
