# ADR 0084 Task 030: The Open Defects Of Music

Status: Implemented - 2026-10-21. Each case R30-01 to R30-20 has a passing test or a guard, and the mechanical gate is Green. The visual checks V1 to V6 stay open until the operator walks them.

## Goal

Correct nine open defects of Music. The short walk of 2026-10-21 found three of them. The phase plan recorded the other six as open findings.

Each correction gets a test or a guard. The packet records the cause of each defect from the source of 2026-10-21.

## Authority

- [ADR 0083](../adr/0083-design-language.md) Decision 5: a destructive action is last, after a divider, and its label ends with "…". It asks for confirmation.
- [ADR 0083](../adr/0083-design-language.md) Decision 6: the row menu and the right-click menu. Packet [006](adr-0083-task-006-row-hover-actions.md) decision D3: "Add to playlist" opens a second level with "New Playlist".
- [ADR 0084](../adr/0084-music-column-browser.md) Decision 4 (one page for each item), Decision 9 (one navigation stack, Back and Forward), Decision 10 (selection, the bar and the download queue) and Decision 12 (the logo frame).
- Packets [018](archive/adr-0084-task-018-detail-column-selection.md) (the held scroll handle of the detail column), [022](adr-0084-task-022-download-queue.md) (the download queue), [025](adr-0084-task-025-playlist-selection.md) (the playlist bar) and [028](adr-0084-task-028-search-open-defects.md) (the search defects).
- [AGENTS.md](../../AGENTS.md): the durable set (renderer portability, typed action state, cautious destructive actions), "Current-view state must update in place" and "Every fix gets a guard".
- Operator answers of 2026-10-21 in the [run record](../reviews/orchestrated-run-2026-10-20.md): one packet for the open defects of the walk.

## Recorded Facts - 2026-10-21

The facts come from the working tree at commit `d298922`. Find each call site again before a change.

### P1: "New Playlist" in the row menu does nothing

The short walk of 2026-10-21 recorded: "ADR 0083 task 006 V5: "Add to playlist", then "New Playlist" | Fail | The click does nothing."

- `build_list_mode` in `src/ui/composites/playlist_popover.rs:298` draws the button `pl-new` with the label "New Playlist". Its click calls `controls.show_create` (line 347). No test clicks `pl-new` or `pl-create-confirm`.
- The row menu (`src/ui/composites/track_row_menu.rs`) gives `show_create: set_creating(true)` (line 185). `set_creating` writes `TrackRowMenuState::creating` and calls `cx.notify()`.
- `TrackRowMenuState` is keyed state from `row_menu_state` (line 102). GPUI 0.3.1 `Window::use_keyed_state` observes the entity and notifies the view that made it. Thus the notify draws the row again.
- `playlist_level` (line 154) reads `creating` at each draw. When `creating` is true, it gives the create mode of the shared popover (line 188).
- `TrackRowMenuCell::render` (lines 295 to 300) sets `creating` to false when `ContextMenu::is_open` gives false and no right-click menu is open. `ContextMenu::is_open` (`src/ui/primitives/context_menu.rs:302`) reads the keyed state `"{id}-state"` at the position of the cell.
- The first level item "Add to playlist" changes `ContextMenuState::level` of the menu itself. The walk shows that this step works. "New Playlist" changes a second entity, `TrackRowMenuState`. The reset rule of `TrackRowMenuCell` is the one step that can clear that entity while the menu stays open.
- The reading does not prove the cause. The packet does not know if `is_open` reads the same keyed state as the open menu, or if a second cause stops the click. The implementer proves the cause with a GPUI test first (decision D1).
- These hosts use the row menu: `TrackRow` (`src/ui/composites/track_row.rs:540`), the `ListRow` menu parts (`src/ui/composites/list_row.rs:351`) and the playlist row (`src/ui/shells/playlist.rs`). The release page header uses `AddToPlaylistPopover`, which keeps its own `creating` flag.

### P2: The delete of a playlist asks for no confirmation

- The playlist page has one delete control. `render_playlist_detail_shell` in `src/ui/shells/playlist.rs:311` to `320` draws it with `ControlStyle::Destructive`, the label "Delete" and the accessibility label "Delete playlist".
- The labels come from `PlaylistDetailActionsDisplay` in `src/view_models/library.rs:6073` to `6075`. The label has no "…".
- `behavior_slots` in `src/ui/shells/library/playlist_detail.rs:218` to `220` maps the click to `LibraryApp::delete_playlist`.
- `LibraryApp::delete_playlist` in `src/library/app_impl.rs:1772` runs the command `DeletePlaylist` at once. It opens no dialog.
- No other path deletes a playlist. A search of `src/` for `DeletePlaylist`, `delete_playlist` and `playlist_delete` finds only these call sites, the command in `src/application/commands/playlist.rs:123`, `playlist_service::delete` and `db::playlist_delete`. The Playlists list rows, the sidebar, the app menu and the keys have no delete.
- The shared dialog `confirmation_dialog` in `src/ui/composites/confirmation_dialog.rs` exists. `src/ui/shells/saved_search_delete_confirmation.rs` uses it for "Delete…" of a saved search (packet 012).

