# Creator and Server Host Guide

This guide is for users who author packs, publish builds, and/or host servers.

## 1. What You Manage

As a creator/server host, you manage:
- pack source in Git,
- build/release through Atlas Hub APIs and CLI/CI,
- channel promotion (`dev`, `beta`, `production`),
- server runtime consumption through runner-v2 + runnerd-v2.

## 2. Recommended Flow

1. Keep pack content/config in Git.
2. Build artifact from source (`atlas pack build` or CI deploy path).
3. Publish through CI/Hub endpoints.
4. Promote channels by moving channel pointers.
5. Server hosts pull/apply via runner tooling.

## 3. CLI Tooling

- Pack/deploy tool: `atlas` (apps/cli)
- Server operator CLI: `atlas-runner` (apps/runner-v2)
- Server daemon: `atlas-runnerd` (apps/runnerd-v2)

Legacy note:
- `apps/runner` (runner v1) is legacy and unsupported.

## 4. Channel Model

- Builds are immutable.
- Channels are mutable pointers.
- Promote by changing channel pointer, not by editing old artifacts.

## 5. Server Host Basics

Runner-v2 + runnerd-v2 target:
- macOS
- Linux
- WSL

Typical cycle:
1. Authenticate/configure runner CLI.
2. Start daemon if not running.
3. Apply/start with `up` flow.
4. Monitor logs and use backup operations.

Quick Linux install path:
- `curl -fsSL "${NEXT_PUBLIC_BETTER_AUTH_URL%/}/download/runner/install" | sudo bash -s --`
- Verify with `atlas-runner --version`.
- See `docs/user/runner-install.md` for details.

Platform notes:
- Linux VPS/dedicated host is the recommended runner deployment target.
- macOS is natively supported with manual `atlas-runner` + `atlas-runnerd` downloads.
- Windows deployments should use WSL and pass `--no-daemon-install` to the install script.

## 6. Troubleshooting

- If CI upload fails: verify CI auth mode (OIDC or valid user token).
- If runner cannot apply: verify daemon connectivity and pack/build availability.
- If Java/runtime issues occur: current provisioning verifies runtime integrity and reinstalls on mismatch.
