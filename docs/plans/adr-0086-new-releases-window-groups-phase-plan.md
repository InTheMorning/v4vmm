# ADR 0086 New Releases Window Groups Phase Plan

## Status

Active - 2026-10-23. This plan is advisory. It states no rule.
[ADR 0086](../adr/0086-new-releases-singles-window.md) owns the rules. The [overhaul plan](design-and-cleanup-overhaul-plan.md#remaining-order---2026-10-23) owns the order of this plan among the other phases.

The operator accepted ADR 0086 on 2026-10-23.

## Goal

New releases and the saved searches of Discover show one tile for the singles of one artist in one window. The window length is a Settings value.

## Non-Goals

- No change to the Releases list, the artist pages or the ADR 0084 Decision 6 group page.
- No new MusicIndex endpoint, and no Stophammer request.
- No schema change.
- No configuration change apart from the one key of ADR 0086 Decision 7.

## Packet Register

Each packet is one session. The operator walks the visual check of each packet before the next packet starts.

| Packet | Scope | Depends on | State |
|---|---|---|---|
| [001](../tasks/adr-0086-task-001-parallel-new-releases-detail-reads.md) | At most 4 feed detail reads of a New releases page run at the same time (Decision 5). The rows keep the order of the list response. It owns item 10 of the responsiveness list of the overhaul plan. It changes the speed only | ADR 0086 Accepted | Implemented 2026-10-23. V3 passed on 2026-10-23. V1, V2 and V4 are open, in the final walk of the run |
| [002](../tasks/adr-0086-task-002-new-releases-window-groups.md) | The window groups of New releases from the loaded rows, in Tiles and List. It adds the chain of Decision 1, the place of Decision 2, and the tile and the row of Decision 3. The window length is the default of 14 days | 001 | Implemented 2026-10-23. Passed: V1 and V2 on 2026-10-23 and V3 on 2026-10-24. V4, V6 and V8 are open, in the final walk of the run. Packet 004 replaced V5 and V7 |
| [003](../tasks/adr-0086-task-003-pager-closes-windows.md) | The pager loads until each window closes, with a maximum of 5 extra pages (Decision 5). It adds the "+" of an open group and the change in place | 002 | Implemented 2026-10-23. V2 and V3 passed on 2026-10-23. V1 and V4 to V10 are open, in the final walk of the run |
| [004](../tasks/adr-0086-task-004-window-group-page.md) | The window group page with its breadcrumb, its main action and its link to the Decision 6 group (Decision 4). It also adds the checkbox of a group tile in selection mode | 002 | Implemented 2026-10-23. Passed: V1 to V3, V5, V7 and V8 on 2026-10-24. V4, V6, V9 and V10 are open, in the final walk of the run |
| [005](../tasks/adr-0086-task-005-saved-search-window-groups.md) | The window groups of the saved searches: the loaded rows only, and the place of the anchor single in the relevance order (Decision 6) | 003 | Implemented 2026-10-23. V1 to V7 are open, in the final walk of the run |
| [006](../tasks/adr-0086-task-006-window-length-setting.md) | The Settings value of the window length, 1 to 60 days, through the shared guarded editor, with the configuration exception of Decision 7 | 002, [ADR 0069 task 002](../tasks/adr-0069-task-002-shared-guarded-editor.md) | Ready 2026-10-23 |

## Open Findings

- 2026-10-23: a `type=feed` search hit carries no `newest_item_at`, and Stophammer leaves its `pub_date` empty (`feed_search_summary` in `../stophammer/src/query.rs`). Packet 005 can thus group only the singles that the Library holds. The Index singles of a saved search group when a Stophammer release adds the date to a feed hit (packet 005 D5). [Open request 11](v4vmm-open-requests.md#11-a-feed-search-hit-gives-its-release-date-and-its-named-feeds) asks for the date and the named feeds of a feed hit.
- 2026-10-23: the window group page of packet 004 finds its members in the New releases rows. Packet 005 opens the same page from the rows of a saved search. Packet 005 stops when the page of packet 004 cannot take these members.
- 2026-10-23: packet 003 found a defect of today's code. A New releases page that arrives late replaces the Tracks rows, because `replace_recent_feeds_page` always sets the source to `RecentMusic`. Packet 003 corrects it, and it is implemented on 2026-10-23.

## Sequence

1. Packets 001 to 005 in register order.
2. ADR 0069 task 002, then packet 006. The groups use the default of 14 days until packet 006 is complete.

## Affected Modules

- New releases page data: `src/application/queries/feed.rs`.
- Groups: `src/view_models/singles_group.rs` and the Discover view models.
- Saved searches: `src/view_models/search_results/saved_search.rs`.
- Settings and configuration: `src/config.rs` and the ADR 0069 Settings owners.
- Guards: `tests/architecture_tests.rs`.

## Risk Areas

- Stale pages. An extra load must not show the rows of an old request after the curator moves to a different view.
- In-place updates. A later page can change a single tile into a group tile. The mounted view must update with no navigation.
- Request load. Packet 003 can send a maximum of 6 list requests and 120 detail reads for one page request. Packet 001 limits the delay first.

## Test Strategy

- Each mechanical criterion of ADR 0086 is a unit test at its owner layer, with the prefix of its packet.
- Each packet adds or updates a situational guard that cites ADR 0086 and its decision.
- Each packet that changes the interface has a visual check that the operator walks.

## Rollback Strategy

Each packet is one commit with no migration. Revert the commit. Packet 006 adds a key with `#[serde(default)]`, so an older build ignores the key and a newer build loads an older `config.toml`.
