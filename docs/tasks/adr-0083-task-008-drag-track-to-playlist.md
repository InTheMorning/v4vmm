# ADR 0083 Task 008: Drag A Track To A Playlist

Status: Implemented - 2026-10-21. Cases R83-8-01 to R83-8-15 pass. Mechanical checks are Green. V1 to V9 are open, in the final walk of the run.

## Goal

The operator can drag one track onto a playlist row. The drop adds the track to that playlist through the same path as the "Add to playlist" menu item. The drag is a second path. The row menu and the action bar stay the only paths for the keyboard.

The drag can start on each page that has a drag source. While the operator holds the track over the sidebar row "Playlists" for a short time, the list column opens Playlists. Then the track drops on a playlist row there.

## Authority

- [ADR 0083](../adr/0083-design-language.md) Decision 6, as amended on 2026-10-20: while a track is held over the sidebar destination "Playlists", the list column opens Playlists, and the track drops on a playlist row there. Each action is reachable with the mouse.
- [ADR 0084](../adr/0084-music-column-browser.md) Decision 2: the sidebar lists the six destinations. The Playlists destination lists its playlists in the list column.
- [ADR 0084](../adr/0084-music-column-browser.md) Decision 9: one navigation stack. A sidebar click is a navigation and starts a new path. A path can start at Playlists and go into a release page, for example "Playlists › 000 › Tiddies In My Face (Single)".
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
- A click on a destination row calls `open_page(FrameNavigationEntry::Destination(destination), NavigationPathStart::NewPath, cx)`. The `Destination` entry fills the list column and clears the detail column.
- `src/ui/tokens.rs` has no motion or time token today.
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

- **D1 The drop targets.** Operator decision of 2026-10-20. The sidebar row "Playlists" is a spring-loaded destination, as ADR 0083 Decision 6 decides.
  - While a dragged track stays over the sidebar row "Playlists" for the delay `Motion::SpringLoadDelay`, the list column opens the Playlists destination.
  - The delay is 500 ms. This value is near the default spring-loading delay of the macOS Finder.
  - The pointer can leave the row before the delay ends. Then nothing opens, and the timer stops.
  - Only the "Playlists" row is spring-loaded. The other destination rows do nothing during a drag.
  - The spring-load works on the expanded row and on the collapsed icon row.
  - The sidebar row "Playlists" accepts no drop. A drop on it does nothing and opens no chooser.
  - Each row of the Playlists list in the list column is a drop target. The track drops there.
  - The spring-load open is one navigation, the same as a click on the sidebar row "Playlists". It starts a new path, so the detail column clears. ADR 0084 Decision 9 makes a sidebar change a navigation.
  - The drag continues after the detail column clears.
  - After the drop, the list column stays on Playlists. Back returns to the location before the spring-load, for example the release page under Releases.
  - When the list column already shows Playlists, the spring-load does nothing.
- **D2 The drag sources.**
  - A `TrackRow` of a Library track in `Normal` mode: the release page of the Library origin and the singles group page.
  - A playlist page row, through its present drag handle. A drop on another playlist row adds a copy of the track to that playlist. The source entry stays.
  - A Tracks list row of a Library track in `Normal` mode (D4).
  - The drag starts on each page that has one of these sources. For example, a drag from a release page under Releases opens Playlists through the D1 spring-load.
- **D3 No drag on an Index row.** **Operator-visible, no ADR decides it.** An Index `TrackRow` has no drag in this packet. Its add path is in `TopApp`, and the drop target is in `LibraryApp`. Its "Add to playlist" menu item stays its path.
- **D4 The list column track rows.** With the spring-loaded sidebar row, a target can now show for a list column row.
  - A Tracks list row of a Library track in `Normal` mode is a drag source. The D1 spring-load replaces the Tracks list with the Playlists list. The drag continues.
  - A Needs attention row has no drag. Its menu has no "Add to playlist" item (task 007 D10), so a drag would give it an action that its row does not have.
