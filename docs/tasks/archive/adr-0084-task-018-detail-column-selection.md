# ADR 0084 Task 018: Selection In The Detail Column

Status: Complete - 2026-10-20. Cases R18-01 to R18-10 pass. Mechanical checks are Green. The operator passed V1 to V7 on 2026-10-20.

## Goal

The lists of the detail column get the selection model and the selection mode of packets 009 and 019. They are the track rows of a release page, the rows of the singles group page and the release rows of the feed page.
The detail column also gets the keyboard focus. The keys move through its rows, keep the focused row in view, and scroll a page that has no rows.

## Authority

- [ADR 0084](../../adr/0084-music-column-browser.md) Decision 10, amended on 2026-10-19: one selection model for each list of releases and tracks, the two modes, the keys and the action bar.
- [ADR 0084](../../adr/0084-music-column-browser.md) Decision 11: each track row keeps a place for audition. No control shows in it.
- [ADR 0084](../../adr/0084-music-column-browser.md) Decision 4: one page for each item. The Library page and the Index page of a release give the same selection.
- [Packet 009](adr-0084-task-009-list-selection-and-action-bar.md) and [packet 019](adr-0084-task-019-selection-mode.md): the model, the rows with members, the checkbox, the keys, the mode, the header button and the bar. This packet uses each of them and changes none of them.
- Operator decisions of 2026-10-20:
  - Packet 018 covers the release page track rows (Library and Index), the singles group page and the feed page. It also covers the keyboard focus and scroll of the detail column.
  - A new packet 020 covers the playlist page and the Discover tiles. On the playlist page, "Remove" removes the selected tracks from the playlist, and they stay in the Library.
  - Packet 010 covers the selection of the search results, because it rebuilds that view. The search results are in the list column today.
  - The "Select" and "Done" button of a detail page is at the trailing end of the detail column header. The action bar is at the bottom of the detail column.
- The durable set in [AGENTS.md](../../../AGENTS.md): typed action state, renderer portability, token discipline, accessibility and cautious destructive actions.

## Recorded Facts - 2026-10-20

- Release page, Library origin: `render_library_feed_detail` in `src/ui/shells/library/feed_detail.rs` runs on `LibraryApp`. Each row comes from `render_library_track_row`, with `LibraryTrackRowVm` (`src/view_models/library.rs`) and `TrackRowVm` (`src/view_models/track_detail.rs`). A click opens `FrameNavigationEntry::Track(track.track_key())`.
- Release page, Index origin: `TopApp::index_feed_track_rows` in `src/app/search_dispatch.rs` builds `SharedTrackRowVm` rows (`src/view_models/entity_detail.rs`) and draws them with `render_release_track_row` (`src/ui/shells/entity.rs`). A track with no track GUID gets no click (`index_track_row_click_key`).
- Both paths end in the `TrackRow` composite (`src/ui/composites/track_row.rs`). It sits above `ListRow`, and has no selection field and no checkbox.
- Singles group page: `render_library_singles_group_page` in `src/ui/shells/library/singles_group.rs`. Each row is `render_library_track_row` with `LibraryTrackRowContext::SinglesGroupPage`, and a click opens the release page of the single. Its main action calls `download_singles` or `add_albums_to_playlist`.
- Feed page: `PublisherPageVm` in `src/view_models/publisher_page.rs` and `render_publisher_page` in `src/ui/shells/publisher.rs`. Each release row is a `PublisherPageAlbumVm` with `in_library` and `open: Option<ReleaseRowOpenAction>` that holds a `ReleaseKey`. `render_album_row` draws it with `ListRow`.
- The detail column has no focus handle. Each selection key handler checks `list_column_focus` (`src/library.rs`). The keys use the `ActivePane && !Input` context (`src/app/keyboard.rs`).
- Each detail page scrolls through `music_column_scrollbar`, with no held `ScrollHandle`. No key scrolls the detail column today. The guard `adr_0084_scrollbars_r14_01_music_columns_use_the_shared_helper` lists the scroll owner files.
- The frame shell has a typed header action slot (`FrameHeaderActionDisplay`). `WorkspaceSlots::header_action_for` gives it only to `WorkspaceFrameKind::ContentList` (`src/ui/shells/workspace.rs`).
- The list column bar comes from `LibraryApp::render_selection_action_bar` (`src/library/app_impl.rs`).
- Library download paths: `LibraryApp::download_rss_added_track(track_id)` for a Library track that is not downloaded, and `download_singles(&feed_ids)` for Library releases.
- Index download paths: `TopApp::download_index_track(feed, track)` for an Index track, and `TopApp::download_index_album_of_track(feed_guid)` for an Index release by its feed GUID.
- Playlist paths: `add_track_to_playlist(track_id, playlist_id)` and `add_albums_to_playlist(&feed_ids, playlist_id)`. Each takes Library ids only.

