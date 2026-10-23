# ADR 0086 Task 005: Saved Search Window Groups

Status: Implemented - 2026-10-23. Cases R86-5-01 to R86-5-25 pass. Mechanical checks are Green. V1 to V7 are open, in the final walk of the run of 2026-10-23.

Orchestrator note - 2026-10-23: packets 002 to 004 are implemented. The run record lists their deviations.

`WindowGroupPageVm::build(key, layout, rows, tree, fills, busy)` takes a layout and its rows. Guard R86-4-24 lets one `LibraryViewModel` method call it. A saved search builds its own layout from its rows. Then either add a source to `WindowGroupKey` that `window_group_page` reads, or widen guard R86-4-24 to one method for each source. Keep the rule of the guard: the view model builds the page, and the shell never does.

## Goal

The Releases tab of a saved search page shows one group row for two or more singles of one artist in one window. The groups use the key, the chain and the window length of New releases (packet 002). The release date of a single comes from ADR 0086 Decision 1, not from the relevance order.

A saved search groups only its loaded rows. It sends no extra page request for a window. A group that a later page can change shows the "+" of packet 003. The group row takes the place of its anchor single in the relevance order. Each other row keeps its place.

A click on a group row opens the window group page of packet 004. In selection mode, the group row has the checkbox of packet 004.

This packet starts after packets 002, 003 and 004 are complete.

## Authority

- [ADR 0086](../adr/0086-new-releases-singles-window.md) Decision 6: the saved search views of Discover make window groups with the same key and window as New releases. A relevance order never closes a window. A saved search groups only its loaded rows and loads no extra page for a window. Each group that a later page can change shows the "+" of Decision 5. The group tile takes the place of its anchor single in the relevance order.
- ADR 0086 Decision 1: the window group, its key, and the release date of each kind of single. A single with no release date joins no group.
- ADR 0086 Decision 3: the facts of a group row. ADR 0086 Decision 4: the window group page and the group checkbox.
- ADR 0086 "Operator Answers - 2026-10-23", answers 1 and 2: a saved search groups only its loaded rows, and a Library single takes the latest `pub_date` of its tracks.
- [ADR 0084](../adr/0084-music-column-browser.md) Decision 2: the saved searches are views of the Discover column. Decision 3: a saved search runs its query against the Library and the Index. Decision 8: the state words "New" and "In library".
- [Packet 002](adr-0086-task-002-new-releases-window-groups.md): the chain, the tile view model and "Names For Later Packets".
- Packet 003: the "+" of an open group. Packet 004: the window group page and the group checkbox.
- [ADR 0084 packet 012](adr-0084-task-012-saved-searches.md): the saved search page. ADR 0084 packet 010: the merge of the Library rows and the Index rows.
- ADR 0075 packet 047, Required Change 2: an Index search sends no detail request for a returned hit.
- [AGENTS.md](../../AGENTS.md): renderer portability, typed action state, "Current-view state must update in place", "Every fix gets a guard" and "Column text does not call `truncate()`".

## Recorded Facts - 2026-10-23

The orchestrator read each fact in the code on 2026-10-23. Packets 002 to 004 are not implemented. The last subsection lists the facts that come from the packet 002 design and are not in the code today.

### How a saved search page shows its rows

- `src/view_models/search_results/mod.rs:58-78` `SearchResultsInspectorPageVm` holds the three tabs `releases`, `artists` and `tracks`, the `entry` and one `selection`. The saved search page and the toolbar search use this one type.
- `src/view_models/search_results/mod.rs:398-403` `release_kind_placement` gives `Badge` for a `FrameNavigationEntry::SavedSearch` entry. Thus the view model knows a saved search page by its entry.
- `src/view_models/search_results/mod.rs:364-392` `visible_rows` gives one `SearchResultRowVm` for each row of the active tab. The page shows rows. It has no Tiles mode.
- `src/view_models/search_results/rows.rs:178-216` `release_row_vm` makes a release row. Line 185 replaces the kind of the row with the Library kind when the Library holds the release.
- `src/ui/shells/search_results_inspector.rs:420-490` `render_active_result_list` renders each row with `render_result_row` (`src/ui/shells/search_result_rows.rs:65`). That function uses `ListRow`, a `Thumbnail` or a `CollectionArt` of `ThumbnailSize::Sm`, the download-state icon and a `TagBadge`.
- `src/ui/shells/search_results_inspector.rs:66` and `:142` the slots hold `rows: Vec<SearchResultRowVm>`, set by `with_rows`.
- `src/app/saved_search_dispatch.rs:226-300` `render_saved_search_page_content` reads `search_lookup()` and calls `visible_rows` one time for each render (line 244). `src/app.rs:1399` does the same for the toolbar search.
- `src/app.rs:1338-1351` `resolve_search_result_arts` resolves the art of each row by its row id, with `ThumbnailSize::Sm`.
- `src/app/search_selection.rs:406-420` `focused_search_row_id` calls `visible_rows` again. `src/app/search_selection.rs:131` `click_search_result_row` handles a row click. `src/app/search_selection.rs:346` reads `focused_row_index`.
- `src/view_models/search_results/mod.rs:520-533` `focused_row_index` gives the index of the focused row among all the rows of the tab. Packet 028 D2 scrolls to this index.
- `src/view_models/search_results/mod.rs:409-424` `selection_rows` gives one selection row for each release row, through `search_release_rows` (`src/view_models/detail_selection.rs:988-1002`).

