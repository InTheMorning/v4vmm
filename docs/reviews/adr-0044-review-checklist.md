# ADR 0044 Review Checklist

## Reviewed Artifacts

- `docs/adr/0044-playlist-drag-handle-reordering.md`
- `docs/plans/adr-0044-playlist-drag-handle-reordering-phase-plan.md`
- `docs/tasks/adr-0044-task-001-playlist-reorder-vm-contract.md`
- `docs/tasks/adr-0044-task-002-playlist-drag-shell.md`
- `docs/tasks/adr-0044-task-003-playlist-reorder-guards-visual.md`

## Gate Status

Status: Awaiting operator visual recheck after follow-up fixes on 2026-05-14.

Readiness decision: Pending visual verification.

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

- User visual review on 2026-05-13 found that dropping a dragged playlist
  row could leave playlist view without reordering, track removal from a
  playlist-origin detail did not return to the playlist or refresh the
  unavailable row in place, and the Playlists `+` control collapsed the
  Playlists disclosure.
- Fixed on 2026-05-13: application-event Library refresh now preserves the
  active detail; playlist-origin track inspectors carry a VM-owned `Back to
  Playlist` action and return to the playlist after library removal; playlist
  drop targets use a wider tokenized hit area; and the Playlists disclosure
  click target is limited to the heading cluster.
- Follow-up visual review on 2026-05-14 found playlist reorder still required
  a view switch to refresh, row drops still required a precise separator
  pixel, and playlist-origin library removal still failed to refresh the
  playlist availability state in place. The Playlists `+` control and Back to
  Playlist behavior passed.
- Fixed on 2026-05-14: same-playlist selection now refreshes the paged
  playlist listing instead of preserving a stale warm snapshot, and ready
  playlist rows are also drop destinations with a stable top insertion border
  so users can drop on the nearest row instead of hunting for the separator
  band.
- Second follow-up visual review on 2026-05-14 found reorder and removal
  commit, but the playlist view briefly fell back to placeholder rows until
  mouse activity, and row-level drop feedback looked like an overwrite target.
- Fixed on 2026-05-14: fresh playlist actors are now primed from the already
  loaded playlist rows before publish, preventing the post-mutation placeholder
  flash; row-level drag feedback now shows only the insertion border instead
  of tinting the destination row.
- Third visual review on 2026-05-14 confirmed reorder, removal refresh, and
  toolbar behavior, but found inactive thin insertion feedback could still
  appear on a row-level no-op drop target. Fixed by suppressing drag-over
  feedback unless the computed reorder target would dispatch a move.
- Operator feedback then clarified the desired quantization: row hover should
  resolve to the nearest insertion point, always draw the thick insertion cue,
  and treat only same-place or outside-playlist drops as no-ops. Fixed by
  making row-level targets quantize by drag direction and by reserving a
  tokenized thick insertion border for row hover feedback.
- Follow-up correction: the row-level insertion line must draw on the actual
  destination edge, not always on the hovered row's top edge. Downward drags
  now draw the thick cue on the hovered row's bottom edge; upward drags draw
  it on the top edge.
- HIG/architecture drift review on 2026-05-14 found that separator drop zones
  still overlapped row-edge targets, invalid cross-playlist drags had no
  explicit rejection feedback, same-playlist reselects discarded the warm
  actor cache, and playlist-origin track detail used a one-off optional id.
  Fixed on 2026-05-14: row-edge targets are now the only reorder drop
  mechanism, invalid destinations show muted/no-drop feedback, same-playlist
  reselect refreshes the existing actor, and track inspector return state is a
  typed `InspectorOrigin`.
- Visual proof still needs operator recheck because this execution session
  cannot inspect the running display directly.

## Optional Improvements

- None recorded yet.

## Architectural Drift

- None recorded yet.

## Missing Tests

- None recorded for automated coverage. Visual coverage is pending operator
  recheck after the 2026-05-14 follow-up fixes.

## Merge Recommendation

Pending. Do not mark ADR 0044 ready or commit as complete until light and
dark visual proof confirms reorder drops commit in place, the handle,
Actions menu, unavailable row, playlist-origin return path, and insertion-line
feedback, including row-level forgiving drop targets.
