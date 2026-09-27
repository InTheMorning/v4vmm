# Pending Human Checks

## Scheduling

The operator paused visual checks on 2026-09-19. Prioritise the
[metadata contract refactor](plans/adr-0075-metadata-contract-phase-plan.md)
before requesting more visual checks. The first four groups retain five open visual packets.
Their acceptance and configuration gates remain open. Group 5 records the remaining metadata document review.
Groups 6 and 7 record the metadata migration and storage-failure presentation gates.

Group 8 records the Library comparison and hydration presentation gate.
Group 9 records the feed check and feed update presentation gate.
Group 12 records the Library artist view gate of the ADR 0079 artist storage removal.

Group 13 records the playlist RSS check gate of ADR 0076 packet 001.
Group 14 records the RSS comparison and report gate of ADR 0076 packet 002.
Group 15 records the stored value projection gate of ADR 0075 packet 020.
Group 16 records the stored payment route and readiness gate of ADR 0076 packet 003.
Group 17 records the tag update confirmation gate of ADR 0076 packet 004.
Group 18 records the check and scan follow-up gate of ADR 0076 packet 005.

Group 19 records the credit list gate of ADR 0076 packet 006.

The operator accepted ADR 0075 on 2026-09-19. Its placement decision adds future
visual checks for the labelled identity sections on a track page. Phase 005 owns
those checks. Do not request them during this pause.

The current evidence fixture is `/tmp/v4vmm-governance.ie6k8TQf`.
Its cleanup remains unconfirmed. No app launch is requested during this pause.

## Purpose

Some acceptance criteria need a person. An agent must not run this app, because
GPUI does not start an X11 client in an agent session.

This file holds the checks that are open today. It is not a history. When a
check passes, do all three in the same change:

1. Record it in the `Status:` line of the owning packet in `docs/tasks/`.
2. Record it in the row in `docs/plans/broadcast-chain-delivery-order.md`.
3. Remove the section from this file, and number the sections that stay, so the
   order is still `1` to `n`.

When no check is open, this file keeps the function and the method sections
only. A closed check leaves no entry here.

## 1. Music And Settings Scrolling — ADR 0030 Task 006

Open - reconciled 2026-09-10. The earlier task review records residual manual
verification. Separate Discovery and Recent Feeds paths are retired under
ADRs 0047/0048/0060/0062; bounded scrolling survives on current Music details
and Settings.