## Design

### The state

- Add one detail selection to `LibraryApp`: a `ListSelection` and a `SelectionMode`, and the `FrameNavigationEntry` of the page that they belong to. Put the pure part in `src/view_models/list_selection.rs` or a new `src/view_models/detail_selection.rs`.
- The page kind gives the keys:
  - A release page: track rows, member and row key `TrackKey`.
  - The singles group page and the feed page: release rows, member and row key `ReleaseKey`.
- A selection that belongs to another page entry is empty and in `Normal`. Each move to another page clears it and sets `Normal`. Back and Forward do the same.
- A row with no key, for example an Index track with no track GUID, has no checkbox and no focus stop.
- The detail selection and the two list column selections are independent. A change in one changes no other.

### The rows

- `TrackRow` gets the checkbox slot and the selected fill of `ListRow`, through typed fields. It puts the checkbox before the track number. It keeps the audition place of Decision 11, with no control in it.
- `LibraryTrackRowVm` and `SharedTrackRowVm`, or the row slot that the renderer fills, get a `RowSelectionVm`. The Library row and the Index row of one track give the same `RowSelectionVm`.
- `PublisherPageAlbumVm` gets a `RowSelectionVm`. The feed page row passes it to its `ListRow`.
- The rules of packet 019 apply. In `Normal`, no row shows a checkbox, and a plain click opens the page. In `Selecting`, each row shows its checkbox, and a plain click toggles the row.
- Ctrl+click, Shift+click, Ctrl+A and Space start `Selecting`.

### The header button

- The view model gives a typed header action for the detail column: "Select" in `Normal` and "Done" in `Selecting`. The accessibility label names the rows, for example "Select tracks", "Done selecting tracks", "Select releases".
- The action is absent on a page with no selectable row: a track page, a playlist page, an empty release, and each page outside the three of this packet.
- `WorkspaceSlots` gives the action and its handler to `WorkspaceFrameKind::Detail`, through the same typed slot as the list column. The shell decides no label.

### The action bar

- The detail bar uses `SelectionActionBarVm` and the `SelectionActionBar` composite at the bottom of the detail column. It shows in `Selecting` only. With no selected row it reads "Select items".
- Track rows of a release page:

| Action | Label example | Shows when | Effect |
|---|---|---|---|
| Download | "Download 3 tracks" | One or more selected tracks are not downloaded | A Library track uses `download_rss_added_track`. An Index track uses `download_index_track` |
| Add to playlist | "Add 4 tracks to playlist" | One or more selected tracks are in the Library. The count names only those | `add_track_to_playlist` for each |
| Remove | "Remove 2 tracks…" | One or more selected tracks are downloaded | The selection removal confirmation of packet 009 |
| Clear | "Clear" | Rows are selected | Clears the rows and keeps `Selecting` |

- Release rows of the singles group page and the feed page: the four actions of the Releases list bar of packet 009. "Download" uses `download_singles` for a Library release and `download_index_album_of_track` for an Index release. "Add to playlist" and "Remove" count Library releases only.
- After an action the selection stays. A removal drops the removed rows through `retain_visible`. The page updates in place.
- TopApp composes the bar for each page, because the Index page is TopApp's. Library actions go to `LibraryApp`. Index downloads stay in `TopApp`.

### The keyboard focus and scroll

