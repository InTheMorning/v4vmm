# ADR 0086 Task 002: New Releases Window Groups

Status: Implemented - 2026-10-23. Cases R86-2-01 to R86-2-33 pass. Mechanical checks are Green. V1 to V4, V6 and V8 are open, in the final walk of the run of 2026-10-23. Packet 004 replaced D8, D9, V5 and V7 on 2026-10-23.

## Goal

New releases shows one group tile for two or more singles of one artist in one window. The window length is the default of 14 days. The groups come from the loaded rows only. The pager does not change in this packet.

The packet adds the chain of ADR 0086 Decision 1, the place of Decision 2, and the facts of Decision 3. It does this in the Tiles mode and in the List mode. A click on a group tile opens the release page of its newest single until packet 004 adds the window group page.

This packet starts after packet 001 is complete. Packet 001 changes `fetch_recent_feed_result_rows`. This packet does not change that function.

## Authority

- [ADR 0086](../adr/0086-new-releases-singles-window.md) Decision 1: the window group, its key, the release date of each kind of single, and the chain from newest to oldest.
- ADR 0086 Decision 2: the group tile takes the place of its anchor single.
- ADR 0086 Decision 3: the title, the date span, the "n Singles" badge, the state word and the Library count. The List mode shows the same facts.
- ADR 0086 Decision 5: the definition of a closed window. Packet 003 uses it. This packet only exposes it.
- [ADR 0084](../adr/0084-music-column-browser.md) Decision 3: New releases is the first Discover view, in the recency order.
- ADR 0084 Decision 6: `SinglesGroupKey` and the "Matched by name" rule of 2026-10-22. The Releases list keeps its groups.
- ADR 0084 Decision 8: the state words "New" and "In library".
- ADR 0084 Decision 10: the two selection modes.
- ADR 0084 Decision 12: the art of a collection. One cover shows as a square. No cover shows the logo in the entity color.
- [Packet 001](adr-0086-task-001-parallel-new-releases-detail-reads.md): the parallel detail reads of a page.
- [AGENTS.md](../../AGENTS.md): renderer portability, token discipline, typed action state, "Current-view state must update in place", "Every fix gets a guard" and "Column text does not call `truncate()`".

## Recorded Facts - 2026-10-23

The orchestrator read each fact in the code on 2026-10-23.

### Where the rows come from

- `src/application/queries/feed.rs:112-147` `fetch_recent_feed_result_rows` reads `GET /v1/feeds/recent` with no `include`. Then it reads each feed with `INDEX_FEED_DETAIL`. A failed detail read keeps the list feed (`.unwrap_or(feed)`, line 131).
- `src/application/queries/feed.rs:136` each row is `index_feed_display(&feed_guid, Some(EntityDetail::Feed(detail)))`.
- `src/application/queries/search.rs:810-845` `index_feed_display` builds a `FeedResultDisplay` with the id `index-feed:<guid>`. It attaches `FeedView::from_api(feed.clone())` as `remote_feed`. It does not keep `newest_item_at`.
- `src/application/request_profiles.rs:81` `FEED_FULL` holds `tracks`, `remote_items` and `publisher`, and no `named_feeds`. Lines 113-114 give `INDEX_FEED_DETAIL` with `FEED_FULL`.
- `tests/fixtures/musicindex-openapi-0.13.0.json`: `GET /v1/feeds/{guid}` supports no `named_feeds` include. `GET /v1/feeds/recent` supports `named_feeds`. Both responses use `FeedResponse`, which has `newest_item_at`.
- `src/api.rs:204` `Feed::newest_item_at: Option<i64>`. `src/api.rs:221` `Feed::publisher`. `src/api.rs:238` `Feed::named_feeds`.
- `src/view_models/recent_feeds.rs:11` `RecentFeedResultRow` is `(SearchResultItemId, FeedResultDisplay)`. Lines 15-22 give `RecentFeedsPageBatch` with `rows`, `cursor` and `has_more`.
- `src/view_models/recent_feeds.rs:89-105` `RecentFeedsPageVm::finish_load` extends the loaded rows on an append. `has_more` is at line 133.
- `../stophammer/src/query.rs:4173` `recent_feed_rows` orders by `COALESCE(newest_item_at, -1) DESC, feed_guid DESC`.

### How app state stores the rows

- `src/library.rs:255` `LibraryApp::recent_music_page: RecentFeedsPageVm` holds the pager.
- `src/view_models/library.rs:3277-3280` `LibraryViewModel::replace_recent_music_content` calls `ContentListPageVm::replace_recent_feeds_page`, then `mark_recent_library_releases`.
- `src/view_models/library.rs:2429-2455` `replace_recent_feeds_page` sets `cached_rows` from `content_list_rows_from_recent_feeds` (lines 5138-5142).
- `src/view_models/library.rs:2363-2372` `ContentListPageVm` holds `cached_rows`. One `ContentListPageVm` serves New releases, Tracks and Needs attention. `source` tells them apart.
- `src/view_models/library.rs:3028-3034` `replace_tree`, `:3051-3054` `show_tracks_destination`, `:3081-3085` `show_needs_attention_report`, `:3092-3098` `refresh_needs_attention_report` and `:3273-3275` `begin_recent_music_content_load` also change `cached_rows` or the tree.
- `src/view_models/library.rs:2560` `visible_rows` dedupes the rows by id and applies the text filter. No production caller sets the text filter.

