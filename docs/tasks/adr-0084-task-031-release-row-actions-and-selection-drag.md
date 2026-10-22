# ADR 0084 Task 031: Release Row Actions And The Drag Of A Selection

Status: Implemented - 2026-10-21. Each case R31-01 to R31-14 has a passing test or a guard, and the mechanical gate is Green. Two deviations change R31-01 and R31-04 (see "Deviations"). The visual checks V1 to V9 stay open until the operator walks them.

## Goal

Part A. A release row on a feed page or a name page gets the same actions as a row of the Releases list. One shared owner gives the actions of each release row, so the lists cannot differ.

Part B. In selection mode, a drag that starts on a checked track row carries each checked track to a playlist.

## Authority

- [ADR 0083](../adr/0083-design-language.md) Decision 5: the action hierarchy, the "⋯" menu, and a destructive item last with "…" and a confirmation.
- [ADR 0083](../adr/0083-design-language.md) Decision 6, amended on 2026-10-21 by operator decision: a drag in selection mode carries each checked track, in list order, with a count on the drag image.
- [ADR 0084](../adr/0084-music-column-browser.md) Decision 5 (the kind in action labels), Decision 7 (the feed level and the name level of the artist page) and Decision 10 (selection, the bar and the download queue).
- Packets [007](adr-0083-task-007-list-and-playlist-row-actions.md) (decision D2), [008](adr-0083-task-008-drag-track-to-playlist.md) (decision D5 and check V7), [009](archive/adr-0084-task-009-list-selection-and-action-bar.md), [016](adr-0084-task-016-library-and-everything-scopes.md), [017](adr-0084-task-017-name-level-artist-page.md), [018](archive/adr-0084-task-018-detail-column-selection.md) and [019](archive/adr-0084-task-019-selection-mode.md).
- Operator answers of 2026-10-21 in the [run record](../reviews/orchestrated-run-2026-10-20.md):
  - A release row on an artist page or a name page gets the same actions as a row of the Releases list, with "Select".
  - In selection mode, a drag that starts on a checked track row carries each checked track.
- The durable set in [AGENTS.md](../../AGENTS.md): typed action state, renderer portability, accessibility and cautious destructive actions.

## Recorded Facts - 2026-10-21

The facts come from the working tree at commit `d298922`. Packet 030 changes some of these files. Find each call site again before a change.

### The walk

- The short walk of 2026-10-21 recorded: "Release rows on an artist page or a name page have no row actions | Fail | Name page "Rap Central · by name": the rows show "In Library" and no "⋯" or right-click actions."
- It also recorded: "Drag of a selection of several tracks | Fail, operator request | No drag starts in selection mode (task 008 decision, V7)."

### The Releases list row

- `render_destination_list_row` in `src/ui/shells/library/destination_lists.rs:285` draws each Releases row and each Artists row with `ListRow`. It draws the checkbox in selection mode and attaches a click. It has no "⋯" button and no right-click menu.
- Packet 007 decision D2 gave no menu to these rows: the Releases rows, the singles group rows, the Artists rows and the Playlists rows. It also gave no menu to the feed page release rows and the Discover release rows.
  - Its reason: each row opens its page, and the release page menu and the bar hold the release actions.
  - D2 says: "A later packet can add a release row menu if the operator asks for it."
- Thus the Releases list row has these actions today:
  - A click opens the release page (`LibraryApp::click_release_row`).
  - "Select" in the list column header starts selection mode (packets 009 and 019).
  - The bar gives "Download n releases", "Add n releases to playlist", "Remove n releases…" and "Clear" (`SelectionActionBarVm::from_counts`, `src/view_models/selection_action_bar.rs:133`).

### The feed page row and the name page row

- `render_release_row` in `src/ui/composites/release_section_list.rs:132` draws each release row of the feed page and of the name page. It uses `ListRow`. It draws the checkbox in selection mode (lines 182 to 206), the click, and the badges "Not listed by this feed" and "In Library". It has no menu.
- `TopApp::release_section_rows` in `src/app/publisher_dispatch.rs:447` sets the selection facts of each row from `LibraryApp::detail_selection`. The feed page (`render_publisher_page`) and the name page (`render_name_level_page_content`, `src/app/name_level_dispatch.rs`) both use it.
- `TopApp::detail_page_rows` in `src/app/detail_selection.rs:80` gives `DetailPageRows::Releases(feed_page_release_rows(...))` for `PublisherDetail` and for `ArtistName`. Thus both pages already have "Select" and the bar (packets 017 and 018).
- `PublisherPageAlbumVm` (`src/view_models/publisher_page.rs:220`) holds `in_library`, `not_listed`, `open` and `selection`. It holds no release kind and no menu.
- `DestinationListRowVm` (`src/view_models/library.rs:1848`) holds `open`, `art` and `selection`. It holds no menu.

