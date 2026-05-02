# ADR 0031 Task 001 Review

## Reviewed Artifact

- ADR: `docs/adr/0031-release-detail-presentation-contract.md`
- Plan: `docs/plans/adr-0031-release-detail-presentation-contract-phase-plan.md`
- Task: `docs/tasks/adr-0031-task-001-contract-types-and-projection-tests.md`
- Diff scope:
  - `src/view_models/entity_detail.rs`
  - `docs/tasks/adr-0031-task-001-contract-types-and-projection-tests.md`

## Pass / Fail

Pass.

## Required Fixes

None.

## Optional Improvements

- Task 002 should render `ReleasePanelVm::Identity` as a demoted panel below
  summary/action areas.
- Task 002 should retire or narrow legacy default header and slot paths after
  the shell renders `ReleaseDetailVm::page()`.

## Architectural Drift

None found. The implementation keeps the new contract in
`src/view_models/entity_detail.rs`, reuses existing action and track-list
projections, and does not import GPUI, UI, screen, DB, API-client, or service
modules into the view-model layer.

## Missing Tests

No missing tests for Task 001 scope. Renderer behavior and visual smoke remain
covered by later ADR 0031 tasks.

## Merge Recommendation

Task 001 can be merged as the first ADR 0031 phase after verification gates are
green.

## Next Task Adjustment

Task 002 should consume `ReleaseDetailVm::page()` in the shared shell, keep
screen-provided action elements and images outside the projection layer, and
remove broad slots that can override hero, description, summary, or panel
placement.
