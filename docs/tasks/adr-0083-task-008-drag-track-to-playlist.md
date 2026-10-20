# ADR 0083 Task 008: Drag A Track To A Playlist

Status: Ready - 2026-10-20. It starts after task 006 is implemented. D1 needs operator confirmation before the visual walk closes.

## Goal

The operator can drag one track onto a playlist row. The drop adds the track to that playlist through the same path as the "Add to playlist" menu item. The drag is a second path. The row menu and the action bar stay the only paths for the keyboard.

## Authority

- [ADR 0083](../adr/0083-design-language.md) Decision 6: "A track can be dragged onto a playlist in the sidebar." Each action is reachable with the mouse.
- [ADR 0084](../adr/0084-music-column-browser.md) Decision 2: the sidebar lists the six destinations. The Playlists destination lists its playlists in the list column.
- [ADR 0084](../adr/0084-music-column-browser.md) Decision 9: one navigation stack. A path can start at Playlists and go into a release page, for example "Playlists › 000 › Tiddies In My Face (Single)".
- [ADR 0084](../adr/0084-music-column-browser.md) Decision 10: no step needs a drag. The action bar holds "Add n to playlist".
- The archived [ADR 0044](../adr/archive/0044-playlist-drag-handle-reordering.md): the reorder drag handle, and the menu items that do the same work with no drag.
- Packet [006](adr-0083-task-006-row-hover-actions.md): the `TrackRow` composite and its "Add to playlist" item. Its decision D15 gives the drag to this packet.
- Packet [025](adr-0084-task-025-playlist-selection.md): in selection mode, the playlist row has no drag handle and no drop target.
- AGENTS.md "Current-view state must update in place".

## Recorded Facts - 2026-10-20

### The present drag

- `PlaylistTrackDragPayload` in `src/ui/shells/playlist.rs` holds `playlist_id`, `from_position` and `title`. It holds no track id.
- `render_playlist_drag_handle` attaches the one `.on_drag(` of the file. The preview is `PlaylistTrackDragPreview`: the title in bold `FontSize::Micro` text, on `color::bg_surface()`, with a `color::accent()` border.
- The handle stops the propagation of the left mouse-down, so the root does not start a text selection (ADR 0044). `adr_0044_playlist_drag_does_not_start_text_selection` proves it.
- `render_playlist_track_row` attaches `.can_drop(`, `.drag_over(` and `.on_drop(` for `PlaylistTrackDragPayload` only inside `if let (Some(on_reorder), true) = (on_reorder, reorder_visible)`.
  - A payload of another playlist gets the no-drop cursor, a muted border and a line through the text.
  - A payload of the same playlist shows an accent top or bottom border, then calls `on_reorder`.
- `playlist_reorder_display_contract_uses_drag_handle_and_menu_fallbacks` in `tests/architecture_tests.rs` requires exactly one `.on_drag(` in `src/ui/shells/playlist.rs`.
- No other file in `src/ui` calls `.on_drag(`. `TrackRow` has no drag.

### Where the playlists are

- The sidebar (`render_destination_row` in `src/ui/shells/library/sidebar.rs`) draws one row for each destination. It draws no row for each playlist.
- `LibraryApp::render_list_column` (`src/library/app_impl.rs`) draws the list of `self.vm.active_destination()`. For `MusicDestination::Playlists`, it calls `render_library_playlists_list`.
- `render_library_playlists_list` in `src/ui/shells/library/destination_lists.rs` draws one `ListRow` for each `PlaylistSidebarRowVm`. A click opens `FrameNavigationEntry::PlaylistDetail`. No row accepts a drop.
- `PlaylistSidebarRowVm` (`src/view_models/library.rs`) has `id`, `element_id`, `name`, `track_count_label`, `selected` and `art`.
- The list column shows the root destination of the path. The detail column shows the page at the end of the path. So the Playlists list and a release page show at the same time only on a path that starts at Playlists.

### The add path