### Which view model and which shell build the tiles and rows

- `src/view_models/library.rs:1343-1380` `ContentListRowDisplay` is the one row type of a Tile and of a List row. `src/view_models/library.rs:1249-1255` `ContentListRowKind::Release(Box<FeedResultDisplay>)` holds the release.
- `src/view_models/detail_selection.rs:1059-1069` `discover_rows_with_selection` fills `row.selection` from the detail selection. Its only production caller is `src/ui/shells/library/content_list.rs:121`.
- `src/ui/shells/library/content_list.rs:107-142` `render_discover_new_releases_detail` renders the Discover view. It calls `render_library_content_list` (lines 61-94).
- `src/ui/shells/library/content_list.rs:204-259` `render_content_list_tiles` and `:309-357` `render_content_list_tile` render a tile with `SelectableTile`. Lines 149-202 and 261-307 render the List mode rows with `ListRow`.
- `src/ui/shells/library/content_list.rs:420-454` `render_content_list_tile_badges` shows the kind badge, the `library_badge` label and the `state_label`.
- `src/library/app_impl.rs:4827` the list column also calls `render_library_content_list`, for Tracks and Needs attention.
- `src/library/app_impl.rs:5061-5085` `render_discover_view_detail` builds the thumbnails by row id, then calls `render_discover_new_releases_detail`.

### Release kind and Library state of a row today

- `src/view_models/library.rs:1324-1329` `release_kind_of_feed_result` resolves the kind from `remote_feed.release_kind` and `FeedView::track_count`. It is private. `src/views.rs:529-535` `track_count` uses the tracks, else `episode_count`.
- `src/view_models/library.rs:1313-1318` `badge_label` gives "Album", "EP" or "Single".
- `src/view_models/library.rs:105` `NEW_RELEASE_LABEL` is "New". `:1019-1024` an Index row gets "New" as `state_label`.
- `src/view_models/library.rs:3037-3047` `mark_recent_library_releases` collects the feed GUID of each album of the tree. `:2666-2675` `mark_library_releases` marks each row whose GUID the tree holds. `:1555-1559` `mark_in_library` sets the `library_badge` to "In library" and clears "New".
- `src/view_models/library.rs:1098-1103` `ContentListLibraryBadgeState::label` gives "In library" for `InLibrary` and no word for `NotInLibrary`.

### The artist identity and the group key

- `src/view_models/singles_group.rs:85` `pub struct SinglesGroupKey(pub ArtistIdentity)` derives `Ord`.
- `src/view_models/artist_identity.rs:51-58` `ArtistIdentity::resolve(named_feeds, artist_text)` gives `Publisher(guid)` for a confirmed or unverified artist feed. Else it gives `Name(name_sort_key(artist_text))`.
- `src/views.rs:149` `FeedView::named_feeds`. `src/views.rs:417` `FeedView::from_api` builds it from `publisher` and `remote_items` through `named_feeds_from_api` (lines 547-593). The detail read asks for both.
- `src/view_models/singles_group.rs:463-469` `index_release_identity` takes an `api::Feed` and reads `named_feeds`, the include of packet 026 (`INDEX_FEED_SINGLES`, `src/application/request_profiles.rs:171-172`). A New releases row holds a `FeedView`, not an `api::Feed`.
- `src/view_models/library.rs:5669` `AlbumNode::artist_identity` calls the same `ArtistIdentity::resolve` with `AlbumNode::named_feeds`. `:5658` `AlbumNode::release_kind` and `:5640` `AlbumNode::release_key`.
- `src/view_models/library.rs:2246` `tree_name_feed_matches(tree)` gives the match of a name key to the Library artist feeds. `src/view_models/artist_identity.rs:183-195` `NameFeedMatches::get` gives `NoFeed` for the key of "Unknown Artist".
- `src/view_models/singles_group.rs:389-425` `library_singles_grouped` moves a Library single of a name with `MatchedFeed(guid)` to the group of `guid`. Packet 032 added it.
- The New releases row thus has the kind and the named feeds. It lacks `newest_item_at`, which is the release date of a `MusicIndex` single.

### The existing group types

- `src/view_models/singles_group.rs:89-101` `GroupSingle` holds `&ArtistNode`, `&AlbumNode`, `key`, `date` and `in_library`. Line 405 sets `date` to the latest `pub_date` of the tracks.
- `src/view_models/singles_group.rs:105-115` `IndexGroupSingle` holds an `&api::Feed`. Line 563 sets `date` to `newest_item_at`.
- `src/view_models/singles_group.rs:118-160` `GroupMember` is `Library` or `Index`. Its `date` method (line 137) is private.

