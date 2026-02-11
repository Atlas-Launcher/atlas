# Platform Support

This is the current support policy for Atlas components in this repository.

If a platform is not listed, it is unsupported.

## Supported Targets

| Component | Supported Platforms | Notes |
|---|---|---|
| Launcher (`apps/launcher`) | Windows, macOS, Linux | Linux is best effort. |
| Runner CLI + Runnerd (`apps/runner-v2`, `apps/runnerd-v2`) | macOS, Linux, WSL | Includes Linux systemd install path where available. |
| Web (`apps/web`) | Vercel | Production hosting target. |

## Unsupported / Legacy

- Runner v1 (`apps/runner`) is legacy and unsupported.

## Practical Guidance

- New feature work should target launcher + runner-v2/runnerd-v2 + web.
- Avoid adding behavior that depends on unsupported platforms unless explicitly requested.
