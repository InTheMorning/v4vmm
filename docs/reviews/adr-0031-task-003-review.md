# ADR 0031 Task 003 Review

## Reviewed Artifact

- ADR: `docs/adr/0031-release-detail-presentation-contract.md`
- Plan: `docs/plans/adr-0031-release-detail-presentation-contract-phase-plan.md`
- Task: `docs/tasks/adr-0031-task-003-track-section-parity.md`
- Diff scope:
  - `src/ui_entity.rs`
  - `src/ui_track.rs`
  - `src/library.rs`
  - `src/ui/composites/track_row.rs`
  - `src/view_models/library.rs`

## Pass / Fail

Pass.

## Required Fixes

None.

## Optional Improvements

- Task 004 should visually verify that Library row popovers still align well
  below the row after moving the wrapper into `render_release_track_row`.
- Task 004 should include both Library and Discovery rows with and without
  thumbnails so the shared fallback thumbnail behavior is visible.

## Architectural Drift

None found. Row geometry moved into shared UI code, while screen modules still
own click handlers and surface-specific actions. No service, schema, playback,
download, playlist, or metadata behavior changed.

## Missing Tests

No missing automated tests for Task 003 scope. Manual visual smoke remains
required for row geometry and popover placement.

## Merge Recommendation

Task 003 can be merged.

## Next Task Adjustment

Task 004 should perform the visual smoke fixture pass and remove any remaining
dead screen-local composition helpers only after screenshots confirm the
contract path.
