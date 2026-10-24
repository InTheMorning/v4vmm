# Orchestrated Run Of 2026-10-23

## Status

Implementation complete - 2026-10-23. Each packet of the run is implemented and committed. The final walk is open.

This record is advisory. It states no rule. Each rule stays in its ADR.

## Operator Decisions - 2026-10-23

- The run implements ADR 0086 packets 001 to 005, then ADR 0087 packets 001 to 005, in register order. ADR 0087 packet 004 can come before packet 003.
- ADR 0086 packet 006 waits for ADR 0069 task 002. ADR 0087 packet 006 waits for the Stophammer release of open request 10. Neither is in this run.
- The visual checks of the run come last, in one final walk. The walk starts with the open checks of ADR 0084 packets 020 and 026. The remaining order of the overhaul plan puts these checks before ADR 0086.
- The orchestrator first commits the planning documents of 2026-10-23. Then it commits each reviewed, Green packet as HeyCitizen, and never pushes. The Stophammer copy of `docs/plans/v4vmm-open-requests.md` gets one commit in the Stophammer repository.
- When a coding agent stops, the orchestrator decides inside the accepted ADRs, records the decision below, and continues. When a stop needs a new ADR decision, the orchestrator skips that packet. It continues with the packets that do not need it, and keeps the question for the operator.

## Orchestrator Decisions

Each entry names the step, the decision and the reason.

- ADR 0086 packet 002. The orchestrator accepted six deviations of the coding agent:
  - The guard of ADR 0060 forbids each `dead_code` attribute. Names that only later packets call are thus `#[cfg(test)]` for now: `WindowLength::{MIN_DAYS, MAX_DAYS, from_days, days}`, `LibraryViewModel::{window_length, set_window_length}` and `NewReleasesLayout::any_window_open`. The packet that first calls a name removes its `#[cfg(test)]`.
  - Guard R15-09 also lets `src/view_models/window_groups.rs` name `ReleaseKind::Single`. ADR 0086 Decision 1 groups singles only.
  - The 500-line limit of a file moved the group renderers to `src/ui/shells/library/window_groups.rs`, and the list state messages to `content_list_state.rs`.
  - The content list renderers take one tuple `ContentListImages`, because a guard needs the parameter name `thumbnails`.
  - `ContentListItemVm` has `#[expect(clippy::large_enum_variant)]`.
  - The layout keeps no `WindowGroup`. Thus the tile has no `anchor_date` and no `window_start`. Packet 004 adds them to the tile, because its `WindowGroupKey` needs them.
- ADR 0086 packet 003. The orchestrator accepted these deviations:
  - The ADR 0062 guard and the guard R86-2-27 now name the new pager functions. Both guards keep their rule: Music recency uses the pager of `RecentFeedsPageVm`, and one function builds the groups.
  - `extra_pages()` and the test helper `RecentFeedsPageVm::load_recorded_page` are `#[cfg(test)]`. No production code calls them.
  - `content_list_row_from_recent_feed` is new, because a guard requires the signature of `content_list_rows_from_recent_feeds`.
  - The packet also corrects the defect of 2026-10-23: a late New releases page no longer replaces the rows of a different list.
- ADR 0086 packet 004. The orchestrator accepted these deviations:
  - `anchor_date` and `window_start` are only in `WindowGroupTileVm::page_key`. No code reads two more tile fields.
  - `NewReleasesLayout::joined_by_name(row_id)` is new. The page needs it to show the singles that join by a name match under "Matched by name".
  - `singles_group_link(group)` is new, and `render_main_action` takes an element id. The singles group page keeps its labels and output.
  - `shown_row_ids` is `#[cfg(test)]`, because no production code calls it now.
  - The packet replaces packet 002 D8, D9, V5 and V7. The test R86-2-23 reads `discover_page_rows`, and the two R86-2-24 tests are deleted.
- ADR 0086 packet 005. The orchestrator accepted these deviations:
  - `WindowGroupKey` has a source, `WindowGroupSource::{NewReleases, SavedSearch(rows)}`. `LibraryViewModel::window_group_page` reads it and calls `WindowGroupPageVm::build` one time. Guard R86-4-24 does not change. This is smaller than a second build method, because no caller of the page in `LibraryApp` can reach the saved search page of `TopApp`.
  - A saved search page key holds a copy of the member rows at the click. After a removal, a Library member can leave the page.
  - Guard R28-03 now requires `detail.focused_row_index(&lookup)`, because the focused index is an item index. Its rule does not change.
  - A row with no feed GUID gets no group key, as the `WindowEntry` rule of packet 002 says.