### P3: The removal dialog counts entries, not playlists

The short walk of 2026-10-21 recorded: "The removal dialog says "This track is in 2 playlists" for one track that is two times in one playlist. The dialog lists only "packet 025 test"."

- `db::playlist_reference_count_for_track` in `src/db.rs:646` runs `SELECT COUNT(*) FROM playlist_tracks WHERE track_id = ?1`. A playlist with two entries of one track gives 2.
- `db::playlist_names_referencing_track` in `src/db.rs:672` runs `SELECT DISTINCT p.name`. The same playlist gives one name.
- `plan_track_removal` in `src/application/library_removal.rs:145` puts the count in `LibraryRemovalImpact::Track { playlist_reference_count }`.
- `removal_impact_message` in `src/view_models/library_removal.rs:98` makes the text "This track is in {count} {playlist|playlists}." from that count.
- `plan_selection_removal` in `src/library/app_impl.rs:4247` adds the names from `removal_referring_playlist_names`. Thus the count and the list of names come from two different queries.
- `RemovalConfirmation::WhenReferenced` reads only "count above 0". A count of distinct playlists keeps that rule.
- The test `playlist_reference_counts_detect_tracks_that_will_be_stranded` in `src/db.rs:6014` uses one entry for each track. No test has two entries of one track.

### P4: Back to a search page keeps the later text in the toolbar field

The open finding of 2026-10-21 recorded this defect. After Back to a search, the toolbar search field keeps the text of the later query. The list shows the restored query.

- `TopApp::submit_global_search` in `src/app/search_dispatch.rs:223` reads `global_search_input`. It is the one reader of the field.
- No path writes the field from a navigation entry. `restore_content_list_nav_entry` in `src/app.rs:813` restores the page of `FrameNavigationEntry::Search(query)` and starts the MusicIndex search. It does not change `global_search_input`.
- Back (`handle_content_list_back_select`, `src/app.rs:737`), Forward (`src/app.rs:753`) and a breadcrumb segment (`src/app/breadcrumb.rs:96`) all restore through that function. None of them gets a `Window`.
- `InputState::set_value` needs a `Window`. The settings fields use it in `src/app/settings.rs`.

### P5: The fixed scroll ids of the detail pages

The open finding of 2026-10-17 recorded: a track, artist, playlist, publisher or name page has one fixed scroll id for each page type. Two pages of one type, opened one after the other, thus share one scroll position.

- The fixed ids are still in the source: `"track-detail-scroll"` (`src/view_models/library.rs:748` and `src/view_models/track_page.rs:556`), `"playlist-detail-scroll"` (`src/view_models/library.rs:747`), `"publisher-page"` (`src/ui/shells/publisher.rs:70`), `"name-level-page"` (`src/ui/shells/name_level_page.rs:70`) and `"singles-group-page"` (`src/ui/shells/library/singles_group.rs:145`).
- Since packet 018, each of these pages scrolls through one held handle. `music_column_tracked_scrollbar` (`src/ui/composites/music_column_scroll.rs:47`) calls `.track_scroll(scroll)`. A tracked GPUI element reads its offset from the handle, not from the element state of its id.
- `LibraryApp::detail_scroll_handle` (`src/library/detail_selection.rs:45`) gives `detail_column_scroll`, or the Discover handle while "New releases" is open.
- `bind_detail_selection` (`src/library/detail_selection.rs:102` to `107`) sets the offset of `detail_column_scroll` to the top when the page entry changes. `TopApp` calls it at each render (`src/app.rs:1513` to `1516`).
- Thus the reading finds no shared position today. A second page of one type opens at the top. Back also opens the earlier page at the top. The app keeps no scroll position for each entry.
- The release page id from `detail_scroll_key` (`src/view_models/entity_detail.rs:999`) also has no effect when the host gives the held handle. The comment in `src/ui/composites/release_detail_surface.rs:143` to `146` says that the id keeps a scroll position for each release. That comment is not correct since packet 018.
- No test proves that a change between two pages of one type sets the offset to the top.

### P6: "Download album" on an Index track page shows no busy state

