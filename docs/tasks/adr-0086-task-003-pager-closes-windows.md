# ADR 0086 Task 003: The Pager Loads Until Each Window Closes

Status: Ready - 2026-10-23.

Orchestrator note - 2026-10-23: packet 002 is implemented. These names are `#[cfg(test)]` until a packet calls them: `WindowLength::{MIN_DAYS, MAX_DAYS, from_days, days}`, `LibraryViewModel::{window_length, set_window_length}` and `NewReleasesLayout::any_window_open`. Remove the `#[cfg(test)]` of each name that this packet calls. The group renderers are in `src/ui/shells/library/window_groups.rs`.

## Goal

After a New releases page arrives, the pager loads one more page while a window of a loaded group is open. It loads at most 5 extra pages, 100 feeds, for one page request. The pager keeps its loading state during the extra loads. The new tiles show when the loads stop.

When the cap stops the loads, each open group shows "+" after its count, for example "13+ Singles". The count grows in place when the next page arrives. The "+" goes when the window closes. A single tile becomes a group tile in place when a later page adds a single to its window.

A page result applies only to the current request of the pager. It changes the content list only while the content list shows New releases.

This packet starts after packets 001 and 002 are complete.

## Authority

- [ADR 0086](../adr/0086-new-releases-singles-window.md) Decision 5: the closed window, the extra pages, the cap of 5 pages, the loading state, the "+" and the change in place.
- ADR 0086 Decision 1: the chain of the windows. Packet 002 builds it.
- ADR 0086 Decision 6: a saved search loads no extra page, and its groups show the "+". Packet 005 does this. This packet gives the names that packet 005 uses.
- [ADR 0062](../adr/0062-music-content-surface.md), amended by ADR 0086: the recency order and the paging. The pager can load more pages before it shows a page. The recency order stays.
- [ADR 0084](../adr/0084-music-column-browser.md) Decision 3: New releases is the first Discover view.
- ADR 0040: background work runs through the runtime, and `present_command` gives the result to the app.
- [Phase plan](../plans/adr-0086-new-releases-window-groups-phase-plan.md), "Risk Areas": stale pages, in-place updates and the request load.
- [Packet 002](adr-0086-task-002-new-releases-window-groups.md), "Names For Later Packets".
- [AGENTS.md](../../AGENTS.md): renderer portability, typed action state, "Current-view state must update in place", "Every fix gets a guard" and "Write reports for the operator".

## Recorded Facts - 2026-10-23

### Facts from the code of today

The orchestrator read each fact in the code on 2026-10-23. Packets 001 and 002 are not implemented, so these facts are before those packets.

The pager view model:

- `src/view_models/recent_feeds.rs:15-22` `RecentFeedsPageBatch` holds `rows`, `cursor` and `has_more`.
- `src/view_models/recent_feeds.rs:26-36` `RecentFeedsLoadIntent` holds only the `cursor`. `into_cursor` gives it.
- `src/view_models/recent_feeds.rs:40-47` `RecentFeedsPageState` is `Loading`, `Loaded(rows)` or `Error { message, detail }`.
- `src/view_models/recent_feeds.rs:51-56` `RecentFeedsPageVm` holds `state`, `cursor`, `has_more` and `loading`. It has no generation and no request id.
- `src/view_models/recent_feeds.rs:72-86` `begin_load(append)` gives `None` while a request is in flight, for a fresh load and for an append (line 73). It also gives `None` for an append when `has_more` is false.
- `src/view_models/recent_feeds.rs:89-104` `finish_load(batch, append)` sets `loading` to false, stores the cursor and `has_more`, then extends or replaces the rows.
- `src/view_models/recent_feeds.rs:107-122` `fail_load(message, detail, append)` keeps the loaded rows of a failed append and shows no error (lines 114-116). A failed fresh load gives `Error`.

The command and the request:

- `src/application/queries/feed.rs:25-46` `FetchRecentFeedsPage` holds the endpoint, the cursor and `resume_after`. Lines 51-61 give `execute`. It returns `CommandError::Cancelled` when the context is canceled (line 52).
- `src/application/queries/feed.rs:112-147` `fetch_recent_feed_result_rows` requests `PAGE_LIMIT` feeds (line 118). Each row id uses `start_index + index` (lines 123-124 and 137). Thus `resume_after` must count each row that the pager holds.
- `src/api.rs:12` `PAGE_LIMIT` is 20.