- Add a detail column focus handle. A click in the detail column, outside a text field, gives it the focus. The detail column shows the keyboard focus outline of packet 009 on the focused row.
- The selection keys of packet 009 act on the column with the focus: the list column or the detail column. Each handler checks its own focus handle. Add no new key binding.
- On a page with selectable rows, Up, Down, Page Up, Page Down, Home and End move the row focus. The column scrolls to keep the focused row in view. Enter opens the page of the focused row. Escape ends `Selecting` and clears the selection, and keeps its present Back action when the page is not in `Selecting`.
- On a page with no selectable rows, for example a track page, the same keys scroll the detail column. Up and Down move one named step. Page Up and Page Down move one view height less one step. Home and End go to the top and the bottom. Put the step in `src/ui/tokens.rs`.
- Each detail page holds its `ScrollHandle` through `music_column_tracked_scrollbar`. Update the r14 guards for each new file, and keep their rules.

## Required Changes

1. Add the detail selection state and its page entry rule, with tests.
2. Add the selection fields to `TrackRow`, to the two track row view models and to `PublisherPageAlbumVm`.
3. Wire the clicks of the three pages for the two modes.
4. Add the detail header action and the `Detail` header slot.
5. Add the detail bar, its view model functions and its actions.
6. Add the detail focus handle and the key routing by focus. Keep the focused row in view. Add the key scroll of a page with no rows.
7. Add or update each guard.

## Mechanical Acceptance Criteria

Use the prefix `adr_0084_detail_selection_` for behavioral tests beside the owning code. A fact about a `LibraryApp` or `TopApp` method that needs a GPUI context is a source guard in `tests/architecture_tests.rs`, as in packets 009 and 019. Each guard cites ADR 0084 Decision 10.

| Case | Required proof |
|---|---|
| R18-01 | The detail selection of one page entry is empty and in `Normal` for another entry. A page change clears it |
| R18-02 | A Library release page and an Index release page of one release give the same visible rows and the same `RowSelectionVm` for each track. A track with no key has no checkbox |
| R18-03 | In `Normal`, no detail row view model shows a checkbox. In `Selecting`, each row with a key shows one |
| R18-04 | The detail header action reads "Select" and "Done" with an accessibility label on the three pages. It is absent on a track page and on a release with no track |
| R18-05 | The track bar: "Download" counts only the tracks that are not downloaded, "Add to playlist" only the Library tracks, and "Remove" only the downloaded tracks. Each is absent with a count of 0 |
| R18-06 | The release bar of the feed page counts an Index release in "Download" only, and a Library release in "Add to playlist" and "Remove" |
| R18-07 | A source guard: each selection key handler checks the focus handle of its own column, and the list column handlers keep their check |
| R18-08 | A source guard: a page change in `TopApp` or `LibraryApp` clears the detail selection and sets `Normal` |
| R18-09 | The key scroll step of a page with no rows comes from a named token. The view height step is one view height less one step |
| R18-10 | Each packet 009 and packet 019 case still passes |

## Visual Acceptance Criteria

These are for the operator. No test proves them.

- V1: on a Library release page, the detail header shows "Select". "Select" shows a checkbox on each track row and the bar "Select items". A click on a row selects it and opens no page. "Done" and Escape end the mode.
- V2: on an Index release page that is not in the Library, the same as V1. "Download n tracks" downloads the selected tracks only, and the page updates in place.
- V3: on a partly downloaded release, "Download", "Add to playlist" and "Remove" each name the correct count. "Remove" opens the confirmation of packet 009.
- V4: the singles group page and the feed page select release rows. Their bar acts on releases.
- V5: a click in the detail column gives it the keyboard focus. Up and Down move the row focus, and the column scrolls to keep the row in view. Space, Ctrl+A and Enter act on the detail rows. The list column keeps its own selection.
- V6: on a track page, Up, Down, Page Up, Page Down, Home and End scroll the detail column.
- V7: check normal and narrow widths, Light and Dark themes, and the larger type sizes. The button, the checkboxes, the track numbers and the bar stay in place, with no clipped text. In `Selecting`, a track row is not taller than in `Normal`.

## Exclusions

