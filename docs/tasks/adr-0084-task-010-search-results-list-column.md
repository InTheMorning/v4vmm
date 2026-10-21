# ADR 0084 Task 010: Search Results In The List Column

Status: Ready - 2026-10-20

## Goal

The toolbar search shows its results in a view that fits the width of the list column. The view has three tabs: Releases, Artists and Tracks.
Each tab shows each item one time, with the state word of each Library item. A click on a row opens the one page of that item in the detail column.

The Artists tab gets the feeds that `GET /v1/search?type=publisher` finds. A click on such a row opens its feed page. This fixes finding 6 of the Music flow map.
The Releases tab and the Tracks tab get the selection model, the selection mode and the action bar of packets 018 and 019.

## Authority

- [ADR 0084](../adr/0084-music-column-browser.md):
  - Decision 2: search is a toolbar command. Its results fill the list column, and a result opens in the detail column.
  - Decision 4: one page for each item. A Library row and an Index row of one item open the same page.
  - Decision 5: the curator sees "Release" in place of "Feed", and each release shows its kind.
  - Decision 8: search shows everything, with the state word on each Library item. No control shows Index items only.
  - Decision 9: each page change goes into the one navigation stack.
  - Decision 10, amended on 2026-10-19 and on 2026-10-20: the selection model, the two modes, the bar and the download queue.
- [ADR 0085](../adr/0085-artists-by-confirmation.md) Decisions 1, 2 and 7: no interface text calls a feed a "publisher". A level comes from link facts only. The frame title of a feed page comes from its own facts.
- [ADR 0047](../adr/0047-library-search-unification.md), as ADR 0084 amends it: the search results are one content surface for both origins.
- [ADR 0077](../adr/0077-publisher-feed-artist-binding.md) task 006: a name match is a search result, `Tracks matching "<name>"`, not an artist.
- [ADR 0063](../adr/0063-show-dashboard-layout.md): column text does not call `truncate()`.
- [ADR 0075](../adr/0075-metadata-ownership-and-completeness.md) section 6: the stored MusicIndex contract changes in the same change as the requests that use it.
- Operator decisions:
  - 2026-10-12: task 004 put the present search view in the list column.
  - 2026-10-16: an Index item shows no state word.
  - 2026-10-18: packet 010 owns finding 6.
  - 2026-10-20: packet 010 adds the selection model and the selection mode to the search results.
- [Open Stophammer requests](../plans/v4vmm-open-requests.md): request 8 item 5 asked for `type=publisher`. Release 0.12.0 delivered it on 2026-10-18.

## Recorded Facts - 2026-10-20

### The present view

- `render_search_results_inspector` in `src/ui/shells/search_results_inspector.rs` draws the view. `render_inspector_header` puts the query, the filter caption and the tab control in one row.
- `render_tab_strip` gives three segments in this order: Artists, Feeds, Tracks. `SearchResultsTab` in `src/view_models/search_results/tabs.rs` has the same order, and `Artists` is the default.
- `render_result_window` draws one window of rows. `render_result_row` in `src/ui/shells/search_result_rows.rs` draws each row: a thumbnail, two text lines, an origin word and a kind badge. Each text line calls `.truncated()`.
- `origin_label_text` gives "In Library" for a Library row and "Index" for an Index row. The operator decision of 2026-10-16 gives an Index item no word.
- `TopApp::with_search_results_list` in `src/app.rs` mounts the view in the list column. It also mounts the filter chips "All", "Library" and "Index" through `content_list_filter_chip_strip`.
- The search view is the only caller of the filter chips. `ContentFilter` (`src/view_models/workspace/chrome.rs`), `filter_chip_strip` (`src/ui/composites/filter_chip_strip.rs`) and the filter slot of `frame_shell` have no other live caller.
- `SearchResultsPagedTab` in `src/view_models/search_results/paged_tab.rs` keeps three windows for each tab: all, Library and Index.

### The rows

