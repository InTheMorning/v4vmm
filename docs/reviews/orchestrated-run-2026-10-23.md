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

## Steps

| Step | State | Commit |
|---|---|---|
| Planning documents of 2026-10-23 | Done | `e38b6e3`, Stophammer `97aa28e` |
| ADR 0086 packet 001: parallel detail reads | Implemented. V1 to V4 open | `cdbabc4` |
| ADR 0086 packet 002: New releases window groups | Implemented. V1 to V4, V6 and V8 open | `fffac06` |
| ADR 0086 packet 003: the pager closes each window | Implemented. V1 to V10 open | `fbeec62` |
| ADR 0086 packet 004: the window group page | Implemented. V1 to V10 open | (this commit) |
| ADR 0086 packet 005: saved search window groups | Ready | |
| ADR 0087 packet 001: the query parser | Ready | |
| ADR 0087 packet 002: Library fields and Index words | Ready | |
| ADR 0087 packet 004: the field list | Ready | |
| ADR 0087 packet 003: live results | Ready | |
| ADR 0087 packet 005: the values of the field list | Ready | |

## Questions For The Operator

None at the start of the run.

## Final Walk

The orchestrator writes the final walk when the last packet is complete. It starts with ADR 0084 packet 020 V1 to V6 and packet 026 V1 to V10.
