# ADR 0046 Task 002: Frame History View Model

Status: Implemented - 2026-05-14.

## Goal

Wire frame history into `LibraryApp` so playlist-origin navigation is
represented in `FrameNavigationState`, not in `InspectorFrame.origin`. Keep
the visible inspector "Back to Playlist" control functional in this
task by reading the destination from frame nav state. Task 003 removes
the button itself.

## Files to Inspect

- `docs/adr/0046-workspace-frame-architecture.md`
- `docs/tasks/adr-0046-task-001-workspace-model-types.md`
- `src/view_models/workspace.rs`
- `src/library.rs`
- `src/library/app_impl.rs`
- `src/ui/shells/library/playlist_detail.rs`
- `src/ui/shells/library/track_detail_metadata.rs`

## Files Likely to Change

- `src/view_models/workspace.rs` (push/pop helpers if missing)
- `src/library.rs`
- `src/library/app_impl.rs`

## Do Not Touch

- `src/ui/*` composites (frame_shell does not exist yet)
- `src/app/tab_bar.rs`
- `src/db.rs`, playback engine
- Search and Settings paths

## Constraints

- Observable behavior unchanged: clicking a playlist track still opens
  the inspector; "Back to Playlist" still returns to the playlist.
- Inspector continues to render the back button in this task.
- The back button destination is sourced from `FrameNavigationState`,
  not from `InspectorFrame.origin`.
- Leave `InspectorFrame.origin` and `InspectorOrigin` in place for this
  task, but stop using them as the playlist-return source of truth
  (deletion happens in task 003).
- All new fallible operations on the nav state return `Result`.

## Implementation Steps

1. Add `FrameNavigationState` storage on `LibraryApp` keyed by frame
   id (single-frame layout for now).
2. When a playlist is selected, push a `PlaylistDetail(playlist_id)`
   entry onto the current frame's back stack.
3. When a playlist track is selected, push a `TrackDetail(track_id)`
   entry. Stop populating `InspectorFrame.origin`.
4. Add `LibraryApp::frame_back_destination(&self) -> Option<
   FrameNavigationEntry>` reading the current frame's back-stack top.
5. Update `return_to_playlist` (called from the inspector button) to
   pop the back stack and dispatch the resulting entry. Keep the
   playlist-select path for the rendered destination.
6. Unit tests covering: selecting a playlist then a track yields a
   back-stack of `[PlaylistDetail]`; back navigates to the playlist;
   back when stack empty is a no-op.
7. Architecture guard: assert
   `src/library/app_impl.rs` no longer writes to
   `frame.origin =` in playlist track-selection paths.

## Acceptance Criteria

- [x] Playlist track selection pushes back-stack entries onto the
  frame nav state.
- [x] `return_to_playlist` reads destination from the frame nav
  state.
- [x] `InspectorFrame.origin` is not assigned by new code
  paths.
- [x] Existing user-visible behavior (open playlist, open track, click
  back) is preserved.
- [x] Unit tests cover the back/forward boundary cases.

## Implementation Notes

- `LibraryApp` now owns a content-frame `FrameNavigationState`.
- User playlist selection records `PlaylistDetail`; internal playlist
  refreshes use restore mode so reloads and reorder/remove refreshes do
  not fill the back stack.
- Playlist track selection pushes `TrackDetail` while leaving
  `InspectorFrame.origin` unset.
- The temporary inspector Back control still renders during Task 002,
  but its destination is passed from `LibraryApp::frame_back_destination`
  instead of `InspectorFrame.origin`.
- `InspectorFrame.origin` and `InspectorOrigin` intentionally remain as
  dead legacy fields until Task 003 removes the inspector-local Back
  control and deletes the old navigation state.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test workspace
cargo test library
cargo test --test architecture_tests
```

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0046-workspace-frame-architecture.md`
- `docs/tasks/adr-0046-task-001-workspace-model-types.md`
- `src/view_models/workspace.rs`
- `src/library.rs`
- `src/library/app_impl.rs`

Goal:
- Wire `FrameNavigationState` into `LibraryApp` so playlist-origin
  navigation lives in frame history, not in `InspectorFrame`.

Constraints:
- Inspector "Back to Playlist" button remains visible in this task.
- The button destination is sourced from frame nav state.
- No new writes to `InspectorFrame.origin`.
- Observable behavior unchanged.

Do not touch:
- `src/ui/*` composites
- `src/app/tab_bar.rs`
- `src/db.rs`, playback engine
- Search and Settings paths

Acceptance criteria:
- Frame nav state pushes/pops on playlist/track selection.
- `return_to_playlist` reads destination from nav state.
- Tests cover boundary behavior of back/forward stacks.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test workspace`
- `cargo test library`
- `cargo test --test architecture_tests`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- `LibraryApp` cannot host frame nav state without splitting into
  multiple frames now (escalate before doing the split).
- Pushing/popping nav state introduces visible UI regressions.