### The menu parts that exist

- `src/ui/composites/track_row_menu.rs` owns the row menu of packet 006: `track_row_menu_items`, the "Add to playlist" level, `TrackRowMenuCell` and `menu_cell_visibility`. Both the "⋯" menu and the right-click menu use one item function.
- `row_menu_parts` in `src/ui/composites/list_row.rs:334` gives the menu cell and the right-click root of a `ListRow` (packet 007). `RowMenu` (line 290) holds the menu, the playlists, the handlers and the return focus.
- `TrackRowMenuVm` and `TrackRowMenuAction` (`src/view_models/release_page.rs`) hold the track actions: `AddToPlaylist`, `Download`, `InspectSources`, `Remove`, `MoveUp`, `MoveDown` and `RemoveFromPlaylist`.
- `AlbumPageAction::label(kind)` in `src/view_models/album_page.rs:35` gives "Download <kind>", "Add to playlist" and "Remove <kind>…". The release page menu uses these labels.
- `LibraryApp::remove_album_after_confirmation` (`src/library/app_impl.rs:2698`) is the confirmed removal of one release. The release page "Remove album…" uses it.

### The drag

- `ReleaseTrackRowVm::drag` is `Some` only when `selection.checkbox_visible` is false (`src/view_models/release_page.rs:897`).
- `content_list_row_drag` (`src/view_models/library.rs:1607`) gives the drag of a Tracks list row. `content_list_rows_with_selection` sets it only outside selection mode (line 3684).
- `TrackDragVm` (`src/view_models/library.rs:617`) and `TrackDragPayload` (`src/ui/composites/track_drag.rs:30`) hold one `track_id` and one `title`.
- `attach_track_drag` (`src/ui/composites/track_drag.rs:97`) attaches `.on_drag(` with `TrackDragPreview`, which shows the title.
- `PlaylistDrop` holds one `track_id`. The drop handler of the Playlists list calls `add_tracks_to_playlist(vec![drop.track_id], ...)` (`src/ui/shells/library/destination_lists.rs:492`).
- `LibraryApp::add_tracks_to_playlist(track_ids, playlist_id, cx)` (`src/library/app_impl.rs:4139`) takes a list. `begin_playlist_append` sets "Downloading n track(s)...", and `finish_playlist_append` sets "Added a of n to {playlist}".
- The spring-load of packet 008 opens Playlists as a new path. The detail column then shows no page, and `bind_detail_selection` clears the detail selection of the earlier page (packet 018). The drag payload keeps its own data.

## Design

### Part A: one owner of the release row actions

- Add one view model function that gives the actions of one release row from its facts, for example `release_row_menu(facts: ReleaseRowActionFacts) -> Option<ReleaseRowMenuVm>`. The facts are the release key, the title, the kind, the Library state (from `LibraryReleaseFacts`) and the queue phase.
- The same facts give the counts of the bar for a selection of one release. A test proves that the menu items and the bar actions for that one release agree (R31-03).
- The menu holds, in this order: "Download <kind>", "Add to playlist", a divider, then "Remove <kind>…". The labels come from `AlbumPageAction::label(kind)`. An item that cannot act is absent.
- `DestinationListRowVm` and `PublisherPageAlbumVm` each get `menu: Option<ReleaseRowMenuVm>` from that one function. The menu is `None` in selection mode.
- The renderer uses the menu composite of packet 006 and `row_menu_parts` of packet 007. Do not add a second menu composite. If `TrackRowMenuVm` cannot hold the release items, make the composite read one shared row menu type, and keep one item function.
- `render_destination_list_row` and `render_release_row` attach the menu cell and the right-click root. Each row keeps its click, its checkbox and its badges.
- The hosts map the items:
  - "Download <kind>" queues the release through the path of "Download n releases" of the bar, for one release.
  - "Add to playlist" uses the path of "Add n releases to playlist" for one release. "New Playlist" uses the create path of the bar.
  - "Remove <kind>…" calls `remove_album_after_confirmation`, the path of the release page menu.

### Part B: the drag of a selection

- `TrackDragVm` holds a list of track ids and a preview text, for example `track_ids: Vec<i64>` and `label: String`.
- In `Normal`, a row gives a list of one track and its title, as today.
- In `Selecting`, a checked row gives each checked Library track of the list, in list order, and the label "<n> tracks". A row that is not checked gives `None`. A selection of one track gives the label "1 track".
- `TrackDragPayload` and `PlaylistDrop` carry the list. The drop calls `add_tracks_to_playlist(track_ids, playlist_id, cx)` one time.
- The preview shows the label. The style of `TrackDragPreview` does not change.
- The drag sources are the sources of packet 008 D2: a `TrackRow` of a Library track, and a Tracks list row of a Library track. A drag in `Selecting` starts only on such a row.

