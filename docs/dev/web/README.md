# Web (Hub) Developer Guide

Project: `apps/web`

Framework: Next.js app router

## Responsibilities

- Atlas Hub UI and API gateway.
- Auth/session and role/member access checks.
- Pack/build/channel management APIs.
- Launcher/runner API endpoints.
- CI upload/complete flows and download endpoints.
- Storage provider abstraction (R2 / Vercel Blob harness).
- GitHub repo onboarding automation, including pack CI workflow + required Actions secrets.

## API Surface

Route handlers are implemented under:
- `apps/web/app/api`

Download/update endpoints are implemented under:
- `apps/web/app/download`

See:
- `api-spec.md`
- `openapi.yaml`

## Runner service tokens in Hub UI

Pack creators and admins can now manage runner service tokens directly in the
pack dashboard:
- Navigate to **Dashboard -> Pack -> Manage**.
- In **Runner Service Tokens**, create a token with an optional name.
- Copy the plaintext token from the one-time reveal panel.
- Use the table to audit token prefixes and revoke tokens.

Pack creators and admins can also manage CI pack deploy tokens in the same view:
- In **Pack Deploy Tokens**, create a token with an optional name.
- Copy the plaintext token from the one-time reveal panel.
- Use the table to audit token prefixes and revoke tokens.

Implementation lives in:
- `apps/web/app/dashboard/pack-dashboard-client.tsx`
- `apps/web/app/dashboard/components/manage-tab.tsx`
- `apps/web/app/api/v1/runner/tokens/route.ts`
- `apps/web/app/api/v1/packs/[packId]/deploy-tokens/route.ts`

### Pack list dedup guard

`GET /api/v1/packs` now applies a server-side dedup pass by `pack.id` before responding.
This protects dashboard views from legacy/invalid duplicate membership join results and keeps list ordering stable by newest `updatedAt`.

`GET /api/v1/launcher/packs` now also applies a secondary dedup pass by normalized GitHub repo identity (`owner/repo`) after membership deduping by `packId`.
This suppresses duplicate launcher pack cards caused by accidental duplicate pack records mapped to the same repository.

## Invite Onboarding Contract

`POST /api/v1/invites/accept` returns invite acceptance metadata:

- `success`
- `packId`
- `pack: { id, name, slug }`
- `recommendedChannel`

Invite behavior:
- Invite links are multi-use. Accepting an invite does not consume/revoke the link.
- Web invite flow does not issue launcher protocol deep links.
- Existing launcher users are told to open Atlas Launcher and press Refresh.
- Launcher download CTA now links to `/download/app` (guided installer page).

## Copy Standards

Player and creator copy constants are centralized under:
- `apps/web/app/_copy/player.ts`
- `apps/web/app/_copy/creator.ts`

Use these for repeated user-facing messaging to keep tone consistent.

## Shared Visual System

Atlas Hub now uses launcher-aligned visual primitives across public pages, docs, auth, and dashboard surfaces.

- Theme tokens are centralized in:
  - `apps/web/app/globals.css`
- Theme mode runtime (`light` / `dark` / `system`) is centralized in:
  - `apps/web/lib/theme/theme-mode.ts`
  - `apps/web/components/theme/theme-mode-switcher.tsx`
  - `apps/web/app/layout.tsx` (pre-hydration theme boot script + global switcher mount)
- Dark token activation must target root explicitly:
  - `:root.dark` / `:root[data-theme-mode="dark"]` in `apps/web/app/globals.css`
  This prevents mode drift where text variables fail to swap while dark backgrounds are visible.
- Shared visual utility classes are centralized in `globals.css`:
  - `atlas-glass`
  - `atlas-panel`
  - `atlas-panel-soft`
  - `atlas-panel-strong`
  - `atlas-inverse-surface` (for dark cards/buttons that should stay dark in both theme modes)
  - `atlas-shadow-button`
  - `atlas-app-shell`
- Inverse content colors are tokenized to avoid white backdrops in dark mode:
  - `--atlas-inverse-bg`
  - `--atlas-inverse-fg`
  - `--atlas-inverse-muted`
- Atlas tokens use explicit cross-browser color values (`hex`/`rgba`) for reliable rendering:
  - `--atlas-cream`
  - `--atlas-ink`
  - `--atlas-ink-muted`
  - `--atlas-surface-*`
- Launcher line texture is centralized and shared across light/dark mode:
  - `--atlas-lines-image` in `apps/web/app/globals.css`
  - This should match launcher stripe values from `apps/launcher/src/assets/main.css`.
- Footer styling is centralized through:
  - `atlas-footer`
  This keeps public page footers consistent across docs/download/home and avoids fixed white backgrounds in dark mode.
- Brand mark consistency:
  - `apps/web/public/atlas-mark.svg` should mirror `apps/launcher/src-tauri/icons/atlas-mark.svg` (no outer frame stroke) so the navbar icon renders cleanly at small sizes.
- Glass/panel elevation was intentionally softened (reduced blur and shadow spread) for cleaner, less heavy UI density.
- Contrast guardrail: use tokenized text colors (`--foreground`, `--muted-foreground`, inverse tokens) instead of low-opacity text-on-surface combinations.
- Avoid fixed Tailwind gray scale utilities (`text-gray-*`, `bg-gray-*`) on app surfaces. Use Atlas tokens (`--atlas-ink`, `--atlas-ink-muted`, `--atlas-surface-*`) so text/surfaces remain legible in both light and dark modes.
- Global utility guardrail:
  - `text-foreground`, `text-muted-foreground`, and related placeholder/file text utilities are hard-mapped to Atlas tokens in `globals.css` to keep contrast consistent across pages.
