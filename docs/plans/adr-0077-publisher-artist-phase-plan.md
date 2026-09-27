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
| [004](../tasks/adr-0077-task-004-publisher-navigation-and-presentation.md) | Publisher page, navigation from an album and a track, and the Library album values | ADR 0077 Decisions 1 and 2, ADR 0078 | 003 | Implemented 2026-09-26. Mechanical checks Green. Visual gate open and paused |
| [005](../tasks/adr-0077-task-005-feed-owner-text-and-name-search.md) | Feed owner text, removal of the `publisher_text` inspector, the name search and the name grouping label | ADR 0077 Decisions 1, 2 and 6 | 004 | Implemented 2026-09-26. Mechanical checks Green. Visual gate open and paused. Its name search reached only parked code |
| 006 | The live name-keyed artist page: `IndexArtistCandidate` rows and `FrameNavigationEntry::IndexArtistFeedScope` show a name as an artist | ADR 0077 Decisions 1 and 6 | 005 | No task document yet |

Packet 001 added schema version 13, and packet 002 added schema version 14.

## Held Work

Packets 004 and 005 are implemented on 2026-09-26. Packet 006 needs a task document.

Stophammer deployed the album summary fields of its ADR 0059 on 2026-09-26, at commit `264706e`.
The [open Stophammer requests](v4vmm-open-requests.md#verification-by-v4vmm---2026-09-26) record the v4vmm verification.
Packet 003 reads the `remote_*` fields and sends one request for each publisher page.

## Follow-Up Findings

Packet 001 recorded these findings on 2026-09-24. No packet owns them yet.

- `api::Track.artist_credit` has no production reader. Only its decode and the ADR 0075 observation field list name it. MusicBrainz lookup reads its own response. The R1-09 premise of packet 001 was incorrect.
- `ApiSource::fetch_artist` always returns an error, because MusicIndex has no artist route. `ApiSource` has no constructor outside `src/sources.rs`.
- Packet 002 corrected the ADR 0075 evidence rule for `publisher`. Commit `82c3c06` treated the collection as an object. The live contract sends an array.
- A stored publisher relationship is never deleted. Packet 013 keeps MusicIndex replacement disabled until Stophammer states that the collection is complete.
- `SearchApp` in `src/discover.rs` is parked under `#![allow(dead_code)]`, and no composition root constructs it. AGENTS.md requires the deletion of code that no composition root reaches. Packet 005 found this on 2026-09-26. A decision must state which discover code stays live, because some discover screens are live.
- `src/audio_format/probe.rs` test `adr_0066_converter_failures_are_distinct_and_output_is_not_retained` failed once in a full run and passed when run again. It writes a script and runs it at once. A parallel run can cause this failure.

## Session Rules

Each implementation session owns one packet. It completes the packet, checks it, and stops.
A packet that changes user-visible behavior leaves its visual gate open in its `Status:` line and in
[pending human checks](../pending-human-checks.md).
