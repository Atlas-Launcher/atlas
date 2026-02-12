# Atlas Deployment Guide (Developer)

This guide describes how Atlas is deployed across Hub, release pipelines, and runtime consumers.

## 1. Deployment Topology

- `apps/web` (Hub/API): deployed on Vercel.
- Database: managed PostgreSQL used by `apps/web` via Drizzle migrations.
- Artifact/object storage: Cloudflare R2 (or configured storage provider abstraction).
- Release publishing: GitHub Actions workflows publishing immutable distribution artifacts through Atlas APIs.
- Runtime consumers:
- Launcher (`apps/launcher`) downloads updater/release metadata via Hub endpoints.
- Runner (`apps/runner-v2` / `apps/runnerd-v2`) resolves release/build artifacts through Hub APIs.

## 2. Core Deployment Invariants

- Build artifacts are immutable.
- Release channels are mutable pointers (`stable`, `beta`, `dev` for distribution products; pack channels remain separate).
- Storage access is mediated through presigned URLs or Hub download indirection.
- Hub must not proxy artifact upload/download byte streams.
- CI publish paths must be authenticated and scoped.

## 3. Environments and Inputs

Minimum deployment inputs:

- Hub base URL (`ATLAS_HUB_URL` or deployment-equivalent runtime URL for clients/CI).
- Database connection string for web app migrations/runtime.
- Storage provider configuration (R2 or supported provider).
- Auth provider configuration (better-auth and linked providers such as GitHub/Microsoft where enabled).

For release publishing workflows:

- `ATLAS_HUB_URL`
- `ATLAS_APP_DEPLOY_TOKEN` (app deploy token, issued by an admin)

## 4. CI/CD Paths

### Web (Hub)

1. Push to branch / merge to target branch.
2. Vercel deploys `apps/web` manually only (Git-triggered deployments are disabled via `apps/web/vercel.json` with `git.deploymentEnabled: false`).
3. Run required DB migrations before enabling features dependent on new schema.

### Distribution Releases

Release workflows use `.github/actions/atlas-release`:

1. Compute artifact metadata (`sha256`, `size`).
2. Upload artifacts via `/api/v1/storage/presign` using the returned direct URL and any returned `uploadHeaders` (required for providers such as Vercel Blob).
3. For Vercel Blob, uploads should use short-lived, path-scoped client tokens generated per artifact (not the global read/write token in clients/CI).
4. Publish release metadata via `/api/v1/releases/{product}/publish`.
5. Launcher release workflow consumes build outputs from GitHub Actions workflow artifacts, not GitHub Releases.
6. CLI and runner release workflows also use GitHub Actions artifacts only; no workflow publishes GitHub Releases.

For pack deploy workflows:
- Prefer GitHub OIDC (`x-atlas-oidc-token`) where available.
- Optional fallback: `ATLAS_PACK_DEPLOY_TOKEN` (`x-atlas-pack-deploy-token`) scoped to one pack.

Reference:
- `docs/dev/general/release-distribution-api-v1.md`

### Linux CI for Launcher/Tauri

When launcher Tauri backend files are part of the change, Linux CI runners must have native system packages installed before Rust checks:
- `pkg-config`
- `libglib2.0-dev`
- `libgtk-3-dev`
- `libwebkit2gtk-4.1-dev`
- `libayatana-appindicator3-dev`
- `librsvg2-dev`
- `patchelf`

## 5. Database Migration Policy

- All schema changes must be checked in under `apps/web/drizzle/*.sql` and `apps/web/drizzle/meta/*`.
- Apply migrations before or with the matching web deployment.
- Treat migration + API changes as one release unit to avoid partial-contract runtime failures.

## 6. Post-Deploy Verification

After deploying Hub/API or release publishing changes:

1. Verify health of web routes and auth flows.
2. Verify distribution API lookups:
- `GET /api/v1/releases/{product}/latest/{os}/{arch}`
- `GET /api/v1/download/{downloadId}` redirect behavior
 - `GET /download/runner/latest/linux/{arch}` and `GET /download/runner/install`
3. Verify launcher updater endpoints for target channels/platforms.
4. Confirm runner install/update flows can fetch expected artifacts.
5. Check logs for 4xx/5xx spikes around auth, storage presign, and release publish routes.

## 7. Rollback Strategy

- Web/API rollback: revert to prior Vercel deployment if contracts permit.
- Release rollback: publish previous known-good version and move channel pointers (do not mutate existing artifacts).
- Migration rollback: prefer forward-fix migrations; avoid destructive rollback unless explicitly planned.

## 8. Operational Risks to Watch

- Mismatch between deployed web code and unapplied DB migrations.
- Missing/revoked app deploy token causing release publish failures.
- Storage provider misconfiguration causing broken download indirection.
- Channel pointer changes without validation on target platform artifacts.
