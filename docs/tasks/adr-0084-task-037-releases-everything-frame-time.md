# ADR 0084 Task 037: The Frame Time Of The Releases List In "Everything"

Status: Ready - 2026-10-24. This packet starts after packet 036 is committed. The operator answered Q1 on 2026-10-24 (option B).

## Goal

One frame of the Releases list does work only for the rows in view. The merge of the list builds again only when one of its inputs changes.

The operator measured about 500 ms for each frame in the Releases list with the scope "Everything", after several "Load more" clicks (overhaul plan, responsiveness item 31). Item 16 records the second cause: one frame builds the merge more than one time.

## Authority

- [ADR 0084](../adr/0084-music-column-browser.md):
  - Decision 1: each Music column that scrolls shows a scrollbar with the shared gutter.
  - Decision 6: the singles groups. Decision 8: the two scopes.
  - Decision 10: the selection model, the checkboxes, the keyboard focus and the action bar.
- [Packet 016](adr-0084-task-016-library-and-everything-scopes.md), Design "The sort key and the merge": "The merge is a pure function. It runs at each render." This packet keeps the pure function and changes when it runs. Decision D7: the paging, "Load more" and the watermark. Decision D8: the status rows.
- [Packet 036](adr-0084-task-036-group-row-at-artist-name.md): the order of a group row, and the operator answer to its question Q1.
- [ADR 0063](../adr/0063-show-dashboard-layout.md): the shared scrollbar gutter.
- [ADR 0040](../adr/README.md): background work runs on the async command runner. No screen calls `cx.spawn`.
- The [overhaul plan](../plans/design-and-cleanup-overhaul-plan.md), "Throughout: Responsiveness And Background Feedback", items 16 and 31.
- [AGENTS.md](../../AGENTS.md): renderer portability, "Current-view state must update in place", "Every fix gets a guard" and "Delete dead code".

## Recorded Facts - 2026-10-24

The code facts come from the working tree of 2026-10-24, after commit `e476192`. Packet 036 changes `src/view_models/releases_everything.rs`, `src/view_models/library.rs` and `src/view_models/singles_group.rs`. Find each call site again before you change it.

### Each build of the merge in one frame

| Build | Path | What it uses |
|---|---|---|
| 1 | `TopApp::render` (`src/app.rs:1781`) calls `header_action_for_list_column` (`src/library/app_impl.rs:4308-4312`). That calls `releases_visible_rows` (`:3907-3909`), then `releases_entries` (`src/library/music_scopes.rs:60-67`), then `LibraryViewModel::releases_list` (`src/view_models/library.rs:3611`) | Only `is_empty()` of the result |
| 2 | `render_list_column`, the Releases arm (`src/library/app_impl.rs:4917-4991`), calls `releases_list_with_selection` at line 4921 (`src/view_models/library.rs:3715-3773`). That calls `releases_list` at line 3726 | Each entry |

Each build of `releases_list` does this work:

- `singles_groups_everything` (`src/view_models/library.rs:3620-3621`, `src/view_models/singles_group.rs:525-529`) calls `everything_groups` (`src/view_models/singles_group.rs:535`). It makes the set of Library keys, groups the Library singles and reads each loaded singles row of each feed in `IndexFeedSingles`.
- It makes the set `hidden_keys` (`src/view_models/library.rs:3623-3633`).
- For each Library release row, `groups.iter().any(|group| group.contains(key))` (`src/view_models/library.rs:3644`) reads each group. `SinglesGroup::contains` (`src/view_models/singles_group.rs:314-318`) reads each member.
- It makes one `SinglesGroupRowVm` for each group (`src/view_models/library.rs:3657-3662`).
- `everything_entries` (`src/view_models/releases_everything.rs:215-255`) makes one row for each loaded page row. `library_entries` (`:194-197`) then sorts each entry.

Other callers of `releases_entries` run on an event, not on each frame:

- `releases_visible_members` after a Library reload (`src/library/app_impl.rs:1514`)
- `disclose_focused_group` (`:4159`)
- the selection keys and the focus moves (`:3968`, `:4016`, `:4116`, `:4186` and `:4211`)

### Other work for each row in one frame