- `TrackDetailVm::page_actions` in `src/view_models/track_detail.rs:581` gives an Index track (no `library_state`) the filled action `DownloadAlbum`. Its availability is only "the track has a feed GUID" (lines 584 to 595).
- `TrackPageFacts::from_index_track` (`src/view_models/track_page.rs:176`) takes no queue fact.
- `TopApp::render_index_track_page` (`src/app/search_dispatch.rs:1323`) reads no `BusyItems`. `run_index_track_page_command` (line 1350) queues `index_release_job_by_guid` through `download_index_album_of_track` (line 1447).
- The queue drops a second job of one release (packet 022). Thus a second click does nothing, and the page tells nothing.
- The Library track page shows `subscription_message` "Downloading..." while its job runs (`src/view_models/track_page.rs:731` to `733`, test).
- `BusyItems::index_track_work(key, track_guid)` (`src/view_models/library.rs:5232`) already gives the phase of an Index track job, then the phase of a release job of the key. The Index release page rows use it (packet 006 D8).

### P7: `index_target` joins two GUIDs in one text

- `TrackKey::index_target` in `src/view_models/workspace/nav.rs:341` gives `<feed_guid>:<item_guid>`, or the item GUID alone. The keys `(None, "urn:uuid:x")` and `(Some("urn"), "uuid:x")` give the same text.
- These readers use the text as a key:
  - `resolve_track_page_source` (`nav.rs:375`) puts it in `TrackPageSource::Index { target }`.
  - `TopApp::track_page_labels` (`src/app.rs:264`) holds the opening labels by the text (`src/app/search_dispatch.rs:763`, `875`).
  - The stale-result check `index_track_detail_result_is_current` (`src/app/search_dispatch.rs:798`) and `index_track_page_track` (line 1390) compare the text.
  - `index_track_description_key` (`src/view_models/track_page.rs:215`) keys the description state.
  - `track_navigation_label` (`src/app/breadcrumb.rs:83`) reads the labels.
- No code divides the text since packet 028. A collision needs a feed GUID that is the start of a track GUID with a colon.

### P8: The search row renderer selects the art frame kind

- `render_result_row` in `src/ui/shells/search_result_rows.rs:99` to `107` draws `CollectionArt` with `CollectionKind::Artist` for each row with art. The comment says that a row with art is a name row of the Artists tab.
- `SearchResultRowVm.art` (`src/view_models/search_results/rows.rs:141`) is `Option<CollectionArtVm>`. It holds no kind.
- The other renderers that name a `CollectionKind` draw a list or a page of one kind: `src/ui/shells/library/destination_lists.rs:115`, `182` and `503`, `publisher.rs:121`, `name_level_page.rs:101`, `playlist.rs:197` and `singles_group.rs:85`. Each list function or page gives one kind to each row.

### P9: A unit test depends on the speed of the machine

- `adr_0066_paged_actor_releases_its_exclusive_configured_connection` in `src/application/paged_track_list.rs:290` failed one time in a full `cargo test` run on 2026-10-21 and passed alone. The run record does not hold the failure output.
- The test opens a database with `locking_mode=EXCLUSIVE`, gives the connection to `PagedTrackListActor` on a multi-thread `tokio` runtime, and calls `session.begin_drain()`.
- It then calls `session.wait_for_work(Duration::from_secs(1)).unwrap()` (line 316). This is the one wall-clock bound in the test. A full run starts many runtimes at the same time, and a scheduler delay of more than 1 second fails the test.
- `SessionLifecycle::spawn_actor` (`src/application/session_lifecycle.rs:137`) drops the actor future, and thus the connection, before it drops the work guard. So after `wait_for_work` gives `Ok`, the connection is closed. The two assertions after it do not depend on time.
- `wait_for_work(Duration::ZERO)` gives the state with no wait.

## Design

1. P1: write the GPUI test of R30-01 first. It must fail on the present code. Then find the cause, correct it at its owner, and record the cause in the implementation result. If the cause is the reset rule of `TrackRowMenuCell`, the rule must not clear `creating` while the menu is open.
2. P2: "Delete…" opens the shared confirmation dialog. The view model gives each text. Only the confirm handler calls `LibraryApp::delete_playlist`.
3. P3: the count of playlists counts each playlist one time: `COUNT(DISTINCT playlist_id)`. The text and the list of names thus agree.
4. P4: a restore of a `Search(query)` entry also gives the query to the toolbar field. The restore records the text as a pending value. The next render, which has a `Window`, writes it with `set_value`. A restore of another entry does not change the field.
5. P5: each history entry of the one navigation stack keeps the scroll offset of its page. Back and Forward show that offset, and a new open shows the page at the top (operator answer D6, ADR 0084 Decision 9, amended on 2026-10-21). The packet corrects the comment of `release_detail_surface.rs`.
6. P6: the Index track page reads the phase of its release job and of its track job from `BusyItems`. While a job waits or runs, "Download album" is not available, and the page shows "Queued" or "Downloading...", the same words as a release page track row.
7. P7: a typed target replaces the text. Each map and each check keys on the typed target.
8. P8: the view model gives the frame kind with the art of a search row. The renderer reads it.
9. P9: the test drives a current-thread runtime until the session has no work. It has no wall-clock bound. It keeps each assertion.

