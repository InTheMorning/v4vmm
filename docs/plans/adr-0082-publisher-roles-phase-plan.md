# ADR 0082 Publisher Roles Phase Plan

## Status

Active - 2026-10-02. This plan is advisory. It states no rule.
[ADR 0082](../adr/0082-publisher-roles-belong-to-each-album-link.md) owns the rules.

## Packet Register

| Packet | Scope | Owners | Depends on | State |
|---|---|---|---|---|
| [001](../tasks/archive/adr-0082-task-001-contract-0-7-0-and-link-facts.md) | The 0.7.0 contract copy, the decode of `album_names_as`, `role_agreement` and `co_credited_feeds`, a null `role`, and the storage of the two link facts | ADR 0082, ADR 0075 section 6 | None | Implemented 2026-10-02. Mechanical checks Green. No visual gate |
| [002](../tasks/archive/adr-0082-task-002-publisher-page-role-groups.md) | The publisher page: no type, the groups and role subgroups, the row role text, the header roles and "Shares albums with". ADR 0078 moved to the archive on 2026-10-17 | ADR 0082 Decisions 1 to 5 | 001 | Complete 2026-10-18. The operator passed V3. ADR 0085 retired V1, V2 and V4 |
| [003](../tasks/archive/adr-0082-task-003-release-page-named-feeds.md) | The named feeds of a release. Migration 19 stores the position of each named feed. The packet adds the artist feed selection, the role words, and the role rows and "Also named" rows of the release page | [ADR 0085](../adr/0085-artists-by-confirmation.md) Decisions 5 and 6, ADR 0077 Decisions 2 and 6 | 001, ADR 0085 | Complete 2026-10-18. The operator passed V1 to V7 |

## Sequence

Packet 001 goes first. Packets 002 and 003 then need only its decode and storage. Dispatch them one at a time.

## Session Rules

Each implementation session owns one packet. It completes the packet, checks it, and stops.
A packet that changes user-visible behavior leaves its visual gate open in its `Status:` line and in
[pending human checks](../pending-human-checks.md).