- ADR 0087 packet 001. The orchestrator accepted these deviations:
  - `index_q` writes a backslash in a phrase as `\\`, so a phrase that ends with a backslash gives no broken quote.
  - The field `terms` of `SearchQuery` is private, and `terms()` reads it.
  - The coding agent named the `FieldValueError` variants, because the packet gives no names: `UnknownValue`, `OneValueOnly`, `NotADate`, `NotADuration` and `ExcludedScope`.
  - A value of whitespace only gives `Incomplete`. `-in:` with no value also gives `Incomplete`.
  - The packet is complete with no visual check, so it moves to `docs/tasks/archive/`.
- ADR 0087 packet 002. The orchestrator accepted these deviations:
  - `search_local_library_tracks` is `pub(crate)`, because a `pub` method with a crate-private type gives a warning. `UnsupportedIndexTerms` has no re-export.
  - New names: `library_part_runs(query)`, `LibrarySearchFacts::{tracks, ready_track_ids}` and `TopApp::index_search_result_is_current`. The fields of `LibraryTrackSearchFacts` are `pub(crate)`.
  - `LibrarySearchFacts::track` and `SearchResultsInspectorPageVm::index_skip` are `#[cfg(test)]` until a production caller exists.
  - The five guards of D24 name the new functions and keep their rules. The guard `adr_0060_search_uses_music_section_navigation` changes only its range end marker to `search_results_detail_for_run`.
  - The guard R87-2-29 also requires that only `src/app/search_dispatch.rs` calls `FetchIndexSearchResults::new(`, with `request.clone()`. Without this rule, a copy in the saved search code does not fail.
- ADR 0087 packet 004 runs before packet 003, as the operator decision of the run permits. The orchestrator accepted these deviations:
  - A propagated Enter gives the input a `Change` with the same text after `PressEnter`, and that change opened the list again. The view now ignores the one `Change` after Enter that has the submitted text (`submitted_text`). A walk on X11 or Wayland confirms the real event order.
  - `FieldListEdit::apply` is `#[cfg(test)]`, because the view edits the text through the input.
  - New names: `FieldListRowAvailability::is_available()` and the private `SearchFieldListView::send(event)`.
  - The notices are above the scroll container of the rows, so `scroll_to_item` gets row indexes.
  - The `in:index` conflict of a row comes from `SearchQuery`, so no field syntax is outside the parser.
  - `src/app/search_field_list.rs` is in `SCREEN_FILES`, as the guard of top level GPUI modules requires. The rule of the guard does not change.
- ADR 0087 packet 003. The orchestrator accepted these deviations:
  - `show_toolbar_search_run` takes `&SearchRun`, as clippy requires.
  - `SearchSlotRuns::next_run` also cancels the token of the Index request in flight. A newer run that sends no Index request thus also stops the older request (D12).
  - A result that arrives with no mounted toolbar page builds the page with its own generation. A new generation there drops the result.
  - A canceled retry closes its pending retry intent with a message.
  - `dispatch_index_search` takes a generation and a `CommandContext`, with `#[expect(clippy::too_many_arguments)]` and the helper `index_search_context(slot)`.
  - `LiveRunGate::current_run` and `run_generation()` are `#[cfg(test)]` until a production caller exists.
  - The test `adr_0084_search_list_rejected_query_retains_no_action` expects no retained action for `Cancelled` (D14).
  - Five guards name the moved or renamed code and keep their rules. `src/app/search_live.rs` is in `SCREEN_FILES`.
- ADR 0087 packet 005 stopped on its stop condition for quoted values. Packet 001 gives a quoted field value a `value_range` and a `prefix` inside the quotes. Packet 005 needs both to start at the opening quote. The orchestrator decided:
  - `cursor_term` gives a quoted value a `value_range` from the opening quote to after the closing quote, or to the end of the text. The `prefix` is the raw text from the opening quote to the cursor.
  - The fixed-value match of packet 004 resolves the prefix with `unquote_field_value_prefix`, so `kind:"si` still matches `single`. A quoted `kind:` value is one item, and its commas do not split it.
  - `FieldListRowKind` and `FieldListRowAction` lose `Copy`, because the new variants hold a `String`. The packet 004 tests borrow them.
  - Reason: this follows the design of packet 005 and adds no name. No test of packet 001 asserts the range of a quoted value.
