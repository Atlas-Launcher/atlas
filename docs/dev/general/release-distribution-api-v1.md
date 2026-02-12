# Release Publishing via Distribution API v1

Atlas release workflows now publish immutable artifacts through a shared local GitHub Action:

- `.github/actions/atlas-release`

This action is used by:

- `.github/workflows/launcher-release.yml`
- `.github/workflows/cli-release.yml`
- `.github/workflows/runner-release.yml`

## Purpose

One reusable publish path for all distributable products (`launcher`, `cli`, `runner`, `runnerd`):

1. Compute artifact `sha256` and `size`
2. Upload bytes to Atlas-managed storage via `/api/v1/storage/presign`
3. Register release metadata via `/api/v1/releases/{product}/publish`

## Required Repository Secrets

- `ATLAS_HUB_URL`: Atlas Hub base URL (for example `https://hub.example.com`)
- `ATLAS_APP_DEPLOY_TOKEN`: App deploy token for distribution release publishing (required)
- `TAURI_SIGNING_PRIVATE_KEY`: Required by `launcher-release.yml` for signed Tauri updater artifacts
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`: Password for `TAURI_SIGNING_PRIVATE_KEY` (launcher workflow only)

## Required Repository Variables

- None required today for `launcher|cli|runner|runnerd` release workflows.
- `GITHUB_TOKEN` is provided automatically by GitHub Actions and is used for GitHub Release upload/download steps.

## How to Obtain Each Secret

- `ATLAS_HUB_URL`
  - Use your deployed Hub origin (for example production Vercel URL).
  - Store as a GitHub Actions repository secret named `ATLAS_HUB_URL`.
- `ATLAS_APP_DEPLOY_TOKEN` (admin-only issuance)
  - Create an app deploy token via `POST /api/v1/deploy/app-tokens` with an admin account.
  - Store it as GitHub Actions repository secret `ATLAS_APP_DEPLOY_TOKEN`.
- `ATLAS_PACK_DEPLOY_TOKEN` (optional, pack automation fallback)
  - Create via `POST /api/v1/packs/{packId}/deploy-tokens` as pack `creator/admin` (or global admin).
  - Use in pack CI/deploy automation as `ATLAS_PACK_DEPLOY_TOKEN` when GitHub OIDC is unavailable.
- `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`
  - Generate/export your Tauri updater signing keypair and password from your release key management process.
  - Save the private key and password as GitHub Actions repository secrets with those exact names.

## Manifest Format

The action accepts a manifest file (`files_manifest`) with one row per artifact:

`path|os|arch|kind|filename(optional)`

Where:

- `os`: `windows | macos | linux`
- `arch`: `x64 | arm64`
- `kind`: `installer | binary | signature | updater-manifest | other`

## Notes

- Artifacts are uploaded under: `artifacts/{product}/{version}/{os}/{arch}/{filename}`.
- Upload presign for distribution artifact keys (`artifacts/{launcher|cli|runner|runnerd}/...`) requires admin session or `x-atlas-app-deploy-token`.
- Publishing is platform-scoped; each `{os,arch}` group is registered in a separate publish call.
- Existing GitHub release publishing remains in place for public release assets.
- `ATLAS_HUB_URL` may include a trailing slash; the release action normalizes it before calling Hub APIs.
- The release action publishes artifact payloads using `key` (raw object key) + `provider` from presign responses.
- Launcher workflow kind mapping differentiates installables from updater payloads:
  - `installer`: `.dmg`, `.pkg`, `.exe`, `.msi`, `.deb`, `.rpm`, `.AppImage`
  - `binary`: `.app.tar.gz`, `.app.zip`, `.nsis.zip`, `.msi.zip`, `.AppImage.tar.gz`
  - `signature`: `*.sig` files paired to the exact updater payload filename
- Launcher workflow arch inference first uses explicit arch tokens in filenames (`arm64`, `aarch64`, `x64`, `x86_64`, `amd64`), then falls back to the unique arch already detected for the same OS in the current publish batch.
- Launcher release CI now enforces updater signature completeness:
  - After bundle build, it signs any missing updater payload signatures with `tauri signer sign`.
  - Before publish, it fails the workflow if any `binary` payload in the manifest is missing its matching `.sig`.
  - Signature generation uses `TAURI_SIGNING_PRIVATE_KEY` / `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` from env (via `tauri signer sign <file>`), avoiding direct CLI passing of multiline secrets.
  - Signing now passes absolute payload paths because the signer runs in the launcher workspace (`apps/launcher`) while bundle discovery scans repo-root `target/**/release/bundle/**`.