- No selection on the playlist page and no selection of the Discover tiles. Packet 020 owns them.
- No selection of the search results. Packet 010 owns it.
- No audition control. ADR 0068 owns it.
- No change to the list column selection, its keys or its bar.
- No schema change and no new MusicIndex endpoint.

## Files To Inspect

- [Agent rules](../../../AGENTS.md) and the [source map](../../architecture/source-map.md).
- `src/view_models/list_selection.rs`, `src/view_models/selection_action_bar.rs`, `src/view_models/library.rs`, `src/view_models/publisher_page.rs`, `src/view_models/entity_detail.rs`, `src/view_models/track_detail.rs`, `src/view_models/singles_group.rs`.
- `src/library.rs`, `src/library/app_impl.rs`, `src/app.rs`, `src/app/search_dispatch.rs`, `src/app/keyboard.rs`.
- `src/ui/composites/track_row.rs`, `src/ui/composites/list_row.rs`, `src/ui/composites/music_column_scroll.rs`, `src/ui/composites/frame_shell.rs`, `src/ui/composites/selection_action_bar.rs`.
- `src/ui/shells/library/feed_detail.rs`, `src/ui/shells/library/singles_group.rs`, `src/ui/shells/library/release_detail_surface.rs`, `src/ui/shells/publisher.rs`, `src/ui/shells/entity.rs`, `src/ui/shells/search_results_inspector.rs`, `src/ui/shells/workspace.rs`, `src/ui/shells/library/track_detail.rs`.
- `src/ui/shells/library_removal_confirmation.rs`, `src/view_models/library_removal.rs`.
- `tests/architecture_tests.rs`: the guards `adr_0084_selection_*`, `adr_0084_selection_mode_*`, `adr_0084_scrollbars_r14_*`, and the ADR 0038 line ceiling of `src/ui/shells/library`.

## Checks

```bash
cargo test --lib adr_0084_detail_selection_
cargo test --lib adr_0084_selection_
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy -- -D warnings
cargo check --all-targets
cargo build --bin v4vmm
```

## Rollback

Revert the commit. This packet adds no migration and no stored data. A removal in V3 deletes audio files. Use tracks that you can download again.

## Implementation Result - 2026-10-20

### Files Changed

- `src/view_models/detail_selection.rs` (new), the pure part:
  - The types `DetailRowKey` (a track or a release), `DetailListKind`, `DetailRowClick`, `LibraryReleaseFacts`, `DetailTrackRow`, `DetailReleaseRow` and `DetailPageRows`.
  - `DetailSelection` holds one `ListSelection<DetailRowKey>`, one `SelectionMode` and the `FrameNavigationEntry` of its page. `reset` and `bind` clear it for a different page.
  - `detail_header_action` gives "Select" and "Done". `DetailPageRows::action_bar` gives the counts of the bar.
  - The row builders are `library_release_track_rows`, `index_release_track_rows`, `singles_group_release_rows` and `feed_page_release_rows`.
  - `index_track_guid` moved here from `src/app/search_dispatch.rs`. Tests R18-01 to R18-06 are in this file.
- `src/view_models/selection_action_bar.rs`: new `SelectionActionCounts` and `SelectionActionBarVm::from_counts`, with a separate count for each action. An action with a count of 0 is absent. `new` calls `from_counts`, with no change to the packet 009 behavior.
- `src/view_models/library.rs`: new `LibraryViewModel::library_release_facts`.
- `src/view_models/publisher_page.rs`: `PublisherPageAlbumVm` has a new `selection: RowSelectionVm` field. `album_vm` sets `RowSelectionVm::none()`.
- `src/library.rs`: new `LibraryApp` fields `detail_selection`, `detail_column_focus`, `detail_column_scroll` and `detail_scroll_reveal`.
  `queued_feed_downloads` changed to `queued_downloads`, a queue of `QueuedDownload::Feed` and `QueuedDownload::Track`.
- `src/library/detail_selection.rs` (new), the `LibraryApp` methods of the detail column:
  - The page change (`reset_detail_selection`, `bind_detail_selection`), the rows of a Library page, the clicks and the checkbox.
  - The key methods `move_detail_focus`, `toggle_focused_detail_row`, `select_all_detail_rows`, `focused_detail_row` and `end_detail_selecting`. Each method examines `detail_column_focus`.
