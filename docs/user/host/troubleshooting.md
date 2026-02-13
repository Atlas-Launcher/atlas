---
title: Server host troubleshooting
summary: Recovery guidance for Atlas Runner authentication, daemon, and startup issues.
persona: host
order: 3
keywords: ["troubleshooting", "daemon", "systemd", "auth login"]
intent: troubleshooting
---

# Server host troubleshooting

Use this guide when runner commands fail or server state is inconsistent.

## Authentication failures

Resolve auth and access issues before server lifecycle retries.

- Confirm Hub URL reachability.
- Confirm your account has pack access.
- Retry `atlas-runner auth login` with explicit `--hub-url` when needed.

## Startup blocked by prompts

Non-interactive runs require explicit first-run flags.

- Add `--accept-eula`.
- Add `--max-ram <MB>`.

## Empty or stale logs

If server logs are stale, verify daemon connectivity first.

```bash
atlas-runner daemon status
atlas-runner daemon logs --follow
```

## Console or command connection failures

Server console commands depend on a healthy local `atlas-runnerd` process.

- Confirm daemon is running.
- Confirm your current user can access the daemon socket.

## Linux service install issues

Reinstall service registration when systemd state is invalid.

```bash
atlas-runner host install
```

If permissions fail, rerun with `sudo` and verify
`/usr/local/bin/atlas-runner` exists.

## Minimal recovery checklist

Use this order to restore service quickly.

1. Verify CLI version: `atlas-runner --version`.
2. Verify daemon health: `atlas-runner daemon status`.
3. Re-link pack auth: `atlas-runner auth login`.
4. Retry start with explicit first-run flags when needed.
