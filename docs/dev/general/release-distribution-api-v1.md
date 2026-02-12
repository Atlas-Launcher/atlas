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
- `ATLAS_RELEASE_TOKEN`: Bearer token for a user with `admin` or `creator` role

## Manifest Format

The action accepts a manifest file (`files_manifest`) with one row per artifact:

`path|os|arch|kind|filename(optional)`

Where:

- `os`: `windows | macos | linux`
- `arch`: `x64 | arm64`
- `kind`: `installer | binary | signature | updater-manifest | other`

## Notes

- Artifacts are uploaded under: `artifacts/{product}/{version}/{os}/{arch}/{filename}`.
- Publishing is platform-scoped; each `{os,arch}` group is registered in a separate publish call.
- Existing GitHub release publishing remains in place for public release assets.
