---
title: Server host quick reference
summary: Daily Atlas Runner command set for host operations.
persona: host
order: 2
keywords: ["atlas-runner", "daemon", "backup", "console"]
intent: reference
---

# Server host quick reference

Use this page as a command-first reference during normal operations.

## Command index

This list includes the most-used host commands.

```bash
atlas-runner auth login
atlas-runner server start
atlas-runner server stop
atlas-runner server logs
atlas-runner server command
atlas-runner server console
atlas-runner server backup
atlas-runner daemon status
atlas-runner daemon stop
atlas-runner daemon logs
atlas-runner host install
atlas-runner host path
```

## First-time runbook

Use this runbook to bring up a new host quickly.

```bash
atlas-runner auth login
atlas-runner server start
```

## Daily runbook

Use this runbook for normal monitoring and interaction.

```bash
atlas-runner server logs --follow
atlas-runner server console
atlas-runner server backup
```

## Restart runbook

Use this runbook for controlled service restart.

```bash
atlas-runner server stop
atlas-runner server start
```

## Non-interactive usage note

Automation contexts should pass required flags explicitly.

- Include `--accept-eula` and `--max-ram` when prompts are unavailable.
