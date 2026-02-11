# Atlas Launcher Parallel Execution TODO

## Goal
Deliver Phases 2-6 in parallel using the Phase 1 diagnostics foundation, with minimal merge conflicts and clear ownership.

## Shared Rules
- [ ] Keep backend command contracts stable once published (version fields if shape must change).
- [ ] Reuse existing Phase 1 commands for readiness, fixes, repair, and support bundles.
- [ ] Do not duplicate diagnosis logic in frontend; render backend output.
- [ ] Maintain one source of truth for launch readiness.

## Track A: Launch Readiness Wizard (Phase 2)
Owner: Frontend

- [ ] Add `LaunchReadinessWizard.vue` with single checklist (Atlas login, Microsoft login, link, files, Java).
- [ ] Wire startup visibility in `apps/launcher/src/App.vue` after session/settings load.
- [ ] Add explicit next-action CTA per failed checklist item.
- [ ] Add menu + settings entry points to reopen wizard.
- [ ] Persist wizard dismissed/completed state in launcher settings.
- [ ] Acceptance: Wizard reflects backend readiness exactly and blocks confusion paths.

## Track B: In-App Updater UX (Phase 3)
Owner: Frontend + Tauri integration

- [x] Add `useUpdater.ts` wrapping updater plugin check/download/install.
- [x] Add visible update-available banner/dialog with release info.
- [x] Add install progress and completion state.
- [x] Add restart prompt and restart action after install.
- [x] Add manual check action in settings.
- [x] Acceptance: update flow is discoverable, non-blocking, and restart is one-click.

## Track C: Repair + Support Bundle backend hardening (Phases 4/5 backend)
Owner: Backend

- [x] Finalize `repair_installation` behavior for local vs atlas instances.
- [x] Expand cache/transient cleanup rules safely (preserve saves always by default).
- [x] Improve redaction coverage in `create_support_bundle`.
- [x] Include structured root-cause + attempted-fixes summary in bundle output.
- [x] Add unit tests for redaction and repair decision paths.
- [ ] Acceptance: one-click repair and support bundle are deterministic and privacy-safe.

## Track D: Troubleshooter Dialog UI (Phase 6)
Owner: Frontend

- [ ] Add `TroubleshooterDialog.vue` and route triggers from settings/help/failure states.
- [ ] Read `run_troubleshooter` output and render top finding + confidence.
- [ ] Wire one-click actions to `apply_fix`.
- [ ] Re-run readiness after fixes and display before/after state.
- [ ] Acceptance: common failures become self-serve recovery paths.

## Track E: Cross-track QA + release readiness (Phase 7)
Owner: QA/infra

- [ ] Test matrix: fresh install, expired auth, UUID mismatch, missing files, corrupted runtime, low memory.
- [ ] Verify updater behavior on macOS/Windows/Linux.
- [ ] Verify repair preserves saves and recovers launchability.
- [ ] Verify support bundle contains no secrets/tokens/proofs.
- [ ] Smoke test with `cargo test -p atlas` and launcher UI flow checks.
- [ ] Acceptance: no blocker regressions, known risks documented.

## Merge Order (to avoid blocking)
1. [ ] Track C backend hardening lands first.
2. [ ] Track A wizard + Track D dialog merge next (parallel).
3. [ ] Track B updater UX merges after UI shell is stable.
4. [ ] Track E QA sign-off before release branch cut.

## Coordination Cadence
- [ ] Daily contract sync (15 min): command payload changes, UI assumptions, error mappings.
- [ ] Mid-sprint integration checkpoint on one branch for end-to-end smoke.
- [ ] Pre-release freeze: no payload-shape changes without explicit migration notes.

---

## Deep Dive TODO: Java Runtime and Install Path Hardening
Owner: Backend + Runtime

### Priority P0 (high impact, low effort)

#### 1) Fix `runnerd-v2` RAM injection path and units
Why:
- Current RAM insertion logic only matches literal `"java"` and can skip when launch plans contain absolute java paths.
- CLI RAM prompt uses GB-like values while daemon applies `m` suffix (`-Xmx{}m`), creating unit mismatch risk.

References:
- `apps/runnerd-v2/src/supervisor/server.rs` (`apply_pack_blob`, RAM insertion)
- `apps/runner-v2/src/client/commands/core.rs` (`get_default_max_ram`, prompt path)

Tasks:
- [ ] Match java executable token robustly (basename or path ending with `/java`/`java.exe`) instead of strict equality.
- [ ] Standardize RAM units end-to-end (store explicit MB in config, convert only at presentation).
- [ ] Add tests for:
- RAM args inserted when java token is absolute path.
- RAM args inserted only once (no duplicates on restart paths).
- MB/GB conversion consistency in CLI + daemon.
- [ ] Emit clear status/log line showing resolved RAM flags for observability.

Definition of done:
- RAM limits reliably apply for all launch plan variants.
- No duplicate `-Xmx`/`-Xms` flags after repeated start/restart cycles.

#### 2) Enforce Java override validation in launcher
Why:
- `javaPath` override currently bypasses runtime checks and may fail late at launch.

References:
- `apps/launcher/src-tauri/src/launcher/java.rs` (`resolve_java_path`)
- `apps/launcher/src-tauri/src/launcher/mod.rs` (launch path consumption)

Tasks:
- [ ] Validate override path exists and is executable before accepting it.
- [ ] If override fails validation, return actionable error with remediation.
- [ ] Normalize accepted override values (trim, canonicalize when possible).
- [ ] Add unit tests for valid/invalid override paths and fallback behavior.

