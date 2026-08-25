import * as React from "react";
import { cva, type VariantProps } from "class-variance-authority";
import { cn } from "@/lib/utils";

const badgeVariants = cva("inline-flex items-center gap-1 rounded-pill px-2.5 py-0.5 text-[10.5px] font-bold whitespace-nowrap", {
  variants: {
    variant: {
      default: "bg-primary/10 text-primary",
      secondary: "bg-muted text-muted-foreground",
      destructive: "bg-destructive/10 text-destructive",
      success: "bg-success/10 text-success",
      warning: "bg-warning/10 text-warning",
      outline: "border border-border text-foreground/70",
    },
  },
  defaultVariants: { variant: "default" },
});

export interface BadgeProps extends React.HTMLAttributes<HTMLSpanElement>, VariantProps<typeof badgeVariants> {}

function Badge({ className, variant, ...props }: BadgeProps) {
  return <span className={cn(badgeVariants({ variant }), className)} {...props} />;
}

export { Badge, badgeVariants };
