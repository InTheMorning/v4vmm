# ADR 0077 Publisher Artist Phase Plan

## Status

Active - 2026-09-24. This plan is advisory. It states no rule.
[ADR 0077](../adr/0077-publisher-feed-artist-binding.md), [ADR 0078](../adr/0078-publisher-page-type-from-stated-role.md)
and [ADR 0079](../adr/0079-remove-musicindex-artist-subject-storage.md) own the rules.

## Packet Register

| Packet | Scope | Owners | Depends on | State |
|---|---|---|---|---|
| [001](../tasks/adr-0077-task-001-remove-dead-artist-storage.md) | Delete the ADR 0045 binding and the ADR 0029 artist subject storage | ADR 0077 Decision 7, ADR 0079 | None | Implemented 2026-09-24. Mechanical checks Green. Visual gate moved to the [overhaul plan](design-and-cleanup-overhaul-plan.md#visual-requirements-moved-from-pending-checks---2026-10-07) on 2026-10-07 |
| [002](../tasks/archive/adr-0077-task-002-publisher-relationship-transport-and-storage.md) | Decode the publisher relationship and store it for each Library feed | ADR 0077 Decisions 2 and 5 | 001 | Implemented 2026-09-24. Mechanical checks Green. No visual gate |
| [003](../tasks/archive/adr-0077-task-003-publisher-page-view-model.md) | Publisher page query and view model | ADR 0077, ADR 0078 | 002, and Stophammer ADR 0059 | Implemented 2026-09-26. Mechanical checks Green. No visual gate |
| [004](../tasks/adr-0077-task-004-publisher-navigation-and-presentation.md) | Publisher page, navigation from an album and a track, and the Library album values | ADR 0077 Decisions 1 and 2, ADR 0078 | 003 | Implemented 2026-09-26. Mechanical checks Green. Visual gate moved to the [overhaul plan](design-and-cleanup-overhaul-plan.md#visual-requirements-moved-from-pending-checks---2026-10-07) on 2026-10-07 |
| [005](../tasks/adr-0077-task-005-feed-owner-text-and-name-search.md) | Feed owner text, removal of the `publisher_text` inspector, the name search and the name grouping label | ADR 0077 Decisions 1, 2 and 6 | 004 | Implemented 2026-09-26. Mechanical checks Green. Visual gate moved to the [overhaul plan](design-and-cleanup-overhaul-plan.md#visual-requirements-moved-from-pending-checks---2026-10-07) on 2026-10-07. Its name search reached only parked code |
| [006](../tasks/adr-0077-task-006-name-matches-are-search-results.md) | The live name match: an `IndexArtistCandidate` row becomes `Tracks matching "<name>"`, and its page lists the tracks of `/v1/tracks?artist=<name>` | ADR 0077 Decision 1 and the refinement "Index artist page by name" | 005 | Implemented 2026-09-29. Mechanical checks Green. Visual gate moved to the [overhaul plan](design-and-cleanup-overhaul-plan.md#visual-requirements-moved-from-pending-checks---2026-10-07) on 2026-10-07 |
| [007](../tasks/adr-0077-task-007-confirmed-and-unconfirmed-artists.md) | Show the confirmed and unconfirmed artists of a publisher page apart, with the four fields of Stophammer ADR 0061 | ADR 0077, Stophammer ADR 0061 | 004, and the deploy of Stophammer 0.2.0 | Implemented 2026-09-30. Mechanical checks Green. Visual gate moved to the [overhaul plan](design-and-cleanup-overhaul-plan.md#visual-requirements-moved-from-pending-checks---2026-10-07) on 2026-10-07 |

Packet 001 added schema version 13, and packet 002 added schema version 14.

## Held Work

Packets 004 and 005 are implemented on 2026-09-26. Packet 006 is implemented on 2026-09-29.

Packet 007 waits for Stophammer 0.2.0. Stophammer ADR 0061 is Accepted on 2026-09-27. It adds `confirmed_release_artists`, `confirmed_release_artist_count`, `unconfirmed_release_artists` and `unconfirmed_release_artist_count` to a publisher feed read.
On 2026-09-29, the deployed contract is version 0.2.0 and declares the four fields. The packet can be written.

The header fact "Artists" reads `distinct_release_artist_count`. That field counts only the albums that name the publisher.
A publisher that lists albums that do not name it shows 0 artists above its "listed by" group. Packet 007 corrects this.

Stophammer ADR 0061 §4 and §5 agree with the present code. The index gives no artist or label kind, and an album read gives only the publishers that the album names.

Stophammer deployed the album summary fields of its ADR 0059 on 2026-09-26, at commit `264706e`.
The [open Stophammer requests](v4vmm-open-requests.md#verification-by-v4vmm---2026-09-26) record the v4vmm verification.
Packet 003 reads the `remote_*` fields and sends one request for each publisher page.

## Follow-Up Findings

Packet 001 recorded these findings on 2026-09-24. No packet owns them yet.

- `api::Track.artist_credit` has no production reader. Only its decode and the ADR 0075 observation field list name it. MusicBrainz lookup reads its own response. The R1-09 premise of packet 001 was incorrect.
- `ApiSource::fetch_artist` always returns an error, because MusicIndex has no artist route. `ApiSource` has no constructor outside `src/sources.rs`.
- Packet 002 corrected the ADR 0075 evidence rule for `publisher`. Commit `82c3c06` treated the collection as an object. The live contract sends an array.
- A stored publisher relationship is never deleted. Packet 013 keeps MusicIndex replacement disabled until Stophammer states that the collection is complete.
- [ADR 0060 packet 005](../tasks/archive/adr-0060-task-005-delete-parked-discover-code.md) owns this finding, Ready on 2026-09-30. `SearchApp` in `src/discover.rs` is parked under `#![allow(dead_code)]`, and no composition root constructs it. AGENTS.md requires the deletion of code that no composition root reaches. Packet 005 found this on 2026-09-26. A decision must state which discover code stays live, because some discover screens are live.
- `src/audio_format/probe.rs` test `adr_0066_converter_failures_are_distinct_and_output_is_not_retained` failed once in a full run and passed when run again. It writes a script and runs it at once. A parallel run can cause this failure.

## Session Rules

Each implementation session owns one packet. It completes the packet, checks it, and stops.
A packet that changes user-visible behavior leaves its visual gate open in its `Status:` line and in
[pending human checks](../pending-human-checks.md).
