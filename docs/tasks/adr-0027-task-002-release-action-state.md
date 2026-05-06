# ADR 0027 Task 002: Release Action State

## Status

Implemented.

## Goal

Move release-level feed membership and playlist action labels into the shared,
GPUI-free projection layer so Library and Discover render equivalent controls
for the same release.

## Read

- `docs/adr/0027-shared-entity-action-state.md`
- `docs/plans/adr-0027-shared-entity-action-state-phase-plan.md`
- `docs/tasks/adr-0027-task-001-track-row-action-state.md`
- `src/view_models/entity_detail.rs`
- `src/view_models/library.rs`
- `src/view_models/search.rs`
- `src/library.rs`
- `src/search.rs`

## Files Changed

- `src/view_models/entity_detail.rs`
- `src/view_models/library.rs`
- `src/view_models/search.rs`
- `src/library.rs`
- `src/search.rs`
- `docs/plans/adr-0027-shared-entity-action-state-phase-plan.md`
- `docs/tasks/adr-0027-task-002-release-action-state.md`
- `docs/reviews/adr-0027-task-002-review.md`

## Do Not Touch

- Do not change database schema or migrations.
- Do not change command implementations.
- Do not change MusicIndex, MusicBrainz, download, playlist, or playback
  service behavior.
- Do not move GPUI imports into shared projection modules.
- Do not solve MusicBrainz/provenance action state in this task.

## Constraints

- Release action state must be plain data.
- Shared projections may emit `EntityActionVm`; screens still own click
  handlers, popover state, and command dispatch.
- Library and Discover must use the same feed-level labels for download/remove
  and playlist actions.
- Repeated release controls should avoid introducing another button vocabulary.

## Implementation Summary

- Added `ReleaseMembershipState` and `ReleaseActionState` to
  `src/view_models/entity_detail.rs`.
- Added release action projection tests for Discover, Library, busy, and
  playlist-open states.
- Updated Discover action rows to adapt feed subscription state into the shared
  release descriptor.
- Updated Library album detail actions to derive the remove-feed and
  add-to-playlist labels from the shared release descriptor.
- Changed Library album detail copy from `Unsubscribe Feed` and
  `Add album to playlist` to the shared `Remove Feed` and
  `Add feed to playlist` vocabulary.

## Acceptance Criteria

- [x] Shared projection tests cover release membership, busy, tone, disabled,
  and playlist-open states.
- [x] Library and Discover release-level feed actions use the same descriptor
  vocabulary.
- [x] Existing command handlers are unchanged.
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

- The slice requires changing command behavior.
- MusicBrainz or provenance actions are needed to make the release membership
  actions coherent.
- Existing adapters force service or GPUI dependencies into shared projection
  code.