- `releases_list_with_selection` calls `destination_release_row_menu` for each release row (`src/view_models/library.rs:3882-3896`). That calls `release_row_menu`, then `release_row_state` (`:3904-3913`), then `album_for_release_key` (`:3776-3783`).
  - `album_for_release_key` reads the albums of the tree one by one. For each album, `AlbumNode::release_key` (`:5920-5925`) makes a copy of the feed GUID.
  - A `MusicIndex` row is not in the tree, so the read examines each album. The work is the row count multiplied by the album count, in each frame.
- `releases_selection_action_bar` (`src/view_models/library.rs:3855-3877`) calls `release_row_state` for each selected key, in each frame. After Ctrl+A, this is the same read for each shown row.
- The Releases arm calls `thumbnail_for_url` for each row and for each single of an open group (`src/library/app_impl.rs:4929-4958`). It also calls `collection_art_image` for each group.
  - `thumbnail_for_url` (`src/library/app_impl.rs:2243-2283`) starts one `FetchThumbnail` command for each row with no loaded image, also for a row out of view.
  - Each answer calls `cx.notify()` (`src/library/app_impl.rs:2278-2279`). Thus each image answer gives one more full frame.
- `render_library_releases_list` (`src/ui/shells/library/destination_lists.rs:132-164`) makes one element for each entry and for each single of an open group.
- `render_destination_list_container_tracked` (`src/ui/shells/library/destination_list_scroll.rs:73-100`) puts each row in one container. `music_column_tracked_scrollbar` (`src/ui/composites/music_column_scroll.rs:47-65`) gives it `overflow_y_scroll` and `track_scroll`. Thus each row does layout and paint in each frame, also a row out of view.
- Item 31 names `render_destination_list`. No function of that name exists. The functions above do that work.

### What starts a frame of the list column

- `LibraryColumnView` observes `LibraryApp` (`src/library/column_view.rs:42`). Each `cx.notify()` of `LibraryApp` thus renders each Music column again.
- The app has no cached view: `rg "\.cached\(" src` gives no result.
- A scroll wheel event on a scroll container notifies the view that holds it (gpui-pre 0.3.1 `src/elements/div.rs:3271` and `:3315`). A hover change of an element with a hover style also notifies that view (`src/elements/div.rs:2794-2805`).
- `set_hovered_thumb` (`src/library/app_impl.rs:636-640`) notifies `LibraryApp` when the hovered image changes.
- Thus each scroll step, each hover change and each image answer renders the list column again. Each such frame does build 2 and the work above. When `TopApp` renders in the same frame, it also does build 1.

### Virtual lists in this app and in its libraries

- No file in `src` uses `uniform_list`, `list(`, `ListState` or a virtual list (search of 2026-10-24). Each Music column scrolls through `music_column_scrollbar` or `music_column_tracked_scrollbar`.
- gpui-pre 0.3.1 gives two virtual lists:
  - `uniform_list` (`src/elements/uniform_list.rs:22`) needs one height for each item.
  - `list(state, render_item)` (`src/elements/list.rs:24`) with `ListState` accepts items of different heights. It renders only the items in view and in the overdraw area.
- `ListState::splice(old_range, count)` (`src/elements/list.rs:503-550`) keeps the logical scroll top on its item when the change is above that item. `ListState::reset` (`:355-369`) clears the scroll position.
- `ListState::scroll_to_reveal_item` (`src/elements/list.rs:677`) shows an item in the view. `ListState::remeasure_items` (`:412`) keeps the scroll position.
- `Context::processor` (gpui-pre 0.3.1 `src/app/context.rs:264-272`) gives a `'static` item closure that updates the entity.
- gpui-base (gpui-kit `5463fe4`, `crates/base/src/scrollbar.rs:120`) implements `ScrollbarHandle` for `ListState`. Thus `vertical_scrollbar` accepts a `ListState`.
- gpui-component 0.6.1 exports `v_virtual_list` from gpui-base. It needs the size of each item before the first layout.

### Guards and fields that name this code

- `adr_0084_scrollbars_r14_01_music_columns_use_the_shared_helper` (`tests/architecture_tests.rs:21801-21844`) reads `destination_lists.rs` and `destination_list_scroll.rs` together. It needs `music_column_scrollbar(` or `music_column_tracked_scrollbar(`, and it forbids `.overflow_y_scroll()`.
- `adr_0084_release_row_actions_r31_07_rows_with_no_menu` (`tests/architecture_tests.rs:22452-22486`) reads the text between `fn render_singles_group_rows(` and `for row in singles {`. It also reads the text between `pub(crate) fn render_library_destination_rows(` and `pub(crate) fn render_library_releases_list(`.
- Other guards name `src/ui/shells/library/destination_lists.rs` at `tests/architecture_tests.rs:3124`, `:6182`, `:16200`, `:22311`, `:22371`, `:23090` and `:23118`. Read each one before you move code.
- `LibraryApp::releases_list_scroll: ScrollHandle` (`src/library.rs:300`, made at `src/library/app_impl.rs:1165`) has one other use: `scroll_to_item(index + leading)` at `src/library/app_impl.rs:4125`. `leading` is 1 when a status row is at the list start.
- No navigation history entry records the scroll position of the list column. ADR 0084 Decision 9 applies to the pages of the detail column.

