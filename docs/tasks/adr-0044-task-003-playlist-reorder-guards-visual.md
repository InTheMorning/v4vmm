# ADR 0044 Task 003: Playlist Reorder Guards and Visual Readiness

Status: Blocked - display access unavailable on 2026-05-11.

## Goal

Add final guards, run verification, and record visual proof for playlist
drag-handle reordering.

## Files to Inspect

- `docs/adr/0044-playlist-drag-handle-reordering.md`
- `docs/plans/adr-0044-playlist-drag-handle-reordering-phase-plan.md`
- `docs/tasks/adr-0044-task-001-playlist-reorder-vm-contract.md`
- `docs/tasks/adr-0044-task-002-playlist-drag-shell.md`
- `docs/reviews/adr-0044-review-checklist.md`
- `tests/architecture_tests.rs`

## Files Likely to Change

- `tests/architecture_tests.rs`
- `docs/reviews/adr-0044-review-checklist.md`
- Possibly minor fixes in playlist files touched by Tasks 001-002

## Do Not Touch

- Do not introduce new playlist behavior beyond guards/fixes needed for
  readiness.
- Do not alter database schema.
- Do not refactor unrelated Library or Search surfaces.

## Constraints

- Guards must map directly to ADR 0044 invariants.
- Visual proof must include light and dark themes.
- Visual proof must include a normal playlist row, an unavailable row,
  row Actions menu, and insertion-line feedback.
- Required checks must be green before recording `Proceed`.

## Implementation Steps

1. Done: add architecture guards that playlist rows do not render up/down
   arrow labels or arrow-specific ids.
2. Done: add guards that drag handle and menu fallback display come from
   `PlaylistTrackRowVm`.
3. Done: add guards that playlist shell uses the semantic icon catalog for the
   handle rather than raw glyph strings.
4. Done: run the required checks.
5. Blocked: capture or review light/dark visual evidence.
6. Done: update the ADR 0044 review checklist with pass/fail, evidence, and
   merge recommendation.

## Acceptance Criteria

- [x] Architecture guards enforce the new handle/menu ownership contract.
- [x] Required checks are green.
- [ ] Visual proof confirms handle, menu fallback, unavailable row, and
  insertion line are legible in light and dark themes.
- [x] Review checklist records `Proceed` only if all gates pass.

## Verification

- Green: `cargo fmt -- --check`
- Green: `cargo check`
- Green: `cargo test`
- Green: `cargo clippy -- -D warnings`

## Visual Evidence Attempt

Visual proof is blocked because the local display cannot be opened:

```text
DISPLAY=:0 wmctrl -l
Authorization required, but no authorization protocol specified
Cannot open display.
```

The ADR 0044 review remains blocked until light and dark screenshots can
verify the handle, Actions menu, unavailable row, and insertion-line
feedback.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test
cargo clippy -- -D warnings
```

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0044-playlist-drag-handle-reordering.md`
- `docs/plans/adr-0044-playlist-drag-handle-reordering-phase-plan.md`
- `docs/reviews/adr-0044-review-checklist.md`
- `tests/architecture_tests.rs`

Goal:
- Add guards and visual-readiness evidence for playlist drag-handle
  reordering.

Constraints:
- Add no new feature behavior.
- Guards must map to explicit ADR 0044 invariants.
- Visual proof must cover light and dark themes.

Do not touch:
- Database schema
- Search/Discover UI
- Unrelated Library surfaces

Acceptance criteria:
- Guards enforce handle/menu ownership and no arrow controls.
- Required checks are green.
- Review checklist has a clear readiness decision.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test`
- `cargo clippy -- -D warnings`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- Visual proof shows ambiguous drop location or handle/body gesture
  conflict.
- Guards require broad fragile string baselines instead of direct
  ownership checks.
