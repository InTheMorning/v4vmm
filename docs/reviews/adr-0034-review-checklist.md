# ADR 0034 Review Checklist

## Reviewed Artifacts

- `docs/adr/0034-scale-aware-ui-tokens-and-controls.md`
- `docs/plans/adr-0034-scale-aware-ui-tokens-phase-plan.md`
- `docs/tasks/adr-0034-task-001-scale-shared-primitives.md`
- `docs/tasks/adr-0034-task-002-scale-playlist-popover-layout.md`
- `docs/tasks/adr-0034-task-003-scale-regression-guards.md`
- `docs/tasks/adr-0034-task-004-visual-smoke-and-readiness-gate.md`

## Gate Status

Status: Not ready for richer playlist/playback feature work that depends on
popover, button, label, icon, or surface scaling until Tasks 001-004 are
implemented and this checklist records `Proceed`.

## Structural Review Questions

- Do shared primitives resolve user-facing dimensions through scaled tokens?
- Did the implementation avoid screen-local scale compensation?
- Does `AddToPlaylistPopover` remain the single owner of playlist popover
  layout and behavior?
- Does `+ New Playlist` remain present wherever create mode is wired?
- Are all remaining unscaled shared UI dimensions allowlisted with a specific
  non-user-facing reason?
- Did visual smoke use user-provided screenshots rather than pointer
  automation?

## Task Results

| Task | Status | Required Evidence | Notes |
|---|---|---|---|
| Task 001 scale shared primitives | Pending | Primitive render paths use scaled tokens; checks green |  |
| Task 002 scale playlist popover layout | Pending | Playlist popover local dimensions use scaled tokens; ownership tests green |  |
| Task 003 scale regression guards | Pending | New architecture guard and ADR enforcing-test docs |  |
| Task 004 visual smoke and readiness gate | Pending | Full checks and user screenshot review |  |

## Visual Smoke

- Library playlist popover at medium scale: pending.
- Library playlist popover at alternate scale: pending.
- Discovery recents grid: pending.
- Now-playing chrome: pending.

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
