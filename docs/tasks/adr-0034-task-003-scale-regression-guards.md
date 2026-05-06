# ADR 0034 Task 003: Scale Regression Guards

## Goal

Add architecture tests that prevent shared UI render paths from introducing
unscaled token `.px()` usage for user-facing dimensions.

## Files to Inspect

- `docs/adr/0033-hig-ui-architecture-governance.md`
- `docs/adr/0034-scale-aware-ui-tokens-and-controls.md`
- `docs/plans/adr-0034-scale-aware-ui-tokens-phase-plan.md`
- `tests/architecture_tests.rs`
- `src/ui/tokens.rs`
- `src/ui/primitives/`
- `src/ui/composites/`

## Files Likely to Change

- `tests/architecture_tests.rs`
- `docs/adr/0033-hig-ui-architecture-governance.md`
- `docs/adr/0034-scale-aware-ui-tokens-and-controls.md`
- `docs/reviews/adr-0034-review-checklist.md`

## Do Not Touch

- Runtime UI implementation files unless a test cannot be written without a
  tiny marker comment.
- Backend, database, services, schema, playlist behavior, playback behavior

## Constraints

- The test must scan shared UI render paths, not token definitions.
- The allowlist must be narrow and documented.
- Do not ban `.px()` globally; token definitions, tests, hairlines, fixed
  artwork/media, and low-level geometry can remain fixed when justified.
- The test name must clearly describe the ADR rule.

## Implementation Steps

1. Add a named architecture test such as
   `shared_ui_render_paths_use_scale_aware_tokens`.
2. Scan `src/ui/primitives` and `src/ui/composites` for unscaled calls like
   `Spacing::X.px()`, `Radius::X.px()`, `FontSize::X.px()`, `Size::X.px()`,
   and icon-size `.px()` inside render-relevant code.
3. Add an allowlist struct with file, pattern, count, and reason fields.
4. Keep token definitions and unit tests out of scope.
5. Update ADR 0033 enforcing-test list and ADR 0034 consequences.
6. Update the review checklist with pass/fail status.

## Acceptance Criteria

- Architecture tests fail if a new shared UI render path uses unscaled token
  `.px()` for user-facing dimensions.
- Any remaining fixed shared UI dimensions are allowlisted with reasons.
- ADR 0033 and ADR 0034 list the new guard.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test --test architecture_tests
git diff --check
```

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0033-hig-ui-architecture-governance.md`
- `docs/adr/0034-scale-aware-ui-tokens-and-controls.md`
- `docs/plans/adr-0034-scale-aware-ui-tokens-phase-plan.md`
- `docs/tasks/adr-0034-task-003-scale-regression-guards.md`
- `tests/architecture_tests.rs`

Goal:
- Add architecture tests that prevent unscaled token usage from returning in
  shared UI render paths.

Constraints:
- Use a narrow allowlist.
- Do not ban token definitions or tests.
- Do not edit runtime UI unless the test exposes an actual remaining
  violation that belongs to this task.

Do not touch:
- Backend/database/service/schema files.
- Playlist or playback behavior.

Acceptance criteria:
- The new architecture guard passes and would fail on new unscaled shared UI
  render usage.
- ADR enforcing-test documentation is updated.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test --test architecture_tests`
- `git diff --check`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- If the scan finds many legitimate fixed dimensions, stop and split the
  allowlist into reviewed categories rather than adding a broad escape hatch.
- If a fixed dimension affects readability or hit targets, move it back to
  Task 001 or Task 002 instead of allowlisting it.
