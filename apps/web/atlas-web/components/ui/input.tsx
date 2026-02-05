import * as React from "react";

import { cn } from "@/lib/utils";

const Input = React.forwardRef<HTMLInputElement, React.InputHTMLAttributes<HTMLInputElement>>(
  ({ className, type, ...props }, ref) => (
    <input
      type={type}
      className={cn(
        "flex h-12 w-full rounded-2xl border border-[var(--atlas-ink)]/20 bg-white px-4 text-sm text-[var(--atlas-ink)] placeholder:text-[var(--atlas-ink-muted)] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--atlas-accent)]",
        className
      )}
      ref={ref}
      {...props}
    />
  )
);
Input.displayName = "Input";

export { Input };
