# ADR 0077 Publisher Artist Phase Plan

## Status

Active - 2026-09-24. This plan is advisory. It states no rule.
[ADR 0077](../adr/0077-publisher-feed-artist-binding.md), [ADR 0078](../adr/0078-publisher-page-type-from-stated-role.md)
and [ADR 0079](../adr/0079-remove-musicindex-artist-subject-storage.md) own the rules.

## Packet Register

| Packet | Scope | Owners | Depends on | State |
|---|---|---|---|---|
| [001](../tasks/adr-0077-task-001-remove-dead-artist-storage.md) | Delete the ADR 0045 binding and the ADR 0029 artist subject storage | ADR 0077 Decision 7, ADR 0079 | None | Ready |
| [002](../tasks/adr-0077-task-002-publisher-relationship-transport-and-storage.md) | Decode the publisher relationship and store it for each Library feed | ADR 0077 Decisions 2 and 5 | 001 | Ready |
| [003](../tasks/adr-0077-task-003-publisher-page-view-model.md) | Publisher page query and view model | ADR 0077, ADR 0078 | 002, and the Stophammer answer | Held |
| [004](../tasks/adr-0077-task-004-publisher-navigation-and-presentation.md) | Navigation, screens, feed owner text and the name search | ADR 0077 Decisions 1, 2 and 6 | 003 | Held |

Packet 002 depends on packet 001 for the migration order only.
Packet 001 adds schema version 13, and packet 002 adds schema version 14.

## Held Work

Packets 003 and 004 wait for the answer to the
[publisher album summary request](stophammer-publisher-album-summary-request.md).
The operator selected this route on 2026-09-24 over a paged album fetch.

When Stophammer adds the album summary, packet 003 reads it and sends one request for each publisher page.
When Stophammer rejects the request, the operator selects a new route before packet 003 starts.

## Session Rules

Each implementation session owns one packet. It completes the packet, checks it, and stops.
A packet that changes user-visible behavior leaves its visual gate open in its `Status:` line and in
[pending human checks](../pending-human-checks.md).
