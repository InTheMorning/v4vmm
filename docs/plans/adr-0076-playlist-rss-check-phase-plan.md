# ADR 0076 Playlist RSS Check Phase Plan

## Status

Active - 2026-09-24. This plan is advisory. It states no rule.
[ADR 0076](../adr/0076-playlist-rss-check-for-stale-musicindex-records.md) owns the rules.
The operator accepted it on 2026-09-24, with its three numeric values.

## Packet Register

Dispatch the packets in this sequence. Each one needs the packets before it.

| Packet | Scope | Owners | Depends on | Schema | State |
|---|---|---|---|---|---|
| [001](../tasks/adr-0076-task-001-playlist-rss-document-check.md) | The playlist check actor: host pacing, conditional GET, HTTP 429 stop, the "Check RSS" command and its progress | ADR 0076 Decision 2, the accepted values | ADR 0077 packet 002 | Version 15 | Implemented 2026-09-24. Mechanical checks Green. Visual gate open and paused |
| [002](../tasks/adr-0076-task-002-rss-comparison-apply-and-report.md) | The comparison of each element, the automatic apply, the hold, new and removed tracks, and the report | ADR 0076 Decisions 3 to 7 | 001 | Version 16 | Implemented 2026-09-24. Mechanical checks Green. Visual gate open and paused |
| [020](../tasks/adr-0075-task-020-stored-value-projection.md) | One shared projection of the stored values with their owners, for views and for tag frames | ADR 0076 Decision 1, ADR 0075 | 002 | None | Implemented 2026-09-24. Mechanical checks Green. Visual gate open and paused |
| [003](../tasks/adr-0076-task-003-stored-payment-route-and-readiness.md) | The stored payment route as the source of each file write, and the two new readiness states | ADR 0076 Decisions 7 and 9, ADRs 0059 and 0065 | 002, 020 | Version 17 | Implemented 2026-09-24, with the removal actions of the operator decision. Mechanical checks Green. Visual gate open and paused |
| [004](../tasks/adr-0076-task-004-tag-update-confirmation.md) | The "Update n file(s)" button, its popup, the in-use rule, and the retained count | ADR 0076 Decision 8 | 003 | None | Implemented 2026-09-24. Mechanical checks Green. Visual gate open and paused |
| [005](../tasks/adr-0076-task-005-check-and-scan-follow-ups.md) | The 60-second `Retry-After` limit, "Copy feed URL", two scan triggers, and the in-place update of a playlist row after Confirm | ADR 0076 Decisions 2, 4, 7 and 8 | 004 | None | Implemented 2026-09-25. Mechanical checks Green. Visual gate open and paused |
| [006](../tasks/adr-0076-task-006-credit-list-projection.md) | One credit list for each owner, chosen by the projection rule | ADR 0076 Decisions 1 and 5 | 005 | None | Implemented 2026-09-25. Mechanical checks Green. Visual gate open and paused |
| [007](../tasks/adr-0076-task-007-guard-reads-test-files-as-test-code.md) | The guard helper reads a test-only file as test code | ADR 0076 Decision 9 guard | None | None | Implemented 2026-09-30. Mechanical checks Green. No visual gate |
| [008](../tasks/adr-0076-task-008-confirmation-list-with-many-items.md) | Each confirmation item keeps its height when the list is taller than its column. The operator found an empty "Update 34 files" popup on 2026-10-03 | ADR 0076 Decision 8 | None | None | Implemented 2026-10-03. Mechanical checks Green. Visual gate open |
| [009](../tasks/adr-0076-task-009-download-writes-stored-values.md) | A download writes the stored values after the RSS update, and the RSS check keeps both copies of the channel link equal | ADR 0076 Decision 10 | ADR 0066 task 014 | None | Ready 2026-10-03 |

Packet 020 keeps its ADR 0075 number. ADR 0076 reduced it to a projection with no source selection.

## Sequence Reasons

- Packet 001 fetches and records. It compares nothing, so its report names only fetch outcomes.
- Packet 002 writes the stored slots. Some views still read a MusicIndex fact before the slot until packet 020 corrects the projection.
  The report is correct from packet 002. The screens follow in packet 020.
- Packet 003 needs the removed mark of packet 002 and the projection of packet 020 for the route frame.
- Packet 004 compares file tags against the projection of packet 020 and the stored route of packet 003.

## Trigger Mapping

ADR 0076 Decision 2 runs the check when the operator selects a playlist for a show.
The app has no Show playlist selection today. The only command that takes a playlist into playback is `PlayPlaylistAt`.
Packet 001 uses that command as the automatic trigger. Playback does not wait for the check.

[ADR 0068](../adr/0068-show-cue-and-audition-isolation.md) is Proposed. When the operator accepts it, the trigger moves to the cue load.
The orchestrator selected this mapping on 2026-09-24. The operator can change it.