### The logo frame

- `src/view_models/collection_art.rs:53-71` `CollectionArtVm::resolve` gives `SingleCover` for one cover and `EntityColor(kind)` for no cover.
- `src/presentation/collection_art.rs:67-90` `collection_art_image` loads the image of a `CollectionArtVm` off the UI thread. `src/library.rs:195` `LibraryApp` implements `CollectionArtHost`.
- `src/ui/composites/collection_art.rs` `CollectionArt::new(image, kind, size)` draws one `CollectionArtImage`.
- `src/ui/composites/thumbnail.rs:24-34` `ThumbnailSize` has `Sm`, `Md`, `Lg` and `XXl`. It has no size for the 152 px tile artwork. `src/ui/shells/library/content_list.rs:393` says `Size::ContentTileArtwork` shares the `Xl` footprint of `ImageSize`.
- `src/ui/shells/library/destination_lists.rs:67` `RowArts` maps an element id to a `CollectionArtImage`.

### Selection and the click of a tile

- `src/view_models/detail_selection.rs:1030-1033` `discover_row_key` gives `DetailRowKey::Release(ReleaseKey::FeedGuid(guid))` from the row id.
- `src/view_models/detail_selection.rs:1039-1053` `discover_release_rows` gives the selection rows of the view. `src/library/detail_selection.rs:240-245` reads them from `visible_rows()`.
- `src/view_models/detail_selection.rs:606` `DetailSelection::mode` and `:626` `row_selection(key, title)`.
- `src/library/detail_selection.rs:307-321` `click_discover_row` changes the selection, or opens the row through `open_content_list_row`. `src/library/app_impl.rs:513-519` emits `OpenIndexFeedDetail` for an `index-feed:` row.

### Render cost

- One render of New releases calls `visible_rows` three times or more: `src/ui/shells/library/content_list.rs:121`, and `src/view_models/library.rs:2616` and `:2623` in `page_state_display`. Handlers call `library_detail_page_rows` again (`src/library/detail_selection.rs:295`, `:314`).
- The overhaul plan responsiveness items 16 and 18 record the same risk for the Releases list and the search list.

## Design

### The pure chain

`src/view_models/window_groups.rs` holds the chain of Decision 1. It reads no tree and no row type.

- `WindowLength` is the window length in days. `WindowLength::DEFAULT` is 14 days. `from_days` accepts 1 to 60.
- `WindowEntry` is one loaded row in the order of the list.
- `window_groups(entries, length)` gives a `Vec<WindowItem>`. Each item is `Plain(position)` or `Group(WindowGroup)`.

`window_groups` follows these steps:

1. Take the entries with a `group_key` and a `release_date`. Each other entry is `Plain`.
2. For each key, sort the entries by `release_date`, newest first. Equal dates keep the list order.
3. The first entry that is in no group is the anchor. `window_start` is `anchor_date - length.seconds()`.
4. The group takes each entry of the key whose `release_date` is at or after `window_start`.
5. Repeat step 3 until each entry of the key is in a group.
6. A group with one member is `Plain`.
7. Give the items in the list order. A group goes at the position of its anchor. Each other member gives no item.

`WindowGroup::closed_by_rows` is true when one entry has an `order_date` before `window_start`. An entry with no `order_date` also closes the window, because Stophammer orders it last (`COALESCE(newest_item_at, -1)`).

### The facts of one New releases row

`NewReleasesLayout::build(rows, tree, length)` makes one `WindowEntry` for each row. It builds the album map and the name matches one time.

- **Library row.** The tree holds an album with the feed GUID of the row. The album gives the kind, the identity and the date. The identity is `AlbumNode::artist_identity(&artist.name)`. The date is the latest `pub_date` of its tracks (ADR 0086 Decision 1).
- **Index row.** The `remote_feed` gives the kind, through `release_kind_of_feed_result`. The identity is `ArtistIdentity::resolve(&feed.named_feeds, feed.artist)`. The date is the new `FeedResultDisplay::newest_item_at`.
- **Matched by name.** When the identity is `Name(key)` and `tree_name_feed_matches(tree).get(key)` gives `MatchedFeed(guid)`, the key is `Publisher(guid)`.
- **No key.** A row that is not a single has no `group_key`. A row with no feed GUID in its `remote_feed` has no key. A name whose sort key is empty, or the key of "Unknown Artist", has no key.
- **Order date.** `order_date` is `FeedResultDisplay::newest_item_at` for each row, also for a Library row.
- **Library state.** `in_library` is true when the row's `library_badge.state` is `InLibrary`. This is the test of `mark_library_releases`.

### The group tile

`WindowGroupTileVm` is the view model of a group tile and of a group row.

- `title`: the artist name. A Library anchor gives its artist node name. An Index anchor gives the title of its named feed of the key, else `FeedView::artist`, else "Unknown Artist".
- `date_span`: `window_span_text(oldest_date, anchor_date)`, for example "Oct 8 to Oct 20".
- `kind_badge`: "n Singles", for example "3 Singles".
- `state`: `WindowGroupStateVm::New { in_library }` or `WindowGroupStateVm::InLibrary`. `New` gives the word "New". Its `count_text` gives "5 in library" when `in_library` is 1 or more.
- `art`: `CollectionArtVm::resolve(None, [anchor cover], CollectionKind::SinglesGroup)`.
- `open_row_id`: the row id of the anchor. A click and Enter open this row.
- `a11y_label`, for example "jSmovie: 3 singles, Oct 8 to Oct 20, New, 1 in library".
- `window_closed`: the `closed_by_rows` of the group.
- `selection`: the selection facts of the anchor key.

### The cached layout

`LibraryViewModel` holds one `NewReleasesLayout` and one `WindowLength`. `refresh_new_releases_layout` builds the layout again. It runs at the end of each method that changes the New releases rows, the tree or the length. When the source of the content list page is not `RecentMusic`, the layout is empty.

The render and the handlers read `LibraryViewModel::new_releases_layout()`. They never call `window_groups` or `NewReleasesLayout::build`.

### The items of one render

`discover_items_with_selection(rows, layout, selection)` replaces `discover_rows_with_selection`. It gives a `Vec<ContentListItemVm>`.

- In `SelectionMode::Normal`, it follows the layout. A plain item gives its row. A group item gives its `WindowGroupTileVm`.
- In `SelectionMode::Selecting`, each visible row is a `ContentListItemVm::Row`, with its checkbox (decision D7).

`render_library_content_list` takes `Vec<ContentListItemVm>`. The list column wraps each of its rows in `ContentListItemVm::Row`.

### Names For Later Packets

Packets 003, 004, 005 and 006 use these exact names. Do not rename them.

- `src/view_models/window_groups.rs`:
  - `pub(crate) struct WindowLength(u32)`, with `WindowLength::DEFAULT` (14 days), `WindowLength::MIN_DAYS` (1), `WindowLength::MAX_DAYS` (60), `fn from_days(days: u32) -> Option<WindowLength>`, `fn days(self) -> u32` and `fn seconds(self) -> i64`.
  - `pub(crate) struct WindowEntry { position: usize, key: ReleaseKey, group_key: Option<SinglesGroupKey>, release_date: Option<i64>, order_date: Option<i64>, in_library: bool }`.
  - `pub(crate) struct WindowGroup { key: SinglesGroupKey, anchor: usize, members: Vec<usize>, anchor_date: i64, oldest_date: i64, window_start: i64, in_library_count: usize, closed_by_rows: bool }`. `members` holds positions, newest first, with the anchor first.
  - `pub(crate) enum WindowItem { Plain(usize), Group(WindowGroup) }`.
  - `pub(crate) fn window_groups(entries: &[WindowEntry], length: WindowLength) -> Vec<WindowItem>`.
  - `pub(crate) fn window_span_text(oldest: i64, anchor: i64) -> String`.
  - `pub(crate) enum WindowGroupStateVm { New { in_library: usize }, InLibrary }`, with `fn word(&self) -> &'static str` and `fn count_text(&self) -> Option<String>`.
  - `pub(crate) struct WindowGroupTileVm { element_id: String, open_row_id: String, key: SinglesGroupKey, member_keys: Vec<ReleaseKey>, title: String, date_span: String, kind_badge: String, count: usize, state: WindowGroupStateVm, a11y_label: String, art: CollectionArtVm, window_closed: bool, selection: RowSelectionVm }`.
  - `WindowGroupTileVm::new(group: &WindowGroup, entries: &[WindowEntry], anchor_row_id: &str, title: &str, anchor_cover: Option<&str>) -> WindowGroupTileVm`.
  - `pub(crate) const WINDOW_GROUP_ELEMENT_PREFIX: &str = "music-window-group-"`.
  - `pub(crate) enum NewReleasesLayoutItem { Release { row_id: String }, WindowGroup(Box<WindowGroupTileVm>) }`.
  - `pub(crate) struct NewReleasesLayout`, with `fn build(rows: &[&ContentListRowDisplay], tree: &LibraryTree, length: WindowLength) -> NewReleasesLayout`, `fn items(&self) -> &[NewReleasesLayoutItem]`, `fn shown_row_ids(&self) -> Vec<&str>` and `fn any_window_open(&self) -> bool`.
  - `pub(crate) enum ContentListItemVm { Row(ContentListRowDisplay), WindowGroup(Box<WindowGroupTileVm>) }`.
- `src/view_models/library.rs`:
  - `LibraryViewModel` fields `new_releases_layout: NewReleasesLayout` and `window_length: WindowLength`.
  - `fn refresh_new_releases_layout(&mut self)` (private).
  - `pub(crate) fn new_releases_layout(&self) -> &NewReleasesLayout`.
  - `pub(crate) fn window_length(&self) -> WindowLength` and `pub(crate) fn set_window_length(&mut self, length: WindowLength)`.
  - `pub(crate) fn discover_selection_rows(&self, mode: SelectionMode) -> Vec<&ContentListRowDisplay>`.
  - `pub(crate) fn release_kind_of_feed_result` (was private).
- `src/view_models/detail_selection.rs`: `pub(crate) fn discover_items_with_selection(rows: Vec<ContentListRowDisplay>, layout: &NewReleasesLayout, selection: &DetailSelection) -> Vec<ContentListItemVm>`.
- `src/view_models/search_results/results.rs`: `FeedResultDisplay::newest_item_at: Option<i64>` and `fn with_newest_item_at(self, value: Option<i64>) -> Self`.
- `src/view_models/format.rs`: `pub fn fmt_month_day(ts: i64) -> Option<String>`.
- `src/ui/composites/thumbnail.rs`: `ThumbnailSize::Xl`.

Each later packet uses these names as follows:

- Packet 003 reads `NewReleasesLayout::any_window_open` and `RecentFeedsPageVm::has_more`. It adds the "+" from `WindowGroupTileVm::window_closed`.
- Packet 004 replaces the target of `open_row_id` with the window group page. It uses `key`, `member_keys`, `WindowGroup::anchor_date` and `WindowGroup::window_start`. It gives the group tile a checkbox in `discover_items_with_selection`.
- Packet 005 builds `WindowEntry` values from the saved search rows. It calls `window_groups` and `WindowGroupTileVm::new`, and it treats each group as open.
- Packet 006 calls `WindowLength::from_days` and `LibraryViewModel::set_window_length`.

## Orchestrator Decisions

- **D1. The module is `src/view_models/window_groups.rs`.** `src/view_models/singles_group.rs` owns the Decision 6 groups and has 3235 lines. ADR 0086 keeps those groups for the Releases list. A separate module keeps the two group rules apart, and packet 005 can use it.
- **D2. The chain takes its own input type, `WindowEntry`.** `GroupSingle` needs an `AlbumNode`, and `IndexGroupSingle` needs an `api::Feed`. A New releases row holds a `FeedView`. `GroupMember::date` is private. The chain needs only the position, the keys, the two dates and the Library state.
- **D3. The Library decides a Library row.** A row whose feed GUID the tree holds takes its kind, its identity and its date from the Library album. Packet 026 decision D8 made the same choice. ADR 0086 Decision 1 gives the track `pub_date` as the date of a Library single.
- **D4. The "Matched by name" rule applies to each row.** A name key that `tree_name_feed_matches` matches to one feed becomes `Publisher(guid)`, for an Index row and for a Library row. ADR 0086 states that `SinglesGroupKey` includes this rule. An empty name key and the key of "Unknown Artist" make no group, because they name no artist.
- **D5. The identity of an Index row comes from `FeedView::named_feeds`.** The detail read already asks for `publisher` and `remote_items`. The Library and the Index release page read the same field. This packet adds no request, no request profile and no schema change. A row whose detail read failed keeps the list feed with no `publisher`, so its artist has the name level.
- **D6. `newest_item_at` goes into `FeedResultDisplay`.** `index_feed_display` sets it before `FeedView::from_api` takes the feed. The list feed of a failed detail read also has it. `FeedView` does not change, because many other pages build it.
- **D7. A member is in the Library when its tile says "In library".** The test is the one of `mark_library_releases`. Thus the count of a group agrees with the single tiles that it replaces.
- **D8. A click on a group tile opens the release page of its anchor single.** The shell calls `click_content_row` with `open_row_id`. Enter on a focused group tile does the same. Ctrl+click and Shift+click start selection mode with the anchor selected. Packet 004 replaces this target with the window group page.
- **D9. Selection mode shows each single as its own tile until packet 004.** In `Selecting`, `discover_items_with_selection` gives every row with its checkbox. "Done" and Escape show the groups again. Each checkbox, range, Ctrl+A and bar count thus stays correct, with no partial group selection.
- **D10. In `Normal`, the selection rows are the shown items.** `discover_selection_rows(Normal)` gives each plain row and each anchor row, in item order. Keyboard focus thus moves from tile to tile, and a group tile takes focus through its anchor key.
- **D11. The groups are built one time for each change.** `refresh_new_releases_layout` runs after a change to the rows, the tree or the length. The render, the selection and the handlers read the cached layout. This avoids the repeated work of responsiveness items 16 and 18. A guard enforces it.
- **D12. The group art is the anchor cover as a square.** ADR 0084 Decision 12 shows one cover as a square, and the logo in the entity color for no cover. The app layer resolves the art through `collection_art_image`. `ThumbnailSize::Xl` gives the 152 px tile size, the same as `ImageSize::Xl`.
- **D13. The window boundary is inclusive.** A single exactly one window length before the anchor joins the group. Equal dates keep the list order, so the earlier row is the anchor. A single with no release date never anchors and never joins.
- **D14. A group with no Library member shows "New" with no count.** "0 in library" gives no new fact, and a single tile with "New" shows no count.
- **D15. The date span follows the calendar day in UTC, as `fmt_date` does.** Two dates in one year give "Oct 8 to Oct 20". Two dates on one day give "Oct 20". Two dates in different years give the full `fmt_date` text of each date.
- **D16. The thumbnail requests do not change.** Each loaded row still loads its cover. Selection mode thus shows each cover at once.