## Orchestrator Decisions

The orchestrator made these decisions on 2026-10-21. "**Flag.**" marks a decision that changes what the operator sees and that no ADR decides. The operator confirms it before the visual walk closes.

- **D1 P1 proof first.** The packet does not guess the cause. The implementer writes R30-01 and R30-02 before a change, runs them, and records the failed assertion. If they pass on the present code, the implementer stops and reports. Then the defect is outside the menu path, for example in a focus or a mouse event of the host.
- **D2 P1 all hosts.** The correction is in the shared menu (`track_row_menu.rs`, `playlist_popover.rs` or `context_menu.rs`), not in a host. Each host of the row menu then gets it.
- **D3 P2 the dialog.** **Flag, minor.**
  - The button label becomes "Delete…". Its accessibility label stays "Delete playlist".
  - The dialog title is "Delete the playlist "<name>"?". The body tells the number of entries and that the tracks stay in the Library. Example: "It has 12 entries. The tracks stay in the Library."
  - The confirm button is "Delete", in the danger style. The cancel button is "Cancel".
  - **Recommendation:** these words. They follow the "Delete…" of a saved search (packet 012).
- **D4 P3 one count.** The count query uses `COUNT(DISTINCT playlist_id)`. Keep the function name. Its one meaning is "the number of playlists that hold the track".
- **D5 P4 the field.** **Flag, minor.**
  - A restore of `Search(query)` writes `query` into the toolbar field, also on Forward and on a breadcrumb segment.
  - A restore of each other entry does not change the field. The field then keeps the text of the last search.
  - The write does not move the focus and does not submit a search.
  - **Recommendation:** these rules. A search field that shows a query that the list does not show is the defect of the walk.
- **D6 P5 the scroll position.** **Flag.** The operator chose "Remember position" on 2026-10-21. Each history entry keeps the scroll offset of its page. Back and Forward show it, and a new open shows the page at the top. See "Operator Answers - 2026-10-21".
- **D7 P5 the fixed ids stay.** The ids stay. Each page still needs one stateful element id. With the held handle, the id gives no scroll position. The guard of R30-12 blocks a page that scrolls with no held handle.
- **D8 P6 the words.** The page uses the words of `TrackRowWork`: "Queued" and "Downloading...". The words show in the message slot that the Library track page uses for "Downloading...". The filled button stays, and it is not available.
- **D9 P6 a track job.** The page reads the phase of an Index track job of its own track first, then the phase of a release job of its feed key. This is the rule of `BusyItems::index_track_work`. A job of another track of the release does not change the page.
- **D10 P7 the type.** Add one type, for example `IndexTrackTarget { feed_guid: Option<String>, item_guid: String }`, with `Ord`. `TrackKey::index_target` gives it. `TrackPageSource::Index`, `track_page_labels`, the description state map and the stale-result check use it. The names are a proposal.
- **D11 P8 search rows only.** The search row is the one renderer where the kind of the art depends on the row. The other renderers draw a list or a page of one kind, and they keep their kind. The view model of a search row gives the art and its `CollectionKind` together, for example `SearchRowArtVm { kind, art }`.
- **D12 P9 no weaker proof.** The test keeps its three assertions: the observer cannot read before the actor starts, the inbox is closed after the drain, and the observer can read after the drain. Only the wait changes.

## Operator Answers - 2026-10-21

- D6: the operator chose "Remember position". Back and Forward show a page at the scroll position of that history entry. A new open shows the page at the top. ADR 0084 Decision 9 records this, amended on 2026-10-21. This replaces the recommendation of D6, and R30-11 must prove both rules: the top on a new open, and the stored offset on Back and Forward.
- The orchestrator decided D3 and D5 by their recommendations, by the operator rule of 2026-10-20 ("decide but keep a record").

## Required Changes