- ADR 0087 packet 005. The orchestrator accepted these deviations:
  - `in:index playlist:fri` has the scope `Conflict`, because the typed `playlist:` value is a correct Library-only field. A Library-only value row is unavailable with the note "Not available with in:index".
  - `unquote_field_value_prefix` also stops at a closing quote with no backslash, so `"Haleen"` gives `Haleen`.
  - `Quoted::content_end` is removed, because no code reads it now.
  - `FieldValueRows::EMPTY` and its empty constant are `#[cfg(test)]`. `FieldListVm::build` is `pub(crate)`.
  - The field list view also observes the Library entity, so an open list updates in place when the Library values change.
  - Open concerns: when no MusicIndex endpoint is set, the `artist:` request fails and the error row shows. One test leaves a cache folder in the temporary folder.
- The file `LICENSE` appeared untracked in the working tree during the run. The run did not make it and does not commit it.
- Commit `d410e9f`, "docs: research request on shared search code", came from a different session during the run. It adds `docs/plans/research-search-request.md`. It changes no file of the run, and the run did not change it.

## Steps

| Step | State | Commit |
|---|---|---|
| Planning documents of 2026-10-23 | Done | `e38b6e3`, Stophammer `97aa28e` |
| ADR 0086 packet 001: parallel detail reads | Implemented. V1 to V4 open | `cdbabc4` |
| ADR 0086 packet 002: New releases window groups | Implemented. V1 to V4, V6 and V8 open | `fffac06` |
| ADR 0086 packet 003: the pager closes each window | Implemented. V1 to V10 open | `fbeec62` |
| ADR 0086 packet 004: the window group page | Implemented. V1 to V10 open | `fcee74d` |
| ADR 0086 packet 005: saved search window groups | Implemented. V1 to V7 open | `3e28b24` |
| ADR 0087 packet 001: the query parser | Complete. No visual check | `93b2c79` |
| ADR 0087 packet 002: Library fields and Index words | Implemented. V1 to V9 open | `ca8411a` |
| ADR 0087 packet 004: the field list | Implemented. V1 to V9 open | `1ab7dff` |
| ADR 0087 packet 003: live results | Implemented. V1 to V9 open | `c4794d1` |
| ADR 0087 packet 005: the values of the field list | Implemented. V1 to V7 open | (this commit) |

## Questions For The Operator

1. 2026-10-23, found in the walk of ADR 0084 packet 026 V1: in the Releases list with the scope "Everything", a singles group row has the place of its first single in title order (packet 026 decision D5, `ReleaseOrderKey::group_at` in `src/view_models/releases_everything.rs`). The row shows the artist name, so the order looks wrong. Groups with a first title that starts with a symbol collect at the top. Both Haleen rows are at "A Greater Call".

   In the scope "Library", a group row has the place of the artist name. The operator found this order hard to use. A change amends ADR 0084 Decision 6 or packet 026 D5. The orchestrator asks the operator after the walk.

2. 2026-10-23, found in the walk of ADR 0084 packet 026 V6: after the download of each MusicIndex single of a group, the group row moved in the scope "Everything". The place of the row is `SinglesGroup::first_title_sort_key` (`src/view_models/singles_group.rs`). MusicIndex gives these singles no `sort_name`, so a title change of the key does not explain the move.

   The operator read the database on 2026-10-23. Only 18 Library releases have a stored link to the feed `64569161` in `feed_publisher_relationships`. "A Greater Call", "Build on Stone" and "A Phoenix Rising" were in the download, and they have no stored link. "Whispers" was in the Library before, and it has a link.

   Thus the 37 downloads stored no feed links, and each new single left the feed group. ADR 0084 packet 034 says that a download stores the feed links of its release. Thus this is also a defect of packet 034, for a download from a singles group.

   On the same day, `GET /v1/feeds/{guid}?include=publisher` gave a `music_to_publisher` entry to `64569161` for "A Greater Call" (`67ae17fe`), the same as for "Whispers" (`09b1bf8b`). The MusicIndex answer is correct, so the defect is in the app: each download calls `hydrate_album_identity_by_feed_guid` (`src/library/downloads.rs`), but no link was stored. Its checks V1 to V4 stay open. A fix needs a packet, and V6 stays open until a person walks it again.

