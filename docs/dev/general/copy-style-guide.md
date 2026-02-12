# Atlas Copy Style Guide

## Personas

### Player (launcher + web onboarding)
- Audience: comfortable users, not technical specialists.
- Voice: calm, direct, reassuring.
- Prefer: one clear action at a time.
- Avoid: modding jargon, infrastructure details, blameful tone.

### Creator (web + atlas CLI)
- Audience: understands packs/mods, limited systems knowledge.
- Voice: practical, concise, task-oriented.
- Prefer: explicit outcomes and next commands.
- Avoid: deep platform internals unless required.

### Server Host (runner install + atlas-runner)
- Audience: basic Linux CLI familiarity.
- Voice: step-by-step, safe defaults first.
- Prefer: copy/paste commands, clear verification checks.
- Avoid: assuming advanced ops knowledge.

## Global Rules
- Use sentence case for labels and actions.
- Keep one primary CTA per step; secondary actions stay minimal.
- Replace vague failures with actionable guidance.
- Prefer "No packs available yet. Create one to get started." over "No packs found."
- Keep repeated terminology stable across surfaces:
  - "Atlas account"
  - "Microsoft account"
  - "Link accounts"
  - "Open Atlas Launcher"
  - "Launch Assist"

## UI Writing Patterns
- Primary action labels: verb-first and specific.
- Helper text: one short sentence, no duplication.
- Error text: what happened + what to do next.
- Empty states: current state + clear next step.

## CLI Writing Patterns
- Output first line states outcome.
- Follow with one explicit next command where relevant.
- Non-interactive failures should always include remediation.
