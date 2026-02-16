# Atlas Monorepo

Atlas is a modpack platform with source-in-Git and binary distribution artifacts.

## Platform Support

If a platform is not listed, it is unsupported.

- Launcher: Windows + macOS + Linux (best effort)
- Runnerd/Runner CLI: macOS + Linux (including WSL)
- Web: Vercel

Legacy status:
- Runner v1 (`apps/runner`) is legacy and unsupported.

## Projects

- `apps/cli`: pack authoring/deploy CLI (`atlas`)
- `apps/launcher`: desktop launcher
- `apps/web`: Hub UI + API
- `apps/runner-v2`: operator CLI (`atlas-runner`)
- `apps/runnerd-v2`: daemon (`atlas-runnerd`)
- `apps/runner`: legacy runner v1 (unsupported)

## Docs

- Developer docs: `docs/dev/README.md`
- User docs: `docs/user/README.md`

## License

This repository is proprietary and source-available under `LICENSE`.
Commercial use and re-hosting are not permitted.