## Design

### One cache of the merge

1. Add `ReleasesListKey` to `src/view_models/releases_everything.rs`. Its fields:
   - the scope (`MusicListScope`)
   - the revision of the release name page
   - the revision of `IndexFeedSingles`
   - the revision of the Library view model, which covers the tree and the busy state
   - the set of open groups (`BTreeSet<SinglesGroupKey>`)
2. Add the revisions. Each one is a `u64` that each change adds 1 to:
   - `IndexNameListPage::revision()`: each `&mut self` method adds 1 (`begin_load`, `begin_pending_retry`, `begin_retry`, `finish_load`, `fail_load` and `fail_pending`, `src/view_models/index_name_list.rs:233-330`).
   - `IndexFeedSingles::revision()`: each `&mut self` method adds 1 (`begin_request`, `record`, `begin_feed_request` and `finish_feed_request`, `src/view_models/index_feed_singles.rs:116-250`).
   - `LibraryViewModel::releases_revision()`: `replace_tree` (`src/view_models/library.rs:3078`), each `update_album_*` method that changes the tree (`:3320-3415`), `begin_feed_removal` and `finish_feed_removal` (`:4811-4820`). `set_download_activity` (`:4806`) adds 1 only when the new activity is not equal to the old one.
   - The implementer lists each other `&mut` path to `snapshot.tree` and `busy`, and adds 1 there too.
3. Add `ReleasesListRows` and `ReleasesListCache` to `src/view_models/releases_everything.rs`.
   - `ReleasesListRows` holds the flat items (step 6), the selection rows of `releases_selection_rows`, and the `ReleaseRowState` of each row key.
   - `ReleasesListCache` holds the last key, an `Rc<ReleasesListRows>` and a build count for the tests.
4. Add the field `releases_list_cache: RefCell<ReleasesListCache>` to `LibraryApp` (`src/library.rs`). `RefCell` is necessary because `TopApp::render` reads `header_action_for_list_column` through `&self` (`src/app.rs:1781`).
5. Add `LibraryApp::releases_rows(&self) -> Rc<ReleasesListRows>`. It makes the key, compares it with the last key, and builds only when they are not equal. Each reader of the list uses it:
   - `header_action_for_list_column`, `releases_visible_rows`, `releases_visible_members` and `disclose_focused_group`
   - the Releases arm of `render_list_column`
   - `releases_selection_action_bar` and the row menu, which read the row state from the cache
   Delete `releases_entries` when no caller stays.

### One virtual list

6. Add `ReleasesListItemVm` to `src/view_models/library.rs`. It has five variants:
   - a status row at the list start
   - a release row
   - a group row with no singles
   - a single of an open group
   - a status row at the list end

   The item order is the order of `releases_selection_rows`, with each status row at its end of the list.
7. Replace `releases_list_scroll: ScrollHandle` with `releases_list_state: ListState`. Use `ListAlignment::Top` and an overdraw from a named token.
8. After each build, compare the old item ids with the new item ids, in order. The id is the `element_id` of the row, and each status row has a fixed id. Call `ListState::splice` for each run of added or removed ids. An item with the same id keeps its measured size. Do not call `ListState::reset`, because it clears the scroll position.
9. Render the list with `list(self.releases_list_state.clone(), cx.processor(...))` and one item render function in `src/ui/shells/library/destination_lists.rs`. The item function does these steps for its item only:
   - it applies the selection facts and the row menu (step 10)
   - it reads the image of the row with `thumbnail_for_url`
   - it renders the row with the present row functions
