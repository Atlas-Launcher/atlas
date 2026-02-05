"use client";

import { ReactNode } from "react";

import { Badge } from "@/components/ui/badge";
import type { Role } from "@/app/dashboard/types";

interface DashboardHeaderProps {
  workspaceName: string;
  email: string;
  role: Role;
  actions?: ReactNode;
  tabs?: ReactNode;
}

export default function DashboardHeader({
  workspaceName,
  email,
  role,
  actions,
  tabs,
}: DashboardHeaderProps) {
  return (
    <div className="pb-4">
      <div className="flex flex-wrap items-center justify-between gap-4">
        <div>
          <p className="text-xs font-semibold uppercase tracking-[0.2em] text-[var(--atlas-ink-muted)]">
            DASHBOARD
          </p>
          <div className="flex flex-wrap items-center gap-2">
            <h1 className="text-xl font-semibold">{workspaceName}</h1>
            <Badge variant="outline" className="text-[10px] uppercase tracking-[0.2em]">
              {role}
            </Badge>
          </div>
          <p className="text-xs text-[var(--atlas-ink-muted)]">{email}</p>
        </div>
        <div className="flex items-center gap-2">{actions}</div>
      </div>
      {tabs ? <div className="mt-4">{tabs}</div> : null}
    </div>
  );
}
