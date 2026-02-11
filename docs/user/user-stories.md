# Atlas User Stories (Current)

These stories reflect how the platform currently works.

## Creator Stories

1. As a creator, I initialize and maintain a pack in Git, then build and publish an immutable artifact.
2. As a creator, I promote stable builds by moving channel pointers, not rebuilding old versions.
3. As a creator, I can let CI upload and complete builds using OIDC or authorized bearer credentials.
4. As a creator, I can grant pack access and membership so collaborators can operate safely.

## Server Host Stories

1. As a server host, I run `atlas-runner` to control lifecycle while `atlas-runnerd` manages process state.
2. As a server host, I can start/stop/log/backup without manually scripting Java launch commands.
3. As a server host, I can trust runtime apply to verify dependencies and Java runtime integrity.

## Player Stories

1. As a player, I use launcher sign-in + account link flow to become launch-ready.
2. As a player, I only launch when readiness checks pass for launch blockers.
3. As a player, if runtime issues occur after install, I use troubleshooter and guided fixes.
4. As a player, I can use launcher support bundle output for support cases.
