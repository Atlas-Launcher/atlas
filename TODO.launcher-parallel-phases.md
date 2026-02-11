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

- [ ] Add `useUpdater.ts` wrapping updater plugin check/download/install.
- [ ] Add visible update-available banner/dialog with release info.
- [ ] Add install progress and completion state.
- [ ] Add restart prompt and restart action after install.
- [ ] Add manual check action in settings.
- [ ] Acceptance: update flow is discoverable, non-blocking, and restart is one-click.

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
