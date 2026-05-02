# One Owner Per Surface Review Checklist

## Reviewed Artifacts

- `docs/plans/one-owner-per-surface-plan.md`
- `docs/tasks/one-owner-per-surface-task-001-recents-surface-ownership.md`
- `docs/tasks/one-owner-per-surface-task-002-fallback-display-accessors.md`
- `docs/tasks/one-owner-per-surface-task-003-composite-display-contract-audit.md`
- `docs/tasks/one-owner-per-surface-task-004-feature-readiness-gate.md`

## Gate Status

Status: Not ready for richer playlist/playback feature work until Tasks
001-004 are implemented and this checklist records `Proceed`.

## Structural Review Questions

- Does each changed surface name exactly one composite or primitive owner?
- Does each changed surface name exactly one VM/display contract owner?
- Did the change remove, rather than relocate, screen-local fallback policy?
- Did the change use existing tokens, roles, components, and icon ownership?
- Did the same change add or strengthen a regression guard?
- Did visual smoke cover the affected user-facing surface?

## Task Results

| Task | Status | Required Evidence | Notes |
|---|---|---|---|
| Task 001 recents surface ownership | Pending | VM/composite label contract, regression guard, Discovery recents visual smoke | Current canary is recent tiles rendering `...`. |
| Task 002 fallback display accessors | Pending | VM tests, architecture fallback guards, ADR 0033 test-list sync | Blocks feature work on surfaces that still own fallback policy in screens. |
| Task 003 composite display-contract audit | Pending | Composite contract notes, narrow allowlist or test guard | Blocks feature work through composites with policy-bearing loose strings. |
| Task 004 feature-readiness gate | Pending | Green checks, visual smoke summary, final gate decision | Records whether richer playlist/playback work may proceed. |

## HIG Review Focus

- Popovers remain compact, anchored, and owned by shared popover/composite
  code. Screens only wire command callbacks.
- Buttons have consistent style/content/role and do not reintroduce bare
  leading glyph strings.
- Layout hierarchy uses tokens and shared rows/headers, not local pixel or
  color choices.
- Dense desktop views remain scannable: title, subtitle, metadata, state, and
  actions have predictable placement across Library and Discovery.

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
