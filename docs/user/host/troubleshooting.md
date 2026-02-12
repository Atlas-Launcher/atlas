---
title: Server Host Troubleshooting
summary: Diagnostics and recovery checklist for authentication, daemon, and startup issues.
persona: host
order: 3
keywords: ["troubleshooting", "daemon", "systemd", "auth login"]
intent: troubleshooting
---

# Server Host Troubleshooting

## `atlas-runner auth login` fails

Check:
- You can reach Atlas Hub URL.
- Your Atlas account has access to the target pack.

Try:
- rerun login and choose the pack interactively,
- pass `--hub-url` explicitly if needed.

## `atlas-runner server start` stops for EULA/RAM prompts

In non-interactive mode, first-run prompts are not available.

Use explicit flags:
- `--accept-eula`
- `--max-ram <MB>`

## Logs are empty or stale

Check daemon status:

```bash
atlas-runner daemon status
```

Then follow daemon logs:

```bash
atlas-runner daemon logs --follow
```

## Server command/console cannot connect

Runner talks to local `atlas-runnerd`. Verify daemon is running and socket access is available for your user.

## systemd install issues (Linux)

Run:

```bash
atlas-runner host install
```

If permissions fail, rerun with `sudo` and confirm `atlas-runner` is installed in `/usr/local/bin`.

## Recovery checklist

1. Confirm CLI version: `atlas-runner --version`.
2. Confirm daemon health: `atlas-runner daemon status`.
3. Confirm pack linkage: rerun `atlas-runner auth login`.
4. Start server with explicit flags if non-interactive.