## Required Changes

1. `src/view_models/search_results/results.rs`: add `FeedResultDisplay::newest_item_at` with the value `None` in `new`, and add `with_newest_item_at`.
2. `src/application/queries/search.rs`: in `index_feed_display`, set `newest_item_at` from the feed before `FeedView::from_api`.
3. `src/view_models/format.rs`: add `fmt_month_day`, with the format `"%b %-d"` in UTC.
4. `src/view_models/window_groups.rs`: add the new module with each name of the `window_groups.rs` group in "Names For Later Packets". Start it with a `//!` comment that names ADR 0086, and `#![warn(clippy::pedantic)]`.
5. `src/view_models/mod.rs`: add `pub(crate) mod window_groups;`.
6. `src/view_models/library.rs`:
   - Make `release_kind_of_feed_result` `pub(crate)`.
   - Add the fields `new_releases_layout` and `window_length`, with `NewReleasesLayout::default()` and `WindowLength::DEFAULT`.
   - Add `refresh_new_releases_layout`, `new_releases_layout`, `window_length`, `set_window_length` and `discover_selection_rows`.
   - Call `refresh_new_releases_layout` at the end of `replace_tree`, `show_tracks_destination`, `show_needs_attention_report`, `refresh_needs_attention_report`, `begin_recent_music_content_load`, `replace_recent_music_content` and `set_window_length`.
7. `src/view_models/detail_selection.rs`: replace `discover_rows_with_selection` with `discover_items_with_selection`. Move its existing tests to the new function.
8. `src/library/detail_selection.rs`: in the Discover arm of `library_detail_page_rows`, read `self.vm.discover_selection_rows(self.detail_selection.mode())`.
9. `src/ui/composites/thumbnail.rs`: add `ThumbnailSize::Xl` with the base 152 px and `ImageSize::Xl`. Add the arm in each `match` of the type.
10. `src/ui/shells/library/content_list.rs`:
    - `render_library_content_list` and the rows and tiles renderers take `Vec<ContentListItemVm>`.
    - `render_discover_new_releases_detail` takes `&NewReleasesLayout` and a `&RowArts` of the group art. It calls `discover_items_with_selection`.
    - Add `render_window_group_tile` and `render_window_group_row`. Each uses `CollectionArt`, the `TagBadge` of `EntityKind::Release` with `kind_badge`, the state word and the count. The colors and sizes are those of the release tile and the release row.
    - A group tile and a group row call `click_content_row` with `open_row_id`. They show no checkbox.
11. `src/library/app_impl.rs`:
    - In `render_discover_view_detail`, resolve the art of each group with `collection_art_image`. Use `ThumbnailSize::Xl` in the Tiles mode and `ThumbnailSize::Sm` in the List mode. Key the `RowArts` by `element_id`.
    - At line 4827, wrap each list column row in `ContentListItemVm::Row`.
12. `tests/architecture_tests.rs`: add the guard of case R86-2-27.

## Mechanical Acceptance Criteria

Use the prefix `adr_0086_window_groups_` for each test. Each test is a unit test beside its owner code, apart from R86-2-27. The dates are UTC midnights of 2026. "One key" means one `SinglesGroupKey`.

| Case | Required proof |
|---|---|
| R86-2-01 | Singles of one key on Oct 20, Oct 15, Oct 8 and Oct 1, with `WindowLength::DEFAULT`: `window_groups` gives one group of three and one single. |
| R86-2-02 | In R86-2-01, the group has its anchor on Oct 20 and the members Oct 20, Oct 15 and Oct 8. The single of Oct 1 is `Plain` at its own position. |
| R86-2-03 | Singles on Oct 20, Oct 8 and Oct 1 give a group of Oct 20 and Oct 8, and Oct 1 as `Plain`. The chain does not slide from Oct 8. |
| R86-2-04 | A single exactly 14 days before the anchor joins the group. A single one second earlier does not join. |
| R86-2-05 | Singles on Oct 20, Oct 15, Oct 1 and Sep 25 give two groups, with their anchors on Oct 20 and Oct 1. |
| R86-2-06 | One single of a key gives `Plain`. `NewReleasesLayout` gives its row as `Release`, and its kind badge stays "Single". |
| R86-2-07 | An album and an EP of the key, dated between two singles, have no `group_key`. They show as `Plain` at their positions, and the two singles still make one group. |
| R86-2-08 | A single with no `release_date` gives `Plain`. It never anchors a group and never joins one. |
| R86-2-09 | Singles of `Publisher(A)` and `Publisher(B)` with one artist text never share a group. A `Name` key and a `Publisher` key never share a group. |
| R86-2-10 | The group item has the position of its anchor. The other members give no item. Each other row keeps its order. |
| R86-2-11 | Two singles with one date: the row with the lower position is the anchor. |
| R86-2-12 | `closed_by_rows` is true when an entry has an `order_date` before `window_start`, or no `order_date`. Else it is false. |
| R86-2-13 | `NewReleasesLayout::build` reads an Index row: the confirmed artist feed of `FeedView::named_feeds` gives `Publisher(guid)`, and `newest_item_at` gives the date. |
| R86-2-14 | An Index row with no named feed, whose artist text matches one Library artist feed G, groups with a row of `Publisher(G)`. A name that matches two feeds keeps its `Name` key. |
| R86-2-15 | A Library row with `newest_item_at` on Oct 19 and a track `pub_date` on Oct 1 takes Oct 1. It does not join the window of an anchor on Oct 20. |
| R86-2-16 | A row with an empty artist text, a row with the text "Unknown Artist", and a row with no feed GUID each give `Plain`. |
| R86-2-17 | The tile of R86-2-01 gives the title of the key, `date_span` "Oct 8 to Oct 20", `kind_badge` "3 Singles" and `count` 3. |
| R86-2-18 | Nine singles with five Library rows give the word "New" and `count_text` "5 in library". Nine Library rows give "In library" and no count. |
| R86-2-19 | A group with no Library row gives the word "New" and no `count_text`. |
| R86-2-20 | The tile gives `a11y_label` "jSmovie: 3 singles, Oct 8 to Oct 20, New, 1 in library", and an `element_id` with `WINDOW_GROUP_ELEMENT_PREFIX`. |
| R86-2-21 | The tile `art` is `SingleCover` of the anchor cover. With no anchor cover, it is `EntityColor(CollectionKind::SinglesGroup)`. |
| R86-2-22 | `window_span_text` gives "Oct 20" for two times on Oct 20. It gives the two full `fmt_date` texts for Dec 28, 2025 and Jan 3, 2026. |
| R86-2-23 | `open_row_id` is the row id of the anchor. `discover_selection_rows(Normal)` gives each plain row and each anchor row, in item order, and no other member. |
| R86-2-24 | `discover_selection_rows(Selecting)` gives each visible row. `discover_items_with_selection` in `Selecting` gives only `Row` items, each with a visible checkbox. |
| R86-2-25 | In `Normal`, `discover_items_with_selection` gives the groups. A group tile is focused when its anchor key has the focus, and it has no visible checkbox. |
| R86-2-26 | The items of the view model are the same in the Tiles mode and in the List mode. |
| R86-2-27 | A situational guard in `tests/architecture_tests.rs`, with the name `adr_0086_window_groups_built_once_in_the_view_model`, passes. It fails against a copy of `src/view_models/library.rs` that calls `NewReleasesLayout::build(` two times. |
| R86-2-28 | `replace_recent_music_content` with an appended page that adds a single to a window gives the group in `new_releases_layout()`, with no other call. |
| R86-2-29 | `replace_tree` with a tree that now holds one member changes the Library count of the group, with no other call. |
| R86-2-30 | `set_window_length` with 7 days divides the group of R86-2-01. `from_days` gives `None` for 0 and 61, a value for 1 and 60, and `DEFAULT.days()` is 14. |
| R86-2-31 | `index_feed_display` sets `newest_item_at` from the detail feed. With the list feed of a failed detail read, it sets the value of the list feed. |
| R86-2-32 | `ThumbnailSize::Xl.image_size()` is `ImageSize::Xl`. |
| R86-2-33 | Each test of packet 020, packet 026, packet 032 and ADR 0084 task 033 still passes. |

The guard of R86-2-27 enforces these rules:

- `src/view_models/library.rs` calls `NewReleasesLayout::build(` one time, inside `fn refresh_new_releases_layout`.
- No file under `src/ui/`, `src/library/` or `src/app/` calls `window_groups(` or `NewReleasesLayout::build(`.
- `window_groups(` is defined only in `src/view_models/window_groups.rs`.

The failure message of the guard is this text:

```text
ADR 0086 §2 A Group Tile Takes The Place Of Its Newest Single: the New releases window groups are built only in `LibraryViewModel::refresh_new_releases_layout`.
Read `LibraryViewModel::new_releases_layout()` in the render and in each handler. Do not build the groups again.
```

## Visual Acceptance Criteria

These are for the operator. No test proves them.

