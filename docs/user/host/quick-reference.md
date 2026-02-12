# Server Host Quick Reference

## Command layout (`atlas-runner`)

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

## Most common flows

### First-time setup

```bash
atlas-runner auth login
atlas-runner server start
```

### Daily operations

```bash
atlas-runner server logs --follow
atlas-runner server console
atlas-runner server backup
```

### Restart flow

```bash
atlas-runner server stop
atlas-runner server start
```

### Daemon checks

```bash
atlas-runner daemon status
atlas-runner daemon logs --follow
```

## Non-interactive usage tips

If running in automation/non-interactive mode:
- pass required flags up front (for example `--accept-eula`, `--max-ram`),
- avoid prompt-dependent flows.
