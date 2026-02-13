import Link from "next/link";

import { PERSONAS, type PersonaId } from "@/lib/docs/types";
import { cn } from "@/lib/utils";

const PERSONA_LABELS: Record<PersonaId, string> = {
  player: "Player",
  creator: "Creator",
  host: "Host",
};

type PersonaContextTabsProps = {
  activePersona: PersonaId;
};

export default function PersonaContextTabs({ activePersona }: PersonaContextTabsProps) {
  return (
    <nav
      aria-label="Documentation persona"
      className="atlas-glass rounded-md p-0.5"
    >
      <div className="grid grid-cols-3 gap-0.5">
        {PERSONAS.map((persona) => {
          const isActive = persona === activePersona;
          return (
            <Link
              key={persona}
              href={`/docs/${persona}`}
              className={cn(
                "rounded-sm px-2 py-1.5 text-center text-[9px] font-semibold uppercase tracking-[0.12em] transition",
                isActive
                  ? "atlas-inverse-surface"
                  : "text-[var(--atlas-ink-muted)] hover:bg-[var(--atlas-surface-strong)] hover:text-[var(--atlas-ink)]"
              )}
            >
              {PERSONA_LABELS[persona]}
            </Link>
          );
        })}
      </div>
    </nav>
  );
}