### The paging of a saved search

- `src/app/saved_search_dispatch.rs:86-150` `run_saved_search` makes the Library rows at once, then sends one `FetchIndexSearchResults` on the runtime. It has no "Load more" and no cursor.
- `src/application/queries/search.rs:507-532` `fetch_index_feed_result_rows` sends one `GET /v1/search?type=feed` with `limit` `PAGE_LIMIT` (20, `src/api.rs:12`) and no cursor. It drops `response.pagination`. `src/api.rs:31-34` `SearchResponse` holds `pagination`, and `src/api.rs:161-164` `Pagination` holds `has_more`.
- `src/application/queries/search.rs:226-276` `fetch_index_search_result_rows` sends the feed, track and publisher requests. A failed feed request alone keeps the other rows (lines 256-263).
- `src/app/search_dispatch.rs:269-286` `search_results_detail_for_query` reads the Library tracks with `search_local_library_tracks(&conn, query, None)`. `src/application/queries/search.rs:29` gives the default limit `DEFAULT_LOCAL_LIBRARY_SEARCH_LIMIT`, 50 tracks.
- `src/view_models/search_results/mod.rs:198-208` `replace_index_results` replaces the Index rows of each tab and keeps the Library rows.

### The merge and the relevance order

- `src/view_models/search_results/paged_tab.rs:31-40` and `:112-135` `merge_rows` puts each Library row first, then each Index row in rank order. An Index row whose `ReleaseKey` is already in the list drops, and the row that stays takes its image when it has none. Thus one release gives one row (packet 010).
- `src/view_models/search_results/results.rs:453` and `:485-488` the Library release rows come from a `BTreeMap<i64, LocalFeedResult>`. Their order is the Library feed id.
- `src/view_models/search_results/results.rs:576-579` the key of a Library release row is `ReleaseKey::FeedGuid` when its track has a feed GUID, else `ReleaseKey::LibraryFeed`.
- `src/view_models/search_results/results.rs:615` the kind of a Library row comes from the count of its matched tracks, not of its album tracks. Thus one matched track of an album gives `Single` before `release_row_vm` replaces it.
- `src/application/queries/search.rs:567-600` `index_feed_result_display` makes an Index row with the id `index-feed:<guid>`, the key `ReleaseKey::FeedGuid` (`src/view_models/workspace/nav.rs:230-232`) and the kind from `episode_count`. Its artist text goes only into `secondary_parts` (lines 589-594). It sets no `remote_feed`.

### The facts that an Index row of a saved search does not have

- `src/api.rs:49-68` `SearchResult` has no `newest_item_at` and no named feeds. It has `pub_date` and `release_artist`.
- `tests/fixtures/musicindex-openapi-0.13.0.json`: `SearchResponseItem` has no `newest_item_at`. The `include` of `GET /v1/search` supports only `link_facts`.
- `../stophammer/src/query.rs:1293-1316` `feed_search_summary` says "A feed row holds no track publication date, so `pub_date` stays empty."
- `tests/architecture_tests.rs:18993-19022` the guard of ADR 0075 packet 047 forbids a detail request in `fetch_index_feed_result_rows`.
- Thus an Index row of a saved search has no release date today. Only a release that the Library holds has a date.

### The Library facts of a search row

