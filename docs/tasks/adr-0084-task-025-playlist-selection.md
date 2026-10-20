# ADR 0084 Task 025: Selection On The Playlist Page

Status: Implemented - 2026-10-20. Cases R25-01 to R25-12 pass. Mechanical checks are Green. V1 to V7 are open, in the final walk of the run.

## Goal

The playlist page gets the selection model and the selection mode of packets 009, 018 and 019. In selection mode, the drag handle and the row menu hide, and the checkbox takes their place.
The bar removes, downloads, copies, moves and deletes the selected tracks. The keys move the row focus and scroll the page.

## Authority

- [ADR 0084](../adr/0084-music-column-browser.md) Decision 10, amended on 2026-10-19 and on 2026-10-20: the selection model, the two modes, the bar and the download queue.
- [ADR 0083](../adr/0083-design-language.md) Decision 5: a destructive item is last, ends with "…", and asks for confirmation.
- The archived packets [009](archive/adr-0084-task-009-list-selection-and-action-bar.md), [018](archive/adr-0084-task-018-detail-column-selection.md) and [019](archive/adr-0084-task-019-selection-mode.md), and packet [022](adr-0084-task-022-download-queue.md).
- Operator decisions of 2026-10-20:
  - In selection mode, the drag handle and the row menu hide. Reorder and the row menu come back after "Done".
  - "Remove n from playlist" removes the selected tracks from the playlist. They stay in the Library.
  - The bar also gives "Download n tracks", "Add n to playlist", "Move to top", "Move to bottom" and "Remove from Library…".
- The [run record of 2026-10-20](../reviews/orchestrated-run-2026-10-20.md): the orchestrator split this packet from packet 020.

## Recorded Facts - 2026-10-20

- The playlist table key is `(playlist_id, position)`. Nothing makes `track_id` unique, and `playlist_append` adds a row each time (`src/db.rs`). A playlist can thus hold one track two times.
- Each playlist row is a plain `div()` in `src/ui/shells/playlist.rs` (`render_playlist_track_row`, `render_playlist_row_identity`, `render_playlist_track_controls`). The host is `render_library_playlist_detail` in `src/ui/shells/library/playlist_detail.rs`.
- `PlaylistTrackRowDisplay` and `PlaylistTrackControlsDisplay` (`src/view_models/library.rs`) give the drag handle id and the "Move up", "Move down" and "Remove" menu items. A guard in `tests/architecture_tests.rs` asserts their text.
- `playlist_reorder` and `playlist_remove_track_everywhere` (`src/db.rs`) move one row and remove rows. `ReorderPlaylistTrack` is in `src/application/commands/playlist.rs`. `LibraryApp::move_playlist_track` and `remove_playlist_track_at` call them.
- The full track list of a playlist is in `PlaylistDetail.tracks` (`src/library.rs`). The page draws rows in pages, and a row that waits shows `Pending`.
- The playlist page scrolls through `music_column_scrollbar`, with no held handle. Packet 018 gave it no key scroll.
- A playback session keeps a playlist position (`src/playback.rs`). A one-row move does not change it today.
- `LibraryApp::download_library_tracks` queues Library tracks (packet 022).

## Design

### The key and the state

- The row key is `PlaylistEntryKey { position: i64, track_id: i64 }`, as `DetailRowKey::PlaylistEntry`. The track id is a check: after a reload, `retain_visible` keeps a key only when the same track is still at that position.
- `DetailListKind::for_entry` gives `PlaylistEntries`, with the noun "track", for a playlist page.
- Add `DetailPageRows::PlaylistEntries` and the builder `playlist_entry_rows`, in the visible order of `PlaylistDetailVm::visible_tracks`. The rows come from the full list, so Ctrl+A also selects a row that waits.
- Add the pure `DetailSelection::rekey_after_move(edge, total)`. It moves each selected key and the focus to the new positions.
- `library_detail_page_rows` gets the arm for a playlist.

### The row

