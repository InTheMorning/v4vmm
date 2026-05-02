# ADR 0035 Review Checklist

## Reviewed Artifacts

- `docs/adr/0035-track-surface-consolidation.md`
- `docs/plans/adr-0035-track-surface-consolidation-phase-plan.md`
- `docs/tasks/adr-0035-task-001-track-detail-vm-contract.md`
- `docs/tasks/adr-0035-task-002-track-detail-surface-composite.md`
- `docs/tasks/adr-0035-task-003-discover-track-surface-migration.md`
- `docs/tasks/adr-0035-task-004-library-track-surface-migration.md`
- `docs/tasks/adr-0035-task-005-guards-and-visual-gate.md`

## Gate Status

Status: Not ready.

Track detail consolidation is not safe for richer track/playlist/playback UI
work until Tasks 001-005 are implemented, checks are green, visual smoke is
recorded, and this checklist records `Proceed`.

## Structural Review Questions

- Does track presentation have exactly one row owner, one inspector-pane owner,
  and one full-detail surface owner?
- Does track display policy live in a GPUI-free VM contract family?
- Do Library and Discover pass typed slots instead of rebuilding shared chrome?
- Does artwork lookup stay screen-owned while artwork display stays
  composite-owned?
- Are loading, missing, and failed track surface states composite-owned?
- Do `TrackRow`, `TrackInspectorPane`, and `TrackDetailSurface` consume the
  `TrackDetailVm` family?
- Are Library advanced metadata workflows preserved?
- Are Discover search/link/lazy-section workflows preserved?
- Do architecture tests block screen-local track row, inspector, and detail
  surface drift?
- Do architecture tests block screen-local fallback labels and canonical field
  labels?
- Do architecture tests block untyped `AnyElement`/`impl IntoElement` slot
  APIs?
- Did visual smoke use user-provided screenshots?

## Task Results

| Task | Status | Required Evidence | Notes |
|---|---|---|---|
| Task 001 track detail VM contract | Pending | `TrackDetailVm`, `TrackRowVm`, labels, slots, load state, unit tests, and export |  |
| Task 002 track surface composites | Pending | `TrackDetailSurface`, `TrackInspectorPane`, `TrackRow` VM binding, and shared UI checks |  |
| Task 003 Discover migration | Pending | Discover rows, inspector, and detail route through shared surfaces |  |
| Task 004 Library migration | Pending | Library rows, inspector, and detail route through shared surfaces; advanced panels preserved |  |
| Task 005 guards and visual gate | Pending | Named ADR 0035 architecture guards, full checks, user screenshots |  |

## Visual Smoke

- Library full detail: pending.
- Library inspector pane: pending.
- Discover full detail: pending.
- Discover inspector pane: pending.
- Library advanced metadata panels, if touched: pending.
- Discover lazy sections, if touched: pending.

## Merge Recommendation Template

Use this for each task review:

```text
Status: Pass / Fail

Required fixes:
- ...

Optional improvements:
- ...

Architectural drift:
- ...

Missing tests or visual proof:
- ...

Feature-readiness impact:
- Proceed / Proceed with constraints / Do not proceed
```