- `search_results_detail_for_query` in `src/app/search_dispatch.rs` reads the Library rows through `search_local_library_tracks`. The SQL in `db::search_library_tracks` (`src/db.rs`) reads only tracks with `is_in_library = 1`.
  A Library release with no downloaded track thus gets no Library row.
- `local_library_result_rows` in `src/view_models/search_results/results.rs` groups the Library tracks. It gives one feed row for each feed id (id `<feed_id>`), and one artist row for each artist text (id `library-artist:<name>`).
- `fetch_index_search_result_rows` in `src/application/queries/search.rs` sends two requests: `type=feed` and `type=track`, each with `limit` 20 (`api::PAGE_LIMIT`). It builds no row from a third type.
- The Index row ids are `index-feed:<feed_guid>`, `index-track:<feed_guid>:<track_guid>` and `index-artist:<name>`. `IndexArtistCandidate::into_display` gives the name row `Tracks matching "<name>"`.
- `SearchResultsPagedTab::replace_index_rows` puts the Library rows first and the Index rows after them. A release that is in the Library and in the Index results shows two times.

### The clicks

- `handle_search_result_selected` in `src/app/search_dispatch.rs` parses the row id and opens the page:
  - `index-track:` goes to `handle_index_track_result_selected`. A Library track id goes to `open_track_in_frame`.
  - `index-feed:` goes to `handle_index_feed_result_selected`. A Library feed id goes to `open_library_release_in_frame`.
  - `index-artist:` goes to `open_name_match_page`. `library-artist:` goes to `open_library_artist_in_frame`, the Library artist page by name.
- Each of these functions pushes an entry into the content frame stack. The search location keeps `FrameNavigationEntry::Search(query)` as its root (`active_search_query` in `src/view_models/workspace/nav.rs`).
- No function in `src/app/search_dispatch.rs` builds `FrameNavigationEntry::PublisherDetail` (flow map finding 6). `TopApp::open_publisher_page` in `src/app/publisher_dispatch.rs` pushes it, with a `PublisherPageContext` of `Library` or `Index`.
- The Artists list resolves each album through `ArtistIdentity` (`artists_list` and `artist_list_row` in `src/view_models/library.rs`). A feed-level row opens `PublisherDetail`. A name row opens `ArtistDetail(name)`.

### The stale-result checks

- `dispatch_index_search` writes a result only when `content_list_nav_matches_search` is true and `detail.query()` equals the query of the request.
- `index_feed_detail_result_is_current` and `index_track_detail_result_is_current` in `src/app/search_dispatch.rs` drop a page result for a replaced key. Tests R47-07 pin them.
- `publisher_page_result_is_current` in `src/app/publisher_dispatch.rs` drops a feed page result for a replaced GUID.
- `sync_search_results_detail_with_nav` builds a new view model when the query of the location changes.

### The selection

- `DetailSelection` in `src/view_models/detail_selection.rs` holds a `ListSelection<DetailRowKey>`, a `SelectionMode` and the bound entry. `DetailPageRows::action_bar` gives the bar of packet 018.
- `DetailTrackRow` holds a `TrackKey`, so an Index track row can have a key. `DetailReleaseRow` holds a `ReleaseKey` and optional `LibraryReleaseFacts`.
- `download_detail_selection` in `src/app/detail_selection.rs` reads `library.detail_selection()`. An Index track needs the `FeedView` of the open release page (`index_release_page_feed`).
- `DownloadJob::IndexTrack` in `src/library/download_queue.rs` needs a `TrackContext`. `DownloadJob::IndexRelease { feed: None }` reads its feed in the slot of the job.
- `library_track_id_for_key` in `src/app/search_dispatch.rs` finds a Library track id from a `TrackKey`. `LibraryViewModel::library_release_facts` gives the facts of a release key.
- Each list key handler in `src/app/keyboard.rs` asks each column to act. Each column checks its own focus handle.

### MusicIndex, read on 2026-10-20

- `GET /node/info` gave `git_revision` `056199b`. `GET /openapi.json` gave `info.version` `0.12.0`, with sha256 `937491997d3e2147db277365b0a7e8602f9d6f58c57996c7939a68e143fa9133`.
- The 0.12.0 contract describes `type` on `GET /v1/search` as "`feed`, `track`, or `publisher`". The stored contract `tests/fixtures/musicindex-openapi-0.11.0.json` names only `feed` and `track`.
- `GET /v1/node/capabilities` lists `publisher` in `entity_types`.
- `GET /v1/search?q=Libre&type=publisher` gave two rows:
  - `entity_type` `publisher`, `entity_id` `34bf795f-f551-4db1-8877-ad19aa5f89c8`, `title` "Sir Libre Records", `episode_count` 0.
  - `entity_type` `publisher`, `entity_id` `c8ec3dba-ed28-5662-9509-094d769f9597`, `title` "Kolomona Myer - AKA Sir Libre", `episode_count` 0.
- A publisher row gives no `feed_guid`. The `entity_id` is the feed GUID. The row gives no link fact, and `include=link_facts` on the search changes nothing.
- `GET /v1/feeds/by-name?medium=publisher&include=link_facts&from=Sir%20Libre` gave Sir Libre Records with `agreed_roles` `["label"]` and `first_named_count` 0. A classification of a search row thus needs a second read for each row.

### Guards that pin the present view

`tests/architecture_tests.rs`:

- `adr_0047_phase_b_view_model_contracts_are_gpui_free_and_shared`: it requires `filter_chip_strip` and `filter_chip_strip_for_width_class` in the search view model, and `ContentFilter` in the workspace chrome.
- `adr_0047_phase_d_filter_controls_render_through_frame_shell`: it requires the filter chip composite and the search filter chips.
- `adr_0047_task_014_search_results_inspector_shell_contract`: it requires the three `Segment::new(tab_segment_display(...))` lines, with `SearchResultsTab::Feeds`.
- `adr_0049_inspector_source_ownership_is_guarded`: it requires `SearchResultsHeaderMode::Tabbed`.
- `adr_0066_search_failure_report_stays_readable_and_vm_owned`, `shared_search_result_rows_accept_resolved_artwork_thumbnails`, `adr_0048_index_search_is_async_and_vm_owned` and `search_results_detail_syncs_with_search_nav_flow`.
- `adr_0024_index_track_detail_uses_rich_track_view_path`, `adr_0024_loading_shape_readiness_gate_is_locked`, `adr_0083_both_track_page_origins_build_one_shared_surface` and `adr_0084_scrollbars_r14_01_music_columns_use_the_shared_helper` read `search_results_inspector.rs`.
- The contract field guard of ADR 0075 task 051 reads `MUSICINDEX_CONTRACT_FIXTURE`.

Unit tests: `src/view_models/search_results/tests.rs`, `src/ui/shells/search_result_rows.rs` (`tab_ids_are_stable`, `origin_labels_use_membership_language`) and `src/view_models/workspace/tests.rs` (`default_for_search_inspector`).

**The stored contract (orchestrator decision of 2026-10-20).** Packets 010, 016 and 017 each need the 0.12.0 contract. The first of the three that an implementer starts replaces `tests/fixtures/musicindex-openapi-0.11.0.json` with `musicindex-openapi-0.12.0.json`, with the guard constant and the `AGENTS.md` line. A later packet finds the 0.12.0 file and changes only its own decode.

## Design

### The view at the width of the list column

- The header has two rows. The first row shows the query as the headline. The second row shows the tab control, with the full width of the column.
- The tabs are "Releases", "Artists" and "Tracks", in that order. `SearchResultsTab::Feeds` becomes `SearchResultsTab::Releases`. A tab label shows no count.
- When the Index results arrive, the first tab with rows opens, as `select_first_populated_tab_if_automatic` does today. A tab that the operator selected stays.
- Each row uses the layout of a Releases list row: the checkbox position of packet 019, the thumbnail, the title, the second line and the state word. Read that renderer first, and use its composite.
- No stacked text calls `truncate()` or `.truncated()`. Follow ADR 0063 and the Releases list row.
- The filter chips go. The view model holds one window for each tab, not three.

### One row for each item

- A release row has a `ReleaseKey`. A Library feed with a feed GUID and an Index hit with the same GUID give one row. The Library facts of that row come from `library_release_facts`.
- An Index release hit whose GUID is a Library feed is a Library row, also when the local search did not find it.
- A track row has a `TrackKey`. An Index track hit whose key gives a Library track id through `library_track_id_for_key` is a Library row.
- The Library rows come first, in their present order. The Index rows come after them, in rank order. A merged row keeps the place of its Library row.
- A Library release row shows the state word of Decision 4: "In Library", "n of m tracks" or "Not downloaded". A queued or running job shows "Queued" or "Downloading". Use the words that the release rows already give.
- A Library track row shows "In Library" or "Not downloaded", or the queue word.
- An Index row shows no state word.
- A release row shows its kind badge of Decision 5. An Index hit uses `ReleaseKind::resolve(None, episode_count)`. A track row shows no kind badge.

### The Artists tab

- A third request, `type=publisher`, adds the feed rows. Its rows have the id `index-feed-page:<entity_id>`.
- No row text says "Publisher". The kind badge of a feed row is "Artist" when the Library Artists list has a feed-level row with that GUID. Each other feed row has the badge "Feed".
- A feed row shows no level word and no role word. The feed page shows them (ADR 0085 Decisions 2 and 7).
- The name rows `Tracks matching "<name>"` stay, with their present label and page.
- A Library artist row resolves through `ArtistIdentity`, as the Artists list does. A feed-level row has the id `library-feed-page:<feed_guid>`, and a name row keeps `library-artist:<name>`.
- One feed GUID gives one row. A Library feed-level row and an Index feed row of one GUID merge, and the Library row stays.
- The Artists tab has no selection model, as the Artists list has none.

### The clicks

- Add a pure function `SearchResultTarget::from_row_id(tab, id)` in the search view model. It gives `Release(ReleaseKey)`, `Track(TrackKey)`, `FeedPage(String)`, `NameMatches(String)` or `ArtistByName(String)`.
- `handle_search_result_selected` matches the target and calls the present open function. It parses no id.
- `FeedPage(guid)` calls `open_publisher_page`. The context is `Library` when the Library tree names that feed, else `Index`.
- Each open function pushes its entry into the one stack. Back returns to the results with the same tab.

### The selection

- `SearchResultsInspectorPageVm` holds one `DetailSelection`, bound to `FrameNavigationEntry::Search(query)`.
- Add `search_release_rows` and `search_track_rows` in `src/view_models/detail_selection.rs`. They build `DetailPageRows::Releases` and `DetailPageRows::Tracks` from the merged rows, in visible order.
- The list column header shows "Select" or "Done" on the Releases tab and on the Tracks tab. The button shows only when the tab has a row with a key. `detail_header_action` gives it.
- A tab change clears the selection and sets `SelectionMode::Normal`.
- The arrival of the Index rows calls `retain_visible`. A selected Library row stays selected, and the focus stays.
- The bar is the release bar or the track bar of packet 018. It shows at the bottom of the list column.
- TopApp holds a focus handle for the search list. The key handlers of `src/app/keyboard.rs` also ask the search list to act, and it acts only when it has the focus.

### The actions

- Change `download_detail_selection`, `detail_library_ids` and `detail_removal_items` so that each takes the rows and the selection as arguments. The detail column and the search list then share them.
- "Download" of an Index release queues `IndexRelease { feed: None }` through `download_index_album_of_track`, as packet 018 does.
- "Download" of an Index track from the search queues one job that reads the feed of the track in its own slot. Then it finds the track by its GUID. Add this as `DownloadJob::IndexTrack` with no context, or as a new variant. Keep the identity rules of packet 022.
- An Index track with no feed GUID has no key, so it has no checkbox.
- "Add to playlist" and "Remove" act on the Library items only, as in packet 018.

### The stale results

- Keep each check of "The stale-result checks".
- A result for query A that arrives after the location changed to query B changes neither the rows nor the selection of query B.
- The `type=publisher` request is part of `FetchIndexSearchResults`. Its failure alone shows the other rows. A failure of all three requests gives the present failure state.

### The contract

- Replace `tests/fixtures/musicindex-openapi-0.11.0.json` with the 0.12.0 contract, as `tests/fixtures/musicindex-openapi-0.12.0.json`. Change `MUSICINDEX_CONTRACT_FIXTURE`.
- Get the file with one `GET https://api.musicindex.org/openapi.json`. Check that `info.version` is `0.12.0`.
- Change the stored contract line in "Stored facts" of `AGENTS.md`.

## Orchestrator Decisions

A decision marked **Flag** changes what the operator sees, and no ADR decides it. The operator can change each flagged decision at the visual check.

1. **Flag.** The view keeps three tabs, not three sections in one list. The tab control is on its own row under the query. The tab order is Releases, Artists and Tracks. A tab label shows no count.

   - Reason: one list of one kind keeps the selection model and the bar of packet 018 unchanged.
   - A segmented control fits the width of the column.
2. The filter chips "All", "Library" and "Index" go. `ContentFilter`, the filter chip composite and the filter slot of the frame shell go with them, because no caller stays.
   ADR 0084 Decision 8 decides this. Packet 016 builds its own two-state scope control. It can copy the pattern from version control.
3. One row for each item. The Library rows come first, then the Index rows in rank order. This keeps the present order.
4. An Index row shows no state word. The word "Index" goes. The operator decision of 2026-10-16 decides this.
5. **Flag.** The feed rows of `type=publisher` go in the Artists tab. A feed row has the badge "Artist" only when the Library Artists list has that feed. Each other feed row has the badge "Feed".

   - Reason: a search row has no link facts, and a read for each row is the pattern that request 8 removes.
   - ADR 0085 Decision 2 decides the Artists list, not the search tab.
   - A label can thus show in the Artists tab with the badge "Feed". Its feed page shows the frame title "Label".
6. A Library artist row resolves through `ArtistIdentity`, as the Artists list does. A feed-level row opens the feed page. A name row opens the Library artist page by name until packet 017.
7. A feed row opens its feed page with the Library context when the Library tree names the feed. Else it uses the Index context.
8. **Flag.** A tab change clears the selection and ends selection mode. Reason: a bar for releases and tracks together has no owner, and one bar acts on one tab.
9. The Artists tab has no "Select" button, as the Artists list has none.
10. An Index track from the search downloads through a job that reads its feed in its own slot. The job of an Index release does the same today.
11. A release row shows its kind badge, and a track row shows no badge. Decision 5 decides the kind.
12. The contract fixture becomes the 0.12.0 contract in this packet, because this packet is the first one that sends `type=publisher`.
13. The empty state of a tab names the tab and the query. An example is "No releases matched "bluegrass"." The empty state has no clear-filter action, because no filter stays.

## Required Changes

1. Rename `SearchResultsTab::Feeds` to `Releases`. Put the tabs in the order Releases, Artists, Tracks. Change the labels and the element ids.
2. Delete the filter chips of the search view, `set_search_results_filter` and `ContentFilter`. Delete the filter chip composite and the filter slots of `frame_shell` and `workspace`. Give `SearchResultsPagedTab` one window.
3. Merge the Library rows and the Index rows by `ReleaseKey` and by `TrackKey`. Give each Library row its state word and its Library facts. Give each Index row no word.
4. Add the `type=publisher` request and the feed rows. Resolve the Library artist rows through `ArtistIdentity`.
5. Add `SearchResultTarget` and route `handle_search_result_selected` through it. Add the feed page entry.
6. Change the header and the rows to the layout at the width of the list column. Remove `.truncated()` from stacked text.
7. Add the selection, the header button, the keys, the bar and the shared action functions.
8. Add the download of an Index track that reads its feed in its own slot.
9. Replace the contract fixture and its constant. Change the stored contract line in `AGENTS.md`.
10. Update the guards in "Guards that pin the present view". Delete the text of a replaced rule in the same change. Add the guards of this packet.

## Mechanical Acceptance Criteria

Use the prefix `adr_0084_search_list_` for unit tests beside the owning code. Each new guard cites ADR 0084 Decision 2, 8 or 10. Tests use recorded rows and a recorded response. No test sends a request to a live host.

| Case | Required proof |
|---|---|
| R10-01 | `SearchResultsTab` gives Releases, Artists and Tracks in that order, with those labels. The search view model holds no `ContentFilter` |
| R10-02 | A Library release and an Index hit with one feed GUID give one release row with the Library state word. An Index-only hit gives a row with no state word |
| R10-03 | An Index release hit whose GUID is a Library feed with no downloaded track gives a Library row with "Not downloaded" |
| R10-04 | A Library track and an Index hit with one `TrackKey` give one track row. The Library rows come before the Index rows |
| R10-05 | `fetch_index_search_result_rows` sends `type=feed`, `type=track` and `type=publisher` to a local test server. The recorded Libre response gives two feed rows with the ids `index-feed-page:<guid>` |
| R10-06 | A feed row has the badge "Artist" when the Library Artists list has its GUID, else "Feed". No row label or accessibility label of the view model contains "Publisher" |
| R10-07 | `SearchResultTarget::from_row_id` gives the correct target for each of the six id forms |
| R10-08 | A guard: `src/app/search_dispatch.rs` builds `PublisherDetail` through `open_publisher_page` for `SearchResultTarget::FeedPage`. It cites flow map finding 6 |
| R10-09 | In `Normal`, no row view model shows a checkbox. In `Selecting`, each Releases row and each Tracks row with a key shows one. The Artists tab gives no header action |
| R10-10 | A tab change clears the selection and gives `Normal`. The arrival of the Index rows keeps a selected Library row selected |
| R10-11 | The two search bars give the counts of packet 018 R18-06 for recorded Library rows and Index rows |
| R10-12 | An Index track job with no context reads its feed in its slot, and keeps the identity rules of packet 022 R22-04 |
| R10-13 | A result for query A after the location changed to query B changes neither the rows nor the selection of query B. Each R47-07 test still passes |
| R10-14 | The name row label `Tracks matching "Survival Guide"` does not change. Each ADR 0077 task 006 test still passes |
| R10-15 | The contract field guard reads `musicindex-openapi-0.12.0.json`. Each packet 018, 019, 020 and 022 test still passes |

## Visual Acceptance Criteria

These are for the operator. No test proves them. V1 and V2 carry §23 V1 and §23 V4 of the overhaul plan.

- V1 (§23 V1): a search for "Survival Guide" shows a row `Tracks matching "Survival Guide"`. The row does not look like an artist identity.
- V2 (§23 V4, row only): normal and narrow widths show each element of the row in its place, in Light and Dark themes, with no clipped text.
- V3: the header shows the query on one row and the tabs on the next row. No filter chip shows. The rows fit the list column with no horizontal scroll.
- V4: a release that is in the Library and in the Index shows one time, with its state word. An Index item shows no word.
- V5: a search for "Libre" shows Sir Libre Records in the Artists tab with the badge "Feed". A click opens its feed page in the detail column, and the frame title is "Label".
- V6: "Select" on the Releases tab shows the checkboxes. "Download 3 releases" queues three downloads, and the progress item shows them. A tab change ends selection mode.
- V7: Back and Forward return to the results, with the same tab. The breadcrumb starts with the search.

## Exclusions