The app wiring:

- `src/library/app_impl.rs:473-511` `start_recent_music_load(append, cx)` is the one dispatch site. It reads `loaded_row_count` before `begin_load` (line 474). It passes `if append { loaded_row_count } else { 0 }` as `resume_after` (line 486).
- `src/library/app_impl.rs:494-499` the success callback calls `finish_load`, then `replace_recent_music_content`, with no check of the open view.
- `src/library/app_impl.rs:500-509` the error callback calls `fail_load` with "Recent music unavailable", then `replace_recent_music_content`, with no check of the open view.
- `src/library/app_impl.rs:1137` the app starts a fresh load at construction. `:1165` `install_runtime` starts a fresh load again.
- `src/library/app_impl.rs:2607-2617` `show_discover_view` projects the pager into the content list with `replace_recent_music_content`.
- `src/presentation/async_command_presenter.rs:18-57` `present_command` drops a result when the runtime session changed (`session.accepts(generation)`, line 40). It calls `cx.notify()` after each callback (line 51). It has no check of the request or of the view.

The content list:

- `src/view_models/library.rs:1732-1739` `ContentListPageSource` is `Tree`, `RecentMusic` or `BroadcastReadiness`.
- `src/view_models/library.rs:2363-2372` one `ContentListPageVm` serves New releases, Tracks and Needs attention.
- `src/view_models/library.rs:2429-2455` `replace_recent_feeds_page` always sets the source to `RecentMusic` and replaces `cached_rows`. `:2499-2510` `replace_rows` sets the source to `Tree` for Tracks.
- Thus a late page result replaces the Tracks rows with New releases rows when the curator opened Tracks during the load.
- `src/view_models/library.rs:3273-3280` `begin_recent_music_content_load` and `replace_recent_music_content` are the two `LibraryViewModel` entry points.
- `src/view_models/library.rs:2642-2653` `load_more_display` gives the "Load more" button when `has_more` is true. It is disabled while `loading` is true. `:1790-1799` `ContentListLoadMoreDisplay` has no failure text. `:1770-1775` `ContentListFailedStateDisplay` holds `title` and `detail`.
- `src/ui/shells/library/content_list_scroll.rs:39-49` the scroll probe calls `start_recent_music_load(true, cx)` near the end of the content. It reads `page.has_more()` and `page.is_loading()` (`src/ui/shells/library/content_list.rs:195-198` and `:252-255`). `src/view_models/pagination.rs:55-75` gives no load while `is_loading` is true.
- `src/ui/shells/library/content_list.rs:570-585` `render_content_list_load_more` renders the button.

The existing patterns and guards:

- `src/view_models/index_name_list.rs:233-255` `IndexNameListPageVm::begin_load` adds 1 to a generation for a fresh load. `:289-298` `finish_load(batch, generation)` gives `false` and changes nothing for an old generation. This is the stale check that this packet copies.
- `tests/architecture_tests.rs:15290-15352` `adr_0062_music_default_content_reuses_recent_feeds_pager` requires the strings `finish_load(&mut self, batch: RecentFeedsPageBatch, append: bool)`, `this.recent_music_page.finish_load(batch, append)`, `this.recent_music_page.fail_load(` and `if append { loaded_row_count } else { 0 }`. It forbids `cursor: Option<String>` in `src/view_models/library.rs`, `src/library.rs`, `src/library/app_impl.rs` and the content list shell.
- `src/view_models/library.rs:6806-6985` and `:10253-10340` the tests of the content list call `finish_load` and `fail_load`.

### Facts from the packet 002 design

These names do not exist in the code yet. Packet 002 adds them. This packet uses them exactly.

- `WindowGroup::closed_by_rows` is true when an entry has an `order_date` before `window_start`, or no `order_date`.
- `WindowGroupTileVm` holds `kind_badge` ("n Singles"), `count`, `a11y_label` and `window_closed`. `WindowGroupTileVm::new` builds them. Packet 002 shows no "+".
- `NewReleasesLayout::build(rows, tree, length)` builds the layout. `NewReleasesLayout::any_window_open` tells if a group has an open window.
- `LibraryViewModel::refresh_new_releases_layout` builds the cached layout at the end of `replace_recent_music_content` and of each other method that changes the rows, the tree or the length.
- The guard of R86-2-27 requires one `NewReleasesLayout::build(` call in `src/view_models/library.rs`, inside `fn refresh_new_releases_layout`.
- A Library row takes its kind, its identity and its date from the tree. The `in_library` flag of an entry comes from the Library badge. It changes the Library count only, not the window.

