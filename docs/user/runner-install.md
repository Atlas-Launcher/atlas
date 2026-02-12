# Atlas Runner Install (VPS-first)

This guide assumes you are installing on a Linux VPS or dedicated host.

## Quick Install

```bash
curl -fsSL "${NEXT_PUBLIC_BETTER_AUTH_URL%/}/download/runner/install" | sudo bash -s --
```

What this does:
- Detects Linux architecture (`x64` or `arm64`)
- Downloads the latest stable runner build
- Installs to `/usr/local/bin/atlas-runner`
- Runs `atlas-runner install` to install/update `atlas-runnerd` systemd service

WSL install (skip daemon install):

```bash
curl -fsSL "${NEXT_PUBLIC_BETTER_AUTH_URL%/}/download/runner/install" | sudo bash -s -- --no-daemon-install
```

## Verify

```bash
atlas-runner --version
```

## Manual Download Links

- `${NEXT_PUBLIC_BETTER_AUTH_URL%/}/download/runner/latest/linux/x64`
- `${NEXT_PUBLIC_BETTER_AUTH_URL%/}/download/runner/latest/linux/arm64`

## Notes

- Linux is the recommended install path for servers and VPS environments.
- macOS is natively supported, but you must manually download both `atlas-runner` and `atlas-runnerd`.
- Windows hosts should run Atlas Runner through WSL and use `--no-daemon-install`.
- Runner daemon setup now runs during install by default unless explicitly skipped.
- When installed as a systemd service with `ATLAS_SYSTEMD_MANAGED=1` and running as root, runnerd can stage and apply stable `runner`/`runnerd` binary updates during the midnight backup window.