- **D5 One track for each drag.** A drag carries one track. In `Selecting`, no row starts a drag, the same as packet 025. The action bar "Add n to playlist" adds a selection.
- **D6 One add path.** A drop calls `add_tracks_to_playlist(vec![track_id], playlist_id, cx)`. A drop and the menu item give the same result, also for a duplicate.
- **D7 The own playlist.** A playlist entry on the row of its own playlist is not accepted. The row shows the no-drop cursor and no highlight. A drop does nothing.
- **D8 Drag-over feedback.** An accepting playlist row shows a 2 px outline in `SemanticColor::Accent` while the pointer is on it. The row does not move and its text does not change.
- **D9 The preview.** One preview composite shows the track title, with the present style of `PlaylistTrackDragPreview`. Both sources use it. The view model gives the title.
- **D10 Feedback on drop.** The drop causes no navigation. Only the D1 spring-load navigates, before the drop. The present add path gives the feedback:
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
- Add the same drag fact to `ContentListRowDisplay`. It is `Some` only for a Tracks list row of a Library track in `Normal`. The track id comes from the `TrackKey` of `content_list_track_row_key`. It is `None` for a Needs attention row, for a row with no key and in `Selecting`.
- Add one pure function, for example `playlist_accepts_drop(source_playlist: Option<i64>, target_playlist: i64) -> bool`. It gives false only when the source playlist is the target playlist.
- The names are a proposal. Keep the meaning if a name collides.

### The spring-load state

- Add one pure state type for the D1 timer, for example `SpringLoadVm` in `src/view_models/library.rs`. It holds no GPUI type and reads no clock. Each call gets the time as an argument.
  - `enter(destination, active_destination, now)` starts the timer only when `destination` is `Playlists` and `active_destination` is not `Playlists`. It gives a new timer generation.
  - `leave()` and `end_drag()` stop the timer.
  - `fire(generation, now, delay)` gives `Some(MusicDestination::Playlists)` one time, only when the generation is the current one and `now` is at or after the start plus `delay`. Then the state stops.
- The names are a proposal. Keep the meaning if a name collides.

### The delay token

- Add a named motion token to `src/ui/tokens.rs`, for example `Motion::SpringLoadDelay`, with a `duration()` of 500 ms.
- The spring-load code reads the delay only from this token. No raw time literal is in the spring-load code.

### The payloads and the preview

- Add `src/ui/composites/track_drag.rs`. It holds:
  - `TrackDragPayload { track_id: i64, title: SharedString }` for a `TrackRow` drag.
  - `TrackDragPreview`, which replaces `PlaylistTrackDragPreview`. Move the present style. Delete `PlaylistTrackDragPreview`.
- Add `track_id: i64` to `PlaylistTrackDragPayload`. `render_playlist_leading_slot` fills it from the row track.

### The drag source

- `TrackRow` gets `.on_drag(TrackDragPayload, ...)` on its row root only when the view model gives `drag`.
- Stop the propagation of the left mouse-down only if the root starts a text selection without it. The click handler of the row stays.
- `render_content_list_row` attaches `.on_drag(TrackDragPayload, ...)` to a Tracks list row only when its drag fact is `Some`. Use the same payload and the same preview. If `ListRow` cannot take the drag, wrap the row in one `div` that takes it.

### The spring-loaded sidebar row

- `render_destination_row` in `src/ui/shells/library/sidebar.rs` gives the "Playlists" row a drag-move handler for `TrackDragPayload` and for `PlaylistTrackDragPayload`, for example `.on_drag_move(`. The handler compares the pointer position with the row bounds.
  - The pointer comes into the row: the handler calls one `LibraryApp` method that calls `SpringLoadVm::enter`.
  - The pointer goes out of the row: the handler calls one `LibraryApp` method that calls `SpringLoadVm::leave`.
- The "Playlists" row attaches no `.on_drop(`. A drop on it does nothing.
- `LibraryApp` (`src/library/app_impl.rs`) owns the timer, not the sidebar renderer. It starts one timer for each generation, with the delay of `Motion::SpringLoadDelay`. Use the GPUI timer pattern of `src/app/bootstrap.rs`.
- When the timer ends, `LibraryApp` calls `SpringLoadVm::fire`. On `Some`, it calls `open_page(FrameNavigationEntry::Destination(MusicDestination::Playlists), NavigationPathStart::NewPath, cx)`, the same call as a sidebar click.
- The end of each drag, with a drop or with no drop, calls `SpringLoadVm::end_drag`.

### The drop target

- `render_library_playlists_list` gives each playlist row these handlers:
  - `.can_drop(` that accepts `TrackDragPayload`, and `PlaylistTrackDragPayload` when `playlist_accepts_drop` is true.
  - `.drag_over(` for each payload type, with the D8 outline.
  - `.on_drop(` for each payload type. Each one calls `LibraryApp::add_tracks_to_playlist(vec![payload.track_id], playlist_id, cx)`.
