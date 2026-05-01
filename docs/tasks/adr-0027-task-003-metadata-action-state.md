# ADR 0027 Task 003: Metadata Action State

## Status

Implemented.

## Goal

Move track metadata actions for ID3 comparison and MusicBrainz lookup into the
shared, GPUI-free action descriptor vocabulary so Library and Discover can bind
equivalent controls from the same state model.

## Read

- `docs/adr/0027-shared-entity-action-state.md`
- `docs/plans/adr-0027-shared-entity-action-state-phase-plan.md`
- `docs/tasks/adr-0027-task-001-track-row-action-state.md`
- `docs/tasks/adr-0027-task-002-release-action-state.md`
- `src/view_models/entity_detail.rs`
- `src/library.rs`
- `src/search.rs`

## Files Changed

- `src/view_models/entity_detail.rs`
- `src/library.rs`
- `src/search.rs`
- `docs/plans/adr-0027-shared-entity-action-state-phase-plan.md`
- `docs/tasks/adr-0027-task-003-metadata-action-state.md`
- `docs/reviews/adr-0027-task-003-review.md`

## Do Not Touch

- Do not change metadata compare, MusicBrainz lookup, staging, or apply command
  behavior.
- Do not change database schema or migrations.
- Do not move GPUI handlers, service calls, or network calls into shared
  projection code.
- Do not redesign the metadata grid.

## Constraints

- Shared metadata action state must be plain data.
- Screen adapters translate their existing panel state into shared state.
- Shared descriptors own labels, enabled state, action kind, and visibility
  gates.
- Screens still own click handlers and command dispatch.

## Implementation Summary

- Added `MetadataPanelState` and `TrackMetadataActionState` to
  `src/view_models/entity_detail.rs`.
- Added shared projection tests for compare and MusicBrainz labels, disabled
  states, local-file gating, and panel visibility.
- Updated Library track detail actions to render `Compare ID3` and
  `MusicBrainz` controls from shared descriptors.
- Updated Library metadata panel visibility to read from the shared metadata
  state.
- Updated Discover track action rows to expose compare and MusicBrainz actions
  through the same descriptors and handlers.
- Updated Discover MusicBrainz column visibility to read from the shared
  metadata state.

## Acceptance Criteria

- [x] Shared projection tests cover compare/MusicBrainz labels and loading
  disabled state.
- [x] Library and Discover bind compare/MusicBrainz controls from shared
  descriptors.
- [x] Existing compare and MusicBrainz handlers are unchanged.
- [x] Shared projection modules remain GPUI-free.
- [x] Required verification commands pass.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test entity_detail
cargo test --test architecture_tests
cargo clippy --lib --tests -- -D warnings
cargo test
```

## Escalation Triggers

- The slice requires changing metadata lookup or compare command semantics.
- Panel visibility requires fetching new state from services.
- Existing adapters force GPUI, service, or DB imports into shared projection
  code.
