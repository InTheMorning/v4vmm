# ADR 0077 Publisher Artist Phase Plan

## Status

Active - 2026-09-24. This plan is advisory. It states no rule.
[ADR 0077](../adr/0077-publisher-feed-artist-binding.md), [ADR 0078](../adr/0078-publisher-page-type-from-stated-role.md)
and [ADR 0079](../adr/0079-remove-musicindex-artist-subject-storage.md) own the rules.

## Packet Register

| Packet | Scope | Owners | Depends on | State |
|---|---|---|---|---|
| [001](../tasks/adr-0077-task-001-remove-dead-artist-storage.md) | Delete the ADR 0045 binding and the ADR 0029 artist subject storage | ADR 0077 Decision 7, ADR 0079 | None | Implemented 2026-09-24. Mechanical checks Green. Visual gate open and paused |
| [002](../tasks/adr-0077-task-002-publisher-relationship-transport-and-storage.md) | Decode the publisher relationship and store it for each Library feed | ADR 0077 Decisions 2 and 5 | 001 | Implemented 2026-09-24. Mechanical checks Green. No visual gate |
| [003](../tasks/adr-0077-task-003-publisher-page-view-model.md) | Publisher page query and view model | ADR 0077, ADR 0078 | 002, and Stophammer ADR 0059 | Implemented 2026-09-26. Mechanical checks Green. No visual gate |
| [004](../tasks/adr-0077-task-004-publisher-navigation-and-presentation.md) | Navigation, screens, feed owner text and the name search | ADR 0077 Decisions 1, 2 and 6 | 003 | Next. Its task review comes first |

Packet 001 added schema version 13, and packet 002 added schema version 14.

## Held Work

Packet 004 is ready for its task review. Packet 003 is implemented.

Stophammer deployed the album summary fields of its ADR 0059 on 2026-09-26, at commit `264706e`.
The [open Stophammer requests](v4vmm-open-requests.md#verification-by-v4vmm---2026-09-26) record the v4vmm verification.
Packet 003 reads the `remote_*` fields and sends one request for each publisher page.

## Follow-Up Findings

Packet 001 recorded these findings on 2026-09-24. No packet owns them yet.

- `api::Track.artist_credit` has no production reader. Only its decode and the ADR 0075 observation field list name it. MusicBrainz lookup reads its own response. The R1-09 premise of packet 001 was incorrect.
- `ApiSource::fetch_artist` always returns an error, because MusicIndex has no artist route. `ApiSource` has no constructor outside `src/sources.rs`.
- Packet 002 corrected the ADR 0075 evidence rule for `publisher`. Commit `82c3c06` treated the collection as an object. The live contract sends an array.
- A stored publisher relationship is never deleted. Packet 013 keeps MusicIndex replacement disabled until Stophammer states that the collection is complete.
- `src/audio_format/probe.rs` test `adr_0066_converter_failures_are_distinct_and_output_is_not_retained` failed once in a full run and passed when run again. It writes a script and runs it at once. A parallel run can cause this failure.

## Session Rules

Each implementation session owns one packet. It completes the packet, checks it, and stops.
A packet that changes user-visible behavior leaves its visual gate open in its `Status:` line and in
[pending human checks](../pending-human-checks.md).
