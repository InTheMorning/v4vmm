# ADR 0086 Task 004: The Window Group Page

Status: Implemented - 2026-10-23. Cases R86-4-01 to R86-4-27 pass. Mechanical checks are Green. The operator passed V1 to V3, V5, V7 and V8 on 2026-10-24. V4, V6, V9 and V10 are open, in the final walk of the run of 2026-10-23.

Orchestrator note - 2026-10-23: packet 002 is implemented. These names are `#[cfg(test)]` until a packet calls them: `WindowLength::{MIN_DAYS, MAX_DAYS, from_days, days}`, `LibraryViewModel::{window_length, set_window_length}` and `NewReleasesLayout::any_window_open`. Remove the `#[cfg(test)]` of each name that this packet calls. The group renderers are in `src/ui/shells/library/window_groups.rs`. The tile has no `anchor_date` and no `window_start`. Add them to `WindowGroupTileVm` from the `WindowGroup` of `NewReleasesLayout::build`.

## Goal

A click on a group tile of New releases opens a window group page. The page is one page of ADR 0084 Decision 4. It goes into the navigation stack of ADR 0084 Decision 9. The breadcrumb shows it, for example "Discover › New releases › jSmovie · 9 singles".

The page lists the singles of the window, newest first. Its main action is "Download n singles" when the Library does not hold each single. Else its main action is "Add to playlist". The page links to the ADR 0084 Decision 6 group page of the key, for example "All 23 singles".

In selection mode, a group tile stays one tile. Its checkbox selects each single of the window (ADR 0084 Decision 10).

This packet replaces two interim decisions of [packet 002](adr-0086-task-002-new-releases-window-groups.md):

- **Packet 002 D8 is replaced.** In packet 002, a click on a group tile opens the release page of its anchor single. After this packet, a click and Enter open the window group page. Ctrl+click and Shift+click select each single of the window, not only the anchor.
- **Packet 002 D9 is replaced.** In packet 002, selection mode shows each single as its own tile. After this packet, selection mode shows the same group tiles as the normal mode. Each group tile shows a checkbox. The checkbox can be checked, mixed or clear.

This packet starts after packet 002 is complete. It does not need packet 003.

## Authority

- [ADR 0086](../adr/0086-new-releases-singles-window.md) Decision 4: the window group page, its breadcrumb, its row order and its main action. It also owns the link to the Decision 6 group and the checkbox of a group tile.
- ADR 0086 Decision 1: the window, its key and the release date of each kind of single.
- ADR 0086 Decision 3: the facts of the group tile.
- ADR 0086 Consequences: "The app adds one page kind: the window group page".
- [ADR 0084](../adr/0084-music-column-browser.md) Decision 4: one page for each item. A page shows its Library state, and its actions follow that state.
- ADR 0084 Decision 6: the group page of a key, its main action, its "All n singles" link, and the "Matched by name" rows.
- ADR 0084 Decision 9: one navigation stack, a location breadcrumb, and the scroll position of Back and Forward.
- ADR 0084 Decision 10: the two selection modes, the action bar and the one download queue.
- ADR 0084 Decision 11: each track row keeps a place for audition.
- [Packet 002](adr-0086-task-002-new-releases-window-groups.md): the "Names For Later Packets", and decisions D8 and D9, which this packet replaces.
- [AGENTS.md](../../AGENTS.md): renderer portability, typed action state, "Current-view state must update in place", "Every fix gets a guard" and "A guard names its class and its ADR".

## Recorded Facts - 2026-10-23

The orchestrator read each fact in the code on 2026-10-23. Packet 002 is not implemented. A fact with the mark **[Packet 002 design]** comes from the packet 002 text, not from the code. Each other fact gives the file and the line in the code of today.

### The Decision 6 group page

- `src/view_models/singles_group.rs:85` `SinglesGroupKey(pub ArtistIdentity)` derives `Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash`.
- `src/view_models/singles_group.rs:89-100` `GroupSingle` holds `artist`, `album`, `key`, `date` and `in_library`. Lines 405-407 set `date` to the latest track `pub_date`, and `in_library` to "each track is in the Library".
- `src/view_models/singles_group.rs:105-115` `IndexGroupSingle` holds an `&api::Feed`. A New releases row holds a `FeedView`, not an `api::Feed`, so a New releases row cannot make an `IndexGroupSingle`.
- `src/view_models/singles_group.rs:524-530` `singles_groups_everything(tree, fills)` gives the groups of the scope "Everything". `:665-671` `find_group(groups, key)` finds a group by its key. `:262-269` `SinglesGroup::contains(key)` tells if a group holds a release.
- `src/view_models/singles_group.rs:830-845` `singles_group_name_link` gives "All {count} singles", with `AlbumNameLinkTarget::SinglesGroup(group.key())`.
- `src/view_models/singles_group.rs:900-911` `group_page_feed_to_read(key, fills)` gives the artist feed to read for a feed key.
- `src/view_models/singles_group.rs:914-922` `SinglesGroupAction` is `DownloadSingles(ReleasesDownloadPlan)` or `AddToPlaylist(Vec<i64>)`. `:925-948` `SinglesGroupActionVm` holds the label, the accessibility labels and `available`.
- `src/view_models/singles_group.rs:952-967` `SinglesGroupPageRowVm` is the row of a Library single: its first track, `date_text`, `work` and `key`.
- `src/view_models/singles_group.rs:970-987` `SinglesGroupIndexRowVm` is the row of a `MusicIndex` single. `:989-1021` its constructor `new` takes an `IndexGroupSingle`.
- `src/view_models/singles_group.rs:1025-1043` `SinglesGroupPageEntryVm` is `Library` or `Index`, with `key()`.
- `src/view_models/singles_group.rs:1046-1083` `SinglesGroupPageVm` holds `title`, `subtitle`, `identity_note`, `rows`, `index_status`, `more_note`, `main_action`, `empty_message`, `art` and `matched_by_name`.
- `src/view_models/singles_group.rs:1086-1091` `MatchedByNameHeadingVm` holds the heading, the note and the count of the trailing rows.
- `src/view_models/singles_group.rs:1093-1170` `SinglesGroupPageVm::from_tree(tree, fills, key, busy)` builds the page at each call. A group is busy when one single downloads or one removal runs (lines 1176-1184).
- `src/view_models/singles_group.rs:1212-1226` `library_single_row` is private. It gives `None` for a single with no track.
- `src/view_models/singles_group.rs:1302-1357` `main_action` is private. It counts the singles that are not in the Library and gives "Download n singles", else "Add to playlist".
- `src/view_models/library.rs:4041-4048` `LibraryViewModel::singles_group_page(key)` calls `from_tree` with the tree, the loaded `MusicIndex` singles and the busy items.

### The shell of the Decision 6 page

- `src/ui/shells/library/singles_group.rs:62-...` `render_library_singles_group_page` renders the art, the heading, the main action, the rows and the "Matched by name" heading.
- `src/ui/shells/library/singles_group.rs:54` `MAIN_ACTION_ID`. `:257` `render_main_action`, `:315` `render_single_row` and `:348` `render_index_single_row` are private functions.
- `src/ui/shells/library/singles_group.rs:378-398` `index_single_dispatch` sends each command of a `MusicIndex` row to `LibraryApp::run_singles_group_index_row_command` (`src/library/release_page.rs:164-208`). That function does not read `LibraryDetail`.
- `src/ui/shells/library/detail.rs:107-119` the `LibraryDetail::SinglesGroup` arm renders the page from `library_vm.singles_group_page(key)` at each render.
- `src/ui/shells/entity.rs:311-355` `render_album_name_links` renders each `AlbumNameLinkVm` as a ghost button.
- `src/library/app_impl.rs:655-694` `open_album_name_link` opens `AlbumNameLinkTarget::SinglesGroup(key)` with `NavigationPathStart::CurrentPath`.

### Navigation and breadcrumb

- `src/view_models/workspace/nav.rs:105-149` `FrameNavigationEntry` derives `Clone, Debug, Eq, PartialEq`. Line 137 is `SinglesGroup(SinglesGroupKey)`.
- `src/view_models/workspace/nav.rs:160-197` `display_label` gives "Singles" for a group page before the Library gives a title.
- `src/view_models/workspace/nav.rs:88` the label of `DiscoverView::NewReleases` is "New releases". Line 45 gives "Discover".
- `src/view_models/workspace/nav.rs:699` `FrameNavigationState::path_entries`, `:745` `push`, `:846` `go_back` and `:863` `go_forward`.
- `src/view_models/artist_identity.rs:67-97` `ArtistNameKey` holds `key` and `label`. Its `PartialEq` compares only `key`.
- `src/view_models/workspace/breadcrumb.rs:124-139` `breadcrumb_entry_id` matches each entry. `:187-192` `singles_group_id` gives the id of a group page.
- `src/app/breadcrumb.rs:28-53` `content_list_breadcrumb_label` asks `LibraryApp::navigation_label` for a playlist page and a group page.
- `src/library/app_impl.rs:2830-2846` `LibraryApp::navigation_label` gives the page title of a group page.
- `src/app.rs:1132` the frame title fallback of a group page is "Singles". `src/app.rs:1512-1528` the detail frame title of a group page is its `navigation_label`.
- `src/app.rs:782-822` `open_music_page` pushes the entry, then calls `restore_content_list_nav_entry` (`:832-850`), which calls `LibraryApp::hydrate_detail_from_nav`.
- `src/library/app_impl.rs:2520-2570` `hydrate_detail_from_nav` sets `LibraryDetail::SinglesGroup` and requests the `MusicIndex` singles of the feed (`src/library/music_scopes.rs:205-215`).
- `src/library/app_impl.rs:2607-2617` `show_discover_view` sets the Discover destination and calls `replace_recent_music_content` with `recent_music_page`.
- `src/library.rs:255` `LibraryApp::recent_music_page` keeps the loaded New releases rows when the curator opens another destination.
- `src/library.rs:55-67` `LibraryDetail` has `None`, `Album`, `Track`, `Playlist`, `Discover` and `SinglesGroup`. `src/library/app_impl.rs:1504-1512` and `:3272-3281` match each variant.

### Selection in the detail column

- `src/view_models/list_selection.rs:75-89` `SelectionRow { id, members }`. `SelectionRow::new(id, members)` makes a row of two or more members.
- `src/view_models/list_selection.rs:109-116` `RowCheckState` is `Checked`, `Mixed` or `Clear`.
- `src/view_models/list_selection.rs:182-196` `ListSelection::toggle` removes each member when each member is selected. Else it adds each member.
- `src/view_models/list_selection.rs:200-233` `extend_to` adds each member of each row of the range. `:236-242` `select_all` adds each member of each row.
- `src/view_models/list_selection.rs:339-352` `row_check_state(members)` gives the checked state of a row.
- `src/view_models/list_selection.rs:421-438` `RowSelectionVm::for_group_row` gives the label "{title}, some singles selected" for `Mixed`. `:444-446` `is_marked` is true only for `Checked`.
- `src/view_models/detail_selection.rs:85-95` `DetailListKind::for_entry` gives `Releases` for `SinglesGroup` and for `DiscoverView(NewReleases)`.
- `src/view_models/detail_selection.rs:285-293` `DetailPageRows` is `Tracks`, `Releases` or `PlaylistEntries`. `:331-336` `selection_rows` gives one member for each row.
- `src/view_models/detail_selection.rs:565-569` `DetailSelection` holds `ListSelection<DetailRowKey>`. `:669-697` `click` and `:700-703` `toggle_row` make a row of one member with `SelectionRow::of_one`.
- `src/view_models/detail_selection.rs:916-934` `singles_group_release_rows` gives the selection rows of a group page.
- `src/view_models/detail_selection.rs:1039-1053` `discover_release_rows` and `:1059-1069` `discover_rows_with_selection` serve the Discover view today.
- `src/library/detail_selection.rs:225-258` `library_detail_page_rows` gives the rows of each Library detail page. Lines 240-245 serve the Discover view.
- `src/library/detail_selection.rs:262-282` `click_detail_row`, `:307-321` `click_discover_row`, `:324-328` `toggle_discover_row` and `:414-418` `toggle_detail_row`.
- `src/library/detail_selection.rs:470-530` the keys Up, Down, Space and Ctrl+A each read `rows.selection_rows()`.
- `src/app/detail_selection.rs:252-261` `open_focused_detail_row` opens the page of the focused key with `open_detail_row`. For a release key, it opens the release page.
- `src/ui/composites/selectable_tile.rs:63-71` `SelectableTile::checkbox` takes a checkbox element and `selected` takes the mark. `src/ui/composites/track_row.rs:613-619` `checkbox_state_for` maps `RowCheckState` to `CheckboxState`, with `Mixed`.

### Downloads

- `src/library/downloads.rs:60-72` `download_release_plan(plan)` queues the Library feeds and the `MusicIndex` releases of a `ReleasesDownloadPlan` through the one download queue.
- `src/view_models/library.rs:5357` `BusyItems::release_downloading`, `:5389` `index_release_work`, `:5397` `index_release_downloading`, `:5403` `removing` and `:5420` `track_row_work`.
- `src/view_models/library.rs:3028-3033` `replace_tree` replaces the tree and marks the New releases rows that the Library holds.

### The rows of New releases

- `src/application/queries/search.rs:810-845` `index_feed_display` sets the row label to the feed title and attaches `FeedView::from_api(feed)` as `remote_feed`.
- `src/views.rs:145` `FeedView::tracks` and `:166` `TrackView::duration_secs`. `src/view_models/track.rs:20` `fmt_dur(secs: i32)`.

### Packet 002 names that this packet uses

- **[Packet 002 design]** `WindowGroupTileVm` holds `element_id`, `open_row_id`, `key`, `member_keys`, `title`, `date_span`, `kind_badge`, `count`, `state`, `a11y_label`, `art`, `window_closed` and `selection`.
- **[Packet 002 design]** `WindowGroup` holds `key`, `anchor`, `members`, `anchor_date`, `oldest_date`, `window_start`, `in_library_count` and `closed_by_rows`. `members` holds positions, newest first.
- **[Packet 002 design]** `WindowEntry` holds `position`, `key`, `group_key`, `release_date`, `order_date` and `in_library`.
- **[Packet 002 design]** `NewReleasesLayout::build(rows, tree, length)`, `items()`, `shown_row_ids()` and `any_window_open()`. `LibraryViewModel::new_releases_layout()` gives the cached layout. `refresh_new_releases_layout` builds it again after each change to the rows, the tree or the length.
- **[Packet 002 design]** `discover_items_with_selection(rows, layout, selection)` gives `ContentListItemVm::Row` or `ContentListItemVm::WindowGroup`. In `Selecting`, it gives only `Row` items (packet 002 D9).
- **[Packet 002 design]** `LibraryViewModel::discover_selection_rows(mode)` gives the plain rows and the anchor rows in `Normal`, and each visible row in `Selecting` (packet 002 D9 and D10).
- **[Packet 002 design]** `render_window_group_tile` and `render_window_group_row` call `click_content_row` with `open_row_id` and show no checkbox (packet 002 Required Change 10).
- **[Packet 002 design]** `window_span_text(oldest, anchor)` gives "Oct 8 to Oct 20". `WINDOW_GROUP_ELEMENT_PREFIX` is "music-window-group-".

## Design

### One new page kind

The window group page is a new page kind. It is not the Decision 6 page with a window filter.

- The entry is `FrameNavigationEntry::WindowGroup(WindowGroupKey)`.
- The Library detail is `LibraryDetail::WindowGroup(WindowGroupKey)`.
- The view model is `WindowGroupPageVm` in the new module `src/view_models/window_group_page.rs`.
- The shell is `render_library_window_group_page` in the new file `src/ui/shells/library/window_group.rs`.

The page reuses the row types, the main action type and the row renderers of the Decision 6 page. Thus one row style and one main action show on the two pages.

### The key of a window

`WindowGroupKey` identifies one window. It is in `src/view_models/window_groups.rs`.

- `group: SinglesGroupKey`: the key of the artist.
- `anchor: ReleaseKey`: the anchor single when the page opened.
- `anchor_date: i64` and `window_start: i64`: the limits of the window when the page opened.
- `artist: String`: the tile title when the page opened. Equality does not compare it, as with `ArtistNameKey`.

The page resolves its members from the key at each render:

1. Take each `WindowEntry` of the cached layout whose `group_key` is `group`.
2. Keep each entry whose `release_date` is at or after `window_start` and at or before `anchor_date`.
3. Sort the entries newest first. Equal dates keep the list order.

Thus Back and Forward show the window that the curator opened, also after later pages or a change of the window length changed the tiles. A later page can add an older single to an open window (packet 003). The page then shows that single in place.

### The rows of the page

The members come from the loaded New releases rows, through the cached layout. The page thus shows the same singles that the tile counts.

- **Library member.** The tree holds an album with the feed GUID of the row. The page makes a `GroupSingle` from the artist node and the album, and builds the row with `library_single_row`. The row is the shared Library track row.
- **Index member.** The row has no Library album. The page builds `SinglesGroupIndexRowVm::from_feed_result` from the `FeedResultDisplay` of the row. The title is the row label, and the image is `thumbnail_href`. The duration is the first track of `remote_feed`, and the date is `release_date`.
- **Matched by name.** A member whose own identity is a `Name` key, joined to `group` by `tree_name_feed_matches`, comes after the other members. The rows keep the heading and the note of `MatchedByNameHeadingVm`.

### The header and the title

- `title`: `window_group_title(artist, count)`, for example "jSmovie · 9 singles". One member gives "jSmovie · 1 single". No member gives the artist name alone.
- The breadcrumb label and the detail frame title are `title`.
- `date_span`: `window_span_text` of the oldest member date and the newest member date. `None` with no member.
- `art`: `CollectionArtVm::resolve(None, [newest member cover], CollectionKind::SinglesGroup)`, the same art as the tile.
- `artist`: the title of the anchor member as in packet 002, else `WindowGroupKey::artist`.

### The main action

`singles_main_action` is the one owner of the label text of the two pages. The Decision 6 page and the window page call it.

- A member is in the Library when each track of its Library album is in the Library. This is the rule of the Decision 6 page.
- When one or more members are not in the Library, the action is "Download n singles". The plan holds the Library feed id of each Library member that is not in the Library. It also holds the feed GUID and the title of each Index member.
- Else the action is "Add to playlist", with the Library feed id of each member.
- The action is not available while one member downloads or one removal of a member runs.

### The link to the Decision 6 group

`all_singles_link` is `Some` when these conditions are true:

- `find_group(&singles_groups_everything(tree, fills), &key.group)` gives a group.
- That group holds each member of the window (`SinglesGroup::contains`).

The label is "All {count} singles", from `singles_group_name_link`. The target is `AlbumNameLinkTarget::SinglesGroup(key.group)`. The link opens the Decision 6 page in the current path. Each open of the window page requests the `MusicIndex` singles of the feed with `request_singles_group_feed`. The link shows in place when the result arrives.

### The window that no longer exists

The loaded New releases rows can change after a new first load. Then the layout can have no row of the window.

- The page shows `WINDOW_GROUP_EMPTY_MESSAGE` and no main action.
- The title and the breadcrumb label are `WindowGroupKey::artist`.
- The link to the Decision 6 group shows when that group exists. It has no member condition, because the window has no member.

### The page in the path

A click on a group tile opens the page with `NavigationPathStart::CurrentPath`. The path is `Destination(Discover)`, `DiscoverView(NewReleases)`, `WindowGroup(key)`. The breadcrumb is "Discover › New releases › jSmovie · 9 singles".

`hydrate_detail_from_nav` for `WindowGroup(key)`:

1. Set the Discover destination.
2. Call `replace_recent_music_content(&self.recent_music_page)`, so the layout holds the loaded rows after another destination.
3. Set `LibraryDetail::WindowGroup(key)`.
4. Call `request_singles_group_feed(&key.group)`.

### The group tile in selection mode

`DetailPageRows::GroupedReleases { rows, tiles }` gives the rows of the Discover view.

- `rows` holds one `DetailReleaseRow` for each shown release and each member of each group, in item order. The action bar counts these rows.
- `tiles` holds one `SelectionRow` for each shown item. A plain item has one member. A group tile has the id of its anchor key and each member key.
- `selection_rows()` gives `tiles`. Up, Down, Space, Ctrl+A and Shift+click thus act on whole tiles.

`DetailSelection::click` finds the row of the key in `visible`. A group tile thus toggles each member. `DetailSelection::toggle_selection_row` is the checkbox of a group tile.

`discover_items_with_selection` follows the layout in both modes. A group tile gets `DetailSelection::tile_selection`, which reads `row_check_state` of its members and calls `RowSelectionVm::for_group_row`.

## Orchestrator Decisions

- **D1. The window group page is a new page kind.** ADR 0086 Consequences state "The app adds one page kind". The Decision 6 page reads the groups of "Everything" and the loaded `MusicIndex` singles of a feed. The window page reads the loaded New releases rows. A filter on the Decision 6 page cannot give the Index members, because `IndexGroupSingle` needs an `api::Feed`.
- **D2. The page reuses the Decision 6 row types, the action type and the row renderers.** `SinglesGroupPageEntryVm`, `SinglesGroupActionVm` and the shell functions `render_single_row`, `render_index_single_row` and `render_main_action` serve the two pages. This keeps one track row style (packet 032) and one main action.
- **D3. A window is the key of the artist, the anchor and the two limits.** `WindowGroupKey` holds `group`, `anchor`, `anchor_date` and `window_start`. Equality compares these four fields. The artist name is a label only. The four fields stay correct after Back and Forward, also when later pages changed the tiles.
- **D4. The page resolves its members from the stored limits, not from the current chain.** A newer single or a shorter window length can change the current tiles. The history entry still shows the window that the curator opened. A later page of older rows can add a member to an open window, and the page shows it in place.
- **D5. The members come from the loaded rows, and the Library decides a Library member.** The page shows the singles that the tile counts. A row whose feed GUID the tree holds is a Library member, as in packet 002 D3. Each other row is an Index member.
- **D6. Matched-by-name members come last, under the "Matched by name" heading.** ADR 0084 Decision 6 marks a single that a name joins as inferred. The window page keeps that mark. The other members come first, newest first.
- **D7. The main action uses the rule of the Decision 6 page.** A member is in the Library when each of its tracks is in the Library. A tile of packet 002 D7 can say "In library" for a single that has tracks that are not downloaded. For that single, the page gives "Download 1 single". This difference is correct, because the two texts give two facts.
- **D8. The link shows only when the Decision 6 group holds each member.** "All n singles" must not show a count lower than the window. A name group holds no `MusicIndex` single, so a name window with Index members has no link.
- **D9. A gone window shows an empty state.** The page shows a message, the artist name and the link to the Decision 6 group. It shows no main action. Back stays available.
- **D10. Packet 002 D8 is replaced.** A plain click and Enter on a group tile open the window group page in the current path. Ctrl+click and Shift+click start selection mode and select each member of the tile. `open_row_id` stays the anchor row id. It now identifies the tile, and it no longer opens the anchor release.
- **D11. Packet 002 D9 is replaced.** Selection mode keeps the group tiles, and each group tile shows a checkbox. `discover_selection_rows` is deleted, because its mode split has no use. `LibraryViewModel::discover_page_rows` replaces it. The tests of packet 002 cases R86-2-23 and R86-2-24 that assert the split are deleted with it.
- **D12. A partial selection shows as mixed, and a click selects each member.** A later page can add a member to a selected group. The checkbox then shows `Mixed`, and the tile has no selected mark. A click, Space or Ctrl+click on a mixed tile selects each member. A second click clears each member.
- **D13. The page builds again at each render.** The page reads the cached layout, the tree, the loaded `MusicIndex` singles and the busy items. A download and a removal thus change the rows, the main action and the link in place, with no navigation.
- **D14. The title is "artist · n singles", and the breadcrumb uses the same text.** This follows the Decision 6 page, whose breadcrumb label is its title. The detail frame title is the same text.

## Required Changes

1. `src/view_models/window_groups.rs`:
   - Add `WindowGroupKey` with the fields of "The key of a window". Derive `Clone` and `Debug`. Implement `PartialEq` and `Eq` on `group`, `anchor`, `anchor_date` and `window_start`.
   - Add the field `WindowGroupTileVm::page_key: WindowGroupKey`. `WindowGroupTileVm::new` fills it from the group and the anchor entry.
   - Add `pub(crate) fn window_members(entries: &[WindowEntry], key: &WindowGroupKey) -> Vec<usize>`, with the three steps of "The key of a window".
   - `NewReleasesLayout` keeps its entries and the row id of each position, in private fields.
   - Add `NewReleasesLayout::window_member_row_ids(&self, key: &WindowGroupKey) -> Vec<&str>` and `NewReleasesLayout::group_tile_for_anchor(&self, anchor: &ReleaseKey) -> Option<&WindowGroupTileVm>`.
2. `src/view_models/singles_group.rs`:
   - Make `library_single_row` `pub(crate)`.
   - Move the body of `main_action` into `pub(crate) fn singles_main_action(missing: ReleasesDownloadPlan, missing_count: usize, library_feed_ids: Vec<i64>, count: usize, busy: bool) -> SinglesGroupActionVm`. `main_action` calls it. The labels do not change.
   - Add `SinglesGroupIndexRowVm::from_feed_result(row: &FeedResultDisplay, key: ReleaseKey, date: Option<i64>, work: TrackRowWork) -> Self`. The element id uses the prefix of `new`.
3. `src/view_models/window_group_page.rs`: add the module. Start it with a `//!` comment that names ADR 0086 Decision 4, and `#![warn(clippy::pedantic)]`. Add:
   - `pub(crate) struct WindowGroupPageVm { title: String, date_span: Option<String>, rows: Vec<SinglesGroupPageEntryVm>, matched_by_name: Option<MatchedByNameHeadingVm>, main_action: Option<SinglesGroupActionVm>, all_singles_link: Option<AlbumNameLinkVm>, art: Option<CollectionArtVm>, empty_message: Option<&'static str> }`.
   - `WindowGroupPageVm::build(key: &WindowGroupKey, layout: &NewReleasesLayout, rows: &[&ContentListRowDisplay], tree: &LibraryTree, fills: &IndexFeedSingles, busy: &BusyItems) -> Self`.
   - `pub(crate) fn window_group_title(artist: &str, count: usize) -> String`.
   - `pub(crate) const WINDOW_GROUP_EMPTY_MESSAGE: &str = "The loaded New releases hold no single of this window."`.
4. `src/view_models/mod.rs`: add `pub(crate) mod window_group_page;`.
5. `src/view_models/workspace/nav.rs`: add `FrameNavigationEntry::WindowGroup(WindowGroupKey)`. `display_label` gives `key.artist`. `opens_detail_page` gives `true`.
6. `src/view_models/workspace/breadcrumb.rs`: add `window_group_id(key)`, with the text `window-group-`, the slug of the identity and `window_start`. Use it in `breadcrumb_entry_id`.
7. `src/view_models/library.rs`:
   - Add `pub(crate) fn window_group_page(&self, key: &WindowGroupKey) -> WindowGroupPageVm`. It is the only caller of `WindowGroupPageVm::build`.
   - Add `pub(crate) fn discover_page_rows(&self) -> DetailPageRows`. It calls `discover_grouped_release_rows` with the cached layout and `library_release_facts`.
   - Delete `discover_selection_rows` and its tests. Move each test that does not assert the mode split to `discover_page_rows`.
8. `src/view_models/detail_selection.rs`:
   - Add `DetailPageRows::GroupedReleases { rows: Vec<DetailReleaseRow>, tiles: Vec<SelectionRow<DetailRowKey, DetailRowKey>> }`. Each method treats `rows` as `Releases` does, except `selection_rows` and `has_selectable_rows`, which read `tiles`.
   - Add `pub(crate) fn discover_grouped_release_rows(layout: &NewReleasesLayout, rows: &[&ContentListRowDisplay], facts: impl Fn(&ReleaseKey) -> Option<LibraryReleaseFacts>) -> DetailPageRows`.
   - `DetailListKind::for_entry` gives `Releases` for `WindowGroup(_)`.
   - `DetailSelection::click` finds the row of `key` in `visible`, else it uses `SelectionRow::of_one`.
   - Add `DetailSelection::toggle_selection_row(&mut self, row: &SelectionRow<DetailRowKey, DetailRowKey>)` and `DetailSelection::tile_selection(&self, row: &SelectionRow<DetailRowKey, DetailRowKey>, title: &str) -> RowSelectionVm`.
   - `discover_items_with_selection` follows the layout in both modes. A group tile gets `tile_selection`.
   - Delete `discover_release_rows` when no production caller stays. Move its tests to `discover_grouped_release_rows`.
9. `src/library.rs`: add `LibraryDetail::WindowGroup(WindowGroupKey)`.
10. `src/library/app_impl.rs`:
    - Add `open_window_group(&mut self, key: &WindowGroupKey, cx)`, which calls `open_page` with `NavigationPathStart::CurrentPath`.
    - Add the `WindowGroup` arm of `hydrate_detail_from_nav`, with the four steps of "The page in the path".
    - `navigation_label` gives `window_group_page(key).title` for `WindowGroup(key)`.
    - Add the `WindowGroup` arm for the row image URLs (lines 5101-5115) and for the header art (line 5161).
    - Add `LibraryDetail::WindowGroup(_)` to each match that the compiler names, with the `SinglesGroup` behavior.
11. `src/library/detail_selection.rs`:
    - The Discover arm of `library_detail_page_rows` gives `self.vm.discover_page_rows()`.
    - Add the `WindowGroup` arm. It gives `DetailPageRows::Releases(singles_group_release_rows(&page.rows, ...))`.
    - Add `click_window_group_tile(&mut self, open_row_id: &str, modifiers: Modifiers, cx)`. It finds the tile through `group_tile_for_anchor`, calls `click_detail_row` with the anchor key, and calls `open_window_group` for a plain click in `Normal`.
    - Add `toggle_window_group_tile(&mut self, open_row_id: &str, cx)`. It calls `toggle_selection_row` with the tile row.
    - Add `window_group_entry_for(&self, key: &DetailRowKey) -> Option<FrameNavigationEntry>`. It gives `WindowGroup(page_key)` when the Discover view is open and `key` is the anchor key of a group tile.
12. `src/library/release_page.rs`: `release_page_row` gives the Library rows of the `WindowGroup` page, as it does for the `SinglesGroup` page.
13. `src/app/detail_selection.rs`: in `open_focused_detail_row`, open `window_group_entry_for(&key)` through `open_music_page` with `CurrentPath`, when it gives an entry. Else keep `open_detail_row`.
14. `src/app/breadcrumb.rs`: add `FrameNavigationEntry::WindowGroup(_)` to the arm that asks `LibraryApp::navigation_label`.
15. `src/app.rs`: the frame title fallback of `WindowGroup(_)` is "Singles". The detail frame title override of line 1517 also applies to `WindowGroup(_)`.
16. `src/ui/shells/library/singles_group.rs`: make `render_main_action`, `render_single_row`, `render_index_single_row` and `render_matched_by_name_heading` `pub(super)`. Do not change their output.
17. `src/ui/shells/library/window_group.rs`: add `render_library_window_group_page`. It renders the art, the title, the date span, the main action, the link through `render_album_name_links`, the rows, the "Matched by name" heading and the empty message. The link calls `LibraryApp::open_album_name_link`. The main action uses the id `window-group-main-action`.
18. `src/ui/shells/library/mod.rs`: add the module `window_group`.
19. `src/ui/shells/library/detail.rs`: add the `LibraryDetail::WindowGroup(key)` arm, which renders `library_vm.window_group_page(key)`.
20. `src/ui/shells/library/content_list.rs`:
    - `render_window_group_tile` and `render_window_group_row` call `click_window_group_tile` with `open_row_id`.
    - When `selection.checkbox_visible` is true, they show a checkbox with `checkbox_state_for(selection.check_state)`. The checkbox calls `toggle_window_group_tile`. The tile mark is `selection.is_marked()`.
21. `tests/architecture_tests.rs`: add the guard of case R86-4-24.

## Mechanical Acceptance Criteria

Use the prefix `adr_0086_window_group_page_` for each test. Each test is a unit test beside its owner code, apart from R86-4-24. The dates are UTC midnights of 2026. "The window of R86-2-01" is the group of Oct 20, Oct 15 and Oct 8 of packet 002.

| Case | Required proof |
|---|---|
| R86-4-01 | `window_members` for the window of R86-2-01 gives the positions of Oct 20, Oct 15 and Oct 8, in that order. |
| R86-4-02 | `window_members` gives no entry of another `SinglesGroupKey`, no album and no single with no release date. |
| R86-4-03 | `WindowGroupKey` values with different `artist` texts are equal. A different `anchor`, `anchor_date` or `window_start` makes them different. |
| R86-4-04 | `WindowGroupTileVm::new` gives a `page_key` with the key, the anchor key, the `anchor_date` and the `window_start` of its group. |
| R86-4-05 | `window_group_page` gives the rows of the window newest first. A Library member gives `Library`, and an Index member gives `Index`. |
| R86-4-06 | A member that a name joins to the feed comes after the other members. `matched_by_name` gives a count of 1. |
| R86-4-07 | With one Index member, the main action is "Download 1 single". Its plan holds the feed GUID and the title of that member. |
| R86-4-08 | With two Library members that are not downloaded and one Index member, the main action is "Download 3 singles". |
| R86-4-09 | When each track of each member is in the Library, the main action is "Add to playlist". It holds the Library feed id of each member. |
| R86-4-10 | While one member downloads, the main action is not available. |
| R86-4-11 | The Decision 6 group of the feed holds 23 singles, with each member. `all_singles_link` gives "All 23 singles" and `AlbumNameLinkTarget::SinglesGroup` of the key. |
| R86-4-12 | `all_singles_link` is `None` when the Decision 6 group does not hold each member. It is also `None` when no group has the key. |
| R86-4-13 | `window_group_title("jSmovie", 9)` gives "jSmovie · 9 singles". A count of 1 gives "jSmovie · 1 single". |
| R86-4-14 | A path of `Destination(Discover)`, `DiscoverView(NewReleases)` and `WindowGroup(key)` gives these breadcrumb labels: "Discover", "New releases", "jSmovie · 9 singles". The label of the last segment comes from `window_group_page(key).title`. |
| R86-4-15 | After a push of `Release(member)`, `go_back` gives `WindowGroup(key)`. Then `go_forward` gives `Release(member)`, and `go_back` gives `WindowGroup(key)` again. |
| R86-4-16 | An appended page with a newer single of the key makes a new anchor for the tile. `window_group_page(old key)` still gives the three members of R86-2-01. |
| R86-4-17 | A first load with no row of the window gives `WINDOW_GROUP_EMPTY_MESSAGE`, no main action, no row, and the title `key.artist`. |
| R86-4-18 | `discover_page_rows` gives `GroupedReleases`. Its `tiles` has one row for each shown item, and a group row holds each member key. Its `rows` holds each member. |
| R86-4-19 | `discover_items_with_selection` in `Selecting` gives the group tile, with a visible checkbox. Packet 002 D9 is replaced. |
| R86-4-20 | A toggle of a group tile selects each member, and the tile is `Checked`. With one member selected, the tile is `Mixed` and `is_marked` is false. |
| R86-4-21 | A toggle of a `Mixed` tile selects each member. A toggle of a `Checked` tile clears each member. |
| R86-4-22 | `select_all` with the tiles selects each member. The action bar counts each member, not each tile. |
| R86-4-23 | `replace_tree` with a tree that now holds an Index member changes its row to `Library`. The main action count goes down by 1, with no other call. |
| R86-4-24 | A situational guard in `tests/architecture_tests.rs`, with the name `adr_0086_window_group_page_built_in_the_view_model`, passes. It fails against a copy of `src/ui/shells/library/window_group.rs` that calls `window_members(`. |
| R86-4-25 | `group_tile_for_anchor` gives the tile of an anchor key, and `None` for a member that is not the anchor. |
| R86-4-26 | `display_label` of `WindowGroup(key)` gives `key.artist`. `DetailListKind::for_entry` gives `Releases`. |
| R86-4-27 | Each test of packet 002 still passes, apart from the R86-2-23 and R86-2-24 tests that D11 deletes. Each test of packets 018, 020, 026 and 032 still passes. |

The guard of R86-4-24 enforces these rules:

- `WindowGroupPageVm::build(` is called only in `src/view_models/library.rs`, inside `fn window_group_page`.
- No file under `src/ui/`, `src/library/` or `src/app/` calls `window_members(` or `WindowGroupPageVm::build(`.
- `src/ui/shells/library/window_group.rs` does not name `NewReleasesLayout`.

The failure message of the guard is this text:

```text
ADR 0086 §4 The Group Opens A Window Group Page: the members, the main action and the link of a window group page are built only in `LibraryViewModel::window_group_page`.
Read `LibraryViewModel::window_group_page(key)` in the render and in each handler. Do not resolve the members of a window outside the view model.
```

## Visual Acceptance Criteria

These are for the operator. No test proves them.

- V1: open Music, Discover, New releases, in the Tiles mode, at normal width. Click a group tile of jSmovie. The window group page opens. Its rows are the singles of the window, newest first. Examine Light and Dark.
- V2: the breadcrumb shows "Discover › New releases › jSmovie · n singles". The detail frame title shows the same text as the last segment.
- V3: the header shows the art of the newest single, the title and the date span. The main action is "Download n singles" when some singles are not in the Library. Else it is "Add to playlist".
- V4: the link "All n singles" shows near the header. Click it. The Decision 6 group page opens, and the breadcrumb adds its title.
- V5: click Back two times. New releases shows at the same scroll position. Click Forward. The window group page shows the same singles.
- V6: on the window group page, click "Download n singles". Each row shows its progress. When the downloads finish, the rows and the main action change with no navigation.
- V7: on New releases, click "Select". The group tiles stay. Each group tile shows a checkbox. Click the checkbox of a group tile. The bar counts each single of the window.
- V8: in the List mode, do V1 and V7 again. The group rows act as the group tiles do.
- V9: make the window narrow. Do V1 and V3 again. No text overlaps, and no text goes outside the page.
- V10: with the keyboard, move the focus to a group tile and push Enter. The window group page opens.

## Exclusions

- No change to the chain, the tiles or the facts of packet 002, apart from D10, D11 and D12.
- No extra page loads and no "+" on a count (packet 003).
- No window group page for a saved search (packet 005). Packet 005 decides the source of the rows of that page.
- No Settings value and no `config.toml` key (packet 006).
- No change to the Decision 6 group page, apart from the extracted `singles_main_action` and the `pub(super)` renderers. Its labels and its rows do not change.
- No change to the Releases list or to the artist pages.
- No new request, no request profile, no schema change and no new database query.
- No audition control. ADR 0084 Decision 11 keeps its place empty until ADR 0068.

## Files To Inspect

- [AGENTS.md](../../AGENTS.md), the rust-dev skill and [the source map](../architecture/source-map.md).
- [ADR 0086](../adr/0086-new-releases-singles-window.md) Decisions 1, 3 and 4, and ADR 0084 Decisions 4, 6, 9, 10 and 11.
- [Packet 002](adr-0086-task-002-new-releases-window-groups.md), and its code after it is implemented.
- `src/view_models/window_groups.rs` and `src/view_models/library.rs` (`NewReleasesLayout`, `new_releases_layout`, `discover_selection_rows`, `singles_group_page`, `replace_tree`, `replace_recent_music_content`).
- `src/view_models/singles_group.rs` (`SinglesGroupPageVm`, `library_single_row`, `main_action`, `SinglesGroupIndexRowVm`, `singles_group_name_link`, `find_group`).
- `src/view_models/detail_selection.rs` and `src/view_models/list_selection.rs`.
- `src/view_models/workspace/nav.rs` and `src/view_models/workspace/breadcrumb.rs`.
- `src/library.rs`, `src/library/app_impl.rs`, `src/library/detail_selection.rs`, `src/library/release_page.rs` and `src/library/music_scopes.rs`.
- `src/app.rs`, `src/app/breadcrumb.rs` and `src/app/detail_selection.rs`.
- `src/ui/shells/library/singles_group.rs`, `src/ui/shells/library/detail.rs`, `src/ui/shells/library/content_list.rs` and `src/ui/shells/entity.rs` (`render_album_name_links`).
- `tests/architecture_tests.rs`: `adr_0086_window_groups_built_once_in_the_view_model` and `adr_0084_name_level_r17_one_count_function`, for the pattern of a guard and its mutated copy.

## Checks

```bash
cargo check
cargo fmt -- --check
cargo clippy --all-targets -- -D warnings
cargo test --lib adr_0086_window_group_page_
cargo test
cargo test --test architecture_tests
cargo build --bin v4vmm
```

Run `cargo build --bin v4vmm` last. `cargo test` can leave a binary with GPUI test support at `target/debug/v4vmm`.

## Rollback

Revert the commit. This packet adds no migration, no stored data and no configuration key. After the revert, packet 002 D8 and D9 apply again.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `docs/architecture/source-map.md`
- This packet: `docs/tasks/adr-0086-task-004-window-group-page.md`
- Packet 002: `docs/tasks/adr-0086-task-002-new-releases-window-groups.md`
- ADR 0086 Decisions 1, 3 and 4
- ADR 0084 Decisions 4, 6, 9, 10 and 11, and packets 018, 020, 026 and 032
- Each file in "Files To Inspect"

Goal:
- Make each change in "Required Changes".

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- Follow each decision in "Orchestrator Decisions". Do not change a decision.
- Use each name of packet 002 "Names For Later Packets" exactly. Delete only `discover_selection_rows` and `discover_release_rows`, as D11 and Required Change 8 state.
- Use each new name of this packet exactly.
- The view model gives each text, image and availability. The renderer only composes. Use named tokens. Column text does not call `.truncate()`.
- Tests use recorded values. No test sends a request to a live service.
- Background work runs through the ADR 0040 runtime. Do not call `cx.spawn` from a screen.
- Never run `git add`, `git rm`, `git commit`, `git checkout`, `git restore`, `git stash` or `git reset`.
- Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- The database schema and the migration registry.
- The Releases list and the artist pages.
- `src/application/queries/feed.rs` and `src/application/request_profiles.rs`.
- `src/config.rs` and `config.toml`.
- Any ADR, and each document other than this packet.

Acceptance criteria:
- Each case R86-4-01 to R86-4-27 has a passing test or guard.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.
- The packet has an "Implementation Result - <date>" section and an "Operator Visual Check" section for V1 to V10.

At the end, report: files changed, tests run, behavior changed, deviations, unresolved concerns, and the document changes that the orchestrator must make. These document changes include the source map row of the group page, packet 002 V5 and V7, which this packet replaces, and the pending human checks.

Stop and report the problem, and do not guess, when:
- Packet 002 is not complete, or one of its "Names For Later Packets" is missing or has a different signature.
- Packet 002 did not keep the `WindowEntry` values or the row ids, and `NewReleasesLayout` cannot get them with no second call to `NewReleasesLayout::build(`.
- A change needs a new architecture decision.
- A change needs a file in "Do not touch".
- `ListSelection` cannot hold a row of two or more members in `DetailSelection` with no change to its type.
- A change of `singles_main_action` changes a label or a test of the Decision 6 page.
- The Index row of a New releases member has no feed GUID, so it has no `ReleaseKey`.
