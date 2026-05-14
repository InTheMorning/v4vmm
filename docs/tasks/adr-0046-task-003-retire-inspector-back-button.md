# ADR 0046 Task 003: Retire Inspector Back-to-Playlist Button

Status: Implemented - 2026-05-14.

## Goal

Remove the inspector-owned "Back to Playlist" control and the
`InspectorFrame.origin` / `InspectorOrigin` navigation state. Frame nav state (task 002)
already records the origin; the frame shell composite (task 006) will
later surface Back. In the interim, users return to the playlist via
the source list.

## Files to Inspect

- `docs/adr/0046-workspace-frame-architecture.md`
- `docs/tasks/adr-0046-task-002-frame-history-vm.md`
- `src/library.rs`
- `src/library/app_impl.rs`
- `src/view_models/library.rs`
- `src/ui/shells/library/track_detail_metadata.rs`
- `tests/architecture_tests.rs`

## Files Likely to Change

- `src/library.rs`
- `src/library/app_impl.rs`
- `src/view_models/library.rs`
- `src/ui/shells/library/track_detail_metadata.rs`
- `tests/architecture_tests.rs`

## Do Not Touch

- Frame nav state added in task 002 (do not remove)
- `src/db.rs`, playback engine
- Search, Settings, Discover surfaces
- Playlist refresh-preserving path in `LibraryApp`

## Constraints

- Remove the visible inspector back control.
- Remove `LibraryTrackPlaylistReturnDisplay` and the
  `playlist_return_display` VM helper.
- Remove `InspectorFrame.origin` and the `InspectorOrigin` enum if no
  non-navigation caller remains.
- Keep `LibraryReloadMode::PreserveDetail` and `refresh_selected_detail`.
- Keep `return_to_playlist` only if frame nav state still calls it;
  otherwise delete.
- Architecture guards must forbid re-introducing the back button.

## Implementation Steps

1. Delete `LibraryTrackPlaylistReturnDisplay` and
   `LibraryTrackActionVm::playlist_return_display` from
   `src/view_models/library.rs`.
2. Remove the back-button render path in
   `src/ui/shells/library/track_detail_metadata.rs`.
3. Remove `InspectorFrame.origin` from `src/library.rs`. Drop
   `InspectorOrigin` and origin assignment from playlist-track
   selection if no non-navigation caller remains.
4. Decide whether `select_playlist_track` survives. If yes, simplify
   to record only frame nav state; if no, replace call sites with
   `select_track`.
5. Drop `return_to_playlist` if unreachable. Otherwise narrow its
   responsibilities to frame-nav-driven selection.
6. Update architecture tests: remove guards that asserted the back
   button exists; add guards that the button is absent.
7. Update or remove tests that asserted the `playlist_return_display`
   contract.

## Acceptance Criteria

- [x] Track inspector no longer renders a back-to-playlist control.
- [x] `InspectorFrame.origin` is gone or no longer carries navigation state.
- [x] `LibraryTrackPlaylistReturnDisplay` is gone.
- [x] Architecture guards assert the back button is absent.
- [x] Frame nav state still records playlist origin entries.

## Implementation Notes

- Removed `InspectorOrigin` and `InspectorFrame.origin` from the
  Library inspector model.
- Removed `LibraryTrackPlaylistReturnDisplay`,
  `LibraryTrackActionVm::playlist_return_display`, and the related unit
  assertion.
- Removed the inspector action-row Back-to-Playlist control and the
  `navigate_back_to_playlist` wrapper.
- Kept Task 002 frame navigation state and the private frame-history
  restore helper used after library-removal success.
- Updated architecture guards so the old inspector-local return
  contract is forbidden rather than required.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test
cargo test --test architecture_tests
cargo clippy -- -D warnings
```

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0046-workspace-frame-architecture.md`
- `docs/tasks/adr-0046-task-002-frame-history-vm.md`
- `src/library.rs`
- `src/library/app_impl.rs`
- `src/view_models/library.rs`
- `src/ui/shells/library/track_detail_metadata.rs`
- `tests/architecture_tests.rs`

Goal:
- Remove inspector "Back to Playlist" control and
  `InspectorFrame.origin` navigation use. Keep frame nav state intact.

Constraints:
- Do not remove frame nav state added in task 002.
- Preserve playlist refresh-preserving path.
- Architecture guards forbid the back button.

Do not touch:
- Frame nav state additions from task 002
- `src/db.rs`, playback engine
- Search, Settings, Discover

Acceptance criteria:
- Inspector back-to-playlist is gone in code and tests.
- `InspectorFrame.origin` navigation use is gone.
- Guards assert absence of the back button.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test`
- `cargo test --test architecture_tests`
- `cargo clippy -- -D warnings`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- Removing `InspectorFrame.origin` triggers unrelated refactors in
  `LibraryApp` reload paths.
- Frame nav state is insufficient to express the playlist origin
  (signals task 002 was incomplete).
