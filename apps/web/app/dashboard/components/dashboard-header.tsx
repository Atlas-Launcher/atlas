"use client";

import { ReactNode } from "react";

import { Badge } from "@/components/ui/badge";
import type { Role } from "@/app/dashboard/types";

interface DashboardHeaderProps {
  workspaceName: string;
  email?: string;
  subtitle?: string;
  role?: Role;
  identifier?: string;
  eyebrow?: string;
  leading?: ReactNode;
  meta?: ReactNode;
  actions?: ReactNode;
  tabs?: ReactNode;
}

export default function DashboardHeader({
  workspaceName,
  email,
  subtitle,
  role,
  identifier,
  eyebrow = "Dashboard",
  leading,
  meta,
  actions,
  tabs,
}: DashboardHeaderProps) {
  const detail = subtitle ?? email;

  return (
    <div className="pb-4">
      <div className="flex flex-wrap items-center justify-between gap-4">
        <div className="flex min-w-0 items-start gap-3">
          {leading ? <div className="shrink-0">{leading}</div> : null}
          <div>
            <p className="text-xs font-semibold uppercase tracking-[0.2em] text-[var(--atlas-ink-muted)]">
              {eyebrow}
            </p>
            <div className="flex flex-wrap items-center gap-2">
              <h1 className="text-xl font-semibold">{workspaceName}</h1>
              {role ? (
                <Badge variant="outline" className="text-[10px] uppercase tracking-[0.2em]">
                  {role}
                </Badge>
              ) : null}
            </div>
            {identifier ? (
              <div className="mt-1">
                <Badge variant="outline" className="text-[10px] tracking-[0.16em] font-mono">
                  {identifier}
                </Badge>
              </div>
            ) : null}
            {detail ? <p className="text-xs text-[var(--atlas-ink-muted)]">{detail}</p> : null}
            {meta ? <div className="mt-2 flex flex-wrap items-center gap-2">{meta}</div> : null}
          </div>
        </div>
        <div className="flex items-center gap-2">{actions}</div>
      </div>
      {tabs ? <div className="mt-4">{tabs}</div> : null}
    </div>
  );
}
