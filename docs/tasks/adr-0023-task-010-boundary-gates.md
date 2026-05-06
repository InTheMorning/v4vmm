# ADR 0023 Task 010: Boundary Gates

## Status

Completed 2026-04-30.

## Task Goal

Add automated tests that enforce the ADR 0023 architecture boundary and token
audit claims.

## Files To Inspect

- `src/view_models/mod.rs`
- `src/ui/tokens.rs`
- `src/ui/theme.rs`
- `src/app.rs`
- `src/library.rs`
- `src/search.rs`
- Existing tests under `tests/`
- `Cargo.toml`

## Files Likely To Change

- New or existing architecture test file under `tests/`
- Possibly `src/view_models/mod.rs` documentation
- ADR 0023 docs/task status

## Do Not Touch

- Runtime UI behavior.
- Service modules.
- Database schema.
- Existing tests unrelated to architecture boundaries.

## Constraints

- Tests may scan source files, but allow intentional literals in
  `src/ui/tokens.rs`, `src/ui/theme.rs`, primitives, and composites.
- Tests must be precise enough not to block legitimate GPUI presentation
  code.
- Tests must fail if `src/view_models/*` imports `gpui`, `gpui_component`,
  `crate::ui`, or screen modules.
- Tests must fail if screen modules reintroduce raw `rgb(...)`,
  `px(<number>)`, or hardcoded dark render defaults outside approved
  compatibility paths.

## Implementation Steps

1. Add an architecture test file that reads relevant source files.
2. Check `src/view_models/*.rs` for forbidden imports/usages.
3. Check screen modules for raw color/layout literals and hardcoded dark
   defaults.
4. Keep allowlists explicit and documented in the test.
5. Run the architecture test, full clippy, and full tests.

## Acceptance Criteria

- [x] Boundary tests fail on GPUI imports in `view_models`.
- [x] Boundary tests fail on raw `rgb(...)` or numeric `px(...)` literals in
  `app.rs`, `library.rs`, and `search.rs`.
- [x] Boundary tests fail on hardcoded `Appearance::Dark` in screen render paths
  unless explicitly allowlisted with a documented reason.
- [x] Existing full test suite remains green.

## Result

- Added `tests/architecture_tests.rs` with three ADR 0023 boundary gates.
- The view-model gate scans every Rust file under `src/view_models/` and
  rejects GPUI, GPUI Component, UI-layer, and screen-module dependencies.
- The screen literal gate rejects raw `rgb(...)` and numeric `px(...)`
  literals in `src/app.rs`, `src/library.rs`, and `src/search.rs`.
- The appearance gate rejects new hardcoded `Appearance::Dark` screen defaults
  except for the explicitly documented bootstrap/settings compatibility paths
  that exist today.

## Test Commands

- `cargo fmt -- --check`
- `cargo check`
- `cargo test --test architecture_tests`
- `cargo clippy --lib --tests -- -D warnings`
- `cargo test`

## Expected Final Summary Format

1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- A legitimate screen exception needs product/architecture approval.
- The test requires a brittle parser instead of simple source scanning.
- Existing code violates a claimed ADR boundary.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0023-design-system-and-view-models.md`
- `docs/plans/adr-0023-finalization-plan.md`
- `src/view_models/mod.rs`
- `src/ui/tokens.rs`
- `src/ui/theme.rs`
- `src/app.rs`
- `src/library.rs`
- `src/search.rs`

Goal:
- Add automated tests for ADR 0023 import and token-literal boundaries.

Constraints:
- Allow literals in token/theme/component layers.
- Reject forbidden imports in `view_models`.
- Reject raw screen literals and hardcoded dark defaults in screens.
- Keep tests simple and maintainable.

Do not touch:
- Runtime behavior.
- Service modules.
- Schema/migrations.

Acceptance criteria:
- Architecture tests exist and pass.
- Tests would catch the boundary regressions ADR 0023 cares about.
- Full verification remains green.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test --test architecture_tests`
- `cargo clippy --lib --tests -- -D warnings`
- `cargo test`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns
