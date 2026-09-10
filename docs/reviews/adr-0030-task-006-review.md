# ADR 0030 Task 006 Review: Scroll Containers

## Current Gate

Open - reconciled 2026-09-10. The old Discovery/Recent Feeds scroll paths
are retired by ADRs 0047/0048/0060/0062. Bounded scrolling survives on Music's
local and Index details and Settings. Follow
[Scroll Containers](../runbooks/inherited-ui-checks.md#scroll-containers--adr-0030-task-006).

Record wheel, scrollbar, and supported keyboard results for each overflowing
pane in Light and Dark. Missing fixtures leave that subcheck open.
The original review below records mechanical evidence and the unmet check.

## Historical Review

## Reviewed Artifact

- `src/ui/composites/release_detail_surface.rs`
- `src/library.rs`
- `src/search.rs`
- `src/app.rs`
- `docs/tasks/adr-0030-task-006-scroll-containers.md`

## Pass/Fail

Pass.

## Required Fixes

None.

## Optional Improvements

- Manual smoke should verify wheel, scrollbar, and keyboard scrolling for
  Library artist, album/feed, playlist, track, Discovery inspector, and
  Settings panes.

## Architectural Drift

None. The change is limited to bounded flex sizing on existing scroll leaves and
their immediate split-pane ancestors.

## Missing Tests

No automated UI scroll test was added. The existing architecture test gate
passes, but GPUI scroll interaction still needs manual verification.

## Merge Recommendation

Merge Task 006 after manual smoke, or merge as a layout-only fix with the manual
smoke noted as residual verification. Command gates passed on 2026-05-01.