10. Move the body of `releases_list_with_selection` to `LibraryViewModel::releases_item_with_selection(item, selection, mode)`, which applies the facts to one item. Delete `releases_list_with_selection`. The selection, the selection mode, the keyboard focus, the hover and the images are not in the cache key.
11. Add `music_column_list_scrollbar(list, &ListState)` to `src/ui/composites/music_column_scroll.rs`. It gives the same scrollbar and the same gutter as `music_column_tracked_scrollbar`. Add its name to the r14_01 guard.
12. Replace `self.releases_list_scroll.scroll_to_item(index + leading)` (`src/library/app_impl.rs:4125`) with `self.releases_list_state.scroll_to_reveal_item(index + leading)`.
13. With no item, the list shows the present text "No releases yet", with no `list` element.
14. The status rows keep `render_index_list_status_row` and its actions. "Load more" stays the last item while `has_more` is true. Its action stays `load_more_release_names`.

## Decisions

- **D1.** The virtual list is the gpui `list` with `ListState`. Reasons:
  - `uniform_list` needs one height, and a status row is not as high as a release row. The error row text can use two lines.
  - `v_virtual_list` needs each item size before the first layout.
  - `ListState::splice` keeps the first row in view when a change is above it. Packet 036 option A needs that.
- **D2.** The cache key holds revisions, not copies of the inputs. A copy of the loaded pages for each frame costs the work that the cache removes.
- **D3.** The selection facts and the row menu are applied to each item in view, after the cache. A selection change thus builds no merge.
- **D4.** **Flag.** The app reads the image of a row only when the row is in view or in the overdraw area. Today it starts a request for each loaded row. A row that scrolls into view thus shows its image a short time later.
- **D5.** The rules of the merge do not change. The rows from the cache are equal to the rows of `releases_list` for the same inputs (packets 016, 026 and 036).
- **D6.** The packet adds no timing log. This crate has no `tracing` dependency (AGENTS.md "Logging"). The tests count builds, and the operator examines the feel.

## Questions

- **Q1. Does the Artists list get the same virtual list in this packet?**

  Facts: `render_library_destination_rows` (`src/ui/shells/library/destination_lists.rs:107-122`) renders each Artists row through `render_destination_list_container`. That container is not virtual. The Artists list in "Everything" also grows at each "Load more". `LibraryApp::render_list_column` calls `artists_list` one time in each frame (`src/library/app_impl.rs:4994-4996`). Nobody measured the Artists list.

  - Option A: only the Releases list. The Artists list keeps its container.
  - Option B: the Releases list and the Artists list use the same virtual list. The Artists list gets no cache of its merge.

  **Recommendation: Option B.** The two lists share one container today, so one list method keeps one code path. The Artists rows have no group, no menu and no checkbox, so the change is small.

## Required Changes

1. `src/view_models/index_name_list.rs` and `src/view_models/index_feed_singles.rs`: the revisions of Design step 2.
2. `src/view_models/library.rs`: `releases_revision`, `ReleasesListItemVm` and `releases_item_with_selection`. Delete `releases_list_with_selection`.
3. `src/view_models/releases_everything.rs`: `ReleasesListKey`, `ReleasesListRows`, `ReleasesListCache` and the pure function of the id comparison of Design step 8.
4. `src/library.rs`: replace `releases_list_scroll` with `releases_list_state`. Add `releases_list_cache`.
5. `src/library/app_impl.rs` and `src/library/music_scopes.rs`: `releases_rows`, and each reader of Design step 5. The Releases arm of `render_list_column` keeps no loop over each row. Change the focus move of Design step 12.
6. `src/ui/shells/library/destination_lists.rs` and `src/ui/shells/library/destination_list_scroll.rs`: the item render function and the `list` element. With option B of Q1, the Artists list too.
7. `src/ui/composites/music_column_scroll.rs`: `music_column_list_scrollbar`.
8. `tests/architecture_tests.rs`: add `music_column_list_scrollbar(` to the r14_01 guard. Change the markers of the r31_07 guard if the group row function changes. Add the guard R37-12.
9. Add each test of "Mechanical Acceptance Criteria". Change the tests of `releases_list_with_selection` (`src/view_models/library.rs:9488`, `:9554` and `:9626`) so that they use `releases_item_with_selection`.
10. Documents, in the same change:
    - Packet 016, Design "The sort key and the merge": the merge runs when one of its inputs changes. Name this packet.
    - The overhaul plan, items 16 and 31: name this packet as their fix. The items stay until the operator walks V1.
    - [Pending human checks](../pending-human-checks.md): add V1 to V6 of this packet.

## Mechanical Acceptance Criteria