- `LibraryApp::add_tracks_to_playlist(track_ids, playlist_id, cx)` and `add_track_to_playlist` call `self.vm.begin_playlist_append`, then `spawn_subscribe_then_append`.
- `begin_playlist_append` sets the status text to "Downloading n track(s)...".
- `SubscribeThenAppendToPlaylist` downloads each track that is not downloaded, then appends it.
- On success, `finish_playlist_append` sets the status to "Added a of n to {playlist}", and `reload_playlists` reloads the playlist rows. On failure, `fail_playlist_append` sets "Error adding to playlist: …".
- The status text shows at the bottom of the sidebar, from `self.vm.status_snapshot()`.
- `db::playlist_append` appends with no check for a duplicate. The "Add to playlist" menu item adds a duplicate entry today.
- The Index route `TopApp::add_index_track_to_playlist` (`src/app/search_dispatch.rs`) first calls `ensure_feed_in_db` and `find_track_id`. `LibraryApp` cannot call it.

## Orchestrator Decisions

The orchestrator made these decisions on 2026-10-20. The operator can audit each one. A decision with the mark **Operator-visible, no ADR decides it** changes what the operator sees in a way that no ADR decides. The operator must confirm it before the visual walk closes.

- **D1 The drop targets.** **Operator-visible, no ADR decides it.** Each row of the Playlists list in the list column is a drop target. The sidebar destination row "Playlists" is not a drop target.
  - ADR 0083 Decision 6 says "a playlist in the sidebar". ADR 0084 Decision 2 moved the playlists from the sidebar to the Playlists list. This packet reads "the sidebar" as that list.
  - The result: a drag has a target only on a path that starts at Playlists. A drag on the Releases, Artists or Tracks paths has no visible target.
  - A drop on the destination row "Playlists" names no playlist, so it has no single result.
  - The other choice is an ADR 0084 amendment that lists each playlist under the sidebar destination "Playlists". A later packet can then make those rows drop targets with the same code. The operator decides between the two choices.
- **D2 The drag sources.**
  - A `TrackRow` of a Library track in `Normal` mode: the release page of the Library origin and the singles group page.
  - A playlist page row, through its present drag handle. A drop on another playlist row adds a copy of the track to that playlist. The source entry stays.
- **D3 No drag on an Index row.** **Operator-visible, no ADR decides it.** An Index `TrackRow` has no drag in this packet. Its add path is in `TopApp`, and the drop target is in `LibraryApp`. Its "Add to playlist" menu item stays its path.
- **D4 No drag on a list column track row.** The Tracks and Needs attention rows have no drag. They are in the list column, so the Playlists list cannot show at the same time.
- **D5 One track for each drag.** A drag carries one track. In `Selecting`, no row starts a drag, the same as packet 025. The action bar "Add n to playlist" adds a selection.
- **D6 One add path.** A drop calls `add_tracks_to_playlist(vec![track_id], playlist_id, cx)`. A drop and the menu item give the same result, also for a duplicate.
- **D7 The own playlist.** A playlist entry on the row of its own playlist is not accepted. The row shows the no-drop cursor and no highlight. A drop does nothing.
- **D8 Drag-over feedback.** An accepting playlist row shows a 2 px outline in `SemanticColor::Accent` while the pointer is on it. The row does not move and its text does not change.
- **D9 The preview.** One preview composite shows the track title, with the present style of `PlaylistTrackDragPreview`. Both sources use it. The view model gives the title.
- **D10 Feedback on drop.** No navigation occurs. The present add path gives the feedback:
  - The status at the bottom of the sidebar shows "Downloading 1 track...", then "Added 1 of 1 to {playlist}".
  - `reload_playlists` changes the track count of the target row in place.
  - A failure shows "Error adding to playlist: …" in the same status text.
- **D11 Accessibility.** The drag is for the mouse only. It adds no accessibility label and no tooltip. The keyboard path is "Add to playlist" in the row menu, and "Add n to playlist" in the action bar (ADR 0084 Decision 10).
- **D12 Click and drag.** A click on a `TrackRow` opens the page, as today. A drag starts only after the pointer moves. A drag from a `TrackRow` does not start a text selection.
- **D13 The reorder drag.** The reorder drag on the playlist page does not change. Its drop target on the playlist rows does not accept the new track payload.

## Design

### The view model

- Add a drag fact to `ReleaseTrackRowVm`, for example `drag: Option<TrackDragVm>`, with `track_id: i64` and `title: String`.
  - `Some` only for a Library row in `Normal`.
  - `None` for an Index row and in `Selecting`.
