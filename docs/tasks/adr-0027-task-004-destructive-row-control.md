# ADR 0027 Task 004: Destructive Row Control Treatment

## Status

Implemented.

## Goal

Bind descriptor-level `DestructiveQuiet` row actions to an ADR 0025 control
role so repeated row removals remain compact while still communicating
destructive intent.

## Read

- `docs/adr/0027-shared-entity-action-state.md`
- `docs/plans/adr-0027-shared-entity-action-state-phase-plan.md`
- `docs/tasks/adr-0027-task-001-track-row-action-state.md`
- `docs/tasks/adr-0027-task-002-release-action-state.md`
- `docs/tasks/adr-0027-task-003-metadata-action-state.md`
- `src/ui/control_styles.rs`
- `src/library.rs`

## Files Changed

- `src/ui/control_styles.rs`
- `src/library.rs`
- `docs/plans/adr-0027-shared-entity-action-state-phase-plan.md`
- `docs/tasks/adr-0027-task-004-destructive-row-control.md`
- `docs/reviews/adr-0027-task-004-review.md`

## Do Not Touch

- Do not change remove/download command behavior.
- Do not reintroduce filled destructive controls for repeated row actions.
- Do not add screen-local color literals.
- Do not change database schema, services, or metadata flows.

## Constraints

- The visual treatment must live in the ADR 0025 control-style boundary.
- Row controls remain compact `Plain` buttons.
- Destructive row controls use semantic danger text tokens rather than raw
  colors.
- Screens choose the control style from shared action descriptor tone.

## Implementation Summary

- Added `ControlStyle::DestructiveRowAction`.
- Mapped it to a compact plain button using `SemanticColor::DangerLabel`.
- Added a control-style unit test for the role.
- Updated Library album track rows to choose `DestructiveRowAction` when the
  shared action descriptor tone is `DestructiveQuiet`.

## Acceptance Criteria

- [x] Repeated Library row removal controls are compact plain controls.
- [x] Destructive row intent is expressed through ADR 0025 control styles.
- [x] No raw colors are introduced.
- [x] Existing command handlers are unchanged.
- [x] Required verification commands pass.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test control_styles
cargo test --test architecture_tests
cargo clippy --lib --tests -- -D warnings
cargo test
```

## Escalation Triggers

- The change requires altering command behavior.
- A broader metadata-action or release-action destructive role is needed before
  final visual smoke.
- The role needs new tokens instead of existing semantic danger labels.
