---
name: vue-styling
description: Atlas Launcher Vue styling system. Use when editing launcher UI visuals, theming, dark/light behavior, glass surfaces, elevation, backgrounds, or component-level UX polish so changes stay aligned with launcher design language.
---

# Vue Styling (Launcher)

Use this skill for UI work in `apps/launcher`.

## Source Of Truth

1. `apps/launcher/src/assets/main.css`
2. `apps/launcher/tailwind.config.ts`
3. Existing launcher components using `glass` and status styles (for example `TitleBar.vue`, `PlayCard.vue`, `InstancesCard.vue`)

If guidance and code disagree, follow code.

## Design Language

- Theme modes: `light`, `dark`, `system`.
- Brand mood: warm paper + cool blue + amber accents in light mode; deep navy glass in dark mode.
- Surfaces: blurred translucent glass panels over radial-gradient atmospheric backgrounds.
- Elevation: layered, soft shadows; no harsh black drop-shadows.
- Typography: `Sora` for UI text, `IBM Plex Mono` for technical/meta text.

## Required Styling Rules

- Keep all palette/shadow/glass values centralized in `main.css` variables.
- Prefer semantic utility classes (`glass`, status classes, tokenized Tailwind colors) over one-off hex values.
- Preserve `dark` class behavior and `system` fallback via launcher settings logic.
- Use `rounded-2xl` / `rounded-full` panel language unless a component already intentionally differs.
- Keep hover/focus states subtle and readable in both modes.

## Implementation Checklist

1. Confirm token impact first (`:root` + `.dark` in `main.css`).
2. Reuse existing launcher classes (`glass`, pills, status dots) before creating new patterns.
3. Validate legibility on layered backgrounds in both themes.
4. Confirm contrast for disabled, muted, and destructive states.
5. Avoid introducing isolated visual styles not derived from tokens.

## UX Guardrails

- Prioritize clarity over decoration.
- Keep actionable controls visibly interactive.
- Keep density compact but scannable.
- Use motion sparingly (`fade-up` style transitions only when it improves hierarchy).