3. 2026-10-23, from the operator during the walk: the operator plans a new overhaul of the visual design, because of the use of screen space and the size and color of the interface. That work needs its own decision record. It can retire the visual criteria of size and color in the open packets. The behavior criteria stay.

## Final Walk

Each packet of the run is implemented and committed. No person walked a visual check of the run. Each gate stays open in its packet `Status:` line, in `docs/pending-human-checks.md` section 6 and in the delivery order.

### Setup

The run adds no migration and no configuration key, so no backup is necessary.

1. Open a terminal in a desktop session.
2. Build the release binary for the checks of speed: `cd ~/build/v4vmm && cargo build --release`.
3. Build the debug binary for the other checks: `cargo build --bin v4vmm`.
4. Confirm that the debug binary is a normal build: `nm -C target/debug/v4vmm | grep -c TestDispatcher` gives `0`.
5. Start the app: `./target/release/v4vmm` or `./target/debug/v4vmm`.

The checks need these states:

- `https://api.musicindex.org` is reachable.
- New releases holds an artist with many singles, for example jSmovie.
- The Library holds two or more singles of one artist with release dates within 14 days. A saved search finds them.
- The Library holds one or more playlists, for the field `playlist:`.

### Order

Walk each check in Light and Dark, and at normal and narrow width. Each packet names what is wrong for each check in its "Visual Acceptance Criteria".

1. ADR 0084 [packet 020](../tasks/adr-0084-task-020-discover-selection.md) V1 to V6 and [packet 026](../tasks/adr-0084-task-026-index-singles-groups.md) V1 to V10. These checks come first, so that an older defect is not mixed with a defect of the run.
2. ADR 0086 [packet 001](../tasks/adr-0086-task-001-parallel-new-releases-detail-reads.md) V1 to V4, with the release binary.
3. ADR 0086 [packet 002](../tasks/adr-0086-task-002-new-releases-window-groups.md) V1 to V4, V6 and V8.
4. ADR 0086 [packet 003](../tasks/adr-0086-task-003-pager-closes-windows.md) V1 to V10. The checks of the load time use the release binary.
5. ADR 0086 [packet 004](../tasks/adr-0086-task-004-window-group-page.md) V1 to V10.
6. ADR 0086 [packet 005](../tasks/adr-0086-task-005-saved-search-window-groups.md) V1 to V7.
7. ADR 0087 [packet 002](../tasks/adr-0087-task-002-library-fields-and-index-words.md) V1 to V9.
8. ADR 0087 [packet 004](../tasks/adr-0087-task-004-field-list.md) V1 to V9.
9. ADR 0087 [packet 003](../tasks/adr-0087-task-003-live-results.md) V1 to V9.
10. ADR 0087 [packet 005](../tasks/adr-0087-task-005-field-list-values.md) V1 to V7.

ADR 0087 packet 001 has no visual check.

### Record

Tell the orchestrator each result with its check name, for example "0086 003 V4 pass" or "0087 004 V2 fail: the list takes the focus". The orchestrator records each result in a walk log in this record, in the packet `Status:` line, in the delivery order and in `docs/pending-human-checks.md`.

### Walk Log

