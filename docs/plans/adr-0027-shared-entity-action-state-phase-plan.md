# ADR 0027 Shared Entity Action State Phase Plan

## Status

Proposed - 2026-05-01. Tasks 001-002 implemented and verified.

## Goal

Make equivalent Library and Discover release/track actions render from shared,
GPUI-free action-state inputs while keeping command dispatch and popover state
in screen/application adapters.

## Non-Goals

- Do not move command execution into shared projection or UI modules.
- Do not move database, MusicIndex, MusicBrainz, download, or playlist service
  calls into `src/views.rs`, `src/view_models/entity_detail.rs`, or
  `src/ui_entity.rs`.
- Do not redesign navigation or sidebars.
- Do not solve local source-fact persistence in this ADR.
- Do not implement non-URL artwork resolution.

## Current State

- ADR 0026 shell parity is implemented.
- `EntityActionVm` descriptors exist in `src/view_models/entity_detail.rs`.
- Library and Discover still derive visible action labels, tones, busy state,
  and row controls from separate screen-local view-models.
- Library repeated track removal is visually dominant compared with Discover's
  quiet row action treatment.
- Library detail can show redundant downloaded state even when membership is
  implied by remove actions.

## Target State

- `ReleaseDetailVm` and shared track-row projections receive pure action-state
  inputs.
- Action descriptors consistently represent membership, removal/download
  state, playlist affordances, and optional MusicBrainz affordances.
- Screen adapters bind those descriptors to existing command handlers.
- Detail rows suppress redundant downloaded state when the action state already
  communicates Library membership.
- Projection tests cover labels, tones, busy/disabled state, and redundant-row
  suppression.

## Affected Modules

- `src/view_models/entity_detail.rs`
- `src/view_models/library.rs`
- `src/view_models/search.rs`
- `src/library.rs`
- `src/search.rs`
- `src/ui_entity.rs`
- `tests/architecture_tests.rs`

## Proposed Sequence

1. Track-row membership action state. Implemented by
   `docs/tasks/adr-0027-task-001-track-row-action-state.md`.
2. Release-level membership and playlist action state. Implemented by
   `docs/tasks/adr-0027-task-002-release-action-state.md`.
3. MusicBrainz and compare/provenance action state.
4. Destructive row control treatment under ADR 0025, if needed after shared
   descriptors land.
5. Final visual smoke against the same Library/Discover release fixture.

## Schema/API Implications

None. ADR 0027 consumes already-known screen/application state. If a phase
needs new local source facts or query results, stop and route that work to ADR
0024 or a schema/persistence ADR.

## Risk Areas

- Accidentally moving GPUI handlers or services into shared projections.
- Trying to solve all Library/Discover differences in one large edit.
- Encoding screen-specific popup state into the shared action-state structs.
- Changing command semantics while only intending to change action projection.

## Test Strategy

- Unit tests for new action-state projection inputs.
- Existing screen view-model tests for adapter state conversion.
- Architecture tests preventing GPUI, screen, service, DB, and client imports
  in shared projection/action-state modules.
- Manual visual smoke comparing the same release in Library and Discover.

## Rollback Strategy

- Keep each phase behind narrow adapter changes.
- If a phase produces confusing descriptors, revert that phase without
  changing the ADR 0026 shell.
- Do not delete existing screen-local action paths until the matching shared
  action projection is covered by tests and visual smoke.
