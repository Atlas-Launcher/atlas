# Player Troubleshooting

## Launch Assist Modes

- Readiness mode: resolves launch blockers (Atlas sign-in, Microsoft sign-in, account linking).
- Recovery mode: analyzes failures after readiness is satisfied and suggests fixes.

Launch Assist is the single recovery entrypoint in launcher UI.

## Common Signals

- Out of memory / heap errors: increase memory settings where applicable.
- Missing metadata/runtime files: run the in-app repair step from Launch Assist.
- Java not ready: launcher runtime repair/reinstall path should recover.
- Account mismatch: relink Atlas + Microsoft identities.

## Support Bundle

Launcher support bundle includes:
- readiness state,
- recent logs/status,
- structured findings,
- redacted sensitive values.

Use this bundle when reporting issues.

## Fix and Retry

Launch Assist can apply a top fix action, rerun diagnostics, and retry launch from the same flow.