## Design

### One page request is one chain

A page request is one call of `start_recent_music_load`: the fresh load at start, one "Load more" click, or one scroll probe. The request holds one generation. Its first page and its extra pages are the chain of the request.

1. The app dispatches the first page with `FetchRecentFeedsPage`.
2. The page arrives. `RecentFeedsPageVm::receive_page(generation, batch)` adds the rows to `staged`. It stores the cursor and `has_more`. It gives `false` for an old generation, and the app stops.
3. `LibraryViewModel::recent_music_windows_open(&page)` builds a layout of the loaded rows and the staged rows. It gives `any_window_open()` of that layout.
4. `RecentFeedsPageVm::next_step(windows_open)` decides. It gives `LoadExtra(intent)` when a window is open, `has_more` is true and `extra_pages` is less than `EXTRA_PAGE_CAP`. Else it moves the staged rows into the loaded rows, sets `loading` to false and gives `Finished`.
5. For `LoadExtra`, the app dispatches the intent, and step 2 starts again. For `Finished`, the app calls `LibraryViewModel::apply_recent_music_page(&page)`.

The first page and the extra pages use the same generation. A chain thus sends at most 6 list requests: the first page and 5 extra pages.

### The loading state and the tiles

`RecentFeedsPageVm::state()` never holds the staged rows. Thus a projection during the chain shows only the rows of earlier requests, with `is_loading()` true. The content list keeps its loading display, and "Load more" stays disabled. The scroll probe reads `is_loading()`, so it sends no second request.

The staged rows show only at `Finished`. For a fresh load, the first tiles show when the chain stops. For an append, the earlier tiles stay, and the new tiles show when the chain stops.

### The stale check

Two checks stop a stale result.

- **The request.** `RecentFeedsPageVm` holds a `generation`. A fresh load adds 1 to it, also while a chain is in flight. `receive_page` and `fail_page` give `false` and change nothing for an old generation. The chain of the old request thus stops at its next page.
- **The view.** `apply_recent_music_page` changes the content list only when its source is `ContentListPageSource::RecentMusic`. Else it gives `false` and changes nothing. A result that arrives while Tracks or Needs attention shows thus never replaces their rows.

The pager keeps the rows of its current request when the curator moves to a different view. When the curator opens New releases again, `show_discover_view` projects the pager with `replace_recent_music_content`. A chain in flight then shows its loading state, and its tiles show when it stops.

### A failed page

`RecentFeedsPageVm::fail_page(generation, message, detail)` follows these rules.

- A failed first page of a fresh load gives `Error`, as today. The content list shows "Recent music unavailable".
- Else the pager moves the staged rows into the loaded rows and sets `loading` to false. The rows of the chain that arrived thus show. The cursor and `has_more` keep the values of the last page that arrived, so "Load more" requests the failed page again.
- The pager stores the detail in `older_pages_failure`. `begin_load` clears it.

`ContentListLoadMoreDisplay::failure` gives a `ContentListFailedStateDisplay` with the title "Older releases did not load from MusicIndex" and the detail of the error. The shell shows it above the enabled "Load more" button. A group whose window stays open shows the "+", as after the cap.

### The end of the list

When `has_more` is false, no older row exists. Thus each window is closed. `refresh_new_releases_layout` calls `NewReleasesLayout::mark_list_end` when the content list has no more pages. That method sets `window_closed` to true for each group tile. `next_step` gives `Finished` when `has_more` is false.

### The "+" text

`window_group_count_badge(count, window_closed)` gives "13 Singles" for a closed window and "13+ Singles" for an open window. `OPEN_COUNT_SUFFIX` is "+". `WindowGroupTileVm::new` sets `kind_badge` from this function with `WindowGroup::closed_by_rows`.

The accessibility label of an open group says "13 or more singles", for example "jSmovie: 13 or more singles, Oct 8 to Oct 20, New, 1 in library". A closed group keeps the label of packet 002.

`WindowGroupTileVm::set_window_closed(closed)` sets `window_closed`, `kind_badge` and `a11y_label` together. `mark_list_end` calls it. The renderer shows `kind_badge` and never adds the "+" itself.