- Owner: [task 006](tasks/adr-0030-task-006-scroll-containers.md).
- Check: [Scroll Containers](runbooks/inherited-ui-checks.md#scroll-containers--adr-0030-task-006).
- Needs overflowing artist, release, playlist, track, Index detail, and
  Settings content. Verify wheel, scrollbar, and supported keyboard scrolling
  in Light and Dark. Missing overflowing content leaves that subcheck open.

## 2. Track Identity And Detail Parity — ADR 0037 Task 002

Open - reconciled 2026-09-10. Compare the same entity through local and Index
origins in Music. The separate Library/Discover screen requirement is retired
by ADRs 0047/0048/0060. Track identity and detail requirements survive.

- Owner: [task 002](tasks/adr-0037-task-002-track-header-action-parity.md).
- Check: [Identity And Detail Parity](runbooks/inherited-ui-checks.md#identity-and-detail-parity--adr-0037-tasks-001-and-002).
- Use the same track with known Website/Nostr facts and a downloaded local copy.
  Check both origins in Light and Dark. Compare headers, actions, section order
  and contextual disclosure. Empty source facts do not close the gate.
- Record track results and fixture cleanup in the
  [checklist](reviews/adr-0037-review-checklist.md).
- The operator confirmed the private copy at
  `/tmp/v4vmm-governance.ie6k8TQf` on 2026-09-19. Track selection and source-fact
  checks remain open. All 79 library tracks have feed GUIDs and copied audio.
  None has a stored track Website fact. Nine have Nostr facts. All 79 Index
  lookups succeeded, with no Website or Nostr candidates counted. A
  `website`-only check can miss `web_page` links. A follow-up Index request
  confirmed MoeFactz contributor claims with Nostr and website evidence.
  Source inspection found app request and presentation gaps for contributor
  facts. Keep those facts separate from track header identity. No visual
  acceptance or cleanup is recorded yet.

## 3. Stored Metadata In Details — ADR 0054 Tasks 004 And 005

Open - reconciled 2026-09-10. The task reviews require operator inspection;
the guard-only task 006 supplied no visual closure. ADR 0054 and its phase
plan are Accepted, with implementation recorded and visual acceptance open.

- Owners: [task 004](tasks/adr-0054-task-004-feed-read-model-hydration.md)
  and [task 005](tasks/adr-0054-task-005-track-read-model-hydration.md).
- Check: [Metadata Hydration](runbooks/inherited-ui-checks.md#metadata-hydration--adr-0054-tasks-004-and-005).
- Needs known persisted feed/track metadata, reachable Index for comparison,
  and an unavailable endpoint for local fallback. Use the disposable config.
  Record feed and track results separately in the
  [checklist](reviews/adr-0054-review-checklist.md), in both themes.

The three inherited groups above use the runbook's private database/audio copy and cleanup.
The numbers group checks. They do not change the approved delivery priority.

## 4. Optional Tool Isolation — ADR 0066 Task 004

Open - implementation and mechanical checks recorded 2026-09-11.

- Owner: [task 004](tasks/adr-0066-task-004-optional-tool-isolation.md).
- Check: [Optional Tool Isolation](runbooks/startup-recovery-check.md#task-004-optional-tool-isolation).
- Needs a Linux desktop, Python 3.11+, this checkout's debug binary, installed
  mpv and working desktop audio for the producer-failure case. The fixture
  supplies local tracks, broken paths and external-service stubs.
- For paired Index/player failure, check local search, playlist visibility and
  retained reports. Check that Play and Ctrl+Alt+P cannot execute playback from Show.
  Task 007 supplies enabled repair routes while retaining the execution restriction.
- Check producer failure with audible playback. Check publisher failure with
  an independent producer and encoder. Check partially applied path repair.
- Check report copy and preservation for the publisher and path-repair cases.
  Record final fixture cleanup. Producer preservation passed on 2026-09-11.
  Its permitted workspace preference changes do not establish playback acceptance.
- Playback checks remain paused. The operator requires cue loading and playback
  from Show, with other Play buttons using a separate audition path.
  Current Music Play shares the Show session. The producer screenshot also
  reports an unresolved mpv IPC read error. Report-only checks cannot establish
  audio or publication behavior. See [task 004's correction](tasks/adr-0066-task-004-optional-tool-isolation.md#playback-workflow-correction--2026-09-11).
- [ADR 0068](adr/0068-show-cue-and-audition-isolation.md) proposes that separation.
  Its visual/audio check cannot run before implementation. The proposal closes
  no gate and reopens no accepted case.
- Task 005's completion closes none of this packet's remaining checks.
  The phase plan retains its scheduling exception. The inherited checks above
  remain separate.

## Method: Reach A Publisher Service State

This section is not a check. It is the method that each publisher check needs.
It stays here when every check above is closed.

A temporary drop-in file makes the unit enter a test state. The cleanup removes
only the named test files. Before each setup, check whether `zz-force-fail.conf`
already exists at that path. If it exists, stop. Do not overwrite an existing drop-in.

| State | How to reach it |
|---|---|
| `Active` | override `ExecStart=/bin/sleep infinity` |
| `Inactive` | `systemctl --user stop <unit>` |
| `Starting` | override `ExecStartPre=/bin/sleep 60`, then look while it starts |
| `Stopping` | stop a unit that takes time to shut down |
| `Failed` | see the two variants below |
| `NotInstalled` | the unit files are not linked into `~/.config/systemd/user/` |
| `Unknown` | no normal operation reaches this state |

### Failed With A Start Limit

A revoked token drives the unit to `failed` with `Result=start-limit-hit`. The
unit file sets `StartLimitBurst=5` and `RestartSec=5s`. Systemd starts the unit
five times, then stops. The unit sits in `Starting` between attempts.

```bash
mkdir -p ~/.config/systemd/user/musicindex-live-publisher@mixxx.service.d
printf '[Service]\nExecStart=\nExecStart=/bin/false\n' \
  > ~/.config/systemd/user/musicindex-live-publisher@mixxx.service.d/zz-force-fail.conf
systemctl --user daemon-reload
systemctl --user start musicindex-live-publisher@mixxx.service
```

Wait about 25 seconds.

### Failed Immediately

Use this alternative when you need an immediate failure and do not need the
start limit. `Restart=no` stops the restart loop.

```bash
mkdir -p ~/.config/systemd/user/mixxx-now-playing.service.d
printf '[Service]\nExecStart=\nExecStart=/bin/false\nRestart=no\n' \
  > ~/.config/systemd/user/mixxx-now-playing.service.d/zz-force-fail.conf
systemctl --user daemon-reload
systemctl --user start mixxx-now-playing.service
systemctl --user show mixxx-now-playing.service \
  --property=LoadState,ActiveState,SubState,Result
```

The result is `ActiveState=failed` and `Result=exit-code`.

### Cleanup

Use only the cleanup for the method you ran. The methods do not change the
app configuration. They require no app configuration restore. Other drop-ins
must remain intact.

For **Failed With A Start Limit**:

```bash
rm -f -- ~/.config/systemd/user/musicindex-live-publisher@mixxx.service.d/zz-force-fail.conf
systemctl --user daemon-reload
systemctl --user reset-failed musicindex-live-publisher@mixxx.service
```

For **Failed Immediately**:

```bash
rm -f -- ~/.config/systemd/user/mixxx-now-playing.service.d/zz-force-fail.conf
systemctl --user daemon-reload
systemctl --user reset-failed mixxx-now-playing.service
```

## References

- `docs/adr/0061-executable-governance.md`, for the mechanical and visual rule
- `docs/plans/broadcast-chain-delivery-order.md`

## 5. Metadata Contract Document Review — ADR 0075

Closed for the deferred policies - 2026-09-21. The operator deferred the nine remaining
field policies in packets 031 and 034. ADR 0075 Decision A, amended, records the reduced
scope. The accepted field decisions stay recorded below. This group requests no visual
check or app launch.

- Owners: document packets 002–008, 031, 034, and 035 in the [phase plan](plans/adr-0075-metadata-contract-phase-plan.md).

- Accepted policy: Decisions D–H and the individual field decisions in [ADR 0075](adr/0075-metadata-ownership-and-completeness.md#status).
  Description, website, page, artwork, publisher, artist, and language policies have individual acceptance.
  Feed and proven track explicit-state source priority, removal, conflict, and stale-state rules are accepted.
  Known feed state can appear separately when track explicit state is unknown.

- Legacy feed explicit state: MusicIndex `false` without evidence of a valid clean marker stays unknown, with its evidence retained.
- Legacy track explicit state: MusicIndex booleans with unknown ownership stay in source details without declaring a track state.
- Feed publication-date source priority: prefer valid fresh RSS channel `pubDate`, then MusicIndex claims with actual channel publication evidence.
- Feed publication-date refinements: apply accepted removal, conflict, and stale-state rules. Retain original date text, source evidence, and selected absence.
- Feed publication-date precision: show valid partial dates at their supplied precision without invented date parts or timezones.
- Feed publication timestamp display: show UTC for known instants. Metadata details retain the source timezone and original text.
- Track publication-date source priority: prefer valid fresh item RSS `pubDate`, then MusicIndex claims with track publication evidence.
- Track publication-date refinements: apply the feed date's removal, conflict, and stale-state rules. Retain original text, source evidence, and selected absence.
- Track publication-date precision: apply the feed precision rule. Show valid partial dates without invented parts or timezones.
- Track publication timestamp display: show UTC for known instants. Metadata details retain the source timezone and original text.
- Track publication-date fallback: show the available feed date separately as "Feed publication date" when the track date is absent.
- Feed release-date evidence: require direct release-date evidence. Keep publication, build, and oldest-item dates separate, with derivation evidence retained.
- Feed release-date refinements: apply the publication-date removal, conflict, and stale-state rules. Retain original text, source evidence, and selected absence.
- Feed release-date source priority: prefer fresh supported RSS assertions, then MusicIndex assertions, with proof of an actual release date.
- Feed release-date precision: preserve year-only and year-month precision without inventing a missing day or time.
- Feed release timestamp display: show UTC when the source timezone is known. Retain original text and source timezone in metadata details.
- Track release dates: require direct evidence and apply the feed release-date source, removal, conflict, and stale-state rules.
- Track release-date precision: apply the feed precision rule without invented date parts. Retain original text and source precision.
- Track release timestamp display: show UTC when the source timezone is known. Retain original text and source timezone in metadata details.
- Track release-date fallback: show the available feed release date separately as "Feed release date" when the track release date is absent.
- Date format interpretation: require unambiguous formats with known source rules for all four date fields. Retain other text as unresolved evidence.
- Track duration metadata: prefer fresh valid RSS iTunes duration, then MusicIndex. Keep measured file duration separate.
- Track duration refinements: apply accepted removal, conflict, and stale-state rules. Retain original text, source evidence, and selected absence.
- Track duration precision: retain valid fractional seconds at source precision without rounding stored values to whole seconds.
- Track duration validation: accept explicitly supplied zero. Reject negative and malformed durations while retaining source evidence.
- RSS duration formats: accept seconds, `MM:SS`, and `HH:MM:SS`, with fractional seconds and valid component ranges.
- Measured duration presentation: when duration metadata is absent, show available measured file duration separately as "File duration".
- Track transcript source priority: prefer fresh direct RSS claims over MusicIndex. Retain all candidates and source evidence.
- Track transcript refinements: apply the description fields' removal, source-order, conflict, and stale-state rules, including retained selected absence.
- MusicIndex transcript representation: prefer full transcript claims over legacy transcript links. Retain both forms of evidence.
- Legacy transcript recognition: require explicit transcript, caption, or subtitle evidence. Filename-only matches remain unresolved evidence.
- Transcript alternatives: offer different languages and formats with their declared labels. Retain each alternative's source evidence.
- Legacy transcript ownership: retain unknown ownership in source details only, without active track transcript actions.
- Transcript URL actions: allow only valid HTTP or HTTPS URLs. Retain other URLs as source evidence without a transcript action.
- Feed website actions: allow only valid HTTP or HTTPS URLs. Retain other schemes as source evidence without a website action.
- Track page actions: allow only valid HTTP or HTTPS URLs. Retain other schemes as source evidence without a page action.
- Feed website comparison: normalize scheme, host, and default ports. Preserve path, query, and fragment differences.
- Track page comparison: apply the feed website normalization while preserving path, query, and fragment differences.
- Feed title source priority: prefer fresh direct RSS titles over MusicIndex. Retain both source assertions and original evidence.
- Feed title refinements: apply accepted removal, conflict, and stale-state rules. Retain original title text, source evidence, and selected absence.
- MusicIndex feed title representation: prefer `title`, then legacy `name`. Retain both values and their field paths.
- Missing feed title presentation: keep "Unknown Feed" as a display label only, without storing it as source metadata.
- Feed title placeholders: hide only confirmed generated placeholders. Retain literal publisher-supplied titles and derivation evidence.
- Track title rules: apply the feed title source priority, title/name order, removal, conflict, stale-state, and placeholder rules.
- Missing track title presentation: display its GUID, then "Untitled" when no GUID exists, without creating source metadata.
- Feed-title reference fallback: allow a track response's `feed_title` as a labeled feed reference without a separately selected feed title.
- Feed-title reference removal: verified feed-title removal hides the reference while retaining its evidence.
- Feed-title reference refinements: apply the feed title's conflict, stale-state, and generated-placeholder rules.

- Remaining field review: deferred on 2026-09-21. Packets 031 and 034 keep their written
  proposals. A deferred field follows the general source rule in ADR 0075 Decision I.
- Check source claims against the named functions. Check proposed rules against the accepted ADR and constructed examples.
- The [field inventory](schema/adr-0075-metadata-field-inventory.md) assigns additional rules to packets 031 and 034.
  Those rules are deferred. Packet 035's comparison and URL action rules have individual acceptance.
- The operator authorized completion orchestration on 2026-09-20. Accepted-rule code can proceed after technical review.
- New product policies still need operator acceptance. Document completion does not close that gate.
- This document review creates no fixture and requires no cleanup. The existing evidence fixture remains unmodified.

## 6. Metadata Migration Repair Report And Readiness — ADR 0075 Task 012

Open and paused - implementation, technical review, and mechanical checks are complete on 2026-09-20. Operator inspection is pending.

- Owner: [packet 012](tasks/adr-0075-task-012-provider-snapshot-migration.md#operator-visual-check).
- Check: separate version-11 repair and version-12 readiness reports, retained backup paths, recorded times, report copy, and normal/narrow presentation.
- Run the packet's temporary-fixture procedure only after mechanical review and the operator resumes visual checks.
- Preserve configuration, audio files, secret files, playlists, source records, and the selected event during the fixture check.
- Retain a failing fixture. Confirm successful preservation and fixture cleanup before closing this gate.

## 7. Metadata Observation Storage Failure — ADR 0075 Task 014

Open and paused - implementation, technical review, and mechanical checks are complete on 2026-09-21. Operator inspection is pending.

- Owner: [packet 014](tasks/adr-0075-task-014-provider-observation-retention.md#operator-visual-check).
- Check: the existing Library status identifies a metadata storage failure without claiming MusicIndex unavailability or successful persistence.
- Verify delayed failure reporting after navigation without changing the selected track's metadata or unrelated Library state.
- [Packet 013](tasks/adr-0075-task-013-verified-snapshot-replacement.md) passed technical and mechanical review. Its provider-state reads preserve this open presentation gate.
- The packet supplies exact temporary-fixture setup, failure injection, launch, inspection, and cleanup commands.
- Run that procedure only after technical review and the operator resumes visual checks.
- Use disposable data and scripted local services. Confirm fixture cleanup before closing this gate.

## 8. Library Comparison And Hydration Storage Failure — ADR 0075 Task 038

Open and paused - implementation, technical review, and mechanical checks are complete on 2026-09-21. Operator inspection is pending.

- Owner: [packet 038](tasks/adr-0075-task-038-library-reader-observation-retention.md#operator-visual-check).
- Check: ordinary comparison errors stay in their panel. Ordinary hydration errors remain silent.
- Verify that storage failures remain reported after navigation without changing selection or unrelated Library state.
- The packet supplies prospective isolated setup, failure injection, desktop inspection, and cleanup commands.
- Run the procedure only after technical and mechanical review and after the operator resumes visual checks.
- Confirm fixture preservation and cleanup before closing this gate.

## 9. Feed Check And Feed Update Storage Failure — ADR 0075 Task 039

Open and paused - implementation, technical review, and mechanical checks are complete on 2026-09-21. Operator inspection is pending.

- Owner: [packet 039](tasks/adr-0075-task-039-feed-check-and-update-observation-retention.md#operator-visual-check).
- Check: an ordinary feed error keeps its existing per-feed message and placement.
- Verify that a storage failure ends only the current feed operation and claims no successful persistence.
- Verify that **Check all feeds** keeps its existing results, controls, and placement.
- The packet supplies prospective isolated setup, failure injection, desktop inspection, and cleanup commands.
- Run the procedure only after the operator resumes visual checks.
- Confirm fixture preservation and cleanup before closing this gate.

## 10. Request Reuse And Freshness Policies — ADR 0075 Task 018

Closed - the operator decided all seven policies on 2026-09-21, and two more on 2026-09-22. This group requested no visual
check and no app launch. The implementation and its checks stay with the packet.

- Owner: [packet 018](tasks/adr-0075-task-018-request-reuse-and-freshness.md#accepted-policies---2026-09-21).
- P18-1: reuse a successful Library track detail response for 30 minutes.
- P18-2: reuse a successful feed response for 15 minutes, for each distinct include list.
- P18-3: reuse a parsed RSS document for 15 minutes, keyed by its feed URL.
- P18-4: never reuse a failed request.
- P18-5: hold at most 64 feed responses, 256 track responses, and 32 RSS documents, and remove the least recently used entry first.
- P18-6: hold reused responses in memory only, so a restart clears them.
- P18-7: an explicit refresh removes every entry of the named feed and its tracks.
- P18-8: an Index track detail response gets no reuse window.
- P18-9: a reused response replays the receipt of the fetch that produced it.
- The 30-minute track window depends on the existing check-for-updates control.
- The 15-minute RSS window can delay stale-MusicIndex detection by 15 minutes during passive browsing.

## 11. MusicIndex API Change Request — ADR 0075

Open - the operator sent the request on 2026-09-22, and the fixes are live on 2026-09-23.
Changes 1, 2, and 3 are verified against the deployed API. Two questions stay open, and each
landed change still needs its own packet. This group needs no visual check and no app
launch.

- Owner: the [API change request](plans/musicindex-api-change-request.md).
- Change 1: return summary fields with search results.
- Change 2: return track and feed artwork as separate fields.
- Change 3: record which element supplied a feed publication date.
- Change 4: never rename a response field without a version.
- The request also asks which revision is deployed. The inspected revision is `a220f44` in a local checkout.
- Changes 1, 2, and 3 are live, and the [verification](plans/musicindex-api-change-request.md#verification-against-the-deployed-api) records the evidence.
- Change 4 is a release policy. No external check can prove it.
- Open question: the new `last_build_date` claim type has no accepted field rule in this app.
- Open question: `Feed.name`, `Track.name`, `Track.artist_credit`, and `Track.feed_url` no longer arrive. Ask Stophammer whether they were removed, renamed, or null in every sampled row.
- The deployed revision stays unconfirmed. The published contract declares a static version string.
- This client implements none of the three landed changes. Each one needs its own packet.
- The [answer table](plans/musicindex-api-change-request.md#what-each-answer-changes-here) records the work that each landed change releases.

## 12. Library Artist View Without Artist Storage — ADR 0077 Task 001

Open and paused - implementation and mechanical checks are complete on 2026-09-24. Operator inspection is pending.

- Owner: [packet 001](tasks/adr-0077-task-001-remove-dead-artist-storage.md#operator-visual-check).
- Check: a Library artist view shows its tracks and albums, with no aliases, area, active years or source subjects.
- A missing track, an empty view, or an error report is wrong.
- Migration 13 deletes the stored artist rows. Make the SQLite backup in the packet procedure before the first run of the new build.
- Run the procedure only after the operator resumes visual checks.
- Keep the backup until acceptance. The packet gives the restore and cleanup commands.

## 13. Playlist RSS Check Button And Report — ADR 0076 Task 001

Open and paused - implementation and mechanical checks are complete on 2026-09-24. Operator inspection is pending.

- Owner: [packet 001](tasks/adr-0076-task-001-playlist-rss-document-check.md#operator-visual-check).
- V1: the playlist page shows the "Check RSS" button with its state, in Light and Dark themes.
- V2: during a check, the page shows the progress in place, without navigation. Playback from the playlist does not wait for the check.
- V3: after the check, the page shows each feed outcome and each stopped host in readable text.
- Each check sends real HTTP requests to the feed hosts of the playlist.
- Migration 15 adds two tables. Make the SQLite backup in the packet procedure before the first run of the new build.
- Run the procedure only after the operator resumes visual checks.
- Keep the backup until acceptance. The packet gives the restore and cleanup commands.

## 14. RSS Comparison, Apply And Report — ADR 0076 Task 002

Open and paused - implementation and mechanical checks are complete on 2026-09-24. Operator inspection is pending.

- Owner: [packet 002](tasks/adr-0076-task-002-rss-comparison-apply-and-report.md#operator-visual-check).
- V1: after a check with differences, the playlist page shows the report in place. Each row gives the feed, the field, the old value, the new value and the time. Each stale feed has a podping.me button. Check in Light and Dark themes.
- V2: a new track shows with its download action. A removed track shows its mark on the playlist row.
- V3: the report is readable at normal and narrow widths. Stacked text follows the column text rule.
- Each check sends real HTTP requests to the feed hosts of the playlist, and it writes RSS values into the stored values.
- Migration 16 drops three tables. Make the SQLite backup in the packet procedure before the first run of the new build.
- Run the procedure only after the operator resumes visual checks.
- Keep the backup until acceptance. The packet gives the fixture, restore and cleanup commands.

## 15. Stored Value Projection — ADR 0075 Task 020

Open and paused - implementation and mechanical checks are complete on 2026-09-24. Operator inspection is pending.

- Owner: [packet 020](tasks/adr-0075-task-020-stored-value-projection.md#operator-visual-check).
- V1: after a check applied a title change, the album page and the track page show the new title without navigation. Check in Light and Dark themes.
- The track page shows the channel title as the album name. A title with "(old)" after the check is wrong.
- The check sends real HTTP requests to the feed hosts of the playlist, and it writes RSS values into the stored values.
- This packet adds no migration. The procedure needs schema version 16 from packet 002.
- Run the procedure only after the operator resumes visual checks.

## 16. Stored Payment Route And Readiness — ADR 0076 Task 003

Open and paused - implementation and mechanical checks are complete on 2026-09-24. Operator inspection is pending.

- Owner: [packet 003](tasks/adr-0076-task-003-stored-payment-route-and-readiness.md#operator-visual-check).
- Scheduling: run the procedure only after the operator resumes visual checks. Run it after group 14, because it needs schema version 16 from packet 002.
- V1: the Music readiness list shows a "Route out of date" row with no button and a "Removed from feed" row with a **Confirm** button. Check in Light and Dark themes.
- V2: the Show Source card counts both states as not ready and names them in its detail.
- V3: **Confirm** on a removed track changes the row in place, without navigation.
- The readiness row of a removed track also has **Remove from library**. The procedure opens its ADR 0044 confirmation and cancels it, because a removal deletes the audio file.
- V4: a playlist row of a removed track shows a row error with **Remove from playlist** and **Remove from all playlists**. The second action lists each playlist in its confirmation. Each action changes the playlist in place.
- Migration 17 adds and fills two columns. Make the SQLite backup in the packet procedure before the first run of the new build.
- The fixture changes the database only. It changes no audio file. It adds two fixture playlists, and the cleanup deletes them.
- Keep the backup until acceptance. The packet gives the fixture, restore and cleanup commands.

## 17. Tag Update Confirmation — ADR 0076 Task 004

Open and paused - implementation and mechanical checks are complete on 2026-09-24. Operator inspection is pending.

- Owner: [packet 004](tasks/adr-0076-task-004-tag-update-confirmation.md#operator-visual-check).
- Scheduling: run the procedure only after the operator resumes visual checks. Run it after group 16, because it needs schema version 17 from packet 003.
- V1: the Music section shows the "Update n file(s)" button with the count. The popup lists each file with its title, album and frames. Check in Light and Dark themes.
- V2: a file in use by the show shows the mark "In use by the show". **Write Tags** does not write it. After the write, the button count equals the files in use.
- V3: the popup is readable at normal and narrow window widths. Stacked text clips and does not show only an ellipsis.
- This packet writes audio tags. Copy the fixture audio files before the first confirm, as the packet procedure tells.
- Keep the copies until acceptance. The packet gives the restore and cleanup commands.

## 18. Check And Scan Follow-Ups — ADR 0076 Task 005

Open and paused - implementation and mechanical checks are complete on 2026-09-25. Operator inspection is pending.

- Owner: [packet 005](tasks/adr-0076-task-005-check-and-scan-follow-ups.md#operator-visual-check).
- Scheduling: run the procedure only after the operator resumes visual checks. Run it after group 16, because it needs schema version 17 and the readiness list from packet 003.
- V1: the check report shows **Copy feed URL** adjacent to the **Open podping.me** button. The pasted text is the feed URL. Check in Light and Dark themes.
- V2: after a **Confirm** in the readiness list, the playlist page shows no error on that row. No check and no restart occur between the two steps.
- The check in V1 sends real HTTP requests to the feed hosts of the playlist, and it writes RSS values into the stored values.
- This packet adds no migration. Make the SQLite backup in the packet procedure before the first run.
- Keep the backup until acceptance. The packet gives the fixture, restore and cleanup commands.

## 19. Credit List Projection — ADR 0076 Task 006

Open and paused - implementation and mechanical checks are complete on 2026-09-25. Operator inspection is pending.

- Owner: [packet 006](tasks/adr-0076-task-006-credit-list-projection.md#operator-visual-check).
- Scheduling: run the procedure only after the operator resumes visual checks. It needs schema version 17 from packet 003.
- V1: after a check that changed the credits, the track page shows the RSS credits one time each, in RSS order, with no MusicIndex credit and no provider label. Check in Light and Dark themes.
- The Library track page now shows the stored credit list, also when the MusicIndex fetch succeeds. The operator can reject this at V1.
- The check sends real HTTP requests to the feed hosts of the playlist, and it writes RSS values into the stored values.
- This packet adds no migration, so it needs no backup. The packet gives the fixture and cleanup commands.

## 20. Publisher Page And Navigation — ADR 0077 Task 004

Open and paused - implementation and mechanical checks are complete on 2026-09-26. Operator inspection is pending.

- Owner: [packet 004](tasks/adr-0077-task-004-publisher-navigation-and-presentation.md#operator-visual-check).
- Scheduling: run the procedure only after the operator resumes visual checks. It needs a Library album whose album hydration stored a `music_to_publisher` row.
- V1: an album page opens its publisher page, with its title, its type and its albums, in Light and Dark themes.
- V2: a track page opens the publisher page of its album.
- V3: a Library publisher page shows the Library albums and the other albums as two groups. With MusicIndex unreachable, the Library group stays and the report comes first.
- V4: a stated role and an assumed role show different text. A "Not listed" album and a derived artist count are marked.
- V5: normal and narrow widths show each element in its place, with no clipped text.
- The Index track page shows no "Open publisher" action. The packet records this as a deviation.
- V3 changes the MusicIndex endpoint in Settings. Restore it after the check.

## 21. Feed Owner Text And Name Grouping — ADR 0077 Task 005

Open and paused - implementation and mechanical checks are complete on 2026-09-26. Operator inspection is pending.

- Owner: [packet 005](tasks/adr-0077-task-005-feed-owner-text-and-name-search.md#operator-visual-check).
- Scheduling: run the procedure only after the operator resumes visual checks.
- V1: an album page shows `publisher_text` as "Feed owner" text that opens nothing, in Light and Dark themes.
- V3: a Library artist page without a publisher relationship shows "Grouped by name".
- V4: normal and narrow widths show each element in its place, with no clipped text.
- V2 cannot be walked. The name search of this packet reached only parked code. ADR 0077 packet 006 owns the live name-keyed artist page.