- Shared UI primitives consume these tokens/classes:
  - `apps/web/components/ui/button.tsx`
  - `apps/web/components/ui/card.tsx`
  - `apps/web/components/ui/dialog.tsx`
  - `apps/web/components/ui/tabs.tsx`
  - `apps/web/components/ui/input.tsx`
  - `apps/web/components/ui/textarea.tsx`
  - `apps/web/components/ui/input-group.tsx`

Implementation rule: add/adjust colors, shadows, and glass behavior in tokens/utilities first; only then patch leaf components.

## Distribution API v1

The web app now exposes a unified distribution registry for launcher/cli/runner/runnerd:
- `GET /api/v1/releases/{product}/latest/{os}/{arch}`
- `GET /api/v1/releases/{product}/{version}/{os}/{arch}`
- `GET /api/v1/download/{downloadId}`
- `GET /api/v1/launcher/updates/{os}/{arch}`
- `GET /api/v1/launcher/updates/{channel}/{os}/{arch}`
- `POST /api/v1/releases/{product}/publish`

Data is backed by `distribution_releases`, `distribution_release_platforms`, and `distribution_artifacts` (Drizzle migration `0011_distribution_api_v1.sql`).

## Repository Onboarding

When packs are imported/created with GitHub repo setup, Hub configures repository automation by:
- Updating `atlas.toml` (`pack_id`, `hub_url`)
- Ensuring `.github/workflows/atlas-build.yml`
- Enabling GitHub Actions/workflows
- Creating/updating Actions secrets:
  - `ATLAS_HUB_URL`
  - `ATLAS_PACK_DEPLOY_TOKEN` (managed pack-scoped deploy token)

Implementation notes:
- GitHub Contents API paths must be encoded per path segment (not as a single `encodeURIComponent` call on the full path), otherwise nested paths like `.github/workflows/atlas-build.yml` can fail.
- Writing workflow files requires GitHub App installation permissions that include workflow write access.
- Repository onboarding now always provisions `ATLAS_HUB_URL` and
  `ATLAS_PACK_DEPLOY_TOKEN` GitHub Actions secrets, including fallback paths
  where `atlas.toml` is missing from a template repository.
- Pack creation endpoints now include an idempotency guard by normalized GitHub repo identity for the same owner account:
  - `POST /api/v1/packs` (repo import path)
  - `POST /api/v1/github/repos` (template repo creation path)
  If a matching owned pack already exists, the endpoint returns that pack instead of creating a duplicate.

Legacy GitHub-release proxy download routes were removed in favor of distribution-native downloads:
- Primary artifact redirects now resolve through `GET /api/v1/download/{downloadId}`.
- Install pages consume `GET /api/v1/releases/{product}/latest/{os}/{arch}` data.

## Deployment Target

- Vercel

If a target is not listed, it is unsupported.

## User Docs Site

User-facing docs now render directly in Hub under:
- `/docs`
- `/docs/{persona}`
- `/docs/{persona}/{slug}`

Legacy reader-mode routes now redirect to the standard docs shell:
- `/docs/read/{persona}` -> `/docs/{persona}`
- `/docs/read/{persona}/{slug}` -> `/docs/{persona}/{slug}`

Implementation lives in:
- `apps/web/app/docs`
- `apps/web/components/docs`
- `apps/web/lib/docs`

Content source and nav control live outside the app package:
- `docs/user/**/*.md` (frontmatter required)
- `docs/user/navigation.json` (sidebar + prev/next order)

Local search index is generated from user docs content at runtime/build using `apps/web/lib/docs/content.ts` and scoped to `docs/user` only.
Developer docs under `docs/dev` are intentionally not exposed by the public docs routes.
Docs markdown parser ignores level-1 (`#`) headings because page chrome already renders document titles.

### Docs readability focus (current UX)

- Docs now use a traditional documentation shell:
  - left navigation sidebar
  - central content column
- docs layout keeps the branded Atlas grid + soft radial background accents
- sidebar includes a persona context tab switcher (`Player`, `Creator`, `Server Host`) for quick context changes
- sidebar section subtitle text is hidden in favor of the tab switcher
- sidebar includes docs search (collapsed to input by default) with glass-style background for legibility
  - doc pages place "On this page" TOC in the left sidebar under search, also with glass-style background
- on desktop, docs default to dual-pane (`sidebar + content`) with a `Hide sidebar` / `Show sidebar` toggle in the content column
- primary text panels use centralized glass/panel utility classes to improve readability on the patterned background
- docs routes avoid fixed `bg-white*` backdrops in favor of `atlas-panel`/`atlas-panel-soft` surfaces so contrast stays consistent in both theme modes
- Persona landing pages still prioritize the two essentials first (`startSlug` and `troubleshootingSlug`) before listing remaining references.
- Search keeps the same scoring behavior (title/keywords + optional priority paths) and expands to filters/results on focus or query.
- Doc pages no longer include a separate reader-mode toggle.

## Lint Guardrails

- Prefer `unknown` + narrowing over `any` in route handlers.
- In React components, avoid synchronous `setState` calls inside `useEffect` when the value can be derived during render/init.
- Define callback functions before effects that reference them, and use `useCallback` when dependency tracking is required.
- Use escaped apostrophes in JSX text where required by lint rules.
- Use `next/link` for internal navigation paths instead of raw `<a href>` tags.
