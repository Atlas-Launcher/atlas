import * as React from "react";
import { cva, type VariantProps } from "class-variance-authority";

import { cn } from "@/lib/utils";

const badgeVariants = cva(
  "inline-flex items-center rounded-full border border-transparent px-3 py-1 text-[10px] font-semibold uppercase tracking-[0.2em]",
  {
    variants: {
      variant: {
        default: "bg-[var(--atlas-ink)] text-[var(--atlas-cream)]",
        secondary: "bg-[var(--atlas-cream)]/70 text-[var(--atlas-ink)] border-[var(--atlas-ink)]/10",
        outline: "border-[var(--atlas-ink)]/20 text-[var(--atlas-ink)]",
      },
    },
    defaultVariants: {
      variant: "secondary",
    },
  }
);

export interface BadgeProps
  extends React.HTMLAttributes<HTMLDivElement>,
    VariantProps<typeof badgeVariants> {}

function Badge({ className, variant, ...props }: BadgeProps) {
  return <div className={cn(badgeVariants({ variant }), className)} {...props} />;
}

export { Badge, badgeVariants };