### Only groups ask for extra pages

A single tile is not a group (ADR 0086 Decision 1). `any_window_open` reads the group tiles only. Thus the window of a single tile asks for no extra page, and a single tile shows no "+". When a later page adds a single to its window, the layout gives a group tile at the position of that single.

### The change in place

The chain ends with `apply_recent_music_page`, which calls `replace_recent_music_content`. That method calls `refresh_new_releases_layout` (packet 002). `present_command` calls `cx.notify()`. The mounted view thus shows the new count, the new group tile or the removed "+" with no navigation. The element id of a group tile is the id of its anchor, so the tile keeps its place.

### The one build site of the layout

`recent_music_windows_open` needs a layout of rows that the content list does not show yet. It reads the pager rows, not `cached_rows`. Thus it gives the same answer while Tracks shows. The Library badge is not set on these rows, because the window does not use it.

The private helper `LibraryViewModel::layout_of_rows(rows)` is the one call of `NewReleasesLayout::build(`. `refresh_new_releases_layout` and `recent_music_windows_open` call it. This packet changes the guard of R86-2-27 to match.

### Names For Later Packets

Packets 004, 005 and 006 use these exact names. Do not rename them.

- `src/view_models/recent_feeds.rs`:
  - `pub(crate) const EXTRA_PAGE_CAP: usize = 5`.
  - `RecentFeedsLoadIntent` fields `generation: u64`, `cursor: Option<String>` and `resume_after: usize`, with `fn generation(&self) -> u64`, `fn resume_after(&self) -> usize` and `fn into_cursor(self) -> Option<String>`.
  - `RecentFeedsPageVm` fields `generation: u64`, `append: bool`, `staged: Vec<RecentFeedResultRow>`, `extra_pages: usize` and `older_pages_failure: Option<String>`.
  - `pub(crate) enum RecentFeedsPageStep { LoadExtra(RecentFeedsLoadIntent), Finished }`.
  - `pub(crate) fn receive_page(&mut self, generation: u64, batch: RecentFeedsPageBatch) -> bool`.
  - `pub(crate) fn next_step(&mut self, windows_open: bool) -> RecentFeedsPageStep`.
  - `pub(crate) fn fail_page(&mut self, generation: u64, message: impl Into<String>, detail: impl Into<String>) -> bool`.
  - `pub(crate) fn chain_rows(&self) -> impl Iterator<Item = &RecentFeedResultRow>`: the loaded rows, then the staged rows.
  - `pub(crate) const fn extra_pages(&self) -> usize` and `pub(crate) fn older_pages_failure(&self) -> Option<&str>`.
- `src/view_models/window_groups.rs`:
  - `pub(crate) const OPEN_COUNT_SUFFIX: &str = "+"`.
  - `pub(crate) fn window_group_count_badge(count: usize, window_closed: bool) -> String`.
  - `WindowGroupTileVm::set_window_closed(&mut self, closed: bool)`.
  - `NewReleasesLayout::mark_list_end(&mut self)`.
- `src/view_models/library.rs`:
  - `pub(crate) fn recent_music_windows_open(&self, page: &RecentFeedsPageVm) -> bool`.
  - `pub(crate) fn apply_recent_music_page(&mut self, page: &RecentFeedsPageVm) -> bool`.
  - `fn layout_of_rows(&self, rows: &[&ContentListRowDisplay]) -> NewReleasesLayout` (private).
  - `ContentListLoadMoreDisplay::failure: Option<ContentListFailedStateDisplay>`.
  - `ContentListPageVm::OLDER_PAGES_FAILED_TITLE` with the text "Older releases did not load from MusicIndex".
- `src/library/app_impl.rs`: `fn dispatch_recent_music_page(&mut self, intent: RecentFeedsLoadIntent, cx: &mut Context<Self>)` (private).

Each later packet uses these names as follows:

- Packet 005 builds each saved search tile with `WindowGroupTileVm::new`, then calls `set_window_closed(false)` on each group that a later page can change. It sends no extra page and does not call `next_step`.
- Packet 004 reads `window_closed` to show the "+" in the count of the window group page title.
- Packet 006 changes the length through `set_window_length`. The layout is built again with no extra page. The next page request uses the new length in `recent_music_windows_open`.

