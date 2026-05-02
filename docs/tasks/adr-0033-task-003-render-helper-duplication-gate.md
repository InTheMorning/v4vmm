# ADR 0033 Task 003: Render Helper Duplication Gate

## Goal

Add an architecture test that prevents new duplicated screen-local
`render_*` helpers from appearing across `SCREEN_FILES`, while explicitly
baselining the remaining known Library/Search duplicates scheduled for later
post-ADR0033 consolidation tasks.

## Files to Inspect

- `docs/adr/0033-hig-ui-architecture-governance.md`
- `docs/plans/post-adr-0033-ui-consolidation-plan.md`
- `tests/architecture_tests.rs`
- `src/library.rs`
- `src/search.rs`

## Files Likely to Change

- `tests/architecture_tests.rs`
- `docs/adr/0033-hig-ui-architecture-governance.md`
- `docs/tasks/adr-0033-task-003-render-helper-duplication-gate.md`
- `docs/reviews/adr-0033-task-003-review.md`

## Do Not Touch

- Backend, database, API, service, and command modules.
- UI rendering implementation in `src/library.rs` or `src/search.rs`.
- The remaining helper consolidations; those are separate packets.

## Constraints

- The test must inspect `SCREEN_FILES` only.
- The test must detect duplicate `render_*` function definitions even when a
  helper has visibility such as `pub(crate)`.
- The baseline must be explicit by helper name and file set.
- New duplicate helper names must fail unless deliberately added to the
  baseline with a note.
- Existing architecture tests must remain green.

## Implementation Steps

1. Add a `RenderHelperDuplicationBaseline` type and
   `RENDER_HELPER_DUPLICATION_BASELINES`.
2. Add a helper parser for screen-local `render_*` function definition names.
3. Add `screens_do_not_duplicate_render_helpers_without_baseline`.
4. Add the new test name to ADR 0033's enforcing-test list.
5. Run formatting, architecture tests, build, full tests, and clippy.

## Acceptance Criteria

- `cargo test --test architecture_tests` includes the new test and passes.
- The baseline lists only the remaining known Library/Search helper pairs from
  the post-ADR0033 plan.
- The new test fails for any duplicate screen helper that is not explicitly
  baselined.
- ADR 0033 names the new enforcing test.

## Test Commands

- `cargo fmt -- --check`
- `cargo check`
- `cargo test --test architecture_tests`
- `cargo test`
- `cargo clippy --lib --tests -- -D warnings`

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0033-hig-ui-architecture-governance.md`
- `docs/plans/post-adr-0033-ui-consolidation-plan.md`
- `tests/architecture_tests.rs`
- `src/library.rs`
- `src/search.rs`

Goal:
- Add a test that blocks new duplicated screen-local `render_*` helpers across
  `SCREEN_FILES`.

Constraints:
- Inspect only `SCREEN_FILES`.
- Detect helpers with optional visibility such as `pub(crate) fn render_x`.
- Keep remaining Library/Search duplicates in an explicit baseline with notes.
- Do not modify screen rendering behavior.

Do not touch:
- Backend/service/database/API modules.
- Existing render helper implementations.
- Later consolidation packets.

Acceptance criteria:
- The architecture test suite passes and includes the new duplication gate.
- ADR 0033 lists the new enforcement test.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test --test architecture_tests`
- `cargo test`
- `cargo clippy --lib --tests -- -D warnings`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns
