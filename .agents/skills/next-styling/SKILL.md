---
name: next-styling
description: Atlas Hub Next.js styling system. Use when editing apps/web UI themes, backgrounds, surfaces, shadows, glass treatment, dark/light mode behavior, or shared component visual language.
---

# Next Styling (Hub)

Use this skill for visual and UX styling work in `apps/web`.

## Source Of Truth

1. `apps/web/app/globals.css`
2. Shared UI primitives in `apps/web/components/ui/*`
3. Theme mode runtime files:
   - `apps/web/lib/theme/theme-mode.ts`
   - `apps/web/components/theme/theme-mode-switcher.tsx`
   - `apps/web/app/layout.tsx`

Prefer tokenized, centralized styles over per-page ad hoc values.

## Design Language

- Launcher-aligned theming: `light`, `dark`, `system`.
- Background atmosphere: radial color washes + subtle diagonal texture.
- Surfaces: translucent glass and panel variants driven by CSS vars.
- Elevation: layered, soft shadows (`--atlas-shadow-*`).
- Typography hierarchy: clear headings, compact utility text, mono only for technical identifiers.

## Required Styling Rules

- Centralize palette, surface, glass, and shadow definitions in `globals.css` variables.
- Reuse shared utility classes (`atlas-glass`, `atlas-panel`, `atlas-panel-soft`, `atlas-panel-strong`, `atlas-shadow-button`).
- Keep dark-mode compatibility: avoid hardcoded light-only colors unless intentionally overridden.
- Favor shadcn primitives in `components/ui` for consistent interaction states.
- Ensure all modified UI remains legible on textured backgrounds.

## Implementation Checklist

1. Start with tokens/utilities before touching leaf components.
2. Update shared primitives (`button`, `card`, `dialog`, `tabs`, inputs) for system-wide consistency.
3. Apply shell/background classes at layout level, not per-component hacks.
4. Verify light, dark, and system mode behavior.
5. Ensure hover/focus/active/disabled states remain coherent across themes.

## UX Guardrails

- Keep information hierarchy obvious.
- Keep controls discoverable and keyboard focus-visible.
- Maintain visual consistency between public pages, docs, and dashboard.
- Reduce one-off effects; use standardized surfaces and elevation.