- `src/library/app_impl.rs`:
  - `hydrate_detail_from_nav` resets the detail selection. A Library reload drops removed rows through `retain_visible`.
  - New `request_selection_removal`, which the two list column removals also use. `add_tracks_to_playlist` and `create_playlist_and_add_tracks` are `pub(crate)`.
  - New `download_library_tracks`. `start_next_queued_download` downloads a queued feed or a queued track.
- `src/app/detail_selection.rs` (new): the `TopApp` part. `detail_page_rows` gives the rows of the open page, also for the Index release page and the feed page.
  It also has the header action, the clicks, the key routing, `render_detail_column_frame` (the focus and the bar) and the bar actions. Library actions go to `LibraryApp`. Index downloads stay in `TopApp`.
- `src/app.rs`: the render binds the detail selection to the open page. It wraps the detail slot in `render_detail_column_frame` and gives the header action to the `Detail` frame. The new helper `index_release_page_detail` replaces the inline code of the Index release branch.
- `src/app/keyboard.rs`: each selection key goes to the list column and to the detail column. Escape ends the detail selection mode before Back. Enter opens the focused detail row when the list column opens no row.
- `src/app/search_dispatch.rs`: the Index release rows carry the detail selection facts, the checkbox click and the click that follows the mode. The Index release page and the Index track page hold the detail scroll handle.
  `feed_guid_from_view`, `open_track_in_frame`, `download_index_track` and `download_index_album_of_track` are `pub(super)`.
- `src/app/publisher_dispatch.rs`: new `publisher_page_rows`, which sets the selection facts of each feed page row and the two click callbacks.
- `src/app/name_match_dispatch.rs`: the name-match page gets the detail scroll handle.
- `src/ui/composites/track_row.rs`: `TrackRow` gets the typed fields `selection`, `on_toggle_selection` and `scroll_probe`. It puts the checkbox in the `ListRow` checkbox slot, before the track number. The audition slot shows no control.
- `src/ui/composites/list_row.rs`: new `scroll_probe` slot.
- `src/ui/composites/music_column_scroll.rs`: new `ScrollReveal`, a probe that keeps the focused row in view. New `key_scroll_offset_y` and `key_scroll`. A test of the reveal distance.
- `src/ui/composites/release_detail_surface.rs`: an optional held `scroll_handle`.
- `src/ui/tokens.rs`: new `Size::KeyScrollStep` (40 px), and the R18-09 test.
- `src/ui/shells/workspace.rs`: `detail_header_action`, `on_detail_header_action` and `take_detail_slot`. `WorkspaceFrameKind::Detail` gets the typed header action slot.
- `src/ui/shells/entity.rs`: `ReleaseTrackRowSlot` has `selection`, `on_toggle_selection` and `scroll_probe`. `ReleaseDetailBehaviorSlots` has `scroll_handle`.
- `src/ui/shells/track.rs`: `TrackDetailBehaviorSlots` has `scroll_handle`.
- `src/ui/shells/publisher.rs`: `render_publisher_page` takes `PublisherPageRows` in place of `on_open_release`. Each release row gets the checkbox, the selected fill, the focus outline and the probe.
- `src/ui/shells/library/detail.rs`: new `DetailRowSlot`, `LibraryTrackRowPlace`, `LibraryTrackRowContext::with_slot` and `with_detail_row_slot`. The dispatcher passes the row slots and the scroll handle.
- `src/ui/shells/library/feed_detail.rs`, `singles_group.rs`, `track_detail.rs`, `feed_list.rs`, `src/ui/shells/name_match_page.rs` and `src/ui/shells/search_results_inspector.rs`: each detail page holds the detail scroll handle through `music_column_tracked_scrollbar`.
- `tests/architecture_tests.rs`: new guards `adr_0084_detail_selection_r18_07_each_key_handler_checks_its_own_column` and `adr_0084_detail_selection_r18_08_a_page_change_clears_the_detail_selection`. `src/app/detail_selection.rs` is in `SCREEN_FILES`. The r14-01 guard lists `src/ui/shells/library/singles_group.rs`.