- If `ListRow` cannot take the drop handlers, wrap the row in one `div` that takes them. Do not copy `ListRow`.

## Required Changes

1. Add `track_drag.rs` with the payload and the preview. Move the preview style from `playlist.rs`.
2. Add `track_id` to `PlaylistTrackDragPayload`.
3. Add the drag fact to `ReleaseTrackRowVm` and to `ContentListRowDisplay`, and add `playlist_accepts_drop`.
4. Attach the drag to `TrackRow` and to the Tracks list row when the drag fact is `Some`.
5. Attach the drop handlers to each row of `render_library_playlists_list`.
6. Add `Motion::SpringLoadDelay` to `src/ui/tokens.rs`, and `SpringLoadVm`.
7. Attach the drag-move handler to the sidebar row "Playlists". Add the timer and the spring-load methods to `LibraryApp`.
8. Update the guards in "Recorded Facts" with the same meaning. Add the tests and the guards of this packet.

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
| R83-8-11 | `ContentListRowDisplay.drag` is `Some` with the track id and the title for a Tracks list row of a Library track in `Normal`. It is `None` for a Needs attention row, for a row with no track key and for each row in `Selecting` |
| R83-8-12 | `SpringLoadVm` with recorded times: `enter` on `Playlists`, then `fire` before the delay gives `None`. `fire` at the delay gives `Some(Playlists)` one time only. `enter`, then `leave` or `end_drag`, then `fire` gives `None`. A stale generation gives `None`. `enter` on another destination, or when the active destination is `Playlists`, starts no timer |
| R83-8-13 | A source guard: `Motion::SpringLoadDelay` is in `src/ui/tokens.rs`, and its value is 500 ms. The spring-load code in `src/library/app_impl.rs` and `src/ui/shells/library/sidebar.rs` reads the delay from the token. It holds no `Duration::from_millis(` or `Duration::from_secs(` literal |
| R83-8-14 | A navigation test at the owning layer: the spring-load open is one `Destination(Playlists)` entry with `NavigationPathStart::NewPath`. After it, Back gives the location before the spring-load |
| R83-8-15 | A source guard: `src/ui/shells/library/sidebar.rs` has no `.on_drop(`. Its drag-move handler calls only the `LibraryApp` spring-load methods |

## Visual Acceptance Criteria

These are for the operator. No test proves them.

- V1: open a Library release page under Releases. Drag a track row. The preview shows the track title.
- V1a: hold the track over the sidebar row "Playlists" for about half a second. The list column opens Playlists. Move the pointer onto a playlist row and drop the track there.
- V1b: move the track over the sidebar row "Playlists" and off it quickly. Nothing opens.
- V1c: drop the track on the sidebar row "Playlists" before it opens. Nothing happens, and no chooser opens.
- V1d: after the drop, the list column stays on Playlists. Back returns to the release page under Releases.
- V1e: in the Tracks list, drag a Library track row. The spring-load and the drop work the same. In the Needs attention list, no row starts a drag.
- V1f: collapse the sidebar. The spring-load works on the "Playlists" icon row.
- V2: the pointer on another playlist row shows the accent outline. The pointer off the row removes it.
- V3: a drop adds the track. The status at the bottom of the sidebar shows the result. The track count of the row changes with no navigation.
- V4: drag a playlist entry by its handle onto another playlist row. The track is added there. The source playlist does not change.
- V5: the same entry on the row of its own playlist shows the no-drop cursor and no outline.
- V6: a click on a track row with no move still opens the track page.
- V7: in selection mode, no row starts a drag.
- V8: an Index release page row starts no drag. Its "Add to playlist" menu item still adds the track.
- V9: Light and Dark themes. The outline and the preview are clear in both.

## Exclusions

- A row for each playlist in the sidebar.
- A drag from an Index row (D3), and from a Needs attention row (D4).
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
- `src/ui/shells/library/sidebar.rs` (`render_destination_row`): edited for the D1 spring-load.
- `src/ui/shells/library/content_list.rs` (`render_content_list_row`): edited for the D4 drag source.
- `src/ui/tokens.rs`: edited for `Motion::SpringLoadDelay`.
- `src/app/bootstrap.rs`: read only, for the GPUI timer pattern.
- `src/view_models/workspace/`: read only, for the navigation stack and Back.
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