- No scope control. Packet 016 owns the scopes of Releases and Artists.
- No name level page. Packet 017 replaces the Library artist page by name and the name match page.
- No saved searches. Packet 012 adds them.
- No paging of the search results beyond 20 rows for each type.
- No level word and no role word on a search row. No read for each row.
- No change to the name match page, the release page, the track page or the feed page.
- No new Stophammer request. The orchestrator can add a request for link facts on search rows.

## Files To Inspect

- [Agent rules](../../AGENTS.md) and the [source map](../architecture/source-map.md).
- `src/ui/shells/search_results_inspector.rs`, `src/ui/shells/search_result_rows.rs`, `src/ui/shells/name_match_page.rs`.
- `src/view_models/search_results/` (each file), `src/view_models/detail_selection.rs`, `src/view_models/list_selection.rs`, `src/view_models/selection_action_bar.rs`.
- `src/app.rs` (`with_search_results_list`, `set_search_results_filter`), `src/app/search_dispatch.rs`, `src/app/detail_selection.rs`, `src/app/publisher_dispatch.rs`, `src/app/keyboard.rs`.
- `src/application/queries/search.rs`, `src/api.rs` (`search`, `SearchResult`).
- `src/view_models/library.rs` (`artists_list`, `artist_list_row`, `library_release_facts`, the Releases list rows), `src/view_models/release_kind.rs`.
- `src/library/download_queue.rs`, `src/library/downloads.rs`.
- `src/view_models/workspace/chrome.rs`, `src/ui/composites/filter_chip_strip.rs`, `src/ui/composites/frame_shell.rs`, `src/ui/shells/workspace.rs`.
- The archived packets 009, 018 and 019, and packets 020 and 022.
- `tests/architecture_tests.rs`: each guard in "Guards that pin the present view".

## Checks

```bash
cargo test --lib adr_0084_search_list_
cargo test --lib adr_0084_detail_selection_
cargo test --lib adr_0084_download_queue_
cargo test --lib adr_0077_name_matches
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy -- -D warnings
cargo check --all-targets
cargo build --bin v4vmm
```

## Rollback

Revert the commit. This packet adds no migration and no stored data. The revert also restores the 0.11.0 contract fixture.

## Operator Answers - 2026-10-20

- D1: confirmed: three tabs, Releases, Artists and Tracks.
- D5: publisher rows stay labeled "Feed" until Stophammer request 9 item C gives link facts on search rows.
- D8: confirmed: a change of tab clears the selection and ends selection mode.
- ADR 0084 Decision 7, amended on 2026-10-20: a name that matches exactly one artist feed has no name row. Search shows the feed row of that artist instead. Packet 017 owns the match function.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `docs/architecture/source-map.md`
- This packet: `docs/tasks/adr-0084-task-010-search-results-list-column.md`
- ADR 0084 Decisions 2, 4, 5, 8, 9 and 10, and ADR 0085 Decisions 1, 2 and 7
- The archived packets 009, 018 and 019, and packets 020 and 022
- Each file in "Files To Inspect"

Goal:
- Make each change in "Required Changes": the tabs, the merged rows, the feed rows, the targets, the layout, the selection and the contract.

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- The view model gives each text, each state word, each badge and each availability. The screen only composes. Use named tokens and named icons.
- Keep each stale-result check of "Recorded Facts".
- Tests use recorded rows and recorded responses. No test sends a request to a live host or writes a file.
- Send one `GET https://api.musicindex.org/openapi.json` for the fixture, and no other live request.
- Never run `git checkout`, `git restore`, `git stash`, `git reset`, `git add` or `git commit`.
- Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- The name match page, the release page assembly, the track page and the feed page.
- The database schema and the migration registry.
- Any ADR, and each document other than this packet and the stored contract line of `AGENTS.md`.

Acceptance criteria:
- Each case R10-01 to R10-15 has a passing test or guard.
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
- The 0.12.0 contract does not declare a field that `src/api.rs` decodes.
- A merged row needs a Library fact that `LibraryViewModel` does not give.
- The search list cannot get the keyboard focus without a change to the detail column focus.
- A change needs a file in "Do not touch".