### Behavior Changed

- Three detail lists get the selection model and the two modes of packets 009 and 019. They are the track rows of a release page (Library and Index), the singles group page rows and the feed page release rows.
- In `Normal`, no detail row shows a checkbox, and a plain click opens the page of the row. In `Selecting`, each row with a key shows its checkbox, and a plain click changes the selection of the row.
- Ctrl+click, Shift+click, Ctrl+A, Space and a checkbox click start `Selecting`. "Done" and Escape end it and clear the selection.
- The detail column header shows "Select" or "Done" on the three pages. The accessibility label is "Select tracks" or "Select releases", or the "Done selecting" form. It is absent on a track page, a playlist page, a release with no track and each other page.
- In `Selecting`, the action bar shows at the bottom of the detail column. With no selected row, it reads "Select items".
- Track bar: "Download n tracks" downloads the selected tracks that are not downloaded. Library tracks download one at a time. "Add n tracks to playlist" adds the selected Library tracks. "Remove n tracks…" opens the removal confirmation of packet 009 for the downloaded tracks.
- Release bar: "Download" queues Library releases through `download_singles`, and starts `download_index_album_of_track` for each Index release. "Add to playlist" and "Remove…" act on the Library releases.
- After an action, the selection stays. A Library reload drops a removed row through `retain_visible`.
- A move to a different page clears the detail selection, sets `Normal` and scrolls the detail column to the top. Back, Forward and a breadcrumb do the same.
- A click in the detail column gives it the keyboard focus. On a page with selectable rows, Up, Down, Page Up, Page Down, Home and End move the row focus. The column scrolls by the smallest distance that shows the focused row.
  Enter opens the page of the focused row.
- On a detail page with no selectable row, the same keys scroll the page. Up and Down move 40 px. Page Up and Page Down move one view height less 40 px. Home and End go to the top and the bottom.
- The list column selection, its keys and its bar do not change.

### Checks - 2026-10-20

| Check | Result |
|---|---|
| `cargo test --lib adr_0084_detail_selection_` | Green, 9 tests |
| `cargo test --lib adr_0084_selection_` | Green, 29 tests |
| `cargo test` | Green, 1989 unit tests and 329 guards. Ten documentation examples stay ignored |
| `cargo test --test architecture_tests` | Green, 329 guards |
| `cargo fmt -- --check` | Green |
| `cargo clippy -- -D warnings` | Green |
| `cargo check --all-targets` | Green, no warning |
| `cargo build --bin v4vmm` | Green |

No application launch and no production-data change occurred.

### Deviations From The Task

1. The rows of a detail page are not direct children of the scroll container. `ScrollHandle::scroll_to_item` cannot find them. Thus a focused row carries the `ScrollReveal` probe. After a key moves the focus, the probe reads the bounds of the row at the next layout and scrolls the column.
2. All detail pages share one held scroll handle, `LibraryApp::detail_column_scroll`. A page change scrolls it to the top. The Discover view keeps the handle of the content list.
3. The key scroll works on the release page, the singles group page and the two track pages. It also works on the artist page, the feed page, the name-match page and Discover.
   The playlist page is in "Do not touch", so its keys do not scroll it. The loading, failure and empty states of the detail column also keep `music_column_scrollbar`.
4. The R18-09 test is in `src/ui/tokens.rs`. The composite guards forbid a `px(..)` literal and an unscaled `.px()` in `src/ui/composites`, also in tests. Thus the scroll functions use `f32`.
5. A page change resets the selection in `LibraryApp::hydrate_detail_from_nav`. Some page opens push a navigation entry with no call to that function. Thus each render of the top app also binds the selection to the open page. A bind to a different page clears the selection.
   The tab change back to Music also calls `hydrate_detail_from_nav`, so it clears the detail selection.