- Add one pure function, for example `playlist_accepts_drop(source_playlist: Option<i64>, target_playlist: i64) -> bool`. It gives false only when the source playlist is the target playlist.
- The names are a proposal. Keep the meaning if a name collides.

### The payloads and the preview

- Add `src/ui/composites/track_drag.rs`. It holds:
  - `TrackDragPayload { track_id: i64, title: SharedString }` for a `TrackRow` drag.
  - `TrackDragPreview`, which replaces `PlaylistTrackDragPreview`. Move the present style. Delete `PlaylistTrackDragPreview`.
- Add `track_id: i64` to `PlaylistTrackDragPayload`. `render_playlist_leading_slot` fills it from the row track.

### The drag source

- `TrackRow` gets `.on_drag(TrackDragPayload, ...)` on its row root only when the view model gives `drag`.
- Stop the propagation of the left mouse-down only if the root starts a text selection without it. The click handler of the row stays.

### The drop target

- `render_library_playlists_list` gives each playlist row these handlers:
  - `.can_drop(` that accepts `TrackDragPayload`, and `PlaylistTrackDragPayload` when `playlist_accepts_drop` is true.
  - `.drag_over(` for each payload type, with the D8 outline.
  - `.on_drop(` for each payload type. Each one calls `LibraryApp::add_tracks_to_playlist(vec![payload.track_id], playlist_id, cx)`.
- If `ListRow` cannot take the drop handlers, wrap the row in one `div` that takes them. Do not copy `ListRow`.

## Required Changes

1. Add `track_drag.rs` with the payload and the preview. Move the preview style from `playlist.rs`.
2. Add `track_id` to `PlaylistTrackDragPayload`.
3. Add the drag fact to `ReleaseTrackRowVm` and `playlist_accepts_drop`.
4. Attach the drag to `TrackRow` when the drag fact is `Some`.
5. Attach the drop handlers to each row of `render_library_playlists_list`.
6. Update the guards in "Recorded Facts" with the same meaning. Add the tests and the guards of this packet.

## Mechanical Acceptance Criteria

Use the prefix `adr_0083_track_drag_` for unit tests beside the owning code. Each new guard cites ADR 0083 Decision 6 and is situational.

| Case | Required proof |
|---|---|
| R83-8-01 | `ReleaseTrackRowVm.drag` is `Some` with the track id and the title for a Library row in `Normal`. It is `None` for an Index row and for each row in `Selecting` |
| R83-8-02 | `playlist_accepts_drop` gives false for an entry of the same playlist, and true for another playlist and for a `TrackRow` source |
| R83-8-03 | A GPUI test: a drag from a `TrackRow` onto a playlist row calls the drop handler one time. The call holds the track id and the playlist id |
| R83-8-04 | A GPUI test: a drag from a playlist handle onto the row of another playlist calls the drop handler with the track id. A drop onto the row of its own playlist calls no handler |
| R83-8-05 | A GPUI test: a drag from a `TrackRow` does not start a text selection. A click with no move still opens the page |
| R83-8-06 | A GPUI test: a `TrackRow` with `drag` `None` starts no drag |
| R83-8-07 | A source guard: each `.on_drop(` in `destination_lists.rs` calls `add_tracks_to_playlist`. No drop handler calls a second add path |
| R83-8-08 | A source guard: `PlaylistTrackDragPreview` is absent from `src/`. `track_drag.rs` holds the one preview |
| R83-8-09 | A source guard: the reorder drop of `render_playlist_track_row` accepts only `PlaylistTrackDragPayload` |
| R83-8-10 | `playlist_reorder_display_contract_uses_drag_handle_and_menu_fallbacks`, `adr_0084_playlist_selection_r25_11_selection_mode_row_accepts_no_drop` and `adr_0044_playlist_drag_does_not_start_text_selection` keep their meaning. Each packet 006, 007 and 025 test still passes |

## Visual Acceptance Criteria

These are for the operator. No test proves them.

- V1: on a path that starts at Playlists, open a Library release page. Drag a track row. The preview shows the track title.
- V2: the pointer on another playlist row shows the accent outline. The pointer off the row removes it.
- V3: a drop adds the track. The status at the bottom of the sidebar shows the result. The track count of the row changes with no navigation.
- V4: drag a playlist entry by its handle onto another playlist row. The track is added there. The source playlist does not change.
- V5: the same entry on the row of its own playlist shows the no-drop cursor and no outline.
- V6: a click on a track row with no move still opens the track page.
- V7: in selection mode, no row starts a drag.
- V8: an Index release page row starts no drag. Its "Add to playlist" menu item still adds the track.
- V9: Light and Dark themes. The outline and the preview are clear in both.