Each criterion here is mechanical. A unit test beside its owning code has the prefix `adr_0084_task_037_`. A guard is in `tests/architecture_tests.rs` and has the same case number in its name. No test sends a request to `api.musicindex.org`.

| Case | Required proof |
|---|---|
| R37-01 | Mechanical, unit test in `src/view_models/releases_everything.rs`. Two reads of `ReleasesListCache` with the same key give one build. This is the read of the header action and the read of the rows in one frame |
| R37-02 | Mechanical, unit test in `src/library/app_impl.rs` or its view model. Each change below adds 1 to the build count at the next read, and the new rows show the change. The changes are `replace_tree` with a new Library release and `update_album_named_feeds`. They are also `set_download_activity` with a new phase and `begin_feed_removal`. They are also `finish_load`, `fail_load`, `IndexFeedSingles::record`, an open group and a scope change |
| R37-03 | Mechanical, unit test, same file. These changes give no build: a selection change, a selection mode change, a focus move, a hover change and an image answer |
| R37-04 | Mechanical, unit test. For the fixtures of packets 016, 026 and 036, the flat items of the cache give the same labels, in the same order, as `releases_list` |
| R37-05 | Mechanical, unit test. The flat item order equals the order of `releases_selection_rows`, with a status row at the start or at the end. The index of a focused row plus `leading` is its item index |
| R37-06 | Mechanical, unit test of the id comparison. It gives one run for each of these cases. The cases are one row added above, rows added at the end by "Load more", one row removed, and an open group. An item with the same id is in no run |
| R37-07 | Mechanical, gpui test (`#[gpui::test]`, as `adr_0083_selection_drag_r31_12_drop_status_names_the_count`). A list of 1,000 constructed rows in a window 600 px high has debug bounds for row 1 and no debug bounds for row 1,000 |
| R37-08 | Mechanical, gpui test. Scroll the list of R37-07 to row 500. A splice of one item above row 500 keeps row 500 as the logical scroll top |
| R37-09 | Mechanical, unit test. `releases_item_with_selection` gives the checkbox facts of the changed tests of Required Change 9. A group row reads checked, mixed or clear. A single of an open group has its own checkbox. Selection mode gives no row menu |
| R37-10 | Mechanical, unit test. The action bar counts follow packet 016 R16-16 and packet 018 R18-06, with the row states from the cache |
| R37-11 | Mechanical, unit test. While `has_more` is true, the last item is the status row with the accessibility label "Load more MusicIndex releases". After a failure of a later page, the error row is the last item |
| R37-12 | Mechanical, guard. Situational: it cites ADR 0084 Decision 8, packet 016 Design and this packet, and is deleted when ADR 0084 Decision 8 is superseded. Only the build of `ReleasesListCache` calls `releases_list(`. `header_action_for_list_column` and `render_list_column` call `releases_rows(`. The Releases list renders through `list(` with `releases_list_state`. Its failure message names the decision, item 31 and the fix |
| R37-13 | Mechanical. The changed r14_01 and r31_07 guards pass. Each test of packets 015, 016, 018, 026, 031, 033 and 036 passes |

The failure message of R37-12 has this form:

```text
ADR 0084 Decision 8, task 037 D2: the Releases list must read its rows from
LibraryApp::releases_rows. A second build of releases_list in one frame gave
about 500 ms for each frame (overhaul plan item 31). Read the cache in this call.
```

## Visual Acceptance Criteria

Each criterion here is visual. Only a person can examine it. No test proves it.

- V1: in the release binary, in Releases with the scope "Everything", click "Load more" 5 times. The list then scrolls with the mouse wheel and with the scrollbar, with no visible delay. A pointer move over the rows gives no visible delay. Compare with the measured 500 ms for each frame before this packet.
- V2: the scroll position stays when you open a row, click "Select" and "Done", open or close a group, or click "Load more". A group row that a later page adds above, by packet 036 option A, does not move the rows in view.
- V3: in selection mode, each row shows its checkbox, also a row that scrolls into view later. Shift-click selects a range across rows that were out of view. Ctrl+A selects the shown rows. The Up and Down keys keep the focused row in view.
- V4: "Load more" is the last row, and a click loads the next page. With no network, the error row shows "Try again".
- V5: the scrollbar thumb has a size near the size of the view part. A drag of the thumb scrolls the list. The scrollbar covers no row content.
- V6: a row that scrolls into view shows its image a short time later (decision D4).
- V7 (only with option B of Q1): V1, V2 and V5 for the Artists list with the scope "Everything".

