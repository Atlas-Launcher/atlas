# Pack Lifecycle (Creator)

## Author

Maintain pack config and assets in repo source.

## Build

Compile source to artifact with `atlas` CLI or CI pipeline.

## Publish

Use presign + complete build API flow via CI/CLI.

## Promote

Move channel pointers (`dev` -> `beta` -> `production`) as validation completes.

## Consume

- Launcher users resolve build by channel.
- Runner daemon resolves server artifact and applies it.

## Rollback Strategy

Because builds are immutable, rollback is pointer-based:
- repoint channel to known-good previous build.