6. `subscribe_track` starts no second download while one runs. The download queue of the singles group page also holds Library tracks, so "Download n tracks" downloads each selected Library track in turn.
7. With the row slot as a new argument, `render_library_track_row` gets eight arguments. Thus `LibraryTrackRowPlace` joins the page context and the row slot. `with_detail_row_slot` and these new types are in `src/ui/shells/library/detail.rs`, so `feed_detail.rs` stays at 498 lines. No existing code moved out of its owner.
8. `SelectionActionBarVm::from_counts` is new. The `new` function of packet 009 calls it with the same counts as before. This packet changes no packet 009 behavior.
9. R18-02 compares the rows of `library_release_track_rows` and `index_release_track_rows` for one recorded release. The two renderers use these same functions.

### Unresolved Concerns

1. V1 to V7 below need the operator. No test proves a rendered checkbox, the scroll position, the focus outline or a layout at a narrow width.
2. "Download n tracks" on an Index release page calls `download_index_track` one time for each selected track. These downloads run at the same time, not one at a time.
   "Download" on a feed page also starts one `download_index_album_of_track` for each Index release at the same time.
3. After an Index track downloads, the Index release page keeps its Index source until the operator opens it again. Its rows do not show the track as downloaded, and "Download" counts it again.
4. The new visual checks V1 to V7 are not in `docs/pending-human-checks.md` or in the phase plan. The task instructions named only this packet as a document to edit.

## Operator Visual Check

This check needs a Linux desktop session and this checkout. It also needs these items:

- A Library release with three or more tracks, partly downloaded.
- An Index release that is not in the Library, with two or more tracks.
- A Library singles group with two or more singles.
- A feed page with one Library release and one Index release.

V3 removes audio files. Use tracks that you can download again.

1. Build the app: `cargo build --bin v4vmm`. Then start `target/debug/v4vmm` from a desktop session.
2. Open Music, then Releases, in the sidebar.

### V1: A Library release page

3. Open the partly downloaded Library release. Look for a "Select" button at the right end of the detail column header.
4. Click "Select". Look for a checkbox before the track number of each track row. Look for the bar "Select items" at the bottom of the detail column.
5. Click the body of a track row. Its checkbox must become checked, and no page must open.
6. Click "Done". The checkboxes and the bar must go, and the header must read "Select".
7. Click "Select" and check a row. Click in the detail column and push Escape. The result must be the same as step 6.
8. Push Escape again. The app must do its usual Back action.

**Wrong if:** a row shows a checkbox in normal mode, or a click in selection mode opens a page. Also wrong: "Done" or Escape keeps a checkbox or the bar.

### V2: An Index release page

9. Open the Index release that is not in the Library, for example from Discover. Do steps 3 to 6 again.
10. Click "Select" and check two tracks. Click "Download 2 tracks". Only these two tracks must download.
11. Look at the page during and after the download. The page must stay open. After the first track downloads, the page must show each track of the release, with the two tracks downloaded.

**Wrong if:** the bar downloads a track that is not checked, or the page closes or changes to a different page.

### V3: The counts of a partly downloaded release

12. Open the partly downloaded Library release. Click "Select". Check one downloaded track and one track that is not downloaded.
13. Look for "Download 1 track", "Add 2 tracks to playlist" and "Remove 1 track…".
14. Click "Remove 1 track…". The removal confirmation of packet 009 must open and name only the downloaded track. Click Cancel.

**Wrong if:** a label shows an incorrect count, or the confirmation names a track that is not downloaded.

### V4: The singles group page and the feed page

15. Open the singles group. Click "Select" and check two singles. Look for the bar with "release" in each label.
16. Open the feed page. Click "Select" and check the Library release and the Index release.
17. Look for "Download 1 release", "Add 1 release to playlist" and "Remove 1 release…".

**Wrong if:** a bar names tracks in place of releases. Also wrong: "Add to playlist" or "Remove" counts the Index release.

### V5: The keyboard focus of the detail column

18. Open the Library release page. Click an empty area of the detail column.
19. Push Down three times. A focus outline must move down the track rows.
20. On a release with many tracks, push End. The column must scroll, and the focused row must stay in view.
21. Push Space. The focused row must become checked, and selection mode must start.
22. Push Ctrl+A. Each track row must become checked.
23. Click "Done". Move the focus to a row with Down, then push Enter. The track page of that row must open.
24. Open Releases and select a release in the list column. Then push Down in the detail column. The list column selection must not change.