1. P1: the GPUI tests R30-01 to R30-04, the recorded cause, and the correction at its shared owner.
2. P2: the label "Delete…", the dialog display in the view model, the dialog presenter, and the confirm handler.
3. P3: the count query and its tests.
4. P4: the pending toolbar text, its write at render, and the tests.
5. P5: a scroll offset for each history entry of the one navigation stack. Back and Forward restore the offset of the entry, and a new open shows the page at the top (ADR 0084 Decision 9, amended on 2026-10-21). Also the guard of the held handle and the corrected comment. If a restored page is shorter than its offset, the page shows its end.
6. P6: the queue phase in the Index track page facts, the action availability, the state word, and the tests.
7. P7: the typed target, its users, and the tests.
8. P8: the kind in the search row view model, the renderer change, and the guard.
9. P9: the new wait in the test.

## Mechanical Acceptance Criteria

Use the prefix `adr_0084_music_open_defects_` for unit tests beside the owning code. Each new guard is situational. It cites ADR 0083 Decision 5 or 6, or ADR 0084 with its decision, as the case names.

| Case | Required proof |
|---|---|
| R30-01 | P1, a GPUI test of a `TrackRow` with a menu: open "⋯", select "Add to playlist", click "New Playlist". The create mode shows: the name field and "Create & Add" have bounds. The test fails on the present code, or the implementer stops (D1) |
| R30-02 | P1, the same test through the right-click menu of the row |
| R30-03 | P1: in the create mode, a name and a click on "Create & Add" call the create handler one time with that name. The menu then closes |
| R30-04 | P1: `AddToPlaylistPopover` of the release page header shows its create mode after a click on "New Playlist" |
| R30-05 | P2: `PlaylistDetailActionsDisplay` gives the label "Delete…" and a confirmation display with the title, the body, "Delete" and "Cancel". The title holds the playlist name, and the body holds the entry count |
| R30-06 | P2, a source guard (ADR 0083 Decision 5): the delete slot of `playlist_detail.rs` opens the dialog. `delete_playlist(` is called only from the confirm handler |
| R30-07 | P3: one track with two entries in one playlist gives the count 1. One entry in each of two playlists gives 2. No entry gives 0 |
| R30-08 | P3: for each case of R30-07, the count in the message equals the number of names that `playlist_names_referencing_track` gives |
| R30-09 | P4: one pure function gives the toolbar text of a restored entry. `Search("haleen")` gives `Some("haleen")`. Each other entry gives `None` |
| R30-10 | P4: after a restore of `Search(q)` through Back, Forward or a breadcrumb segment, the pending toolbar text is `q`. A render takes it one time |
| R30-11 | P5: `bind_detail_selection` with a second entry of the same page type sets the offset of `detail_column_scroll` to the top. The same entry again does not change the offset |
| R30-12 | P5, a source guard (ADR 0084 packet 018): the track page, the feed page, the name page, the playlist page and the singles group page scroll through `music_column_tracked_scrollbar` with the held handle |
| R30-13 | P6: an Index track page with no job gives "Download album", available. A queued job gives it not available, with the word "Queued". A running job gives it not available, with "Downloading..." |
| R30-14 | P6: a job of another track of the same release does not change the page. A release job of the key does |
| R30-15 | P7: the keys `(None, "urn:uuid:x")` and `(Some("urn"), "uuid:x")` give two different targets. A label stored for one is not found for the other |
| R30-16 | P7: the stale-result check, the labels and the description states use the typed target. No `index_target()` result is formatted into a text key |
| R30-17 | P8: a Library name row of search gives its art with `CollectionKind::Artist` |
| R30-18 | P8, a source guard (ADR 0084 Decision 12): `src/ui/shells/search_result_rows.rs` names no `CollectionKind` variant |
| R30-19 | P9: the test uses a current-thread runtime and a loop with a count bound and no time bound. It keeps the three assertions of D12 |
| R30-20 | Each test of packets 006, 007, 008, 018, 022, 023, 025 and 028 still passes |

## Visual Acceptance Criteria

These are for the operator. No test proves them.

- V1 (P1): on a Library release page, open "⋯" on a track row, then "Add to playlist", then "New Playlist". The name field shows. A name and "Create & Add" make the playlist with the track. Do it again with the right-click menu, on a Tracks list row and on a playlist page row.
- V2 (P2): on a playlist page, "Delete…" opens a dialog with the playlist name. "Cancel" keeps the playlist. "Delete" deletes it, and the list column updates with no navigation.
- V3 (P3): put one track two times in a test playlist. "Remove from Library…" tells "This track is in 1 playlist" and lists that playlist one time.
- V4 (P4): search "haleen", then "survival guide". Push Back. The toolbar field reads "haleen", and the list shows "haleen".
- V5 (P5): scroll down a long track page. Open another track page from a link: it starts at the top. Push Back: the first page shows at its earlier scroll position. Push Forward: the second page shows at its earlier position. A sidebar click opens at the top.
- V6 (P6): on an Index track page, click "Download album". The button becomes not available, and "Queued" or "Downloading..." shows. When the job ends, the page changes with no navigation.

P7, P8 and P9 have no visual check. P8 keeps the art of packet 028 V2.

## Exclusions

- No scroll memory for the list column or for the Discover view. Only the detail column keeps an offset for each history entry (operator answer D6).
- No new delete path for a playlist, for example a row menu on the Playlists list.
- No change to a duplicate entry in a playlist. The menu and the drag still add a second entry.
- No change to the kind of the other collection renderers (D11).
- No change to the drag (packet 031) or to the release row menu (packet 031).
- No schema change.

## Files To Inspect

- [Agent rules](../../AGENTS.md) and the [source map](../architecture/source-map.md).
- P1: `src/ui/composites/track_row_menu.rs`, `src/ui/composites/playlist_popover.rs`, `src/ui/primitives/context_menu.rs`, `src/ui/primitives/popover.rs`, `src/ui/composites/track_row.rs`, `src/ui/composites/list_row.rs`, `src/ui/shells/playlist.rs`, `src/ui/shells/release_page.rs`.
- P2: `src/ui/shells/playlist.rs`, `src/ui/shells/library/playlist_detail.rs`, `src/view_models/library.rs` (`PlaylistDetailActionsDisplay`), `src/library/app_impl.rs` (`delete_playlist`), `src/ui/composites/confirmation_dialog.rs`, `src/ui/shells/saved_search_delete_confirmation.rs`.
- P3: `src/db.rs`, `src/application/library_removal.rs`, `src/view_models/library_removal.rs`, `src/ui/shells/library_removal_confirmation.rs`.
- P4: `src/app.rs` (`restore_content_list_nav_entry`, the render), `src/app/search_dispatch.rs`, `src/app/breadcrumb.rs`, `src/app/tab_bar.rs`.
- P5: `src/library/detail_selection.rs`, `src/ui/composites/music_column_scroll.rs`, `src/ui/composites/release_detail_surface.rs`, and the page shells in "P5" above.
- P6: `src/view_models/track_detail.rs`, `src/view_models/track_page.rs`, `src/app/search_dispatch.rs`, `src/view_models/library.rs` (`BusyItems`), `src/view_models/download_progress.rs`.
- P7: `src/view_models/workspace/nav.rs`, `src/view_models/workspace/tests.rs`, `src/view_models/track_page.rs`, `src/app.rs`, `src/app/search_dispatch.rs`, `src/app/breadcrumb.rs`.
- P8: `src/ui/shells/search_result_rows.rs`, `src/view_models/search_results/{rows,results}.rs`, `src/view_models/collection_art.rs`, `src/ui/shells/search_results_inspector.rs`.
- P9: `src/application/paged_track_list.rs`, `src/application/session_lifecycle.rs`, `src/runtime/actor.rs`.
- `tests/architecture_tests.rs`.

## Checks

```bash
cargo test --lib adr_0084_music_open_defects_
cargo test --lib adr_0066_paged_actor_releases_its_exclusive_configured_connection
cargo test --lib adr_0083_row_actions_
cargo test --lib adr_0083_track_drag_
cargo test --lib adr_0084_search_open_defects_
cargo test
cargo test
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy --all-targets -- -D warnings
cargo check --all-targets
touch src/main.rs && cargo build --bin v4vmm
```

Run the full `cargo test` three times. Each run must be Green, so that the P9 test passes under the load of the full suite.

## Rollback

Revert the commit. This packet adds no migration and no stored data. V1, V2 and V3 change playlists and the Library. Use a test playlist and a track that you can download again.

## Implementation Result - 2026-10-21

### The Cause Of Each Defect

- P1: the reset rule of `TrackRowMenuCell` read the wrong open state. GPUI 0.3.1 renders each stateless component under the element id of its type name (`ViewElement::request_layout`). Thus `ContextMenu::is_open` in the render of the cell read the keyed state `[.., TrackRowMenuCell, "{id}-state"]`. The open menu kept its state at `[.., TrackRowMenuCell, ContextMenu, "{id}-state"]`. The cell always read "closed", so each render after "New Playlist" set `creating` to false again.
  - Proof before the change: R30-01 failed at the assertion "after "New Playlist", the name field must show". The click on `pl-new` occurred, and the create mode did not show.
  - R30-02 passed before the change. The right-click menu records `pointer`, and the reset rule acts only when `pointer` is `None`. Thus only the "⋯" path had the defect. The walk used the "⋯" path.
  - Correction at the shared owner: `ContextMenuHandle` in `src/ui/primitives/context_menu.rs`. The cell makes the handle at its own position and gives it to the menu with `ContextMenu::handle`. The menu and the reset rule read one state. `ContextMenu::is_open` is deleted. Each host of the row menu gets the correction.
  - The same cause also hid the "⋯" cell while its menu was open and the pointer was off the row. The cell now stays visible while the menu is open (R83-6-12).