## Orchestrator Decisions

The orchestrator made these decisions on 2026-10-21. "**Flag.**" marks a decision that changes what the operator sees and that no ADR decides. The operator confirms it before the visual walk closes.

- **D1 A menu on the Releases list too.** **Flag.** The Releases list row has no "⋯" and no right-click menu today (packet 007 D2). The operator answer asks for "the same actions as a row of the Releases list". The walk asks for "⋯" and the right-click menu.
  - Option A: add the one release row menu to the Releases list rows, the feed page rows and the name page rows. This replaces packet 007 D2 for these three lists.
  - Option B: add the menu to the feed page rows and the name page rows only. The Releases list then has fewer actions than the pages, and the lists differ.
  - **Recommendation: Option A.** One owner gives the same actions to the three lists.
- **D2 The rows with no menu.** The singles group rows of the Releases list, the Artists rows, the Playlists rows and the Discover rows keep no menu. A group row is a collection, and its page holds its actions. Packet 007 D2 stays for these rows.
- **D3 The items.** "Download <kind>", "Add to playlist" and "Remove <kind>…". No "Inspect sources" and no "Copy feed URL". The release page menu holds those, one click away.
- **D4 The availability.**
  - "Download <kind>": the release is not fully in the Library, and no job of its key waits or runs.
  - "Add to playlist": always, the same as the bar. A track that is not downloaded downloads first.
  - "Remove <kind>…": the release has a Library track, and no job of its key waits or runs. No removal of it runs.
- **D5 An Index release row.** A row of a release that is not in the Library gives "Download <kind>" and "Add to playlist". Examples are a row in the "Everything" scope and a row on the name page. The hosts use the Index paths of the bar.
- **D6 "Select" stays.** The feed page and the name page already have "Select" and the bar (packets 017 and 018). This packet adds no selection code. V5 checks them.
- **D7 Selection mode hides the menu.** In `Selecting`, a release row shows no "⋯" and has no right-click menu, the same as a track row (packet 006 D9).
- **D8 The drag of a selection.** Operator decision of 2026-10-21. ADR 0083 Decision 6 records it. This replaces packet 008 D5 and changes its check V7.
- **D9 The drag of one track.** Outside selection mode, a drag carries one track, as today. The preview shows the title.
- **D10 The order.** "In list order" is the order of the rows on the page or in the list, not the order of the checks.
- **D11 Only Library tracks.** A drag carries only Library tracks. An Index track row starts no drag (packet 008 D3). On an Index release page, selection mode starts no drag.
- **D12 The selection after the spring-load.** **Flag, minor.** The spring-load opens Playlists as a new path, so the detail selection clears (packet 018). The payload keeps the track ids, so the drop adds each track. Back returns to the page in `Normal`.
  - Alternative: keep the selection of the earlier page for Back. This changes the rule of packet 018 for each page change.
  - **Recommendation:** accept the clear. The drop result is in the status, and the tracks are in the playlist.
- **D13 The playlist page.** **Flag, minor.** The playlist page in selection mode starts no drag. Its drag handle is hidden in `Selecting` (packet 025), and its bar has "Add n to playlist".
  - **Recommendation:** keep it so. A drag of playlist entries to another playlist is a separate request.
- **D14 A release drag.** No drag of a release row or of a selection of releases. The bar and the new row menu hold "Add to playlist".

## Operator Answers - 2026-10-21

- D1: Option A. One shared release row menu goes to the Releases list rows, the feed page rows and the name page rows. This replaces packet 007 D2 for these three lists.
- The orchestrator decided D12 and D13 by their recommendations, by the operator rule of 2026-10-20 ("decide but keep a record"). D12: the spring-load clears the selection, and the drop still adds each track. D13: the playlist page starts no drag in selection mode.
- Remove rule, operator answer of 2026-10-21 after the implementation: "Remove <kind>…" shows when one or more tracks of the release are in the Library, the same as the release page. The bar "Remove n releases…" counts these releases too, so the row menu, the bar and the release page agree. This replaces implementation deviation 1.

## Required Changes

1. Part A: the one release row action function and its facts, with the menu in `DestinationListRowVm` and `PublisherPageAlbumVm`.
2. Part A: the menu cell and the right-click root on `render_destination_list_row` and on `render_release_row`.
3. Part A: the host mapping in `LibraryApp` (the Releases list) and in `TopApp` (the feed page and the name page).
4. Part B: the list in `TrackDragVm`, `TrackDragPayload` and `PlaylistDrop`, and the label of the preview.
5. Part B: the drag fact in `Selecting` for `ReleaseTrackRowVm` and for the Tracks list rows.
6. Part B: the drop handler with the list.
7. The tests and the guards of this packet. Update the tests and the guards of packets 007 and 008 that assert the replaced rules, with the new meaning.

## Mechanical Acceptance Criteria

Use the prefix `adr_0084_release_row_actions_` for Part A and `adr_0083_selection_drag_` for Part B, beside the owning code. Each new guard is situational. It cites ADR 0083 Decision 6 or ADR 0084 Decision 10.

| Case | Required proof |
|---|---|
| R31-01 | The release row action function gives, for a Library album not fully downloaded and with no job: "Download album", "Add to playlist", "Remove album…", in this order. "Remove album…" is destructive and last |
| R31-02 | It gives no "Download <kind>" for a release fully in the Library or with a waiting or running job. It gives no "Remove <kind>…" for a release with no Library track, with a job, or during a removal |
| R31-03 | For each case of R31-01 and R31-02, each menu item has a bar action of the same kind for a selection of that one release, and the reverse is true for "Download", "Add to playlist" and "Remove" |
| R31-04 | An Index release (not in the Library) gives "Download <kind>" and "Add to playlist". A single gives "Download single" |
| R31-05 | In `Selecting`, the menu of a Releases row, a feed page row and a name page row is `None` |
| R31-06 | A source guard (ADR 0083 Decision 6): `destination_lists.rs` and `release_section_list.rs` build the menu through `row_menu_parts`. No file builds release row menu items outside the one function |
| R31-07 | A source guard: the singles group rows, the Artists rows and the Playlists rows get no release row menu (D2) |
| R31-08 | In `Normal`, `ReleaseTrackRowVm.drag` and the Tracks list drag hold one track id and the title, as before |
| R31-09 | In `Selecting`, a checked Library row gives each checked Library track id in list order, and the label "3 tracks" for three tracks. An unchecked row gives `None`. One checked track gives "1 track" |
| R31-10 | A GPUI test: a drag from a checked `TrackRow` in `Selecting` onto a playlist row calls the drop handler one time with each checked track id in list order |
| R31-11 | A GPUI test: a `TrackRow` that is not checked in `Selecting` starts no drag |
| R31-12 | The drop handler of `render_library_playlists_list` calls `add_tracks_to_playlist` one time with the full list. The status after the add names the count, for example "Added 3 of 3 to …" |
| R31-13 | An Index row and a playlist page row in `Selecting` give no drag (D11, D13) |
| R31-14 | Each test of packets 006, 007, 008, 009, 016, 017, 018, 019 and 025 still passes, or it changes with the meaning of D1 or D8, and the implementation result names it |

## Visual Acceptance Criteria

These are for the operator. No test proves them.

- V1: in the Releases list, the pointer on a release row shows "⋯". A right-click opens the same items. A singles group row shows no "⋯".
- V2: on a feed page and on the name page "Rap Central · by name", the pointer on a release row shows "⋯". The right-click menu has the same items as the Releases row of the same release.
- V3: "Download album" on a row queues the release. The item is absent while the job waits or runs.
- V4: "Add to playlist", then a playlist, adds the release. "New Playlist" makes a playlist with the release.
- V5: "Remove album…" asks for confirmation. After the confirmation, the row loses "In Library" with no navigation. "Select" on the feed page and on the name page shows the checkboxes and the bar.
- V6: on a Library release page, push "Select" and check three tracks. Drag one checked row. The drag image shows "3 tracks".
- V7: hold the drag over the sidebar row "Playlists" until the list opens. Drop on a test playlist. The status names 3 tracks, and the playlist count grows by 3.
- V8: in selection mode, a drag that starts on a row that is not checked does nothing. Outside selection mode, a drag of one row shows its title.
- V9: in the Tracks list, do V6 and V7 with two checked tracks. The drag image shows "2 tracks".

## Exclusions

- No menu on the singles group rows, the Artists rows, the Playlists rows and the Discover rows (D2).
- No drag of a release or of a selection of releases (D14).
- No drag from an Index row or from a playlist page in selection mode (D11, D13).
- No change to the bar, the selection model or the spring-load.
- No check for a duplicate entry in a playlist.
- No schema change.

## Files To Inspect

