# ADR 0044 Review Checklist

## Reviewed Artifacts

- `docs/adr/0044-playlist-drag-handle-reordering.md`
- `docs/plans/adr-0044-playlist-drag-handle-reordering-phase-plan.md`
- `docs/tasks/adr-0044-task-001-playlist-reorder-vm-contract.md`
- `docs/tasks/adr-0044-task-002-playlist-drag-shell.md`
- `docs/tasks/adr-0044-task-003-playlist-reorder-guards-visual.md`

## Gate Status

Status: Not started.

Readiness decision: Pending.

## Required Checks

- [ ] Playlist rows no longer show visible up/down reorder buttons.
- [ ] Drag starts from the handle only.
- [ ] Drag handle id and accessibility label come from
  `PlaylistTrackRowVm`.
- [ ] Move Up and Move Down fallback menu items come from
  `PlaylistTrackRowVm`.
- [ ] Move Up is disabled for the first row.
- [ ] Move Down is disabled for the last row.
- [ ] Drop feedback is an insertion line.
- [ ] Same-playlist drops commit through `ReorderPlaylistTrack`.
- [ ] Original-slot and adjacent no-op drops do not dispatch.
- [ ] Pending paged rows are not draggable.
- [ ] No database schema changes were introduced.
- [ ] Architecture tests cover no-arrow controls and handle/menu
  ownership.
- [ ] Light-theme visual proof reviewed for handle, menu, unavailable
  row, and insertion line.
- [ ] Dark-theme visual proof reviewed for handle, menu, unavailable
  row, and insertion line.
- [ ] `cargo fmt -- --check` green.
- [ ] `cargo check` green.
- [ ] `cargo test` green.
- [ ] `cargo clippy -- -D warnings` green.

## Required Fixes

- None recorded yet.

## Optional Improvements

- None recorded yet.

## Architectural Drift

- None recorded yet.

## Missing Tests

- None recorded yet.

## Merge Recommendation

Pending implementation and review.
