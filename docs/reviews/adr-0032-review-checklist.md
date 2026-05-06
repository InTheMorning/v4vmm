# ADR 0032 Review Checklist

## Reviewed Artifact

- ADR: `docs/adr/0032-ui-backend-boundary-and-popover-contracts.md`
- Plan: `docs/plans/adr-0032-ui-backend-boundary-phase-plan.md`
- Task: `docs/tasks/adr-0032-task-001-playlist-popover-contract.md`
- Task: `docs/tasks/adr-0032-task-002-architecture-test-enforcement.md`
- Task: `docs/tasks/adr-0032-task-003-inspector-popover-migration.md`

## Required Checks

- Do view models remain GPUI-free?
- Do UI primitives/composites avoid service, DB, API-client, and screen imports?
- Do screens own command dispatch and callbacks?
- Does `AddToPlaylistPopover` own playlist popover chrome?
- Do Library and Discover share the same add-to-playlist popover family?
- Does every playlist popover include `+ New Playlist` by wiring
  `.on_create(...)`?
- Are raw full-width screen-local playlist panels removed?
- Does the change preserve playlist append semantics?
- Is visual smoke required before closing any popover chrome follow-up?
- Does the task packet include UI/backend boundary checks when it touches
  presentation contracts, screens, or shared UI composites?

## Architectural Drift Checks

- No service calls in `src/ui/composites/playlist_popover.rs`.
- No DB queries in `src/ui/composites/playlist_popover.rs`.
- No GPUI imports added to `src/view_models/library.rs`.
- No screen-local replacement for `Popover` or `AddToPlaylistPopover`.
- No row-child panel used as a popover substitute.
- Architecture tests keep known legacy screen-local playlist popover panels
  at a zero baseline.
- Architecture tests hard-ban the Library release-detail playlist popover
  helper/state names removed by ADR0032.
- Architecture tests reject `AddToPlaylistPopover` call sites without
  `.on_create(...)`.

## Merge Recommendation Template

Pass/fail:

Required fixes:

Optional improvements:

Architectural drift:

Missing tests:

Merge recommendation:

Next task adjustment:
