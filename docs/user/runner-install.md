# Atlas Runner Install (VPS-first)

This guide assumes you are installing on a Linux VPS or dedicated host.

## Quick Install

```bash
curl -fsSL "${NEXT_PUBLIC_BETTER_AUTH_URL%/}/download/runner/install" | sudo bash
```

What this does:
- Detects Linux architecture (`x64` or `arm64`)
- Downloads the latest stable runner build
- Installs to `/usr/local/bin/atlas-runner`

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
- Windows hosts should run Atlas Runner through WSL and follow the Linux install flow.
- Runner daemon setup is handled in runner workflows/commands after install.
