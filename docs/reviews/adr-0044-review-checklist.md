# ADR 0044 Review Checklist

## Reviewed Artifacts

- `docs/adr/0044-playlist-drag-handle-reordering.md`
- `docs/plans/adr-0044-playlist-drag-handle-reordering-phase-plan.md`
- `docs/tasks/adr-0044-task-001-playlist-reorder-vm-contract.md`
- `docs/tasks/adr-0044-task-002-playlist-drag-shell.md`
- `docs/tasks/adr-0044-task-003-playlist-reorder-guards-visual.md`

## Gate Status

Status: Blocked - visual proof unavailable on 2026-05-11.

Readiness decision: Blocked on visual verification.

## Required Checks

- [x] Playlist rows no longer show visible up/down reorder buttons.
- [x] Drag starts from the handle only.
- [x] Drag handle id and accessibility label come from
  `PlaylistTrackRowVm`.
- [x] Move Up and Move Down fallback menu items come from
  `PlaylistTrackRowVm`.
- [x] Move Up is disabled for the first row.
- [x] Move Down is disabled for the last row.
- [x] Drop feedback is an insertion line.
- [x] Same-playlist drops commit through `ReorderPlaylistTrack`.
- [x] Original-slot and adjacent no-op drops do not dispatch.
- [x] Pending paged rows are not draggable.
- [x] No database schema changes were introduced.
- [x] Architecture tests cover no-arrow controls and handle/menu
  ownership.
- [ ] Light-theme visual proof reviewed for handle, menu, unavailable
  row, and insertion line.
- [ ] Dark-theme visual proof reviewed for handle, menu, unavailable
  row, and insertion line.
- [x] `cargo fmt -- --check` green.
- [x] `cargo check` green.
- [x] `cargo test` green.
- [x] `cargo clippy -- -D warnings` green.

## Required Fixes

- Visual proof cannot be completed because `DISPLAY=:0 wmctrl -l` fails
  with `Authorization required, but no authorization protocol specified`
  and `Cannot open display.`

## Optional Improvements

- None recorded yet.

## Architectural Drift

- None recorded yet.

## Missing Tests

- None recorded for automated coverage. Visual coverage is blocked by
  display access.

## Merge Recommendation

Blocked. Do not mark ADR 0044 ready or commit as complete until light and
dark visual proof covers the handle, Actions menu, unavailable row, and
insertion-line feedback.