| Date | Check | Result | Note |
|---|---|---|---|
| 2026-10-23 | ADR 0084 packet 020 V1 to V6 | Pass | Discover selection |
| 2026-10-23 | ADR 0084 packet 026 V1 | Pass | "Haleen · 56 singles · Artist feed", icon "19 of 56". MusicIndex added a confirmed "You Are Enough" (`97745d4b`, Oct 11, 2026) to the feed `64569161` |
| 2026-10-23 | ADR 0084 packet 026 V2 | Pass, one part open | "Haleen · 38 singles · Artist feed". The row of the unconfirmed "You Are Enough" (`ba11c670`) sorts under "Y" and stays open |
| 2026-10-23 | ADR 0084 packet 026 V3 | Pass | Paul Birken group row. No Sir Libre Records group row |
| 2026-10-23 | ADR 0084 packet 026 V4 | Pass | "Load more" adds rows at the end, and no row moves |
| 2026-10-23 | ADR 0084 packet 026 V5 | Pass | The chevron opens the Haleen group in place, newest first, with the state icons |
| 2026-10-23 | ADR 0084 packet 026 V6 | Fail | The download of the 37 MusicIndex singles of Haleen `64569161` finished at 20:44 with "Downloaded 37 of 37". The icons and the group page changed in place. Then the group row of 56 singles left its place in "Everything" |
| 2026-10-23 | ADR 0084 packet 026 V7 | Skipped, open | The Library list needs a scroll to "H". The operator skipped the check |
| 2026-10-23 | ADR 0084 packet 026 V8 | Pass | Walked with Rap Central in place of Paul Birken: an Index single page shows "All 15 singles", and the link opens the group page |
| 2026-10-23 | ADR 0084 packet 026 V9 | Skipped, open | The check needs the network off. The operator skipped it |
| 2026-10-23 | ADR 0084 packet 026 V10 | Fail | At narrow width and in selection mode, the titles of group members keep 3 to 5 letters, for example "You A…" and "Halee…". The checkboxes, the chevrons and the icons are not clipped |
| 2026-10-23 | ADR 0086 packets 001 to 003 and ADR 0087 packets 002 to 005 | Readings from the screenshots of `~/.claude/v4vmm-shots/20261023-210405/`, Dark theme, accepted by the operator | An agent read 57 screenshots. The orchestrator examined files 02, 12 and 19 |
| 2026-10-23 | ADR 0086 001 V3, 002 V1 and V2, 003 V2 and V3 | Pass | jSmovie is one tile, "16+ Singles New". "Knuckle Knock · Oct 6 to Oct 14 · 2+ Singles". At narrow width no tile clips |
| 2026-10-23 | ADR 0087 002 V2, V3 and V5 to V7 | Pass | "MusicIndex cannot filter by after: yet" with "The Library rows show.". `No releases matched "zzzz kind:ep".`. "No results" for `in:index is:ready` |
| 2026-10-23 | ADR 0087 003 V1 and V5 | Pass | The page opens after the pause, and the caret stays. Back shows Discover. Forward shows "haleen dri" |
| 2026-10-23 | ADR 0087 004 V2, V3 and V5 | Pass | "ar" gives `artist:`. Down and Up wrap. Tab gives `artist:`. "after: needs a date, for example 2026-10-01" |
| 2026-10-23 | ADR 0087 004 V1 | Fail | The list shows 9 of 11 rows and stops in the `is:` row. `playlist:` and `has:` cannot be reached, and the list has no scroll bar. Each description is nearer to the next field name than to its own |
| 2026-10-23 | ADR 0087 004 V6 | Fail | The operator typed `in:index ` by hand: the trailing space closes the field list, so the note "Not available with in:index" cannot show |
| 2026-10-23 | ADR 0087 005 V1, V2 and V4 | Pass | Library names with "Library". "Loading names from MusicIndex", then "hacker hereje · MusicIndex". `playlist:` and `release:a` give their values |
| 2026-10-23 | ADR 0087 005 V7 | Fail | A long artist name wraps to three lines, but its row keeps one line of height. Its "Library" mark is drawn into the next row "The Doerfels" |
| 2026-10-24 | ADR 0086 004 V1 to V3, V5 and V7, 002 V3, ADR 0087 002 V1 and V4, 003 V4 and V7, 004 V7 and V8, 005 V5 | Pass | Readings of `~/.claude/v4vmm-shots/20261024-120204/`, release binary, Dark, accepted by the operator. "Discover › New releases › jSmovie · 16 singles", "Download 16 singles", "16 releases selected", `artist:"Able and The Wolf" x`, "MusicIndex cannot exclude words yet" |
| 2026-10-24 | ADR 0087 003 V3 | Fail | A live run of `survival` showed only the 2 Library rows, with no loading row, no Index row and no error, also 5 s later. MusicIndex 0.14.0 gave 17 feeds and 20 tracks for the same word. A search with Enter shows Index rows |
| 2026-10-24 | ADR 0086 004 V8, ADR 0087 004 V4 | Pass | The operator walked them. The jSmovie row in List mode opens its group page and selects 16 singles. A click on `single` gives `kind:single`, and the field keeps the focus |
| 2026-10-24 | ADR 0087 003 V6 | Fail | The operator walked it. After Down and Enter on a value, the text is `artist:"Able and The Wolf"`, the search runs, and the field list closes. The criterion requires that the list stays open |

### Cleanup

The walk changes no file. The tests of ADR 0087 packet 005 can leave a folder in the temporary folder. Remove it with `rm -rf "${TMPDIR:-/tmp}/v4vmm-search-field-list-test"`.