- `src/view_models/search_results/rows.rs:52-64` `SearchLibraryLookup` gives `release`, `track` and `is_artist_feed`. `src/view_models/search_results/rows.rs:27-35` `SearchLibraryRelease` gives `facts`, `download_state` and `kind`. It gives no date and no artist identity.
- `src/view_models/library.rs:5545-5603` `LibrarySearchLookup` implements the trait. `src/view_models/library.rs:4162-4193` `search_lookup` makes it for one render.
- `src/view_models/library.rs:3575-3582` `album_for_release_key` finds the album of a key. It examines each album of the tree for each call.
- `src/view_models/library.rs:5640` `AlbumNode::release_key`, `:5658` `AlbumNode::release_kind` and `:5669` `AlbumNode::artist_identity`.
- `src/view_models/library.rs:2246` `tree_name_feed_matches`. `src/app/saved_search_dispatch.rs:116-120` the Index search command reads the same matches through `name_feed_matches()`.
- `src/view_models/singles_group.rs:85` `SinglesGroupKey(pub ArtistIdentity)`. `src/view_models/singles_group.rs:405` the date of a Library single is the latest `pub_date` of its tracks.
- `src/view_models/search_results/tests.rs:20-40` `RecordedLibrary` implements `SearchLibraryLookup` for the tests.

### Facts from the packet 002 design

These facts are not in the code today. Packet 002 adds them.

- `src/view_models/window_groups.rs` gives `WindowLength`, `WindowEntry`, `WindowGroup`, `WindowItem`, `window_groups`, `window_span_text`, `WindowGroupStateVm` and `WindowGroupTileVm` with `WindowGroupTileVm::new`.
- `WindowGroup::closed_by_rows` is true when one entry has an `order_date` before `window_start`, or no `order_date`. This rule is correct only for the recency order.
- `WindowGroupTileVm::window_closed` is the `closed_by_rows` of the group. Packet 003 shows the "+" from it.
- `FeedResultDisplay::newest_item_at` and `with_newest_item_at`. `FeedResultDisplay::new` sets the value `None`.
- `LibraryViewModel::window_length()`.
- The guard `adr_0086_window_groups_built_once_in_the_view_model` forbids a call to `window_groups(` under `src/ui/`, `src/library/` and `src/app/`.
- Packet 002 decision D4: an empty name key and the key of "Unknown Artist" make no group. D7: a member is in the Library when the Library tree holds the release.

## Design

### Which rows group

Only the Releases tab of a saved search page groups. The toolbar search, the Artists tab and the Tracks tab do not group. A release row is a window entry when its kind is `Single`.

`saved_search_window_entries(rows, lookup)` makes one `WindowEntry` for each merged release row, in the visible order:

- **Library row.** `lookup.release_window_facts(key)` gives `Some`. The facts give the kind, the group key and the release date. The release date is the latest `pub_date` of the album tracks (ADR 0086 Decision 1).
- **Index row.** The facts give `None`. The kind is `FeedResultDisplay::kind`. The group key is `lookup.window_key_for_name(artist_text)`. The release date is `FeedResultDisplay::newest_item_at`. Today the search hit gives no such value, so an Index row has no date and joins no group.
- **No key.** A row that is not a single has no `group_key`. A row with no `ReleaseKey` has no entry key and no `group_key`.
- **Order date.** `order_date` is `None` for each entry. The open state does not come from `closed_by_rows` (see "The open state").
- **Library state.** `in_library` is true when `lookup.release(key)` gives `Some`. This is the test of packet 002 D7.

### The new Library facts

`SearchReleaseWindowFacts` is the window fact of one Library release:

- `kind`: `AlbumNode::release_kind()`.
- `group_key`: `AlbumNode::artist_identity(&artist.name)`, with the "Matched by name" rule of packet 002 D4. `None` for an empty name key and for the key of "Unknown Artist".
- `release_date`: the latest `pub_date` of the album tracks.
- `artist_title`: the name of the `ArtistNode`.

`SearchLibraryLookup` gets three methods:

- `release_window_facts(&self, key: &ReleaseKey) -> Option<SearchReleaseWindowFacts>`.
- `window_key_for_name(&self, artist_text: &str) -> Option<SinglesGroupKey>`. It gives `Publisher(guid)` for a name that matches one Library artist feed, else `Name(key)`. It gives `None` for an empty key and for "Unknown Artist".
- `window_length(&self) -> WindowLength`. `LibrarySearchLookup` gives `LibraryViewModel::window_length()`.

`LibrarySearchLookup` makes the facts map and the name matches one time, at the first call, in a `std::cell::OnceCell`. The toolbar search never calls these methods, so its render cost does not change.

### The open state

`SearchResultsInspectorPageVm::release_rows_complete()` is true only when all four conditions are true:

1. The Index request is not loading.
2. The Index request has no error.
3. `feed_rows_partial` is false. The feed response gave `has_more` false, and the feed request did not fail.
4. `library_rows_partial` is false. The Library track search gave fewer tracks than its limit.

Each group of a saved search sets `WindowGroupTileVm::window_closed` to `release_rows_complete()`. When the page holds every row of the query, no later page can change a group, so no group is open. Else each group is open. Packet 003 owns the "+" text.

### The items of one render

`SearchResultsInspectorPageVm::visible_items(lookup)` gives a `Vec<SearchResultItemVm>`:

- For a toolbar search, for the Artists tab and for the Tracks tab, each row is `SearchResultItemVm::Row`.
- For the Releases tab of a saved search page, it calls `window_groups(entries, lookup.window_length())`. A plain item gives its row. A group item gives `SearchResultItemVm::WindowGroup` at the place of its anchor row. Each other member gives no item.

The group tile is `WindowGroupTileVm::new(group, entries, anchor_row_id, title, anchor_cover)`, then calls `set_window_closed(release_rows_complete())` of packet 003.

- `title`: the `artist_title` of a Library anchor. For an Index anchor with the key `Publisher(guid)`, the title of the feed from the name matches. Else the `artist_text` of the row.
- `anchor_cover`: the `thumbnail_href` of the anchor row.

The page builds the groups at each call of `visible_items`. The Library facts come from the lookup of the current render. Thus a download, a removal and a new Index reply change the groups in place. The release rows are at most 20 Index rows and the Library rows of 50 tracks.

### The group row

The saved search page shows a group as one row in the row style of the page. `render_window_group_result_row` in `src/ui/shells/search_result_rows.rs` uses the same `ListRow`, the `CollectionArt` of `ThumbnailSize::Sm`, the title and second line slots, and the `TagBadge` of a release row. The second line is `date_span`. The trailing slot shows the state word and the count text of the tile, and then the `kind_badge`. The view model gives each text.

### Selection, focus and click

- In `Normal`, `selection_rows` gives each plain row and each anchor row, in item order. A hidden member takes no keyboard focus. `focused_row_index` gives the index of the focused item.
- In selection mode, the group row has the checkbox of packet 004. Its rule is the rule of packet 004 for a New releases group tile. This packet adds no second rule.
- A click and Enter on a group row open the window group page of packet 004, with the members of the group. `window_group_for_row(row_id, lookup)` gives the tile of an anchor row id.
- `visible_members` does not change. Each loaded release key stays a member, so a selected member stays selected after a new Index reply.

### Names added by this packet

- `src/view_models/search_results/rows.rs`: `pub(crate) struct SearchReleaseWindowFacts { kind: ReleaseKind, group_key: Option<SinglesGroupKey>, release_date: Option<i64>, artist_title: String }`, and the trait methods `release_window_facts`, `window_key_for_name` and `window_length`.
- `src/view_models/search_results/saved_search_groups.rs` (new module): `pub(crate) enum SearchResultItemVm { Row(SearchResultRowVm), WindowGroup(Box<WindowGroupTileVm>) }` and `pub(crate) fn saved_search_window_entries(rows: &[FeedResultDisplay], lookup: &dyn SearchLibraryLookup) -> Vec<WindowEntry>`.
- `src/view_models/search_results/mod.rs`: the fields `feed_rows_partial: bool` and `library_rows_partial: bool`, `fn with_library_rows_partial(self, partial: bool) -> Self`, `fn release_rows_complete(&self) -> bool`, `fn visible_items(&self, lookup: &dyn SearchLibraryLookup) -> Vec<SearchResultItemVm>` and `fn window_group_for_row(&self, row_id: &str, lookup: &dyn SearchLibraryLookup) -> Option<WindowGroupTileVm>`.
- `src/view_models/search_results/index_detail.rs`: `IndexSearchResultRows::feed_rows_partial: bool`.
- `src/view_models/search_results/results.rs`: `FeedResultDisplay::artist_text: Option<String>` and `fn with_artist_text(self, value: impl Into<String>) -> Self`.
- `src/view_models/window_groups.rs`: no new tile method. Packet 003 adds `WindowGroupTileVm::set_window_closed(&mut self, closed: bool)`. Add `WindowEntry::new` only when the fields of `WindowEntry` are not visible in `src/view_models/search_results/`.
- `src/ui/shells/search_results_inspector.rs`: `fn with_items(self, items: Vec<SearchResultItemVm>) -> Self` replaces `with_rows`.
- `src/ui/shells/search_result_rows.rs`: `render_window_group_result_row`.

