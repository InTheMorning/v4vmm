# ADR 0087 Search Fields Phase Plan

## Status

Active - 2026-10-23. This plan is advisory. It states no rule.
[ADR 0087](../adr/0087-search-fields-and-live-results.md) owns the rules. The [overhaul plan](design-and-cleanup-overhaul-plan.md#remaining-order---2026-10-23) owns the order of this plan in the other work.

The operator accepted ADR 0087 on 2026-10-23. Its packets come after ADR 0086 packet 005 and before ADR 0069 task 002.

## Goal

The toolbar search reads words, phrases and fields. A field list under the search field shows the fields and their values. The results update while the curator types.

## Non-Goals

- No colored chips in the search text.
- No `OR`, parentheses or fuzzy matches.
- No change to the result tabs or to the search page layout.
- No schema change. A saved search keeps its text in migration 21.

## Packet Register

Each packet is one session. The operator walks the visual check of each packet before the next packet starts.

| Packet | Scope | Depends on | State |
|---|---|---|---|
| [001](../tasks/archive/adr-0087-task-001-query-parser.md) | The query parser (Decision 1). It gives the typed query with its words, phrases, fields, exclusions and errors. It changes no interface | ADR 0086 packet 005 | Complete 2026-10-23 |
| [002](../tasks/adr-0087-task-002-library-fields-and-index-words.md) | The Library search reads the typed query, with one query builder for each field of Decision 2. The Index search sends the words and phrases only. A query with an Index field shows no Index rows and the note of Decision 5. A saved search parses its text (Decision 6) | 001 | Implemented 2026-10-23. V2, V3 and V5 to V7 passed on 2026-10-23. V1, V4, V8 and V9 are open, in the final walk of the run |
| [003](../tasks/adr-0087-task-003-live-results.md) | The results update while the curator types: the pause token, the run generation and the update in place (Decision 4) | 002 | Implemented 2026-10-23. V1 and V5 passed on 2026-10-23. V2 to V4 and V6 to V9 are open, in the final walk of the run |
| [004](../tasks/adr-0087-task-004-field-list.md) | The field list composite with the fields, the matching fields and the errors (Decision 3). It adds the keys, and the focus stays in the search field | 001 | Implemented 2026-10-23. V2, V3 and V5 passed on 2026-10-23, V1 and V6 failed. V1, V4 and V6 to V9 are open, in the final walk of the run |
| [005](../tasks/adr-0087-task-005-field-list-values.md) | The values in the field list (Decision 3). They are the artist names of the Library and of `GET /v1/artist-names`, the release titles, the playlist names and the fixed values | 003, 004 | Implemented 2026-10-23. V1, V2 and V4 passed on 2026-10-23, V7 failed. V3 and V5 to V7 are open, in the final walk of the run |
| 006 | The Index filters of open request 10. It replaces the stored contract in the same change as the decode change (ADR 0075 section 6) | 002, a Stophammer release of request 10 | Waits for Stophammer |

## Open Findings

- 2026-10-23, operator request during the walk: a field `label:`, so that a curator can find a label. A label is a feed that the albums name with the role `label` (ADR 0085). The `type=publisher` search of the Index already gives `agreed_roles` with `include=link_facts` (`src/application/queries/search.rs`). The field needs an amendment of ADR 0087 Decision 2 first. Its Index part can need a role filter in open request 10.

## Sequence

1. Packets 001 to 005 in register order. Packet 004 can come before packet 003, because it needs only packet 001.
2. Packet 006 after the Stophammer release of request 10.

## Affected Modules

- Parser and field list: a new module in `src/view_models/`.
- Toolbar field: `src/app.rs`, `src/app/tab_bar.rs` and `src/view_models/app_toolbar.rs`.
- Search runs: `src/app/search_dispatch.rs` and `src/application/queries/search.rs`.
- Library query: `src/db.rs` (`search_library_tracks`) and `src/library_service.rs`.
- Index request: `src/api.rs` (`Client::search`).
- Field list composite: `src/ui/composites/`.
- Guards: `tests/architecture_tests.rs`.

## Risk Areas

- Focus. The field list must not take the focus from the search field. The popover primitive takes it.
- Stale results. A run of an older generation must not replace a newer result.
- SQL. Each Library value goes into SQL only as a bound parameter.
- Request load. Each pause sends three Index requests, and `artist:` sends one more (responsiveness item 24 of the overhaul plan).
- Saved searches. A saved query with a colon can change its meaning. The rule of Decision 1 keeps an unknown field name as a word.

## Test Strategy

- Each mechanical criterion of ADR 0087 is a unit test at its owner layer, with the prefix of its packet.
- Each packet adds or updates a situational guard that cites ADR 0087 and its decision.
- Each packet that changes the interface has a visual check that the operator walks.

## Rollback Strategy

Each packet is one commit with no migration. Revert the commit.
