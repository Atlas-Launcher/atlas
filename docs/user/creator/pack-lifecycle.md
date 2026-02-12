# Pack Lifecycle (Creator)

## Author

Maintain pack config and assets in your repository.

## Build

Compile source to artifact with `atlas build` or CI pipeline.

## Publish

Use `atlas publish` (or CI) to upload and register a build.

## Promote

Move channel pointers (`dev` -> `beta` -> `production`) with `atlas promote`.

## Consume

- Launcher resolves builds by channel for players.
- Runner resolves builds by channel for servers.

## Rollback strategy

Because builds are immutable, rollback is pointer-based:
- Promote or repoint the channel to a known-good build.