Revert the commit. This packet adds no migration and no stored data. V1a, V1e, V3 and V4 add entries to a playlist. Use a test playlist, and remove the entries after the walk.

## Operator Answers - 2026-10-20

- D1: the operator decided the sidebar "Playlists" row as a spring-loaded target. ADR 0083 Decision 6 records it.
- The delay of 500 ms comes from memory of the macOS Finder default. No source confirms it. The operator walk confirms the value.

## Orchestrator Review - 2026-10-21

Pass. The code needs no fix.

- Known limit: a drag that ends on the "Playlists" row before the delay leaves its timer armed, because that row accepts no drop. If a new drag comes onto the row before the old timer ends, the list column can open earlier than the delay. GPUI 0.3.1 gives no event when a drag ends with no drop, so the orchestrator accepts this.
- Known limit: GPUI 0.3.1 shows the cursor of the drag source during each drag, so V5 cannot show a no-drop cursor. The reorder drag has the same limit. The operator decides V5 in the walk.
- `src/ui/shells/library/destination_lists.rs` has 500 code lines, the ADR 0038 ceiling. The next change to it must remove lines first.
- The orchestrator added the two new files to the source map.

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
- Make each change in "Required Changes": the payloads, the preview, the drag facts, the drag sources, the drop targets, the delay token and the spring-loaded sidebar row.

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- Follow each decision in "Orchestrator Decisions". Do not change a decision.
- The view model gives the drag fact, the drop rule and the spring-load state. The renderer only composes.
- `LibraryApp` owns the spring-load timer. No screen renderer starts a timer or calls `cx.spawn`.
- Use named tokens. Never show a state by color only.
- Tests use recorded values. No test sends a request or writes a file.
- Never run `git checkout`, `git restore`, `git stash`, `git reset`, `git add` or `git commit`.
- Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- The sidebar destination rows in `src/ui/shells/library/sidebar.rs`, except the drag-move handler of the "Playlists" row.
- The reorder logic of `render_playlist_track_row`, except the new `track_id` field of the payload.
- The selection model, the action bar and the download queue rules.
- The add path functions in `src/library/app_impl.rs`, except to call them.
- The database schema and the migration registry.
- Any ADR, and each document other than this packet.

Acceptance criteria:
- Each case R83-8-01 to R83-8-15 has a passing test or guard.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.
- The packet has an "Implementation Result - 2026-10-20" section and an "Operator Visual Check" section for V1 to V9, with V1a to V1f.

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
- GPUI gives no drag-move event or no row bounds for the sidebar row during a drag.
- The drag stops when the spring-load clears the detail column or replaces the Tracks list.
- A guard forbids a timer in `LibraryApp`.
- A change needs a file in "Do not touch".

## Implementation Result - 2026-10-20

### Files Changed

- `src/ui/tokens.rs`: new token `Motion::SpringLoadDelay`. Its `duration()` is 500 ms.
- `src/view_models/library.rs`:
  - New `TrackDragVm`, `playlist_accepts_drop`, `destination_is_spring_loaded` and `spring_load_navigation`.
  - New `SpringLoadVm` and `SpringLoadGeneration`. The state holds no GPUI type and reads no clock.
  - `DestinationRowVm` has the new fact `spring_loaded`. It is true only for "Playlists".
  - `ContentListRowDisplay` has the new field `drag`. `content_list_rows_with_selection` fills it.
  - `PlaylistTrackRowDisplay` has the new field `track_id`.
  - New tests R83-8-02, R83-8-11, R83-8-12 and R83-8-14.
- `src/view_models/release_page.rs`: `ReleaseTrackRowVm` has the new field `drag`. New test R83-8-01.
- `src/ui/composites/track_drag.rs` (new):
  - `TrackDragPayload`, `PlaylistTrackDragPayload` with the new field `track_id`, and `TrackDragPreview`.
  - `attach_track_drag` attaches the drag and stops the left mouse-down.
  - `playlist_drop_target`, `PlaylistDrop`, `PlaylistDropHandler` and `playlist_drop_handler`.
  - GPUI tests R83-8-03, R83-8-05 and R83-8-06. A second R83-8-03 test drags after the source row unmounts.
- `src/ui/composites/track_row.rs`: new builder `TrackRow::drag`. The body of the row takes the drag.
- `src/ui/composites/list_row.rs`: new builder `ListRow::drag`. The row root takes the drag.
- `src/ui/composites/mod.rs`: registers `track_drag`.
- `src/ui/shells/playlist.rs`:
  - `PlaylistTrackDragPayload` and `PlaylistTrackDragPreview` are deleted from this file. The handle fills `track_id` and shows `TrackDragPreview`.
  - New GPUI test R83-8-04.