## Exclusions

- No change to the rules of the merge, the order, the watermark or the groups (decision D5).
- No change to the requests, the page size or the singles requests (responsiveness item 19).
- No cache of the Artists merge, the Tracks list or the search list (responsiveness item 18).
- No timing log (decision D6).
- No visual change of the rows. Phase 7 of the overhaul plan owns the visual design.

## Files To Inspect

- [AGENTS.md](../../AGENTS.md) and the [source map](../architecture/source-map.md).
- `src/app.rs` (`TopApp::render`, the header action at line 1781).
- `src/library.rs` (the `LibraryApp` fields) and `src/library/column_view.rs`.
- `src/library/app_impl.rs` (`render_list_column`, `header_action_for_list_column`, `releases_visible_rows`, `releases_visible_members`, `move_selection_focus`, `disclose_focused_group`, `thumbnail_for_url`, `set_hovered_thumb`).
- `src/library/music_scopes.rs` (`releases_entries`, `load_more_release_names`).
- `src/view_models/library.rs` (`releases_list`, `releases_list_with_selection`, `releases_selection_rows`, `releases_selection_action_bar`, `release_row_state`, `album_for_release_key`).
- `src/view_models/releases_everything.rs`, `src/view_models/index_name_list.rs`, `src/view_models/index_feed_singles.rs` and `src/view_models/singles_group.rs`.
- `src/ui/shells/library/destination_lists.rs`, `src/ui/shells/library/destination_list_scroll.rs`, `src/ui/shells/library/index_list_status.rs` and `src/ui/composites/music_column_scroll.rs`.
- gpui-pre 0.3.1 `src/elements/list.rs` and gpui-kit `5463fe4` `crates/base/src/scrollbar.rs`.
- `tests/architecture_tests.rs` (the r14_01 and r31_07 guards, and each guard that names `destination_lists.rs`).

## Checks

```bash
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy --all-targets -- -D warnings
cargo check --all-targets
touch src/main.rs && cargo build --bin v4vmm
cargo build --release
```

## Rollback

Revert the commit. This packet adds no migration, no stored data and no configuration key.

## Operator Visual Check

This check needs a Linux desktop session and a connection to `https://api.musicindex.org`, except in step 8. Use the release binary for the frame time, because the debug build is slower.

1. Build and start the release binary:

   ```bash
   cd ~/build/v4vmm
   cargo build --release
   ./target/release/v4vmm
   ```

2. Open Music, then Releases. Select the scope "Everything". Click "Load more" at the end of the list 5 times.
3. V1: scroll up and down with the mouse wheel, then drag the scrollbar thumb. Move the pointer over the rows.
   **Wrong if:** the list follows the wheel, the drag or the pointer with a visible delay.
4. V2: do these steps:
   - Scroll to the middle of the list.
   - Click a row, click "Select", click "Done", and open and close a group with its chevron.
   - Scroll to the end and click "Load more".
   **Wrong if:** a step moves the rows in view. A new group row above the view is correct.
5. V3: do these steps:
   - Click "Select". Scroll and examine the checkboxes.
   - Click one row, scroll down, and Shift-click a row far below it.
   - Press Ctrl+A. Press Down 20 times.
   **Wrong if:** a row shows no checkbox, the range misses rows, Ctrl+A misses shown rows, or the focused row leaves the view.
6. V5: examine the scrollbar during steps 3 to 5.
   **Wrong if:** the thumb size is not near the size of the view part, or the scrollbar covers row text.
7. V6: scroll fast to a part of the list that you did not see before.
   **Wrong if:** a row keeps no image after some seconds, when the same row shows an image in the scope "Library".
8. V4: do these steps:
   - Stop the app. Run `nmcli networking off`.
   - Start `./target/release/v4vmm`, and open Releases with the scope "Everything".
   - Run `nmcli networking on` and click "Try again". Click "Load more".
   **Wrong if:** no error row shows, "Try again" does not load the rows, or "Load more" is not the last row.
9. V7, only with option B of Q1: do steps 2, 3, 4 and 6 in the Artists list.

Cleanup: make sure that `nmcli networking on` ran in step 8. This check changes no stored data. Click "Done" if selection mode is still on.

## Operator Answers - 2026-10-24

- Q1: option B. The Releases list and the Artists list use the same virtual list. The Artists list gets no cache of its merge.