- [Agent rules](../../AGENTS.md) and the [source map](../architecture/source-map.md).
- Part A: `src/ui/shells/library/destination_lists.rs`, `src/ui/composites/release_section_list.rs`, `src/ui/composites/list_row.rs`, `src/ui/composites/track_row_menu.rs`, `src/ui/primitives/context_menu.rs`.
- Part A: `src/view_models/library.rs` (`DestinationListRowVm`, `LibraryReleaseFacts`, `BusyItems`), `src/view_models/publisher_page.rs`, `src/view_models/name_level_page.rs`, `src/view_models/album_page.rs`, `src/view_models/selection_action_bar.rs`, `src/view_models/detail_selection.rs`.
- Part A: `src/library/app_impl.rs` (`click_release_row`, the bar paths, `remove_album_after_confirmation`), `src/app/publisher_dispatch.rs`, `src/app/name_level_dispatch.rs`, `src/app/detail_selection.rs`.
- Part B: `src/ui/composites/track_drag.rs`, `src/ui/composites/track_row.rs`, `src/ui/shells/library/content_list.rs`, `src/view_models/release_page.rs`, `src/view_models/library.rs` (`TrackDragVm`, `content_list_row_drag`), `src/library/app_impl.rs` (`add_tracks_to_playlist`).
- `tests/architecture_tests.rs`: the guards of packets 007 and 008.

## Checks

```bash
cargo test --lib adr_0084_release_row_actions_
cargo test --lib adr_0083_selection_drag_
cargo test --lib adr_0083_track_drag_
cargo test --lib adr_0083_row_actions_
cargo test --lib adr_0084_detail_selection_
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy --all-targets -- -D warnings
cargo check --all-targets
touch src/main.rs && cargo build --bin v4vmm
```

## Rollback

Revert the commit. This packet adds no migration and no stored data. V3, V4, V5, V7 and V9 change the Library and playlists. Use a test playlist and a release that you can download again.

## Implementation Result - 2026-10-21

### Part A: One Owner Of The Release Row Actions

- `src/view_models/release_row_actions.rs` is new. It holds the one function `release_row_menu(&ReleaseRowActionFacts)`.
  - `ReleaseRowState` is `Library(LibraryReleaseFacts)` or `Index { job_active }`.
  - `ReleaseRowAvailability::of` gives the availability of "Download", "Add to playlist" and "Remove".
  - `ReleaseActionCounts::of` gives the counts of the bar from the same rule.
  - The labels come from `AlbumPageAction::label(kind)`. The menu is `None` in selection mode and for a row with no available action.
- `LibraryReleaseFacts` has the new fact `removing`. `removable()` is false during a removal. The new `downloadable()` is false while a job or a removal of the release waits or runs.
- `LibraryViewModel::release_row_menu` gathers the facts of a release key and calls the one function. A Library release names the kind of the Library release.
  - The Releases list sets `DestinationListRowVm::menu` in `releases_list_with_selection`. This includes the single rows of an open group.
  - `TopApp::release_section_rows` sets `PublisherPageAlbumVm::menu` through `LibraryApp::release_row_menu`. The feed page and the name page use it.
- `DestinationListRowVm` has the new fields `release_kind` and `menu`. `PublisherPageAlbumVm` has the new fields `kind` and `menu`, and the method `element_id`.
- The menu composite of packet 006 reads `TrackRowMenuVm`. `TrackRowMenuItemVm::label` is now a `String`, so `TrackRowMenuVm` holds the release labels. No second menu composite exists.
- `src/ui/composites/release_row_menu.rs` is new. `release_row_menu` makes the `RowMenu` of packet 007 from a `ReleaseRowMenuVm`. Each item goes to the host as a `ReleaseRowCommand`.
- `render_destination_list_row` and `render_release_row` give the row `ListRow::menu_cell(release_row_menu(...))`. `ListRow` builds the cell and the right-click root through `row_menu_parts`.
- `LibraryApp::run_release_row_command` is the one host of the three lists:
  - "Download <kind>" calls `download_release_plan` with one release. This is the path of "Download n releases" of the Releases bar.
  - "Add to playlist" calls `add_albums_to_playlist`. "New Playlist" calls `create_playlist_and_add_albums`.
  - "Remove <kind>…" calls `remove_album_after_confirmation`.
- On the Releases list, the right-click menu gives the focus back to the list column. On the feed page and the name page, it gives the focus back to the detail column.

### Part B: The Drag Of A Selection