- `src/ui/shells/release_page.rs`: the release page row and the singles group row give `drag` to `TrackRow`.
- `src/ui/shells/library/content_list.rs`: a Tracks list row gives `drag` to `ListRow`.
- `src/ui/shells/library/destination_lists.rs`: each row of the Playlists list is a `playlist_drop_target`. Its handler calls `add_tracks_to_playlist`.
- `src/ui/shells/library/sidebar.rs`: a row with `spring_loaded` attaches two `.on_drag_move(` handlers. They call only `LibraryApp::spring_load_pointer`.
- `src/presentation/delayed_presenter.rs` (new): `present_after` calls a function on the entity when a delay ends.
- `src/presentation/mod.rs`: registers `delayed_presenter`.
- `src/library.rs` and `src/library/app_impl.rs`:
  - New field `spring_load`.
  - New `spring_load_pointer`, `fire_spring_load` and `end_track_drag`.
- `tests/architecture_tests.rs`: new situational guards R83-8-07, R83-8-08, R83-8-09, R83-8-13 and R83-8-15. Each one cites ADR 0083 Decision 6.

### Behavior Changed

- A Library track row on a release page or a singles group page starts a drag when the pointer moves on the row body. The preview shows the title.
- A Tracks list row of a Library track starts the same drag. A Needs attention row, an Index row and each row in selection mode start no drag.
- The drag handle of a playlist entry starts its reorder drag as before. The payload holds the track id too.
- The pointer of a drag stays on the sidebar row "Playlists" for 500 ms. Then the list column opens Playlists on a new path.
  - When the pointer leaves the row first, nothing opens.
  - When the list column shows Playlists, nothing occurs.
  - A drop on the sidebar row does nothing.
- Each row of the Playlists list accepts a track drag, and an entry of another playlist. An accepting row shows a 2 px accent outline under the pointer.
- A drop calls `add_tracks_to_playlist` with one track. The status text and the track count change as for the menu item.
- A click with no move opens the page as before. A drag opens no page and starts no text selection.

### Checks - 2026-10-20

| Check | Result |
|---|---|
| `cargo test --lib adr_0083_track_drag_` | Green, 10 tests |
| `cargo test --lib adr_0083_row_actions_` | Green, 13 tests |
| `cargo test --lib adr_0083_list_row_actions_` | Green, 12 tests |
| `cargo test --lib adr_0044_` | Green, 2 tests |
| `cargo test --lib adr_0084_playlist_selection_` | Green, 12 tests |
| `cargo test` | Green, 2104 unit tests and 355 guards. Ten documentation examples stay ignored |
| `cargo test --test architecture_tests` | Green, 355 guards |
| `cargo fmt -- --check` | Green |
| `cargo clippy -- -D warnings` | Green |
| `cargo clippy --all-targets -- -D warnings` | Green |
| `cargo check --all-targets` | Green, no warning |
| `cargo build --bin v4vmm` | Green |

No application launch and no production-data change occurred.

### Deviations From The Task

1. The drop target and its two `.on_drop(` handlers are in `src/ui/composites/track_drag.rs`, not in `destination_lists.rs`.
   The guard `surface_modules_under_500_loc` (ADR 0038) limits `destination_lists.rs` to 500 code lines. The file has 500 code lines now.
   `destination_lists.rs` gives the handler that calls `add_tracks_to_playlist`. Guard R83-8-07 reads both files.
2. `PlaylistTrackDragPayload` is in `track_drag.rs`, because the playlist page and the drop target use it. It has one definition.
3. The `TrackRow` drag attaches to the clickable body, not to the row root.
   GPUI clears the pending click only on the element that starts the drag. On the root, a drag back onto its own row also opened the page.
   The download cell, the duration and the menu cell start no drag.
4. The Tracks list row takes the drag through the new `ListRow::drag` on its root, which holds the click. No wrapper element is necessary.
5. The guard `cx_spawn_is_restricted_to_presentation_runtime_and_bootstrap` (ADR 0040) forbids `cx.spawn(` in `app_impl.rs`.
   The new `present_after` in `src/presentation/` holds the timer. `LibraryApp` owns the state and starts each timer through it.
