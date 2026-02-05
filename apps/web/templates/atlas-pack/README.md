# Atlas Pack (Demo)

This repository is the default Atlas pack template. Update the TOML files and configs,
push to `main`, and the GitHub Action will compile + upload a `.atlas` to the Atlas Hub.

## Files
- `atlas.toml` — Pack metadata, versions, and CLI defaults.
- `mods.toml` — Mod/asset manifest entries (placeholders).
- `config/` — Game config files and server/client configs.
- `.github/workflows/atlas-build.yml` — CI that compiles + uploads a `.atlas`.

### atlas.toml layout
The template includes:
- `[metadata]` with pack name and description
- `[versions]` with `mc`, `modloader`, `modloader_version`
- `[cli]` with pack-specific CLI defaults:
- `pack_id` (optional; can be left blank and supplied via env)
- `default_channel` (defaults to `dev`)

## GitHub Secrets (required)
Set these secrets in GitHub → Settings → Secrets and variables → Actions:
- `ATLAS_PACK_ID` (from the Atlas Hub)
- `ATLAS_DEPLOY_KEY` (deploy API key from the Atlas Hub)

## Local workflow
To test locally:
```bash
./scripts/build-atlas.sh
```

This produces `dist/atlas-pack.atlas`.

## CI build notes
The GitHub Action installs the Atlas CLI and runs:
```bash
atlas deploy --channel dev --commit-hash "${GITHUB_SHA}"
```
The hub URL defaults to `https://atlas.nathanm.org` via the `[cli]` section in `atlas.toml`.
Update the CLI install URL in `.github/workflows/atlas-build.yml` once the CLI repo is finalized.