## Exclusions

- A drop target in the sidebar, and a row for each playlist in the sidebar (D1).
- A drag from an Index row (D3), and from a Tracks or Needs attention row (D4).
- A drag of a selection, of a release or of a singles group (D5).
- A drop on the playlist page to add a track.
- A check for a duplicate entry. The menu and the drop keep the present behavior.
- No change to the reorder drag, the selection model, the action bar or the download queue.
- No schema change.

## Files To Inspect

- [Agent rules](../../AGENTS.md) and the [source map](../architecture/source-map.md).
- `src/ui/shells/playlist.rs` (`PlaylistTrackDragPayload`, `PlaylistTrackDragPreview`, `render_playlist_drag_handle`, `render_playlist_track_row` and the ADR 0044 drag test).
- `src/ui/shells/library/destination_lists.rs` (`render_library_playlists_list`), `src/ui/composites/list_row.rs`.
- `src/ui/composites/track_row.rs` and the packet 006 changes.
- `src/view_models/release_page.rs` (`ReleaseTrackRowVm`), `src/view_models/library.rs` (`PlaylistSidebarRowVm`, `begin_playlist_append`, `finish_playlist_append`).
- `src/library/app_impl.rs` (`render_list_column`, `add_tracks_to_playlist`, `spawn_subscribe_then_append`).
- `src/ui/shells/library/sidebar.rs`: read only, to confirm D1.
- `tests/architecture_tests.rs`: the guards in "Recorded Facts".

## Checks

```bash
cargo test --lib adr_0083_track_drag_
cargo test --lib adr_0083_row_actions_
cargo test --lib adr_0044_
cargo test --lib adr_0084_playlist_selection_
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy -- -D warnings
cargo check --all-targets
cargo build --bin v4vmm
```

## Rollback

Revert the commit. This packet adds no migration and no stored data. V3 and V4 add entries to a playlist. Use a test playlist, and remove the entries after the walk.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `docs/architecture/source-map.md`
- This packet: `docs/tasks/adr-0083-task-008-drag-track-to-playlist.md`
- Packet 006: `docs/tasks/adr-0083-task-006-row-hover-actions.md`, and its "Implementation Result" section
- ADR 0083 Decision 6, and ADR 0084 Decisions 2, 9 and 10
- The archived ADR 0044 and packet 025
- Each file in "Files To Inspect"

Goal:
- Make each change in "Required Changes": the payloads, the preview, the drag fact, the drag source and the drop targets.

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- Follow each decision in "Orchestrator Decisions". Do not change a decision.
- The view model gives the drag fact and the drop rule. The renderer only composes.
- Use named tokens. Never show a state by color only.
- Tests use recorded values. No test sends a request or writes a file.
- Never run `git checkout`, `git restore`, `git stash`, `git reset`, `git add` or `git commit`.
- Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- The sidebar destination rows in `src/ui/shells/library/sidebar.rs`.
- The reorder logic of `render_playlist_track_row`, except the new `track_id` field of the payload.
- The selection model, the action bar and the download queue rules.
- The add path functions in `src/library/app_impl.rs`, except to call them.
- The database schema and the migration registry.
- Any ADR, and each document other than this packet.

Acceptance criteria:
- Each case R83-8-01 to R83-8-10 has a passing test or guard.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.
- The packet has an "Implementation Result - 2026-10-20" section and an "Operator Visual Check" section for V1 to V9.

Test commands:
- The commands in "Checks", with `cargo fmt -- --check` after the last edit and `cargo build --bin v4vmm` last.

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

Stop and report the problem, and do not guess, when:
- Task 006 is not implemented.
- One element cannot take `.on_drop(` for two payload types.
- A `TrackRow` drag cancels the row click, or a click starts a drag.
- A `TrackRow` drag starts a text selection, and a stop of the mouse-down propagation breaks a row control.
- The GPUI test context cannot simulate a drag between two elements.
- A change needs a file in "Do not touch".