6. The view model gives the new fact `DestinationRowVm::spring_loaded`. The sidebar does not decide which row is spring-loaded.
7. The row of its own playlist does not accept an entry: `.can_drop(` gives false. The row shows no outline, and a drop calls no handler.
8. A drag can end with no drop. The timer then finds no active drag and stops the state.

### Unresolved Concerns

1. V5 cannot show the no-drop cursor. GPUI 0.3.1 sets the window cursor to the cursor of the drag source during each drag.
   The `.cursor_no_drop()` of the present reorder target has the same limit. The operator decides if V5 needs a change.
2. `src/ui/shells/library/destination_lists.rs` is at the limit of 500 code lines. The next change to it must remove lines first.
3. A drag can end on the sidebar row "Playlists" before the delay ends. A new drag can then come onto the row before the old timer ends.
   In this case, the list column opens earlier than 500 ms after the new drag came onto the row.
4. The new files `src/ui/composites/track_drag.rs` and `src/presentation/delayed_presenter.rs` are not in `docs/architecture/source-map.md`.
   The checks V1 to V9 are not in `docs/pending-human-checks.md`. The task instructions named only this packet as a document to edit.
5. D3 needs operator confirmation. The operator walk confirms the delay of 500 ms.

## Operator Visual Check

This check needs a Linux desktop session and this checkout. It also needs these items:

- A Library release with two or more tracks.
- An Index release that is not in the Library.
- Two test playlists. One test playlist holds two or more entries.

V1a, V1e, V3 and V4 add entries to a playlist. Use only the test playlists.

1. Build the app: `cargo build --bin v4vmm`. Then start `target/debug/v4vmm` from a desktop session.
2. V1: open Music, then Releases, then the Library release. Push and hold the mouse button on the title of a track row. Move the pointer.
   - Correct: a preview with the track title follows the pointer.
   - Wrong: the track page opens, or text becomes selected.
3. V1a: hold the track over the sidebar row "Playlists" for about half a second.
   - Correct: the list column shows Playlists, and the detail column becomes empty. The preview stays at the pointer.
   - Move the pointer onto the first test playlist and release the button.
4. V3: look at the bottom of the sidebar and at the row of the first test playlist.
   - Correct: the status shows "Downloading 1 track..." and then "Added 1 of 1 to {playlist}". The track count increases by one.
   - Wrong: a navigation occurs after the drop.
5. V1d: look at the list column, then click Back.
   - Correct: the list column stays on Playlists after the drop. Back gives the release page under Releases.
6. V1b: drag a track row over the sidebar row "Playlists" and off it quickly.
   - Correct: nothing opens.
7. V1c: drag a track row onto the sidebar row "Playlists" and release the button at once.
   - Correct: nothing occurs, and no chooser opens.
8. V2: drag a track row to Playlists again. Move the pointer onto a playlist row, then off it.
   - Correct: the row shows an accent outline while the pointer is on it. The outline goes when the pointer goes. The row does not move.
9. V1e: open Tracks. Drag a row of a Library track to the sidebar row "Playlists", then drop it on a test playlist.
   - Correct: the result is the same as in steps 3 and 4.
   - Open Needs attention and try to drag a row. Correct: no drag starts.
10. V1f: collapse the sidebar. Repeat step 3 on the "Playlists" icon row.
    - Correct: the list column opens Playlists.
11. V4: open the test playlist with two or more entries. Drag an entry by its handle onto the row of the second test playlist.
    - Correct: the second test playlist gets the track. The source playlist does not change.
12. V5: drag the same entry onto the row of its own playlist.
    - Correct: the row shows no outline, and a drop does nothing. See concern 1 for the cursor.
13. V6: click a track row on the release page with no pointer movement.
    - Correct: the track page opens.
14. V7: click "Select" on the release page, then try to drag a row. Do the same in Tracks. Click "Done".
    - Correct: no drag starts in selection mode.
15. V8: open the Index release. Try to drag a track row. Then select "Add to playlist" in the row menu.
    - Correct: no drag starts. The menu item adds the track.
16. V9: change to the Light theme and to the Dark theme. Repeat step 8.
    - Wrong: an outline or a preview that you cannot see clearly.

### Cleanup

1. Open each test playlist. Select "Remove from playlist" in the menu of each entry that the walk added.
2. If the walk added a track that was not downloaded, remove it from the Library with "Remove…" in its row menu.
3. Close the app.
