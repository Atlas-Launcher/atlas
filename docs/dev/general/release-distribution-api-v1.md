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
- `ATLAS_RELEASE_TOKEN`: Bearer token for an Atlas user with `admin` role (required)
- `TAURI_SIGNING_PRIVATE_KEY`: Required by `launcher-release.yml` for signed Tauri updater artifacts
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`: Password for `TAURI_SIGNING_PRIVATE_KEY` (launcher workflow only)

## Required Repository Variables

- None required today for `launcher|cli|runner|runnerd` release workflows.
- `GITHUB_TOKEN` is provided automatically by GitHub Actions and is used for GitHub Release upload/download steps.

## How to Obtain Each Secret

- `ATLAS_HUB_URL`
  - Use your deployed Hub origin (for example production Vercel URL).
  - Store as a GitHub Actions repository secret named `ATLAS_HUB_URL`.
- `ATLAS_RELEASE_TOKEN` (admin-only)
  - Sign in to Hub with an account that has `admin` role.
  - Obtain that account’s bearer token (for Atlas CLI users, run `atlas auth signin --hub-url <hub-url>` and use `access_token` from `~/.atlas/cli-auth.json`).
  - Store it as GitHub Actions repository secret `ATLAS_RELEASE_TOKEN`.
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
- Upload presign for distribution artifact keys (`artifacts/{launcher|cli|runner|runnerd}/...`) is admin-only.
- Publishing is platform-scoped; each `{os,arch}` group is registered in a separate publish call.
- Existing GitHub release publishing remains in place for public release assets.
