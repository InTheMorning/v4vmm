# Orchestrated Run Of 2026-10-23

## Status

Open - 2026-10-23. One orchestrator session runs the steps below. The operator walks the visual checks of the run at its end.

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

None at the start of the run.

## Final Walk

The orchestrator writes the final walk when the last packet is complete. It starts with ADR 0084 packet 020 V1 to V6 and packet 026 V1 to V10.
