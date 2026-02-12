# Atlas Platform Developer Docs

These docs are for engineers working on Atlas code in this repository.

## Scope
- `general/`: cross-project architecture, platform support, and repo map.
- `launcher/`: desktop launcher (Vue + Tauri).
- `web/`: Next.js web app + API + download/update routes.
- `runner-v2/`: operator CLI (`apps/runner-v2`).
- `runnerd-v2/`: daemon/service process (`apps/runnerd-v2`).
- `cli/`: pack authoring/deploy CLI (`apps/cli`).

## Start Here
1. Read `general/executive-summary.md`.
2. Read `general/platform-support.md`.
3. Read `general/deployment.md` for environment, rollout, and verification guidance.
4. Read `general/release-distribution-api-v1.md` for CI release publishing flow.
5. Open the project folder you plan to change.

## Important Status Notes
- `apps/runner` (Runner v1) is legacy and unsupported.
- If a platform is not listed in platform support docs, treat it as unsupported.