## Orchestrator Decisions

- **D1. The app dispatches each page, and the view model decides.** The decision needs the tree, the window length and the window chain. These are view-model state on the UI thread. A loop inside one command would need a copy of the tree and a second place of the chain. Each page also passes the session check of `present_command`.
- **D2. One page request is one call of `start_recent_music_load`.** The fresh load, one "Load more" click and one scroll probe each start a request. The cap counts the extra pages after its first page. The next request gets a new count of 5.
- **D3. The staged rows show only when the chain stops.** ADR 0086 Decision 5 says that the tiles show when the loads stop. `state()` thus holds no staged row. The loading state and the disabled "Load more" stay until `Finished`.
- **D4. A fresh load replaces a chain in flight.** `begin_load(false)` adds 1 to the generation also while `loading` is true. An append still gives `None` while a request is in flight. This copies `IndexNameListPageVm`.
- **D5. The view check is the source of the content list.** `apply_recent_music_page` changes the content list only for `ContentListPageSource::RecentMusic`. The pager keeps the result, and `show_discover_view` shows it later.
- **D6. A failed extra page shows the rows that arrived.** The failed page does not remove the first page. The open groups show the "+". The notice names MusicIndex and the result, and "Load more" requests the failed page again. The app makes no automatic retry.
- **D7. A failed first page of a fresh load keeps today's error.** The content list shows "Recent music unavailable" with the detail, as today.
- **D8. The end of the list closes each window.** `has_more` false proves that no older single exists. `mark_list_end` closes each group, so no "+" shows.
- **D9. Only a group asks for an extra page.** ADR 0086 Decision 5 names "a window of a loaded group". A single tile is not a group.
- **D10. The "+" is in `kind_badge`.** The view model gives the full text. The renderer shows it as the release kind badge of packet 002. The accessibility label says "or more", because a screen reader can read "+" as "plus".
- **D11. `resume_after` comes from the intent.** The pager counts its loaded rows and its staged rows. The row ids thus stay continuous across the extra pages.
- **D12. The candidate layout reads the pager rows.** `recent_music_windows_open` gives the same answer in each view. The layout has one build site, `layout_of_rows`.
- **D13. The fresh load of `install_runtime` stays.** With D4, it now replaces a request whose session ended. The packet makes no other change to `begin_recent_music_content_load`.

## Required Changes

1. `src/view_models/recent_feeds.rs`:
   - Add `EXTRA_PAGE_CAP`, `RecentFeedsPageStep` and each new field and method of "Names For Later Packets".
   - Change `begin_load` for D4. It sets `append`, clears `staged`, sets `extra_pages` to 0 and clears `older_pages_failure`. The intent gives the cursor and `resume_after`.
   - Delete `finish_load` and `fail_load`. Move their tests to `receive_page`, `next_step` and `fail_page`.
   - Change the `//!` comment to name ADR 0086 Decision 5.
2. `src/view_models/window_groups.rs`: add `OPEN_COUNT_SUFFIX`, `window_group_count_badge`, `WindowGroupTileVm::set_window_closed` and `NewReleasesLayout::mark_list_end`. `WindowGroupTileVm::new` sets `kind_badge` and `a11y_label` from `closed_by_rows`.
3. `src/view_models/library.rs`:
   - Add `layout_of_rows`. Move the one `NewReleasesLayout::build(` call into it.
   - `refresh_new_releases_layout` calls `layout_of_rows`, then `mark_list_end` when the content list has no more pages.
   - Add `recent_music_windows_open` and `apply_recent_music_page`.
   - Add `ContentListLoadMoreDisplay::failure` and `ContentListPageVm::OLDER_PAGES_FAILED_TITLE`. `replace_recent_feeds_page` copies `older_pages_failure` from the pager. `load_more_display` gives it.
   - Change the tests at lines 6806-6985 and 10253-10340 to the new pager calls.
4. `src/library/app_impl.rs`:
   - `start_recent_music_load` keeps `begin_load`, `begin_recent_music_content_load` and `replace_recent_music_content`. Then it calls `dispatch_recent_music_page`.
   - Add `dispatch_recent_music_page`. Its success callback calls `receive_page`, `recent_music_windows_open` and `next_step`. For `LoadExtra`, it calls `dispatch_recent_music_page` again. For `Finished`, it calls `apply_recent_music_page`.
   - Its error callback calls `fail_page`. When that gives `true`, it calls `apply_recent_music_page`.
   - Delete `loaded_row_count`. `FetchRecentFeedsPage::new` takes `intent.resume_after()`.