## Orchestrator Decisions

- **D1. Only the singles of the Releases tab of a saved search group.** ADR 0086 Decision 6 names the saved search views of Discover. The toolbar search is not a Discover view. The Artists tab has no releases. The Tracks tab shows tracks, and Decision 1 groups singles.
- **D2. A saved search never uses `closed_by_rows`.** That rule is a proof in the recency order of `GET /v1/feeds/recent`. A relevance order never closes a window (Decision 6). A row with no `order_date` closes a window in that rule, so it would close each saved search window.
- **D3. A group is open when the page does not hold every row of the query.** Decision 6 shows the "+" for each group that a later page can change. A later page exists in three conditions:
  - The feed response gave `has_more` true, or the feed request failed.
  - The Index request is loading or failed.
  - The Library track search reached its limit.

  In each other condition, no row of the query is missing, and no later page can change a group. The packet 002 note "treats each group as open" is advisory. This decision follows the ADR text.
- **D4. This packet sends no new request.** It reads `has_more` from the one feed response that the search already gets. It adds no cursor, no extra page and no detail request (ADR 0086 Decision 6, ADR 0075 packet 047).
- **D5. Flag. An Index row of a saved search has no release date today.** The search hit gives no `newest_item_at`, and Stophammer leaves `pub_date` empty for a feed hit. Decision 1 puts no single with no release date in a group. Thus, today, only the singles that the Library holds group in a saved search.

  The chain reads `FeedResultDisplay::newest_item_at`, so an Index row groups when a later contract gives the value. [Open request 11](../plans/v4vmm-open-requests.md#11-a-feed-search-hit-gives-its-release-date-and-its-named-feeds) asks for the date and the named feeds. This packet does not take a date from New releases or from a cache. A group must not change with the views that the curator opened before.
- **D6. The Library decides a Library row.** A row whose key the Library tree holds takes its kind, its key and its date from the album. The kind of a Library search row counts only the matched tracks (`results.rs:615`), so the album kind is the correct kind. Packet 002 D3 made the same choice.
- **D7. The merge of packet 010 removes the duplicate release.** The entries come from the merged rows. A Library row and an Index row with one `ReleaseKey` give one row, at the place of the Library row, with the Library date. Thus a release is in one group at most, and the count of a group counts it one time.
- **D8. The anchor is the newest member, not the best-ranked member.** The group row goes at the place of the anchor row in the merged order. Each other row keeps its order. The operator rejected the place of the best-ranked single on 2026-10-22 (ADR 0086 "Alternatives Considered").
- **D9. The groups are built at each call of `visible_items`.** Packet 010 gives the Library facts at each render through the lookup, so a download changes a row in place. A cached layout would need a refresh for each change of the tree, and the page has no such signal.

  The render calls `visible_items` one time. A handler calls it one time for each input event. The new lookup facts are made one time for each lookup.
- **D10. The group row uses the row style of the saved search page.** The page shows rows with the `Sm` art. A New releases group row has the layout of the content list. The group facts come from the one shared `WindowGroupTileVm`, so the two views show the same text.
- **D11. The click and the checkbox are those of packet 004.** The window group page opened from a saved search lists the loaded members of the group. This packet adds no page kind and no selection rule.
- **D12. A member is in the Library when the Library tree holds the release.** This is the test of packet 002 D7. Thus one window of one artist gives the same Library count in New releases and in a saved search.
- **D13. The window length is the value of New releases.** `lookup.window_length()` gives `LibraryViewModel::window_length()`. A change of the value in packet 006 changes the saved search groups at the next render.

## Required Changes

1. `src/view_models/search_results/index_detail.rs`: add `IndexSearchResultRows::feed_rows_partial`, `false` by default.
2. `src/application/queries/search.rs`:
   - `IndexFeedSearchRows` keeps `has_more` from `response.pagination.has_more`. Do not read `pagination.cursor`.
   - `fetch_index_search_result_rows` sets `rows.feed_rows_partial` to that `has_more`. It sets `true` when the feed request failed and another request passed.
   - `index_feed_result_display` calls `with_artist_text` with a non-empty `hit.release_artist`. It does not set `newest_item_at`.
3. `src/view_models/search_results/results.rs`: add `FeedResultDisplay::artist_text`, with `None` in `new`, and `with_artist_text`.
4. `src/view_models/search_results/rows.rs`: add `SearchReleaseWindowFacts` and the three trait methods.
5. `src/view_models/library.rs`: implement the three methods in `LibrarySearchLookup`. Make the facts map and the name matches one time, in `OnceCell` fields. Use the "Unknown Artist" and empty-key rule of packet 002 D4. When packet 002 put that rule in a function, call that function.
6. `src/view_models/search_results/tests.rs`: implement the three methods in `RecordedLibrary`.
7. `src/view_models/window_groups.rs`: add `WindowEntry::new` only when it is necessary. Use `set_window_closed` of packet 003.
8. `src/view_models/search_results/saved_search_groups.rs`: add the new module with `SearchResultItemVm` and `saved_search_window_entries`. Start it with a `//!` comment that names ADR 0086 Decision 6, and `#![warn(clippy::pedantic)]`. Add `mod saved_search_groups;` and the `pub(crate) use` in `mod.rs`.
9. `src/view_models/search_results/mod.rs`:
   - Add the fields `feed_rows_partial` and `library_rows_partial`. `replace_index_results` copies `rows.feed_rows_partial`.
   - Add `with_library_rows_partial`, `release_rows_complete`, `visible_items` and `window_group_for_row`.
   - In `Normal`, `selection_rows` of the Releases tab of a saved search gives only the shown rows. `focused_row_index` gives the item index.
   - Keep `visible_rows`. A caller that needs the groups calls `visible_items`.
10. `src/app/search_dispatch.rs`: in `search_results_detail_for_query`, call `with_library_rows_partial(local_tracks.len() >= DEFAULT_LOCAL_LIBRARY_SEARCH_LIMIT)`.
11. `src/app/saved_search_dispatch.rs`: in `render_saved_search_page_content`, call `visible_items` one time and pass the items through `with_items`.
12. `src/app.rs`: the toolbar search passes its rows through `visible_items` and `with_items`. `resolve_search_result_arts` takes the items. It resolves the art of a group with `collection_art_image` and `ThumbnailSize::Sm`, by `element_id`.
13. `src/app/search_selection.rs`: `focused_search_row_id` reads `visible_items`. A click or Enter on an anchor row of a group opens the window group page of packet 004, with the tile from `window_group_for_row`.
14. `src/ui/shells/search_results_inspector.rs`: replace `with_rows` with `with_items`. `render_active_result_list` renders a `WindowGroup` item with `render_window_group_result_row`.
15. `src/ui/shells/search_result_rows.rs`: add `render_window_group_result_row`, with the checkbox of packet 004 in selection mode.
16. `tests/architecture_tests.rs`: add the guard of case R86-5-24.

## Mechanical Acceptance Criteria

Use the prefix `adr_0086_saved_search_groups_` for each test. Each test is a unit test beside its owner code, apart from R86-5-24. The dates are UTC midnights of 2026. "One key" means one `SinglesGroupKey`. "The page" is a `SearchResultsInspectorPageVm` with a `SavedSearch` entry on the Releases tab.

| Case | Required proof |
|---|---|
| R86-5-01 | The page has five release rows. Library singles of one key, on Oct 8 and Oct 20, are at positions 1 and 3. `visible_items` gives four items, and the group item is at the place of the Oct 20 row. |
| R86-5-02 | In R86-5-01, the rows at positions 0, 2 and 4 keep their order before and after the group item. The Oct 8 row gives no item. |
| R86-5-03 | The anchor is the Oct 20 row, also when the Oct 8 row has the better rank. The group item does not go to position 1. |
| R86-5-04 | `LibrarySearchLookup::release_window_facts` of an album with tracks dated Oct 1 and Oct 19 gives the release date Oct 19 and the artist node name as `artist_title`. |
| R86-5-05 | A Library row whose search matched one track of a ten-track album has no `group_key`. Its entry takes the album kind, not the row kind. |
| R86-5-06 | A Library single on Oct 20 has the key `Publisher(G)`. An Index row on Oct 15 has `newest_item_at` and an `artist_text` that matches feed G. The two rows give one group of two. The state word is "New" and `count_text` is "1 in library". |
| R86-5-07 | An Index single row with no `newest_item_at` gives a `Row` item. It never anchors a group and never joins one. |
| R86-5-08 | A Library row and an Index row with one `ReleaseKey` give one entry. The group counts the release one time, and its date is the Library date. |
| R86-5-09 | An album and an EP of the key, dated between two singles, give `Row` items at their places. The two singles still make one group. |
| R86-5-10 | With `feed_rows_partial` true, `release_rows_complete` is false and the tile `window_closed` is false. |
| R86-5-11 | With each condition of "The open state" met, `release_rows_complete` is true and the tile `window_closed` is true. |
| R86-5-12 | `release_rows_complete` is false while the Index request is loading, and false after `set_index_error`. |
| R86-5-13 | `with_library_rows_partial(true)` gives `release_rows_complete` false, with all other conditions met. |
| R86-5-14 | `fetch_index_search_result_rows` with a recorded server sends one `type=feed` request. A response with `has_more` true sets `feed_rows_partial` true. The request has no `cursor`. |
| R86-5-15 | A failed feed request with a passed track request sets `feed_rows_partial` true. |
| R86-5-16 | `index_feed_result_display` sets `artist_text` from `release_artist`. An empty `release_artist` gives `None`. `newest_item_at` stays `None`. |
| R86-5-17 | A page with a `Search` entry, and the Artists tab and the Tracks tab of the page, give only `Row` items. |
| R86-5-18 | `lookup.window_length()` of 7 days gives two `Row` items for the singles of R86-5-01. |
| R86-5-19 | In `Normal`, `selection_rows` gives each plain row and each anchor row, in item order, and no hidden member. `focused_row_index` of a focused anchor gives the index of its item. |
| R86-5-20 | `window_group_for_row` of the anchor row id gives the tile, with `member_keys` of the two members. A plain row id gives `None`. |
| R86-5-21 | `replace_index_results` with a new Index single in the window gives the group in the next `visible_items`, with no other call. A lookup that now holds a member changes the Library count in the next `visible_items`. |
| R86-5-22 | The tile of R86-5-01 gives the `artist_title` as `title`, `date_span` "Oct 8 to Oct 20" and `kind_badge` "2 Singles". |
| R86-5-23 | An Index anchor with the key `Publisher(G)` takes the feed title of G as `title`. An Index anchor with a `Name` key takes its `artist_text`. |
| R86-5-24 | A situational guard in `tests/architecture_tests.rs`, with the name `adr_0086_saved_search_groups_load_no_extra_page`, passes. It fails against a copy of `fetch_index_feed_result_rows` with a second `client.search(` call. It also fails against a copy of `saved_search_groups.rs` that reads `closed_by_rows`. |
| R86-5-25 | Each test of packets 002 to 004, ADR 0084 packets 010 and 012 still passes. |

The guard of R86-5-24 enforces these rules:

- The body of `fn fetch_index_feed_result_rows(` in `src/application/queries/search.rs` holds one `client.search(` call and no `.cursor`.
- `src/view_models/search_results/saved_search_groups.rs` calls `window_groups(` and `set_window_closed(`. It does not contain `closed_by_rows`.
- No file under `src/ui/`, `src/library/` or `src/app/` calls `saved_search_window_entries(`.

The failure message of the guard is this text:

```text
ADR 0086 §6 Saved Searches Group By The Same Window: a saved search groups only its loaded rows, and a relevance order never closes a window.
Keep one `client.search(` call with no cursor in `fetch_index_feed_result_rows`. Take the open state from `SearchResultsInspectorPageVm::release_rows_complete`, not from `WindowGroup::closed_by_rows`.
```

## Visual Acceptance Criteria

These are for the operator. No test proves them.

- V1: open a saved search whose results hold two or more Library singles of one artist in one window. Open the Releases tab, at normal width. One group row shows in place of the singles. Examine Light and Dark.
- V2: the group row shows the anchor cover in the logo frame, the artist name, the date span, the "n Singles" badge and the state word. The art, the title and the badge have the same positions as in a release row of the same page.
- V3: a saved search with more than 20 Index releases shows the "+" of packet 003 after the count. A saved search with few results shows no "+".
- V4: the group row is at the place of the newest single of the group. Each other row has the same order as before this packet.
- V5: click the group row. The window group page of packet 004 opens. The breadcrumb ends with the group, after "Saved: <query>". Back shows the saved search page with the same rows.
- V6: click "Select". The group row shows its checkbox, as in New releases. A check of the group selects each single of the group, and the bar count agrees.
- V7: make the window narrow. Examine Light and Dark. No text overlaps, and no text goes outside its row.

## Exclusions

- No "Load more" for a saved search, no cursor and no extra page (Decision 6).
- No detail request for an Index hit, and no new request profile (ADR 0075 packet 047).
- No date for an Index row from New releases, from a cache or from a track hit (D5).
- No Stophammer request. D5 records the missing `newest_item_at` of a feed hit for the orchestrator.
- No groups in the toolbar search, in the Artists tab or in the Tracks tab (D1).
- No Tiles mode for the saved search page.
- No change to the merge of packet 010 or to the order of the Library rows. A Library album with no feed GUID and an Index row of the same release keep two rows, as before.
- No change to the window group page, to the selection rule or to the "+" text (packets 003 and 004).
- No Settings value (packet 006).
- No schema change and no configuration key.

## Files To Inspect

- [AGENTS.md](../../AGENTS.md), the rust-dev skill and [the source map](../architecture/source-map.md).
- [ADR 0086](../adr/0086-new-releases-singles-window.md) Decisions 1, 3, 4, 5 and 6, and ADR 0084 Decisions 2, 3 and 8.
- Packets 002, 003 and 004 of ADR 0086, with their "Implementation Result" sections.
- ADR 0084 packets 010 and 012.
- `src/view_models/window_groups.rs` (from packet 002).
- `src/view_models/search_results/mod.rs`, `rows.rs`, `results.rs`, `paged_tab.rs`, `index_detail.rs`, `saved_search.rs` and `tests.rs`.
- `src/view_models/library.rs`: `LibrarySearchLookup`, `search_lookup`, `album_for_release_key`, `tree_name_feed_matches` and `AlbumNode`.
- `src/view_models/detail_selection.rs`: `search_release_rows`.
- `src/application/queries/search.rs`: `fetch_index_search_result_rows`, `fetch_index_feed_result_rows` and `index_feed_result_display`.
- `src/app/saved_search_dispatch.rs`, `src/app/search_dispatch.rs`, `src/app/search_selection.rs` and `src/app.rs` (`resolve_search_result_arts`).
- `src/ui/shells/search_results_inspector.rs` and `src/ui/shells/search_result_rows.rs`.
- `tests/architecture_tests.rs`: `adr_0075_request_reuse_index_routes_ask_the_owner` and the guard of packet 002 R86-2-27, for the pattern.

## Checks

```bash
cargo check
cargo fmt -- --check
cargo clippy --all-targets -- -D warnings
cargo test --lib adr_0086_saved_search_groups_
cargo test
cargo test --test architecture_tests
cargo build --bin v4vmm
```

Run `cargo build --bin v4vmm` last. `cargo test` can leave a binary with GPUI test support at `target/debug/v4vmm`.

## Rollback

Revert the commit. This packet adds no migration, no stored data and no configuration key.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `docs/architecture/source-map.md`
- This packet: `docs/tasks/adr-0086-task-005-saved-search-window-groups.md`
- ADR 0086 Decisions 1, 3, 4, 5 and 6
- Packets 002, 003 and 004 of ADR 0086, and ADR 0084 packets 010 and 012
- Each file in "Files To Inspect"

Goal:
- Make each change in "Required Changes".

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- Follow each decision in "Orchestrator Decisions". Do not change a decision.
- Use the names of packet 002 "Names For Later Packets" exactly. Use each name in "Names added by this packet" exactly.
- The view model gives each text, image and availability. The renderer only composes. Use named tokens. Column text does not call `.truncate()`.
- Tests use recorded values. No test sends a request to a live service.
- Background work runs through the ADR 0040 runtime. Do not call `cx.spawn` from a screen.
- Never run `git add`, `git rm`, `git commit`, `git checkout`, `git restore`, `git stash` or `git reset`.
- Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- The database schema and the migration registry.
- `src/view_models/singles_group.rs`, the Releases list and the artist pages.
- `src/application/request_profiles.rs` and `src/config.rs`.
- The New releases pager, the window group page and the "+" text, apart from the call of their existing functions.
- Any ADR, and each document other than this packet.

Acceptance criteria:
- Each case R86-5-01 to R86-5-25 has a passing test or guard.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.
- The packet has an "Implementation Result - <date>" section and an "Operator Visual Check" section for V1 to V7.

At the end, report: files changed, tests run, behavior changed, deviations, unresolved concerns, and the document changes that the orchestrator must make.

Stop and report the problem, and do not guess, when:
- Packets 002 to 004 are not complete.
- A name of packet 002 "Names For Later Packets" is absent or has a different signature.
- The "+" of packet 003 does not come from `WindowGroupTileVm::window_closed`.
- The window group page of packet 004 cannot take its members from a saved search page.
- The group checkbox of packet 004 cannot work on the rows of the search shell.
- A change needs a detail request, a cursor or a second search request.
- A change needs a new architecture decision.
- A change needs a file in "Do not touch".