**Wrong if:** the keys act on the list column while the detail column has the focus. Also wrong: the focused row goes out of view.

### V6: The key scroll of a track page

25. Open a track page with a long metadata section. Click an empty area of the detail column.
26. Push Down, then Up. The page must scroll by a small step.
27. Push Page Down, then Page Up. The page must scroll by almost one view height.
28. Push End, then Home. The page must go to the bottom, then to the top.

**Wrong if:** a key does not scroll the page, or Page Down skips content.

### V7: Widths, themes and type sizes

29. On the Library release page, click "Select". Make the window narrow, then make it wide again.
30. Change between the Light and the Dark theme in Settings.
31. Set a larger type size in Settings, if this setting is available.
32. In each condition, look at the button, the checkboxes, the track numbers and the bar. Compare the height of a track row in selection mode and in normal mode.

**Wrong if:** text is clipped, or an item moves out of its position. Also wrong: a track row is taller in selection mode.

**Cleanup:** click "Done" to end selection mode. V2 adds the Index release to the Library. Remove its two tracks if you do not want them. If V3 removed a track, download it again. Remove each playlist that the check added. Do not delete `/tmp/v4vmm-governance.ie6k8TQf`.

## Orchestrator Review - 2026-10-20

Pass, after one fix. The orchestrator made the fix in the same change.

- After an Index track download, the open release page kept its Index source. Its rows did not show the track as downloaded, and "Download" counted it again. The per-row "Download" of an Index track had the same gap before this packet.
  The success handler of `download_index_track` in `src/app/search_dispatch.rs` now calls `apply_release_page_source` when the open page is the release of that feed. `rss::subscribe_feed` stores each item of the feed, so the page changes to the Library page with each track. The guard `adr_0084_detail_selection_index_track_download_updates_the_open_release_page` holds it.
- The download queue of `LibraryApp` now holds feeds and tracks (`QueuedDownload`). The packet 009 "Download" of releases keeps its order and its one download at a time.
- The other deviations agree with the packet. The detail rows are not direct children of the scroll container, so a probe on the focused row keeps it in view. The playlist page has no key scroll until packet 020.
- `cargo fmt -- --check` and `cargo clippy -- -D warnings` are Green. `cargo test` gives 1989 library tests and 330 guards, all passing.

Open points that no packet owns yet:

- Index track downloads from the bar start at the same time, not one at a time. Each one fetches the feed RSS. The Library downloads use the queue.
- A change of tab, for example to Show and back to Music, clears the detail selection. The cause: the top app binds the selection to the open page at each render.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `docs/architecture/source-map.md`
- This packet: `docs/tasks/archive/adr-0084-task-018-detail-column-selection.md`
- ADR 0084 Decisions 4, 10 and 11, and the archived packets 009 and 019
- Each file in "Files To Inspect"

Goal:
- Make each change in "Required Changes": the detail state, the rows, the clicks, the header button, the bar, the focus and the key scroll.

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- The view model gives each label, count, mode, availability and checked state. The screen only composes. Use named tokens and named icons.
- Use the types of packets 009 and 019. Do not copy the selection model.
- Tests use recorded values. No test sends a request or deletes a file.
- Never run `git checkout`, `git restore`, `git stash`, `git reset` or `git commit`. Do not stage files.
- Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- The playlist page, the Discover tiles and the search results.
- The database schema and the migration registry.
- Any ADR, and each document other than this packet.

Acceptance criteria:
- Each case R18-01 to R18-10 has a passing test or guard.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.
- The packet has an "Implementation Result" section and an "Operator Visual Check" section for V1 to V7.

Test commands:
- The commands in "Checks", with `cargo fmt -- --check` after the last edit and `cargo build --bin v4vmm` last.

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

Stop and report the problem, and do not guess, when:
- The Library row and the Index row of one track cannot give the same `RowSelectionVm` without a change to `TrackKey`.
- A held `ScrollHandle` on a detail page breaks an r14 guard in a way that its rule does not permit.
- A second focus handle breaks a key binding of the list column.
- A file in `src/ui/shells/library` goes above the ADR 0038 line ceiling, and a split would move code out of its owner.
- A change needs a file in "Do not touch".
