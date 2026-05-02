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

Status: Proceed.

Track detail consolidation has its code and architecture-test pass in place.
User-provided screenshots on 2026-05-02 cover Discover detail, Discover row
surfaces, Library feed/detail rows, Library track detail, and Library advanced
metadata panels. Richer track, playlist, and playback UI work may proceed under
the ADR 0033, ADR 0034, and ADR 0035 guards.

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
| Task 001 track detail VM contract | Complete | `TrackDetailVm`, `TrackRowVm`, labels, slots, load state, unit tests, and export | `cargo test track_detail` green |
| Task 002 track surface composites | Complete | `TrackDetailSurface`, `TrackInspectorPane`, `TrackRow` VM binding, and shared UI checks | `track_surface_slots_are_typed` green |
| Task 003 Discover migration | Complete | Discover rows, inspector, and detail route through shared surfaces | User screenshots received 2026-05-02 |
| Task 004 Library migration | Complete | Library rows, inspector, and detail route through shared surfaces; advanced panels preserved | User screenshots received 2026-05-02 |
| Task 005 guards and visual gate | Complete | Named ADR 0035 architecture guards, full checks, user screenshots | Full checks green; visual gate passed with user screenshots |

## Visual Smoke

- Library full detail: passed with user screenshot on 2026-05-02.
- Library inspector pane: passed with user screenshot on 2026-05-02.
- Discover full detail: passed with user screenshot on 2026-05-02.
- Discover inspector pane: passed with user screenshot on 2026-05-02.
- Library advanced metadata panels, if touched: passed with user screenshot on 2026-05-02.
- Discover lazy sections, if touched: passed with user screenshot on 2026-05-02.

## Residual Notes

The lower Library advanced metadata grid may still use provenance-specific
labels such as `Album/Feed` because it is an advanced comparison/provenance
panel, not the shared track summary row. Future changes should move additional
advanced-grid labels into dedicated VMs or typed slots instead of screen-local
presentation code.

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
