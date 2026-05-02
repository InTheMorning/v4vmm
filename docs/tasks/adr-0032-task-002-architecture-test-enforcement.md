# ADR 0032 Task 002: Architecture Test Enforcement

## Status

Completed - 2026-05-02.

## Goal

Make ADR 0032 enforceable by adding architecture tests that prevent Library
release-detail playlist popovers from regressing back to screen-local panels.

## Files To Inspect

- `docs/adr/0032-ui-backend-boundary-and-popover-contracts.md`
- `docs/plans/adr-0032-ui-backend-boundary-phase-plan.md`
- `docs/architecture/ui-backend-boundary.md`
- `src/library.rs`
- `src/search.rs`
- `src/ui_track.rs`
- `tests/architecture_tests.rs`

## Files Changed

- `tests/architecture_tests.rs`
- `docs/adr/0032-ui-backend-boundary-and-popover-contracts.md`
- `docs/plans/adr-0032-ui-backend-boundary-phase-plan.md`
- `docs/reviews/adr-0032-review-checklist.md`
- `docs/tasks/adr-0032-task-002-architecture-test-enforcement.md`
- `docs/reviews/adr-0032-task-002-review.md`

## Do Not Touch

- `src/db.rs`
- `migrations/`
- playlist service behavior
- command dispatch semantics
- legacy inspector panel migrations outside this task

## Constraints

- Do not widen the task into a full migration of every legacy playlist panel.
- Baseline known legacy inspector panels instead of hiding their existence.
- Add a zero-tolerance check for the Library release-detail album/feed row
  patterns removed by Task 001.
- Keep `AddToPlaylistPopover` as the canonical playlist popover chrome.

## Implementation Summary

- Added an architecture-test ratchet for existing screen-local playlist popover
  panels so new copies fail `cargo test --test architecture_tests`.
- Added a Library release-detail guard that rejects the stale album/feed
  screen-local popover state and helper names that caused the full-width
  popover regression.
- Required at least two Library `AddToPlaylistPopover::new` uses so feed-level
  and track-row playlist affordances remain on the shared composite.
- Updated ADR0032 planning and review docs to mark the enforcement task
  complete and make boundary checks explicit.

## Acceptance Criteria

- [x] Architecture tests fail if Library release-detail playlist popover helper
      names or state names are reintroduced.
- [x] Architecture tests fail if screen-local playlist popover panel patterns
      grow beyond the documented legacy baseline.
- [x] Architecture tests require Library release-detail playlist actions to keep
      using `AddToPlaylistPopover`.
- [x] Task/review docs call out UI/backend boundary checks and visual smoke
      expectations.
- [x] No playlist service, database, or command semantics change.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test --test architecture_tests
cargo clippy --lib --tests -- -D warnings
git diff --check
```

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0032-ui-backend-boundary-and-popover-contracts.md`
- `docs/plans/adr-0032-ui-backend-boundary-phase-plan.md`
- `docs/architecture/ui-backend-boundary.md`
- `src/library.rs`
- `src/search.rs`
- `src/ui_track.rs`
- `tests/architecture_tests.rs`

Goal:
- Add architecture-test enforcement for ADR0032 playlist popover ownership.

Constraints:
- Keep command dispatch in screen modules.
- Do not migrate legacy inspector panels in this task.
- Baseline existing legacy screen-local panels and block growth.
- Hard-ban the Library release-detail helper/state names removed by Task 001.

Do not touch:
- `src/db.rs`
- `migrations/`
- playlist service semantics
- unrelated UI chrome

Acceptance criteria:
- `cargo test --test architecture_tests` fails on reintroduced Library
  release-detail raw playlist panels.
- New screen-local playlist panel helper growth fails the architecture tests.
- Library feed and track release-detail actions still use
  `AddToPlaylistPopover`.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test --test architecture_tests`
- `cargo clippy --lib --tests -- -D warnings`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns
