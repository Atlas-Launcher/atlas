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

Legacy GitHub-release proxy download routes were removed in favor of distribution-native downloads:
- Primary artifact redirects now resolve through `GET /api/v1/download/{downloadId}`.
- Install pages consume `GET /api/v1/releases/{product}/latest/{os}/{arch}` data.

## Deployment Target

- Vercel

If a target is not listed, it is unsupported.

## Lint Guardrails

- Prefer `unknown` + narrowing over `any` in route handlers.
- In React components, avoid synchronous `setState` calls inside `useEffect` when the value can be derived during render/init.
- Define callback functions before effects that reference them, and use `useCallback` when dependency tracking is required.
- Use escaped apostrophes in JSX text where required by lint rules.
- Use `next/link` for internal navigation paths instead of raw `<a href>` tags.
