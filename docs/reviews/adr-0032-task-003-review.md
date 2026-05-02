# ADR 0032 Task 003 Review: Inspector Popover Migration

## Reviewed Artifact

- ADR: `docs/adr/0032-ui-backend-boundary-and-popover-contracts.md`
- Plan: `docs/plans/adr-0032-ui-backend-boundary-phase-plan.md`
- Task: `docs/tasks/adr-0032-task-003-inspector-popover-migration.md`
- Diff: inspector playlist popover migration and architecture-test baseline
  tightening.

## Result

Pass - 2026-05-02.

## Required Fixes

None.

## Optional Improvements

- Add a visual smoke screenshot for Library and Discover inspector popovers if
  future changes touch placement, surface width, or trigger styling.
- Consider moving playlist target resolution out of `render_action_row` in a
  future application-query task; this task intentionally preserved the existing
  screen-owned behavior.

## Architectural Drift

- `AddToPlaylistPopover` gained only a visual availability flag; it still does
  not import services or own command dispatch.
- Library and Discover screens still resolve targets and dispatch playlist
  append commands locally.
- Library and Discover screens also own the create-then-append callbacks, so
  every playlist popover exposes `+ New Playlist` without moving service
  behavior into the composite.
- View models no longer carry visual popover-open chrome state for these
  inspector actions.
- Architecture tests now reject the legacy panel helper/toggle patterns with a
  zero baseline.
- Architecture tests now reject `AddToPlaylistPopover` call sites that omit
  `.on_create(...)`.

## Missing Tests

No automated visual screenshot was added. The change is covered by view-model,
compile, clippy, and architecture tests; screenshot smoke remains appropriate
for future placement/styling edits.

## Merge Recommendation

Merge. ADR0032 now has the shared popover contract applied to release-detail
and inspector playlist actions, with architecture gates preventing the old
screen-local panel shape and missing create-mode affordance from returning.