## Operator Details

The operator decided three details on 2026-09-24:

- A `304 Not Modified` response is an observation with outcome `success` and no body. Packet 001.
- A file route differs from the stored route when any recipient field differs, the name included. Packet 003.
- The channel artist text gets its own column, `feeds.album_artist`. Packet 002 adds it, and packet 020 projects it.

The operator decided four more details on 2026-09-25:

- The `Retry-After` wait has a limit of 60 seconds. A longer value stops the host for the check. ADR 0076 records the limit. Packet 005.
- The podping.me link stays, and a "Copy feed URL" action is added. Packet 005.
- The tag scan also runs when a session stops and after a search download. No scan button. Packet 005.
- Each owner shows one credit list, chosen by the projection rule. Packet 006.

No detail is open.

## Follow-Up Findings

Packet 001 recorded these findings on 2026-09-24. No packet owns them yet.

- The check starts on each `PlayPlaylistAt` event, also when the player is unavailable. A playback retry during recovery starts no check.
- The run rows cascade on the deletion of a playlist or a feed.

Packet 002 recorded these findings on 2026-09-24:

- MusicIndex `updated_at` is in seconds. Stophammer sets it on each ingest with a changed content hash (`src/api.rs:1866` at commit `a03c9ea`). Any body change releases a hold, also when no compared field changed.
- The first check of a feed can report fields that MusicIndex never supplied, because the check compares against the current projected value.
- `html5ever` and `markup5ever_rcdom` are now direct dependencies, for the readable-text comparison. Both were in `Cargo.lock` before.
- The Download action of an added track does not show that a download already ran.

Packet 020 recorded these findings on 2026-09-24:

- A track's album title now comes from the channel title and its hold. The track copy is the fallback. The operator can reject this at the packet 020 visual check.
- A subscribe records RSS values and holds like a check, with no difference rows. It deletes a stale hold on each column slot that it writes.
- `apply_feed_updates` writes audio tags from a MusicIndex response. Packet 004 removes that write.

Packet 003 recorded these findings on 2026-09-24:

- The operator decided the removal actions on 2026-09-24. The readiness row has Confirm and "Remove from library". Each playlist row has "Remove from playlist" and "Remove from all playlists".
- A Confirm in the readiness list now clears the playlist row error in place. Packet 005 corrected it on 2026-09-25.
- The Discover tag apply can write a file with no track row. It still writes the MusicIndex route, because no stored route exists for that file.
- After migration 17, a file with a MusicIndex route that differs from RSS in any field shows "Route out of date". The operator check shows how many files this affects.
- `RemovedFromFeed` comes before each other readiness state. The repair-all run skips removed and out-of-date tracks.
- The Show Source card changes at its next readiness scan, 5 minutes or less after a confirm.

Packet 004 recorded these findings on 2026-09-24:

- The scan does not compare `APIC`, `USLT` or `SYLT`. The tests cover MP3 files only.
- A feed update no longer writes audio tags. It no longer stores a MusicIndex route for a track that has none. The ADR 0065 route repair still does.
- The orchestrator corrected a race in `src/runtime/tag_update.rs` on 2026-09-24. After a write, the actor published one snapshot with neither `writing` nor `scanning` set, and the old scan count showed for a moment. The rescan now starts in the same snapshot that ends the write. The existing actor test failed one time in a full run before the correction, and it passed 20 times after it.

Packet 005 recorded these findings on 2026-09-25:

- A stop of a playback session starts a tag scan only when the playback polling actor runs, that is, with a live driver. The app has no Stop control. A session reaches `stopped` through the CLI, another process, or the ADR 0066 session finish.
- A stored run rebuilds its stopped hosts only for HTTP `429`. A `Retry-After` stop shows through the stored feed messages.
- `src/discover/app_impl.rs` has download handlers, and the session found no composition root that constructs `SearchApp`. A dead-code check of `src/discover/` is open. No packet owns it.

Packet 006 recorded these findings on 2026-09-25:

- A subscribe now holds the persons slot, so the RSS credit list wins over an older MusicIndex list.
- The Library track page shows the stored credit list, not the credits of the fetched MusicIndex response. The operator can reject this at the packet 006 visual check.
- The expected ID3 contributor frames use the projected list.

The orchestrator raised the wait limit of `broadcast_readiness_actor_refreshes_on_command` from 1 to 10 seconds on 2026-09-25. The test failed one time in a full parallel run. The limit only bounds a failing test.

## Session Rules

Each implementation session owns one packet. It completes the packet, checks it, and stops.
A packet that changes user-visible behavior leaves its visual gate open in its `Status:` line and in
[pending human checks](../pending-human-checks.md). Visual checks stay paused.