- `TrackDragVm` holds `track_ids` and `label`. `TrackDragVm::one` gives the drag of one track with its title. `TrackDragVm::selection` gives the drag of a checked row with the label "<n> tracks", or "1 track".
- `release_page_vm` sets the drag of each row of a Library release page in `Selecting` (`apply_selection_drags`). A row that is not checked gets `None`.
- `content_list_rows_with_selection` sets the drag of each checked Tracks list row in `Selecting`. The list holds each checked Tracks list row, in list order. A Needs attention row has no drag.
- `TrackDragPayload` holds `track_ids` and `label`. `PlaylistDrop` holds `track_ids`. The drag of a playlist entry gives a list of one track.
- The drop handler of `render_library_playlists_list` calls `add_tracks_to_playlist(drop.track_ids.clone(), drop.playlist_id, cx)` one time.
- The preview shows the label. The style of `TrackDragPreview` does not change.
- An Index row, a singles group page row and a playlist page row start no drag in `Selecting`.

### Behavior Changed

- A release row of the Releases list, a feed page and a name page shows "⋯" on hover. A right-click opens the same items.
- The bar of a release selection counts through the one rule. This changes two counts on each release bar: the Releases list, the feed page, the name page, the singles group page, the Discover "New releases" view and the search list.
  - "Download n releases" does not count a Library release while a job of its key waits or runs, or while its removal runs.
  - "Remove n releases…" does not count a Library release while its removal runs.
  - On the Releases list, "Download n releases" also does not count a `MusicIndex` release while a job of its key waits or runs.
- In selection mode, a drag from a checked track row of a Library release page or of the Tracks list carries each checked track. The preview shows the count.

### Deviations

- R31-01: replaced. The operator answer of 2026-10-21 ("Remove rule" in "Operator Answers - 2026-10-21") replaces this deviation. "Review Corrections - 2026-10-21" records the change.
  - The first implementation showed "Remove <kind>…" only for a fully downloaded release, and the bar counted only these releases.
  - Now the row menu, the bar and the release page use one rule: one or more tracks of the release are downloaded.
- R31-04: an Index release gives "Download <kind>" only. It gives no "Add to playlist".
  - The bar has no "Add to playlist" path for a `MusicIndex` release: `detail_library_ids` and `releases_selected_feed_ids` give only Library feed ids. The Index release page has no "Add to playlist" item.
  - An item that cannot act is absent (typed action state). An Index "Add to playlist" needs a new path that reads the feed and then appends.
- The detail rows hold no job fact of a `MusicIndex` release. Thus the bar of a detail page still counts a `MusicIndex` release in "Download" while its job waits or runs. The menu of that row has no "Download". The queue drops a second job of the same key.
- The "Remove" item of a release row has the label of `AlbumPageAction::RemoveAlbum`, for example "Remove album…", not "Remove from Library…".

### Tests And Guards

- `src/view_models/release_row_actions.rs`: R31-01, R31-02, R31-03 (two tests), R31-04, R31-05 and the command map. Names start with `adr_0084_release_row_actions_`.
- `src/view_models/library.rs`: R31-03 for the Releases list bar, R31-05 for the Releases list and the page menu, R31-07 for the Artists rows. R31-08, R31-09 and R31-13 (playlist page) for the Tracks list, the selection label, and R31-12 for the drop status.
- `src/view_models/release_page.rs`: R31-08, R31-09 and R31-13 (Index release page). Names start with `adr_0083_selection_drag_`.
- `src/ui/composites/track_drag.rs`: the GPUI tests R31-10 and R31-11.
- `tests/architecture_tests.rs`: the new guards R31-06 (`adr_0084_release_row_actions_r31_06_one_release_row_menu`) and R31-07 (`adr_0084_release_row_actions_r31_07_rows_with_no_menu`).
- R31-14: these tests and guards changed with the meaning of D1 and D8:
  - `adr_0083_track_drag_r83_8_07_drop_uses_the_one_add_path` expects `this.add_tracks_to_playlist(drop.track_ids.clone(), drop.playlist_id, cx);`.
  - `adr_0083_track_drag_r83_8_01_release_row_drag`, `adr_0083_track_drag_r83_8_11_content_list_row_drag` and the tests of `track_drag.rs` use `TrackDragVm::one` and `track_ids`.
  - The entry drop test of `src/ui/shells/playlist.rs` reads `drop.track_ids`.
  - Each other test of packets 006, 007, 008, 009, 016, 017, 018, 019 and 025 passes with no change.
- The gate: `cargo test` (2243 unit tests and 369 guards), `cargo test --test architecture_tests`, `cargo fmt -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo check --all-targets` and `cargo build --bin v4vmm`. All Green.

### Review Corrections - 2026-10-21

The review of 2026-10-21 found six problems, A to F. Each correction has a unit test or a guard. No other behavior changed.