- `PlaylistTrackRowDisplay` gets `selection: RowSelectionVm`. `PlaylistTrackControlsDisplay` gets `reorder_visible`, which is true in `Normal` only.
- In `Selecting`, the checkbox takes the leading place of the drag handle, in the same `Size::PointerHitTarget` column, so the row width does not change. The row menu is absent. "Play" stays.
- In `Selecting`, the row accepts no drop.
- The selected fill and the outlines use `row_outline` of `src/ui/composites/list_row.rs`. Do not copy it.
- A body click goes through the detail click of packet 018: a toggle in `Selecting`, the track page in `Normal`.

### The bar

| Order | Label example | Shows when | Effect |
|---|---|---|---|
| 1 | "Remove 3 from playlist" | Rows are selected | Removes the selected entries in one transaction |
| 2 | "Download 2 tracks" | One or more selected tracks are not downloaded. The count names distinct tracks | `download_library_tracks` |
| 3 | "Add 3 to playlist" | Rows are selected. The popover leaves out the open playlist | Adds each track to the chosen playlist |
| 4 | "Move to top" | The selected entries are not already the first entries | Moves them to the top in one transaction |
| 5 | "Move to bottom" | The selected entries are not already the last entries | Moves them to the bottom in one transaction |
| 6 | "Remove from Library…" | One or more selected tracks are downloaded. The count names distinct tracks | The confirmation of packet 009 |
| Last | "Clear" | Rows are selected | Clears the rows and keeps `Selecting` |

- Its accessibility labels name the count and the object, for example "Remove 2 tracks from Library…".
- An action that cannot act is absent.

### The storage

- Add `db::playlist_move_entries(conn, playlist_id, entries, edge)`. In one transaction it reads the rows, checks each position and track id, and moves the entries.
  The selected entries go to the top or the bottom in their present order, and the other entries keep their order. When one check fails, it changes nothing and gives the error "The playlist changed. Select the tracks again."
- Add `db::playlist_remove_entries(conn, playlist_id, entries)` with the same check. It deletes in descending position order and closes the gaps, in one transaction.
- Add the commands `MovePlaylistEntries` and `RemovePlaylistEntries` beside `ReorderPlaylistTrack`, and their service functions. Do not call `playlist_reorder` once for each entry.

### The update in place

- After "Remove n from playlist": reload the playlist, its RSS marks and the readiness report, then clear the rows. The mode stays `Selecting`.
- After a move: reload the playlist, then call `rekey_after_move`. The selection and the focus follow the entries.
- After "Download", "Add" and "Remove from Library…", the selection stays. A reload calls `retain_visible`.

### The keys

- The playlist page holds the detail column scroll handle of packet 018, through `music_column_tracked_scrollbar`.
- Up, Down, Page Up, Page Down, Home and End move the row focus and keep the row in view. Enter opens the track page. Space, Ctrl+A and Escape act as in packet 018.

## Required Changes

1. Add the key, the list kind, the builder and `rekey_after_move`, with tests.
2. Add the two storage functions, their commands and their tests.
3. Change the row view models and the row renderer for the two modes.
4. Add the bar actions and their dispatch.
5. Add the held scroll handle and the keys.
6. Update the guards.

## Mechanical Acceptance Criteria

Use the prefix `adr_0084_playlist_selection_` for unit tests beside the owning code. Each new guard cites ADR 0084 Decision 10.

| Case | Required proof |
|---|---|
| R25-01 | `DetailListKind::for_entry` gives `PlaylistEntries` for a playlist page |
| R25-02 | Two entries of one track have two keys. A toggle of one does not select the other |
| R25-03 | `retain_visible` drops a key whose position now holds a different track |
| R25-04 | The bar gives the six actions and "Clear" in the order of the table. "Download" and "Remove from Library…" count distinct tracks |
| R25-05 | "Move to top" is absent when the selection is already at the top. "Move to bottom" is absent when it is already at the bottom |
| R25-06 | `playlist_move_entries` gives the top order and the bottom order, and keeps the relative order, for a recorded playlist that holds one track two times |
| R25-07 | A stale entry makes the move and the removal fail, and the playlist does not change |
| R25-08 | `rekey_after_move` gives the new keys and the new focus |
| R25-09 | In `Selecting`, `reorder_visible` is false and the checkbox shows. In `Normal`, the opposite |
| R25-10 | The "Add to playlist" options leave out the open playlist |
| R25-11 | A source guard: in selection mode the playlist row attaches no drop handler |
| R25-12 | Each packet 009, 018, 019, 020 and 022 test still passes |

## Visual Acceptance Criteria

These are for the operator, in the final walk of the run. No test proves them.

- V1: "Select" hides the drag handle and the row menu. "Done" shows them again.
- V2: the checkbox takes the place of the drag handle, and the row width does not change.
- V3: each bar action acts on the selection, also for a playlist that holds one track two times.
- V4: "Move to top" and "Move to bottom" keep the order of the selected tracks, and the selection follows them.
- V5: after "Remove n from playlist", the tracks are still in the Library.
- V6: the keys move the row focus and scroll the page.
- V7: Light and Dark themes, a narrow width and the larger type sizes.

## Exclusions

- No update of the playlist position of a playback session after a move or a removal. A one-row move has the same gap today. The orchestrator records it as an open finding.
- No Discover selection. Packet 020 owns it.
- No schema change.

## Files To Inspect

- [Agent rules](../../AGENTS.md) and the [source map](../architecture/source-map.md).
- `src/db.rs` (the playlist table, `playlist_append`, `playlist_reorder`, `playlist_remove_track_everywhere`), `src/playlist_service.rs`, `src/application/commands/playlist.rs`.
- `src/view_models/library.rs` (`PlaylistTrackRowDisplay`, `PlaylistTrackControlsDisplay`, `PlaylistDetailVm`), `src/view_models/detail_selection.rs`, `src/view_models/selection_action_bar.rs`.
- `src/library.rs`, `src/library/app_impl.rs` (`move_playlist_track`, `remove_playlist_track_at`, `select_playlist`), `src/library/detail_selection.rs`, `src/library/downloads.rs`, `src/app/detail_selection.rs`.
- `src/ui/shells/playlist.rs`, `src/ui/shells/library/playlist_detail.rs`, `src/ui/composites/list_row.rs`, `src/ui/composites/selection_action_bar.rs`.
- `tests/architecture_tests.rs`: the playlist row guard, the r14 scroll guards and the packet 018 guards.

## Checks

```bash
cargo test --lib adr_0084_playlist_selection_
cargo test --lib adr_0084_detail_selection_
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy -- -D warnings
cargo check --all-targets
cargo build --bin v4vmm
```

## Rollback

Revert the commit. This packet adds no migration. V3 and V4 change the order and the content of a playlist. Use a test playlist.

## Implementation Result - 2026-10-20

### Files Changed

- `src/db.rs`:
  - New `PlaylistEntryKey { position, track_id }`, `PlaylistEdge` and `PLAYLIST_CHANGED_ERROR`.
  - New `playlist_move_entries` and `playlist_remove_entries`. Each one is one transaction, with the check of each position and track id.
  - Tests R25-06 and R25-07 use a recorded playlist that holds one track two times.
- `src/playlist_service.rs`: new `move_entries` and `remove_entries`.
- `src/application/commands/playlist.rs`: new `MovePlaylistEntries` and `RemovePlaylistEntries` beside `ReorderPlaylistTrack`, with one command test.
- `src/view_models/detail_selection.rs`:
  - New `DetailRowKey::PlaylistEntry`, `DetailListKind::PlaylistEntries`, `DetailPlaylistEntryRow` and `DetailPageRows::PlaylistEntries`.
  - New builder `playlist_entry_rows`, `DetailSelection::rekey_after_move` and `DetailSelection::selected_playlist_entries`.
  - New `DetailPageRows::selected_entries`, `playlist_entry` and `add_to_playlist_choices`. The bar arm counts distinct tracks.
  - Tests R25-01 to R25-05, R25-08 and R25-10.
- `src/view_models/list_selection.rs`: new `ListSelection::rekey` for a list of one member for each row.
- `src/view_models/selection_action_bar.rs`: new kinds `RemoveFromPlaylist`, `MoveToTop` and `MoveToBottom`, new `PlaylistSelectionCounts` and the constructor `for_playlist`.
- `src/view_models/library.rs`:
  - `PlaylistTrackRowDisplay` has `selection` and `with_selection`. `PlaylistTrackControlsDisplay` has `checkbox_id` and `reorder_visible`.
  - `PlaylistDetailVm::visible_tracks` is `pub(crate)`. New `LibraryViewModel::release_download_active_track_ids`. Test R25-09.
- `src/library/detail_selection.rs`: the playlist arm of `library_detail_page_rows`, and new `click_playlist_entry`, `toggle_playlist_entry` and `open_playlist_entry`.
- `src/library/app_impl.rs`: new `move_selected_playlist_entries` and `remove_selected_playlist_entries`. `select_playlist` calls `retain_visible` when the selection belongs to that playlist page.
- `src/app/detail_selection.rs`: the playlist page rows, Enter on a playlist row, the "Add to playlist" choices and the dispatch of the new bar actions.
- `src/ui/composites/selection_action_bar.rs`: the handlers `on_remove_from_playlist`, `on_move_to_top` and `on_move_to_bottom`.
- `src/ui/composites/track_row.rs`: `checkbox_state_for` is `pub(crate)`, so the playlist row uses it.
- `src/ui/shells/playlist.rs`:
  - The leading slot shows the drag handle in `Normal` and the checkbox in `Selecting`. The row menu and the drop target show only when `reorder_visible` is true.
  - The selected fill and the outlines come from `row_outline`. The row carries the scroll probe.
  - The page scrolls through `music_column_tracked_scrollbar` with the held detail column handle.
- `src/ui/shells/library/playlist_detail.rs`: one row builder for the eager path and the paged path. The struct `LibraryPlaylistPage` holds the input. Each row gets its selection slot by position.
- `src/ui/shells/library/detail.rs`: the playlist arm gives the row slots and the scroll handle.
- `tests/architecture_tests.rs`: the playlist row guard expects `reorder_visible`. The new guard `adr_0084_playlist_selection_r25_11_selection_mode_row_accepts_no_drop` holds R25-11 and the held scroll handle.

### Behavior Changed

- The playlist page shows "Select" and "Done" in the detail column header, with the accessibility label "Select tracks".
- In `Selecting`, the checkbox takes the place of the drag handle. The row menu and the drop target are absent. "Play" stays. "Done" shows the drag handle and the row menu again.
- A body click toggles the row in `Selecting` and opens the track page in `Normal`. Ctrl+click, Shift+click, Space, Ctrl+A and Escape act as in packet 018.
- The bar gives "Remove n from playlist", "Download n tracks", "Add n to playlist", "Move to top", "Move to bottom", "Remove from Library…" and "Clear", in this order. An action that cannot act is absent.
- "Remove n from playlist" removes the entries in one transaction. The tracks stay in the Library. The page reloads, the rows clear and the mode stays `Selecting`.
- "Move to top" and "Move to bottom" move the entries in one transaction and keep their order. The selection and the focus follow the entries.
- A stale entry gives the status "The playlist changed. Select the tracks again." and changes nothing.
- The "Add to playlist" choices leave out the open playlist.
- Up, Down, Page Up, Page Down, Home and End move the row focus and keep the row in view. Enter opens the track page of the focused row.

### Checks - 2026-10-20

| Check | Result |
|---|---|
| `cargo test --lib adr_0084_playlist_selection_` | Green, 12 tests |
| `cargo test --lib adr_0084_detail_selection_` | Green, 9 tests |
| `cargo test --lib adr_0084_selection_` | Green, 29 tests |
| `cargo test --lib adr_0084_discover_selection_` | Green, 6 tests |
| `cargo test --lib adr_0084_download_queue` | Green, 21 tests |
| `cargo test` | Green, 2032 unit tests and 338 guards. Ten documentation examples stay ignored |
| `cargo test --test architecture_tests` | Green, 338 guards |
| `cargo fmt -- --check` | Green |
| `cargo clippy -- -D warnings` | Green |
| `cargo check --all-targets` | Green, no warning |
| `cargo build --bin v4vmm` | Green |

No application launch and no production-data change occurred.

### Deviations From The Task

1. The drag handle slot is `TokenSize::MinHitTarget` wide, not `Size::PointerHitTarget`. Thus the checkbox slot has the same `MinHitTarget` width, and the 28 px checkbox is at its center. The row width does not change.
2. The packet 018 test R18-04 stated that a playlist page has no list kind. This packet changes that rule, so the playlist page is now in the list of pages with a selection.
3. `PlaylistEntryKey` and `PlaylistEdge` are in `src/db.rs`. `DetailRowKey::PlaylistEntry` uses that one type, so no second copy exists.
4. "Add n to playlist" adds the track of each selected entry. A track with two selected entries goes into the chosen playlist two times, so the count agrees with the label.
5. "Remove n from playlist" uses the `Secondary` control style. It is not destructive, because the tracks stay in the Library and no confirmation opens.
6. `select_playlist` does not clear the detail selection, so the stop condition did not occur.
7. The r14-01 guard list did not change. `src/ui/shells/playlist.rs` stays in its list and passes through `music_column_tracked_scrollbar`. The R25-11 guard also asserts the held handle.
8. `release_download_active_track_ids` reads the Library tree one time for each call. A tree search for each entry was too slow for a long playlist.

### Unresolved Concerns

1. V1 to V7 below need the operator. They are not in `docs/pending-human-checks.md`, because this packet was the only document to edit.
2. A move or a removal does not update the playlist position of a playback session. The packet excludes this gap.
3. The top app reads the playlist rows a few times at each render. A very long playlist can make the page slow. This concern is not in the overhaul plan list, for the same reason as item 1.
4. `rekey_after_move` reads the selection when the command completes. A selection change during the command can give a wrong key. The next reload then drops that key.
5. A row that waits shows a skeleton and no checkbox. Ctrl+A also selects that row.

## Operator Visual Check

This check needs a Linux desktop session and this checkout. It also needs three or more Library tracks, with one or more downloaded and one or more not downloaded.

V3 and V4 change the order and the content of a playlist. Use only the test playlist of step 3.

1. Build the app: `cargo build --bin v4vmm`. Then start `target/debug/v4vmm` from a desktop session.
2. Open Music, then Releases, in the sidebar.
3. Make the test playlist "Packet 025 test":
   - Open a Library release. Open the "Add to playlist" menu of a downloaded track and use "New Playlist" to make "Packet 025 test".
   - Add one more downloaded track, one track that is not downloaded, and the first track again.
   - The playlist now holds four entries, and one track is in it two times.
4. Make a second playlist "Packet 025 other" in the same way, with one track.
5. Open Music, then Playlists, then "Packet 025 test".

### V1: The two modes

6. Look for a "Select" button at the right end of the detail column header. Look for a drag handle and a row menu on each row.
7. Click "Select". The drag handle and the row menu of each row must go. Look for the bar "Select items".
8. Click "Done". The drag handle and the row menu must show again.

**Wrong if:** a drag handle or a row menu shows in selection mode, or they stay absent after "Done".

### V2: The checkbox place

9. Look at the left edge of a row. Click "Select" and look again.
10. The checkbox must be in the place of the drag handle. The track number, the title and "Play" must not move.

**Wrong if:** the row gets wider or narrower, or the title moves to the side.

### V3: The bar actions

11. In selection mode, check the two entries of the track that is in the playlist two times.
12. Look for "Remove 2 from playlist", "Add 2 to playlist", "Move to top" or "Move to bottom", "Remove from Library…" and "Clear". "Remove from Library…" must name 1 track in its accessibility label.
13. Click "Add 2 to playlist" and choose "Packet 025 other". "Packet 025 test" must not be a choice. The selection must stay.
14. Check the track that is not downloaded. Click "Download 1 track". Only that track must download, and the selection must stay.

**Wrong if:** a label counts one track two times for "Download" or "Remove from Library…". Also wrong: the open playlist is a choice, or an action acts on a row that is not checked.

### V4: Move to top and Move to bottom

15. Check the second and the fourth entry. Click "Move to top".
16. The two entries must be first and second, in their earlier order. Their checkboxes must stay checked. "Move to top" must go from the bar.
17. Click "Move to bottom". The two entries must be last, in the same order, and stay checked.

**Wrong if:** the order of the selected entries changes, the other entries change their order, or the selection stays on the earlier positions.

### V5: Remove from playlist

18. Check one entry. Click "Remove 1 from playlist". The entry must go from the page at once. The mode must stay in selection mode with no checked row.
19. Open Music, then Tracks. The removed track must still be in the Library.

**Wrong if:** the track goes from the Library, or the page shows the removed entry until you open it again.

### V6: The keys

20. Open "Packet 025 test" again and click an empty area of the detail column.
21. Push Down two times. A focus outline must move down the rows.
22. Push End, then Home. On a long playlist, the page must scroll, and the focused row must stay in view.
23. Push Space. The focused row must become checked. Push Escape. Selection mode must end.
24. Push Down, then Enter. The track page of the focused row must open.

**Wrong if:** a key does not move the focus, or the focused row goes out of view.

### V7: Widths, themes and type sizes

25. Click "Select" and check one row. Make the window narrow, then wide again.
26. Change between the Light and the Dark theme in Settings.
27. Set a larger type size in Settings, if this setting is available.
28. In each condition, look at the checkbox, the selected fill, the focus outline and the bar.

**Wrong if:** text is clipped, the checkbox moves out of its slot, or the selected fill is not visible.

**Cleanup:**

- Click "Done" to end selection mode.
- Delete the playlists "Packet 025 test" and "Packet 025 other" with their "Delete" buttons.
- V3 downloads one track. Remove it if you do not want it.
- Do not delete `/tmp/v4vmm-governance.ie6k8TQf`.

## Orchestrator Review - 2026-10-20

Pass. The code needs no fix.

- The deviations agree with the packet. The checkbox uses the 44 px column of the drag handle, so the row width does not change.
- "Remove n from playlist" has no confirmation, because the tracks stay in the Library and the action deletes no file.
- `rekey_after_move` reads the selection when the command completes. A selection change during the command can give a wrong key, and the next reload drops it. The orchestrator accepts this.
- The orchestrator added the row cost to the responsiveness list of the overhaul plan, as item 13.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `docs/architecture/source-map.md`
- This packet: `docs/tasks/adr-0084-task-025-playlist-selection.md`
- ADR 0084 Decision 10, and the packets 009, 018, 019, 020 and 022
- Each file in "Files To Inspect"

Goal:
- Make each change in "Required Changes": the key, the storage, the row, the bar, the keys and the guards.

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- The view model gives each label, count, availability and check state. The screen only composes. Use named tokens.
- Each storage change is one transaction. Treat a stale entry as an error that changes nothing.
- Tests use recorded values and an in-memory database. No test sends a request or deletes a file.
- Never run `git checkout`, `git restore`, `git stash`, `git reset`, `git add` or `git commit`.
- Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- The Discover selection, the download queue and the release page assembly, except calls to them.
- The database schema and the migration registry.
- Any ADR, and each document other than this packet.

Acceptance criteria:
- Each case R25-01 to R25-12 has a passing test or guard.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.
- The packet has an "Implementation Result - 2026-10-20" section and an "Operator Visual Check" section for V1 to V7.

Test commands:
- The commands in "Checks", with `cargo fmt -- --check` after the last edit and `cargo build --bin v4vmm` last.

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

Stop and report the problem, and do not guess, when:
- `select_playlist` clears the detail selection, so the selection cannot stay after an action.
- A file in `src/ui/shells/library` goes above the ADR 0038 line ceiling, and a split would move code out of its owner.
- A change needs a file in "Do not touch".
