# ADR 0082 Publisher Roles Phase Plan

## Status

Active - 2026-10-02. This plan is advisory. It states no rule.
[ADR 0082](../adr/0082-publisher-roles-belong-to-each-album-link.md) owns the rules.

## Packet Register

| Packet | Scope | Owners | Depends on | State |
|---|---|---|---|---|
| [001](../tasks/archive/adr-0082-task-001-contract-0-7-0-and-link-facts.md) | The 0.7.0 contract copy, the decode of `album_names_as`, `role_agreement` and `co_credited_feeds`, a null `role`, and the storage of the two link facts | ADR 0082, ADR 0075 section 6 | None | Implemented 2026-10-02. Mechanical checks Green. No visual gate |
| [002](../tasks/adr-0082-task-002-publisher-page-role-groups.md) | The publisher page: no type, the groups and role subgroups, the row role text, the header roles and "Shares albums with". ADR 0078 moves to the archive | ADR 0082 Decisions 1 to 5 | 001 | Ready 2026-10-02 |
| [003](../tasks/adr-0082-task-003-album-page-publisher-link.md) | The album page publisher link with role, agreement and source | ADR 0082 Decision 6, ADR 0077 Decision 2 | 001 | Ready 2026-10-02 |

## Sequence

Packet 001 goes first. Packets 002 and 003 then need only its decode and storage. Dispatch them one at a time.

## Session Rules

Each implementation session owns one packet. It completes the packet, checks it, and stops.
A packet that changes user-visible behavior leaves its visual gate open in its `Status:` line and in
[pending human checks](../pending-human-checks.md).