- A. The Remove rule of the operator answer of 2026-10-21:
  - `LibraryReleaseFacts` holds `downloaded: ReleaseDownloaded` in place of the field `fully_downloaded`. The states are `NoTrack`, `SomeTracks` and `EachTrack`. `LibraryViewModel::library_release_facts` sets it from `TrackRow::is_in_library`, the fact that the release page reads. An enum is necessary because clippy does not accept more than three `bool` fields in a struct.
  - `LibraryReleaseFacts::removable()` is now `has_library_track()`, with no job and no removal. The row menu, the bar count, `releases_removable_feed_ids` and `detail_removal_items` all use it.
  - Test: `adr_0084_release_row_actions_release_downloaded_state`.
  - The release page and the rows agree during a job and during a removal. During a job, the release page shows no "Remove album…". During a removal, the release page shows "Remove album…" as not available. The row menu and the bar show no "Remove" in both states, because an item that cannot act is absent from a row menu. Thus no surface lets "Remove" act during a job or a removal.
  - Tests: `adr_0084_release_row_actions_r31_01_items_and_order`, `adr_0084_release_row_actions_r31_02_absent_items`, `adr_0084_release_row_actions_r31_03_menu_and_bar_agree` and `adr_0084_release_row_actions_r31_03_counts_use_the_one_rule` use the states "no track", "some tracks" and "each track". `adr_0084_scopes_r16_16_bar_counts` now expects "Remove 2 releases…", because the release "Beta" has one downloaded track of two.
- B. The plans of "Download n releases" use the rule of the bar count:
  - `releases_downloadable_feed_ids` filters with `LibraryReleaseFacts::downloadable`. The Index part of `releases_download_plan` skips a key where `busy.index_release_downloading(key)` is true.
  - `DetailPageRows::release_download_plan` is new. It filters with `ReleaseRowAvailability::of`, the rule of the detail bar count. `TopApp::download_selection` uses it for the feed page, the name page, the singles group page, the Discover "New releases" view and the search list.
  - The helper `release_is_fully_downloaded` had no caller after this change. It is deleted.
  - Tests: `adr_0084_release_row_actions_releases_download_plan_matches_the_bar_count` (`src/view_models/library.rs`) and `adr_0084_release_row_actions_detail_download_plan_matches_the_bar_count` (`src/view_models/detail_selection.rs`). The selections hold a normal release, a fully downloaded release, a release with a job, a release in a removal and Index releases. The Releases list test also holds an Index release with a job. The detail rows hold no job fact of an Index release.
- C. The focus of a click on a row with a drag:
  - `attach_track_drag` does not stop the left mouse-down now. It calls `gpui_base::GlobalState::suppress_text_selection`, so the window root starts no text selection.
  - The mouse-down continues to the column. Thus `track_focus` of the list column or of the detail column gets the focus, also on a checked row in selection mode.
  - `window.prevent_default()` was not used: the `track_focus` handler of GPUI does not move the focus when the default is prevented.
  - Test: the GPUI test `adr_0083_selection_drag_click_on_a_checked_row_focuses_its_column` (`src/ui/composites/track_drag.rs`). It fails with the earlier `stop_propagation`. The test `adr_0083_track_drag_r83_8_05_drag_starts_no_text_selection` still passes.
- D. The tests of R31-12 and R31-13:
  - R31-12 is now the GPUI test `adr_0083_selection_drag_r31_12_drop_status_names_the_count` in `src/library/app_impl.rs`. A drag of a selection payload with three ids drops on a playlist row and gives one `PlaylistDrop`. The view model step of `add_tracks_to_playlist` then gives one append and the status "Added 3 of 3 to Focus". The guard `adr_0083_track_drag_r83_8_07_drop_uses_the_one_add_path` holds the host call. The test is not in `track_drag.rs`, because two guards keep `crate::db` and the append functions out of that composite.
  - R31-13 has the new GPUI test `adr_0083_selection_drag_r31_13_playlist_row_in_selecting_starts_no_drag` (`src/ui/shells/playlist.rs`). A checked row and a row that is not checked show no drag handle, and a press and a move start no drag.
- E. The comment in `TopApp::release_section_rows` names the feed page rows and the name page rows now.
- F. `PublisherPageAlbumVm::element_id` takes the section heading. The id is `publisher-album:{heading}:{feed_guid}`. Thus a release in two sections of one page has two menu states.
  - Test: `adr_0084_release_row_actions_element_id_is_unique_for_each_section` (`src/view_models/publisher_page.rs`).
  - The checkbox id of a release row (`publisher-album-checkbox:{row_id}`) did not change.
