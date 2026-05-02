# ADR 0032 Review Checklist

## Reviewed Artifact

- ADR: `docs/adr/0032-ui-backend-boundary-and-popover-contracts.md`
- Plan: `docs/plans/adr-0032-ui-backend-boundary-phase-plan.md`
- Task: `docs/tasks/adr-0032-task-001-playlist-popover-contract.md`

## Required Checks

- Do view models remain GPUI-free?
- Do UI primitives/composites avoid service, DB, API-client, and screen imports?
- Do screens own command dispatch and callbacks?
- Does `AddToPlaylistPopover` own playlist popover chrome?
- Do Library and Discover share the same add-to-playlist popover family?
- Are raw full-width screen-local playlist panels removed?
- Does the change preserve playlist append semantics?
- Is visual smoke required before closing any popover chrome follow-up?

## Architectural Drift Checks

- No service calls in `src/ui/composites/playlist_popover.rs`.
- No DB queries in `src/ui/composites/playlist_popover.rs`.
- No GPUI imports added to `src/view_models/library.rs`.
- No screen-local replacement for `Popover` or `AddToPlaylistPopover`.
- No row-child panel used as a popover substitute.

## Merge Recommendation Template

Pass/fail:

Required fixes:

Optional improvements:

Architectural drift:

Missing tests:

Merge recommendation:

Next task adjustment:
