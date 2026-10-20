# ADR 0084 Task 025: Selection On The Playlist Page

Status: Ready - 2026-10-20.

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