- V1: open Music, Discover, New releases, in the Tiles mode, at normal width. jSmovie shows one group tile for each window, not one tile for each single. The first screen shows more artists than before. Examine Light and Dark.
- V2: a group tile shows the anchor cover as a square, the artist name, the date span, the "n Singles" badge and the state word. When some singles are in the Library, the count shows after "New". The tile has the same size and alignment as a release tile.
- V3: change to the List mode. The same groups show as rows, with the same facts. The art, the title and the badges have the same positions as in a release row. Examine Light and Dark.
- V4: make the window narrow. In the Tiles mode and the List mode, no text overlaps and no text goes outside its tile or row.
- V5 (replaced by packet 004 D10 on 2026-10-23): click a group tile. The release page of the newest single of the group opens. Back shows New releases at the same scroll position.
- V6: move the keyboard focus with the arrow keys. The focus reaches a group tile, and Enter opens the same page as a click.
- V7 (replaced by packet 004 D11 on 2026-10-23): click "Select". Each single shows as its own tile, with its checkbox. Click "Done". The groups show again.
- V8: click "Load more" until a page adds a single to the window of a group or of a single tile. The tile changes in place, with no navigation.

## Exclusions

- No extra page loads, no "+" on a count and no open state in the interface (packet 003).
- No window group page, no breadcrumb item, no group checkbox and no group selection (packet 004).
- No window groups in the saved searches (packet 005).
- No Settings value and no `config.toml` key (packet 006).
- No change to the Releases list, the artist pages, the Decision 6 group page or `src/view_models/singles_group.rs`.
- No change to `fetch_recent_feed_result_rows`, to a request profile or to the list request.
- No change to the recency order or to the page size.
- No schema change and no new database query.

## Files To Inspect

- [AGENTS.md](../../AGENTS.md), the rust-dev skill and [the source map](../architecture/source-map.md).
- [ADR 0086](../adr/0086-new-releases-singles-window.md), and ADR 0084 Decisions 3, 6, 8, 10 and 12.
- `src/view_models/singles_group.rs`: `SinglesGroupKey`, `GroupSingle`, `library_singles_grouped`.
- `src/view_models/artist_identity.rs`: `ArtistIdentity::resolve`, `NameFeedMatches::get`.
- `src/view_models/library.rs`: `ContentListRowDisplay`, `ContentListPageVm`, `release_kind_of_feed_result`, `mark_recent_library_releases`, `tree_name_feed_matches`, `AlbumNode`.
- `src/view_models/search_results/results.rs` and `src/application/queries/search.rs` (`index_feed_display`).
- `src/views.rs`: `FeedView`, `named_feeds_from_api`, `track_count`.
- `src/view_models/detail_selection.rs` and `src/library/detail_selection.rs`.
- `src/view_models/collection_art.rs`, `src/presentation/collection_art.rs`, `src/ui/composites/collection_art.rs` and `src/ui/composites/thumbnail.rs`.
- `src/ui/shells/library/content_list.rs`, `src/ui/shells/library/destination_lists.rs` (`RowArts`) and `src/library/app_impl.rs` (`render_discover_view_detail`).
- `tests/architecture_tests.rs`: `adr_0084_name_level_r17_one_count_function`, for the pattern of a guard and its mutated copy.

## Checks

```bash
cargo check
cargo fmt -- --check
cargo clippy --all-targets -- -D warnings
cargo test --lib adr_0086_window_groups_
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
- This packet: `docs/tasks/adr-0086-task-002-new-releases-window-groups.md`
- ADR 0086 Decisions 1, 2, 3 and 5
- ADR 0084 Decisions 3, 6, 8, 10 and 12, and packets 020, 026 and 032
- Each file in "Files To Inspect"

Goal:
- Make each change in "Required Changes".

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- Follow each decision in "Orchestrator Decisions". Do not change a decision.
- Use each name in "Names For Later Packets" exactly.
- The view model gives each text, image and availability. The renderer only composes. Use named tokens. Column text does not call `.truncate()`.
- Tests use recorded values. No test sends a request to a live service.
- Background work runs through the ADR 0040 runtime. Do not call `cx.spawn` from a screen.
- Never run `git add`, `git rm`, `git commit`, `git checkout`, `git restore`, `git stash` or `git reset`.
- Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- The database schema and the migration registry.
- `src/view_models/singles_group.rs`, the Releases list and the artist pages.
- `src/application/queries/feed.rs` and `src/application/request_profiles.rs`.
- `src/config.rs` and `config.toml`.
- Any ADR, and each document other than this packet.

Acceptance criteria:
- Each case R86-2-01 to R86-2-33 has a passing test or guard.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.
- The packet has an "Implementation Result - <date>" section and an "Operator Visual Check" section for V1 to V8.

At the end, report: files changed, tests run, behavior changed, deviations, unresolved concerns, and the document changes that the orchestrator must make.

Stop and report the problem, and do not guess, when:
- A change needs a new architecture decision.
- A change needs a file in "Do not touch".
- A recorded `GET /v1/feeds/{guid}` response with `include=publisher` gives no `music_to_publisher` entry for a single that `named_feeds` links to its artist feed. The identity then needs `include=named_feeds` on the list request. That is a new request profile, and the orchestrator must decide it.
- Packet 001 is not complete, or it changed `index_feed_display`.