- P2: the delete slot called `LibraryApp::delete_playlist` at once. The slot now opens `open_playlist_delete_dialog` in `src/ui/shells/playlist_delete_confirmation.rs`. Only its confirm handler deletes the playlist.
- P3: `db::playlist_reference_count_for_track` counted entries. It now counts `DISTINCT playlist_id`. `db::playlist_names_referencing_track` now gives one name for each playlist, so two playlists with the same name give two names. The count and the list of names agree.
- P4: no path wrote the toolbar field from a navigation entry. `restore_content_list_nav_entry` now records the query of a `Search` entry in `PendingToolbarSearchText`. The next `TopApp::render` writes it with `set_value` when the field holds another text. The write does not move the focus and does not submit a search.
- P5: the recorded facts were correct: each page change set the top. The operator answer of D6 asks for a stored offset.
  - Each history entry (`FrameLocation`) now has an id and a stored offset. Back and Forward set `NavigationArrival::History`, and each other move sets `Open`.
  - `LibraryApp::bind_detail_selection` stores the offset of the entry that closes and sets the offset of the new entry.
  - The comment of `release_detail_surface.rs` is correct now.
- P6: `TrackPageFacts::from_index_track` took no queue fact. `with_index_track_work` now reads `BusyItems::index_track_work`. "Download album" is not available while a job waits or runs, and the message slot shows "Queued" or "Downloading...".
- P7: `TrackKey::index_target` gave one text. It now gives `IndexTrackTarget`. The labels, the description states, the stale-result check and the track page source key on the type. Its `Display` text is only for the "ID" row and for an element id.
- P8: `SearchResultRowVm::art` is now `SearchRowArtVm { kind, art }`. The renderer reads the kind.
- P9: the test used a multi-thread runtime and a bound of 1 second. It now drives a current-thread runtime with `yield_now` for at most 1000 turns. The actor is a plain task, so it runs only inside `block_on`. The three assertions of D12 stay.

### Behavior Changed

- "New Playlist" in the "⋯" menu of a track row shows the name field. The "⋯" cell stays visible while its menu is open.
- "Delete…" on a playlist page opens a confirmation. The button label is "Delete…".
- The removal dialog counts each playlist one time.
- Back, Forward or a breadcrumb segment to a search page writes its query into the toolbar field.
- Back and Forward show a detail page at its stored offset. A new open shows it at the top. A page that is shorter than the offset shows its end. A page that loads after its first draw gets the offset when it becomes long enough, until the operator scrolls it.
- An Index track page shows "Queued" or "Downloading..." and makes "Download album" not available while a job waits or runs.

### Deviations

- `Button` sets its element id as the debug selector in a test build only. The GPUI tests find each button by its id.
- The name field of the create mode has the debug selector `pl-name-field` in a test build only.
- `reset_detail_selection` does not set the offset any more. `bind_detail_selection` sets it, because it must first read the offset of the page that closes.
- Two guards changed their expected text: `adr_0084_detail_selection_r18_08_a_page_change_clears_the_detail_selection` now expects `bind_detail_selection(entry, target)`, and `adr_0024_index_track_detail_uses_rich_track_view_path` now expects `TrackPageOrigin::Index(_)`.
- The Discover "New releases" view keeps its own scroll handle. The stored offset applies to the detail column handle only.

### Tests And Guards

- Unit and GPUI tests:
  - R30-01 to R30-04: `track_row_menu.rs` and `playlist_popover.rs`.
  - R30-05: `view_models/library.rs` and `playlist_delete_confirmation.rs`.
  - R30-07: `db.rs`. R30-08: `application/library_removal.rs`.
  - R30-09, R30-10 and R30-15: `view_models/workspace/tests.rs`.
  - R30-11: `library/detail_selection.rs`.
  - R30-13 and R30-14: `view_models/track_page.rs`. R30-17: `view_models/search_results/tests.rs`.
- Guards in `tests/architecture_tests.rs`: R30-06, R30-10 (the restore paths), R30-12, R30-16 and R30-18.
- R30-19: `adr_0066_paged_actor_releases_its_exclusive_configured_connection`.
- R30-20: the full suite.