- The gate after the corrections: `cargo test` (2249 unit tests and 369 guards), `cargo test --test architecture_tests`, `cargo fmt -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo check --all-targets` and `cargo build --bin v4vmm`. All Green.

## Orchestrator Review - 2026-10-22

Pass, after the corrections of "Review Corrections - 2026-10-21". The review found one serious defect, B: the bar count left out a release with a job or a removal, but the "Download" plan still queued it. The two plans now use the rule of the count, and two tests compare each plan with its count. The fix agent stopped before its report, so the orchestrator checked each correction A to F in the code and ran the full gate on 2026-10-22.

## Operator Visual Check

Use the binary that `cargo build --bin v4vmm` makes. Use a test playlist, and a release that you can download again.

1. V1, the Releases list:
   1. Start the app: `cargo build --bin v4vmm && ./target/debug/v4vmm`.
   2. Open Music, then Releases. Put the pointer on a release row. "⋯" must show at the end of the row.
   3. Click "⋯". Then right-click the same row. Both menus must show the same items in the same order.
   4. Open a singles group row with its chevron. The group row must show no "⋯". Each single row below it must show "⋯".
   5. Wrong: a row with no "⋯", two menus with different items, or a group row with "⋯".
2. V2, the feed page and the name page:
   1. Open the artist page of a feed, then the name page "Rap Central · by name".
   2. Put the pointer on a release row. "⋯" must show after the "In Library" badge.
   3. Right-click the row. The items must be the same as the items of the Releases row of the same release.
3. V3, "Download album":
   1. On a row of a release that is not fully downloaded, click "⋯", then "Download album".
   2. The status bar must show the job. Click "⋯" again while the job waits or runs. "Download album" must be absent.
4. V4, "Add to playlist":
   1. On a row of a Library release, click "⋯", then "Add to playlist", then the test playlist. The status must name the count of the added tracks.
   2. Do it again with "New Playlist". Type "packet 031 test" and click "Create & Add". The new playlist must hold the tracks of the release.
5. V5, "Remove album…" and "Select":
   1. On a row of a downloaded release, click "⋯", then "Remove album…". A confirmation must show.
   2. Click the confirm button. The row must lose "In Library" with no navigation.
   3. On the feed page and on the name page, push "Select". The checkboxes and the bar must show. The rows must show no "⋯".
6. V6, the drag image:
   1. Open a Library release page with three or more downloaded tracks. Push "Select" and check three tracks.
   2. Drag one checked row. The drag image must read "3 tracks".
7. V7, the drop:
   1. Hold the drag over the sidebar row "Playlists" until the list opens.
   2. Drop on the test playlist. The status must read "Added 3 of 3 to …". The playlist count must grow by 3.
   3. Push Back. The release page must show in `Normal`, with no checked row (decision D12).
8. V8, the rows that start no drag:
   1. In selection mode, drag a row that is not checked. No drag image must show, and no playlist must change.
   2. Push "Done". Drag one row. The drag image must show its title.
9. V9, the Tracks list:
   1. Open Music, then Tracks. Push "Select" and check two tracks.
   2. Drag one checked row. The drag image must read "2 tracks".
   3. Do step 7 again. The status must read "Added 2 of 2 to …".
   4. After the checks, use the arrow keys and Space in the list. If the focus does not move, report it.

Cleanup:

1. Delete the playlist "packet 031 test" and remove the test entries from the test playlist.
2. Download again each release that V5 removed.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `docs/architecture/source-map.md`
- This packet: `docs/tasks/adr-0084-task-031-release-row-actions-and-selection-drag.md`
- ADR 0083 Decisions 5 and 6, and ADR 0084 Decisions 5, 7 and 10
- Packets 006, 007, 008, 017 and 018
- Each file in "Files To Inspect"

Goal:
- Make each change in "Required Changes".

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- One function gives the actions of each release row. The view model gives each label, each availability and each accessibility label. The screen only composes.
- Use the menu composite of packet 006. Do not copy it.
- Tests use recorded values. No test sends a request to a live service.
- Never run `git add`, `git rm`, `git commit`, `git checkout`, `git restore`, `git stash` or `git reset`.
- Do not run the app.

Do not touch:
- The database schema.
- Any ADR, and each document other than this packet.

Acceptance criteria:
- Each case R31-01 to R31-14 has a passing test or a guard.
- Each command in "Checks" is Green.
- The packet has an "Implementation Result - <date>" section and an "Operator Visual Check" section for V1 to V9.

At the end, report: files changed, tests run, behavior changed, deviations, unresolved concerns, and the document changes that the orchestrator must make.

Stop and report the problem, and do not guess, when:
- A change needs a new architecture decision.
- A change needs a file in "Do not touch".