Definition of done:
- Invalid override is rejected early with clear error.
- Valid override works across launch + loader install flows.

#### 3) Enforce Java major-version compatibility in launcher
Why:
- Manifest parses required Java major version but it is not currently used to gate runtime selection.

References:
- `apps/launcher/src-tauri/src/launcher/manifest.rs` (`javaVersion.majorVersion`)
- `apps/launcher/src-tauri/src/launcher/java.rs`

Tasks:
- [ ] Add runtime/java-binary version probe (`java -version`) to parse major version.
- [ ] Validate selected runtime/override major version against manifest requirement.
- [ ] Fail with actionable message when version mismatch occurs.
- [ ] Add tests for manifest requiring 8/17/21 and mixed runtime cases.

Definition of done:
- Launcher never starts with Java major lower than required by version metadata.

### Priority P1 (resilience + maintainability)

#### 4) Add robust retry/backoff for runtime and installer downloads
Why:
- Network/transient errors currently fail fast in several paths.

References:
- `apps/launcher/src-tauri/src/launcher/download.rs`
- `apps/launcher/src-tauri/src/launcher/java.rs`
- `apps/launcher/src-tauri/src/launcher/loaders/fabric.rs`
- `apps/launcher/src-tauri/src/launcher/loaders/neoforge.rs`

Tasks:
- [ ] Add bounded retries with exponential backoff + jitter for HTTP download paths.
- [ ] Classify retryable vs non-retryable errors (5xx/timeouts/connection reset vs 4xx).
- [ ] Preserve resume behavior where supported.
- [ ] Surface retry progress in launch status events.

Definition of done:
- Transient failures recover automatically within bounded retry budget.
- Non-retryable errors fail fast with precise message.

#### 5) Align diagnostics Java readiness with launcher source-of-truth
Why:
- Diagnostics currently has a separate runtime search path and `"java"` shortcut readiness logic.

References:
- `apps/launcher/src-tauri/src/diagnostics/mod.rs` (`resolve_java_ready`, runtime search)
- `apps/launcher/src-tauri/src/launcher/java.rs` (runtime root/binary resolution)

Tasks:
- [ ] Extract shared Java resolver helpers into a common module.
- [ ] Replace diagnostics ad-hoc checks with shared resolver result.
- [ ] Ensure readiness only reports true when launch path would succeed.
- [ ] Add readiness tests for configured override, managed runtime, and missing runtime cases.

Definition of done:
- Diagnostics readiness and actual launch outcome are consistent for Java-related checks.

#### 6) Consolidate duplicated Java install logic (legacy runner vs v2)
Why:
- Two separate Java installers increase drift and bug surface.

References:
- `apps/runner/src/java.rs` (legacy)
- `crates/runner-provision-v2/src/java.rs` (v2)

Tasks:
- [ ] Decide canonical install implementation (`runner-provision-v2` preferred).
- [ ] Remove or wrap legacy installer behind shared crate API.
- [ ] Ensure checksum verification policy is consistent in all runner paths.
- [ ] Add migration notes if runtime install location changes.

Definition of done:
- Single shared Java install/version mapping implementation for runner paths.

### Priority P2 (quality + cleanup)

#### 7) Expand test coverage for Java-heavy paths
Why:
- Launcher has some unit tests; runner/provision Java paths are under-tested.

References:
- `apps/launcher/src-tauri/src/launcher/tests.rs`
- `crates/runner-provision-v2` (currently no meaningful tests)

Tasks:
- [ ] Add unit tests for version mapping boundaries (1.18, 1.20.4, 1.20.5+).
- [ ] Add integration-style tests for install-idempotency and corrupted install recovery.
- [ ] Add tests for loader installer command construction and failure reporting.
- [ ] Add smoke CI target for Java provisioning path.

Definition of done:
- Core Java detection/install/version logic has regression tests in CI.

#### 8) Remove dead code and warning debt in provisioning/runtime crates
Why:
- Current build shows repeated unused imports/fields/functions.

References:
- `crates/runner-provision-v2/src/apply/*`
- `apps/runner-v2/src/*`
- `apps/runnerd-v2/src/*`

Tasks:
- [ ] Remove unused imports and dead helpers.
- [ ] Either implement or delete unused structures/fields.
- [ ] Keep `cargo check` warnings near-zero for touched crates.

Definition of done:
- No avoidable warning noise in Java/runtime ownership areas.

### Docs and parity validation

#### 9) Reconcile docs with shipped CLI/runtime behavior
Why:
- User/developer docs describe commands/features not present in current binaries.

References:
- `docs/user/atlas-runner-user-guide.md`
- `docs/dev/runner-provision-v2-developer-guide.md`
- `apps/runner-v2/src/main.rs`
- `crates/runner-provision-v2/src/lib.rs`

Tasks:
- [ ] Update runner user guide to match actual v2 command surface.
- [ ] Remove or mark aspirational APIs in provision-v2 docs.
- [ ] Add a "current vs planned" section where behavior is intentionally staged.
- [ ] Add doc verification checklist in release process.

Definition of done:
- Documentation reflects current implementation accurately with no misleading commands/APIs.

### Suggested execution order
1. [ ] P0.1 RAM fix + tests
2. [ ] P0.2 override validation
3. [ ] P0.3 Java major enforcement
4. [ ] P1.4 download retries/backoff
5. [ ] P1.5 diagnostics alignment
6. [ ] P1.6 installer consolidation
7. [ ] P2.7 test expansion
8. [ ] P2.8 warning cleanup
9. [ ] Docs parity update