### Review Corrections - 2026-10-21

- P4: only Back, Forward and a breadcrumb segment record the toolbar text. `restore_content_list_nav_entry`, `open_music_page` and a new submit do not. The write compares `input.value().trim()`.
- P5: a closed entry with an unmet offset keeps that offset, not the shorter offset its page could show.
- P5: a click or a key in the detail column clears the pending offset. A growth of the page after that click or key does not jump it.
- P5: `replace_current` gives the replaced entry a new scroll memory. It does not inherit the earlier page's offset.

## Orchestrator Review - 2026-10-21

Pass, with the four corrections of "Review Corrections - 2026-10-21". Only Back, Forward and the breadcrumb write the toolbar field, and a new search does not. An entry that closes before its page reaches the stored offset keeps that offset. A click or a key in the detail column ends a stored offset that the page did not reach. `replace_current` gives the new page its own scroll memory.

## Operator Visual Check

Use the release binary or the debug binary that `cargo build --bin v4vmm` makes. Use a test playlist and a track that you can download again.

1. V1, the row menu:
   1. Start the app: `cargo build --bin v4vmm && ./target/debug/v4vmm`.
   2. Open a Library release page. Put the pointer on a track row and click "⋯".
   3. Click "Add to playlist", then "New Playlist". The name field and "Create & Add" must show.
   4. Type "packet 030 test" and click "Create & Add". The menu must close. The playlist must show in the sidebar list with the track.
   5. Do steps 2 to 4 again with a right-click on the row.
   6. Do steps 2 to 4 again on a Tracks list row and on a playlist page row.
   7. Wrong: a click that does nothing, a menu that closes before the name field shows, or a "⋯" button that hides while its menu is open.
2. V2, the playlist delete:
   1. Open the playlist page of "packet 030 test". The delete button must read "Delete…".
   2. Click "Delete…". The dialog title must hold the playlist name, and the body must tell the entry count.
   3. Click "Cancel". The playlist must stay.
   4. Click "Delete…" again, then "Delete". The playlist must go from the list column with no navigation.
3. V3, the removal count:
   1. Make a test playlist. Add one downloaded track to it two times.
   2. On the track row, select "Remove from Library…". The dialog must say "This track is in 1 playlist." and list the playlist one time.
   3. Click "Cancel".
4. V4, the toolbar field:
   1. Search "haleen", then search "survival guide".
   2. Push Back. The toolbar field must read "haleen", and the list must show the results of "haleen".
   3. Push Forward. The field must read "survival guide".
   4. Wrong: a field that keeps the text of the other query, or a field that takes the focus.
5. V5, the scroll position:
   1. Open a long track page or a long playlist page and scroll down.
   2. Open another page of the same type from a link. It must start at the top.
   3. Push Back. The first page must show at its earlier scroll position.
   4. Push Forward. The second page must show at its earlier position.
   5. Click a sidebar row. The page must open at the top.
6. V6, the Index track page:
   1. Search for a track that the Library does not hold, and open its page.
   2. Click "Download album". The button must become not available, and "Queued" or "Downloading..." must show below the action row.
   3. When the job ends, the page must change with no navigation.

Cleanup: delete each test playlist. Remove the album of V6 from the Library if you do not want it.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `docs/architecture/source-map.md`
- This packet: `docs/tasks/adr-0084-task-030-music-open-defects.md`
- Packets 006, 007, 008, 018, 022, 023, 025 and 028
- Each file in "Files To Inspect"

Goal:
- Make each change in "Required Changes".

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- The view model gives each text, each availability and each kind. The screen only composes. Use named tokens.
- For P1, write R30-01 and R30-02 first and run them before a change (D1).
- Tests use recorded values. No test sends a request to a live service.
- Never run `git add`, `git rm`, `git commit`, `git checkout`, `git restore`, `git stash` or `git reset`.
- Do not run the app.

Do not touch:
- The database schema.
- Any ADR, and each document other than this packet.

Acceptance criteria:
- Each case R30-01 to R30-20 has a passing test or a guard.
- Each command in "Checks" is Green.
- The packet has an "Implementation Result - <date>" section with the cause of P1, and an "Operator Visual Check" section for V1 to V6.

At the end, report: files changed, tests run, behavior changed, deviations, unresolved concerns, and the document changes that the orchestrator must make.

Stop and report the problem, and do not guess, when:
- R30-01 and R30-02 pass on the present code (D1).
- A change needs a new architecture decision.
- A change needs a file in "Do not touch".