5. `src/ui/shells/library/content_list.rs`: `render_content_list_load_more` shows `failure` above the button. It uses the title and detail style of the failed state of the content list. It adds no color, size or text of its own.
6. `tests/architecture_tests.rs`:
   - Add the guard of R86-3-25.
   - In `adr_0062_music_default_content_reuses_recent_feeds_pager`, replace the required strings of `finish_load`, `fail_load` and `loaded_row_count` with `receive_page`, `next_step`, `fail_page` and `intent.resume_after()`. Keep the forbidden list.
   - In the guard of R86-2-27, require the one `NewReleasesLayout::build(` call inside `fn layout_of_rows`. Require that only `refresh_new_releases_layout` and `recent_music_windows_open` call `layout_of_rows(`.

## Mechanical Acceptance Criteria

Use the prefix `adr_0086_pager_windows_` for each test. Each test is a unit test beside its owner code, apart from R86-3-25 to R86-3-27. The dates are UTC midnights of 2026. A "page" is a `RecentFeedsPageBatch` of recorded rows. No test sends a request.

| Case | Required proof |
|---|---|
| R86-3-01 | After `receive_page`, `next_step(true)` with `has_more` true gives `LoadExtra`. The intent has the same generation and the cursor of the page. |
| R86-3-02 | The intent of R86-3-01 gives `resume_after` equal to the count of the loaded rows and the staged rows. |
| R86-3-03 | With each window open, five `next_step(true)` calls give `LoadExtra`. The sixth call gives `Finished`, and `extra_pages()` is `EXTRA_PAGE_CAP`. |
| R86-3-04 | After `Finished` of R86-3-03, `begin_load(true)` gives an intent, and five more `LoadExtra` steps are available. |
| R86-3-05 | `next_step(false)` after the first page gives `Finished`, with no extra page. |
| R86-3-06 | `next_step(true)` with `has_more` false gives `Finished`. |
| R86-3-07 | During the chain, `is_loading()` is true and `state()` holds no staged row. `chain_rows()` gives the loaded rows, then the staged rows. |
| R86-3-08 | `replace_recent_music_content` during the chain gives the earlier rows only. `loading_display()` is `Some`, and `load_more_display()` is disabled. |
| R86-3-09 | After `Finished`, `state()` holds each row of the chain in the list order, and `is_loading()` is false. |
| R86-3-10 | `recent_music_windows_open` gives true for a group of two singles whose window no row closes. It gives false when a later row closes the window. |
| R86-3-11 | `recent_music_windows_open` gives false for one single whose window no row closes. |
| R86-3-12 | `recent_music_windows_open` reads the pager rows. It gives the same answer while the content list shows Tracks. |
| R86-3-13 | `window_group_count_badge(13, false)` gives "13+ Singles". `window_group_count_badge(13, true)` gives "13 Singles". `OPEN_COUNT_SUFFIX` is "+". |
| R86-3-14 | An open group tile gives `a11y_label` "jSmovie: 13 or more singles, Oct 8 to Oct 20, New, 1 in library". A closed tile keeps the label of R86-2-20. |
| R86-3-15 | A chain that the cap stops with an open group of 13 singles gives `kind_badge` "13+ Singles" and `window_closed` false in `new_releases_layout()`. |
| R86-3-16 | After R86-3-15, a next request adds 2 singles of the window and a row before `window_start`. The tile with the same `element_id` gives `count` 15 and "15 Singles". |
| R86-3-17 | After R86-3-15, a next request that the cap stops again adds 2 singles of the window. The tile gives "15+ Singles". |
| R86-3-18 | A first request shows one single as `Release`. A next request adds a single of its window. The layout gives a group at that position, with `open_row_id` of the newest single. |
| R86-3-19 | `apply_recent_music_page` with `has_more` false gives each group tile `window_closed` true, no "+", and `any_window_open()` false. |
| R86-3-20 | `set_window_closed(false)` on a closed tile gives the "+" in `kind_badge` and "or more" in `a11y_label`. `set_window_closed(true)` removes them. |
| R86-3-21 | A fresh `begin_load(false)` during a chain gives a new generation. `receive_page` and `fail_page` with the old generation give false and change no row. |
| R86-3-22 | `apply_recent_music_page` while the source is `Tree` or `BroadcastReadiness` gives false. The cached rows and the source do not change. A later `replace_recent_music_content` gives the finished rows. |
| R86-3-23 | `fail_page` after a first page of the chain gives `Loaded` with that page. `is_loading()` is false, `has_more()` is true, and the next append uses its cursor. |
| R86-3-24 | After R86-3-23, `load_more_display()` gives `failure` with the title "Older releases did not load from MusicIndex" and the detail. The button is enabled. The next `begin_load` clears it. |
| R86-3-25 | A situational guard `adr_0086_pager_extra_loads_in_the_view_model` passes. It fails against a copy of `src/library/app_impl.rs` that compares `extra_pages()` or calls `replace_recent_music_content(&this.`. |
| R86-3-26 | The changed guard `adr_0062_music_default_content_reuses_recent_feeds_pager` passes, with the new required strings and the same forbidden list. |
| R86-3-27 | The changed guard of R86-2-27 passes. It fails against a copy of `src/view_models/library.rs` that calls `NewReleasesLayout::build(` outside `fn layout_of_rows`. |
| R86-3-28 | `fail_page` after a failed first page of a fresh load gives `Error` with "Recent music unavailable". `load_more_display()` gives no failure. |
| R86-3-29 | Each test of packet 001, packet 002 and ADR 0062 still passes. A packet 002 test that expects "n Singles" uses a group that a row closes. |

The guard of R86-3-25 enforces these rules:

- `EXTRA_PAGE_CAP` is defined only in `src/view_models/recent_feeds.rs`. No file under `src/library/`, `src/app/` or `src/ui/` names it or calls `extra_pages(`.
- `src/library/app_impl.rs` calls `receive_page(`, `next_step(` and `fail_page(` inside `fn dispatch_recent_music_page`.
- Inside `fn dispatch_recent_music_page`, the result reaches the content list only through `apply_recent_music_page(`. That function contains no `replace_recent_music_content(`.

The failure message of the guard is this text:

```text
ADR 0086 §5 The Pager Loads Until Each Window Closes, Up To A Cap: `RecentFeedsPageVm::next_step` decides each extra page, and a page result reaches the content list only through `LibraryViewModel::apply_recent_music_page`.
Call `receive_page`, `next_step` and `apply_recent_music_page` in `dispatch_recent_music_page`. Do not count pages in the app, and do not call `replace_recent_music_content` in a result callback.
```

## Visual Acceptance Criteria

These are for the operator. No test proves them. Use the release build for the timing checks, because the debug build is slower.

- V1: open Music, Discover, New releases, in the Tiles mode, at normal width. The loading state shows until the tiles show. No tile shows and then changes into a group tile during the first load. Examine Light and Dark.
- V2: jSmovie shows one group tile for each window. Its count does not change while the tiles show, unless it shows "+".
- V3: find a group with "+" after its count, for example "13+ Singles". The "+" is part of the kind badge, with the same color and size. When the data has no such group, record "not seen" and keep V3 open.
- V4: scroll to the end of the tiles. The next page loads. The count of a "+" group grows in place, and the "+" goes when its window closes. The scroll position does not jump.
- V5: scroll until a later page adds a single to the window of a single tile. The tile changes into a group tile in place, with no navigation.
- V6: change to the List mode. Do V1, V4 and V5 again. The rows show the same facts. Examine Light and Dark.
- V7: make the window narrow. In the Tiles mode and the List mode, the "13+ Singles" badge does not clip or overlap other text.
- V8: in the release build, measure the time from the start of the app to the first tiles of New releases. Record the time. Wrong: the tiles show in steps, or the loading state ends before the tiles show.
- V9: start the app, and open Tracks before the New releases tiles show. The Tracks list keeps the Library tracks after the load ends. Open Discover, New releases again. The tiles show.
- V10: after the first tiles show, disconnect the network, then scroll to the end. The text "Older releases did not load from MusicIndex" shows above "Load more". Connect the network again, and click "Load more". The next page loads.

## Exclusions

- No change to the page size, the recency order, the list request or the detail reads of packet 001.
- No extra page for a saved search, and no saved search groups (packet 005).
- No window group page and no group checkbox (packet 004).
- No Settings value of the window length (packet 006).
- No automatic retry of a failed page.
- No progress count in the loading state.
- No change to `begin_recent_music_content_load` or to the content list at the start of a request.
- No change to the Releases list, the artist pages or `src/view_models/singles_group.rs`.
- No schema change, no database query and no configuration key.

## Files To Inspect

- [AGENTS.md](../../AGENTS.md), the rust-dev skill and [the source map](../architecture/source-map.md).
- [ADR 0086](../adr/0086-new-releases-singles-window.md) Decisions 1, 5 and 6, and [ADR 0062](../adr/0062-music-content-surface.md).
- [Packet 002](adr-0086-task-002-new-releases-window-groups.md) and its implementation result.
- `src/view_models/recent_feeds.rs`: the pager and its tests.
- `src/view_models/index_name_list.rs`: `begin_load`, `finish_load` and the generation pattern.
- `src/view_models/window_groups.rs`: `WindowGroup`, `WindowGroupTileVm` and `NewReleasesLayout`.
- `src/view_models/library.rs`: `ContentListPageVm`, `ContentListLoadMoreDisplay`, `replace_recent_feeds_page`, `refresh_new_releases_layout` and the recent music tests.
- `src/library/app_impl.rs`: `start_recent_music_load` and `show_discover_view`.
- `src/presentation/async_command_presenter.rs`: `present_command`.
- `src/application/queries/feed.rs`: `FetchRecentFeedsPage` and `fetch_recent_feed_result_rows`.
- `src/ui/shells/library/content_list.rs` and `src/ui/shells/library/content_list_scroll.rs`.
- `tests/architecture_tests.rs`: `adr_0062_music_default_content_reuses_recent_feeds_pager` and `adr_0086_window_groups_built_once_in_the_view_model`.

## Checks

```bash
cargo check
cargo fmt -- --check
cargo clippy --all-targets -- -D warnings
cargo test --lib adr_0086_pager_windows_
cargo test
cargo test --test architecture_tests
cargo build --bin v4vmm
```

Run `cargo build --bin v4vmm` last. `cargo test` can leave a binary with GPUI test support at `target/debug/v4vmm`.

## Rollback

Revert the commit. This packet adds no migration, no stored data and no configuration key. Packet 002 then shows the groups of the loaded rows with no "+".

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `docs/architecture/source-map.md`
- This packet: `docs/tasks/adr-0086-task-003-pager-closes-windows.md`
- ADR 0086 Decisions 1, 5 and 6, and ADR 0062
- Packets 001 and 002, with their implementation results
- Each file in "Files To Inspect"

Goal:
- Make each change in "Required Changes".

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- Follow each decision in "Orchestrator Decisions". Do not change a decision.
- Use each name in "Names For Later Packets" exactly. Use each name of packet 002 exactly.
- The view model gives each text and each availability. The renderer only composes. Use named tokens.
- Tests use recorded values. No test sends a request to a live service.
- Background work runs through the ADR 0040 runtime and `present_command`. Do not call `cx.spawn` from a screen.
- Never run `git add`, `git rm`, `git commit`, `git checkout`, `git restore`, `git stash` or `git reset`.
- Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- The database schema and the migration registry.
- `src/application/queries/feed.rs` and `src/application/request_profiles.rs`.
- `src/view_models/singles_group.rs`, the Releases list and the artist pages.
- `src/view_models/search_results/saved_search.rs`.
- `src/config.rs` and `config.toml`.
- Any ADR, and each document other than this packet.

Acceptance criteria:
- Each case R86-3-01 to R86-3-29 has a passing test or guard.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.
- The packet has an "Implementation Result - <date>" section and an "Operator Visual Check" section for V1 to V10.

At the end, report: files changed, tests run, behavior changed, deviations, unresolved concerns, and the document changes that the orchestrator must make.

Stop and report the problem, and do not guess, when:
- Packet 001 or 002 is not complete.
- A name of the packet 002 "Names For Later Packets" is absent, or has a different signature.
- `any_window_open` reads a single tile, or the window of an entry depends on its Library badge.
- `present_command` cannot dispatch the next page from inside its own success callback. Do not add a `cx.spawn` or a loop command.
- A change needs a cursor field in `src/view_models/library.rs` or `src/library/app_impl.rs`.
- A change needs a new architecture decision.
- A change needs a file in "Do not touch".
