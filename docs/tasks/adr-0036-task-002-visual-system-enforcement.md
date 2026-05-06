# ADR 0036 Task 002: Visual System Enforcement

## Goal

Normalize spacing, typography, icon, row, button, and popover behavior through
tokens/primitives so visible UI polish happens in one place.

## Scope

This task must run after Task 001 is green. It should audit shared primitives,
composites, and screens for repeated raw visual decisions, then move only the
highest-impact repeated decisions into named tokens or shared primitives.

## Acceptance Criteria

- Repeated popover padding and row/action sizing live in shared primitives or
  tokens.
- Screens do not patch primitive spacing locally.
- Architecture tests are tightened for any newly consolidated visual rule.
- User screenshots verify normal Library and Discover feed/track views.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0036-feed-visual-and-provenance-surface-consistency.md`
- `docs/plans/adr-0036-feed-visual-and-provenance-consistency-phase-plan.md`
- `docs/tasks/adr-0036-task-002-visual-system-enforcement.md`
- `src/ui/tokens.rs`
- `src/ui/primitives/`
- `src/ui/composites/`
- `tests/architecture_tests.rs`

Goal:
- Move repeated visible sizing/color/icon decisions into tokens/primitives and
  guard them.

Constraints:
- No screen-local compensation.
- No new feature behavior.
- Screenshot proof must be user-provided.

Do not touch:
- Backend, schema, metadata inference, playlist semantics, playback semantics.

Acceptance criteria:
- Shared primitives own the consolidated visual decisions.
- Tests prevent screen-local regression.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test`
- `cargo clippy -- -D warnings`
- `git diff --check`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns
