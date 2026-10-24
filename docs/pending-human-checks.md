# Pending Human Checks

## Scheduling

The operator resumed visual checks on 2026-10-02. Walk a new check right after its packet.
The older checks below stay open until a person walks them. The operator walked part of a visual batch on 2026-10-02 and stopped after its Part A step 8.

The current evidence fixture is `/tmp/v4vmm-governance.ie6k8TQf`. Its cleanup remains unconfirmed.

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

## 1. Optional Tool Isolation — ADR 0066 Task 004

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
  The phase plan retains its scheduling exception.
- The three groups that used to precede this packet moved to the [overhaul plan](plans/design-and-cleanup-overhaul-plan.md#visual-requirements-moved-from-pending-checks---2026-10-07) on 2026-10-07.

## 2. Final Walk Of The Run Of 2026-10-20

Open - the run closed on 2026-10-23. The Walk Log of the run record shows the checks that the operator passed. Each other check of these packets stays open until a person walks it.

- Owner: the [run record](reviews/orchestrated-run-2026-10-20.md#final-walk).
- The packets of the run. Each item names its packet and its open checks:
  - [ADR 0084 packet 021](tasks/adr-0084-task-021-one-release-page-assembly.md) V1 to V3
  - [ADR 0084 packet 022](tasks/adr-0084-task-022-download-queue.md) V1 to V6
  - [ADR 0084 packet 025](tasks/adr-0084-task-025-playlist-selection.md) V1 to V7
  - [ADR 0084 packet 011](tasks/adr-0084-task-011-logo-frame.md) V1 to V7
  - [ADR 0084 packet 024](tasks/adr-0084-task-024-artist-feed-image.md) V1 to V3
  - [ADR 0083 task 006](tasks/adr-0083-task-006-row-hover-actions.md) V1 to V12
  - [ADR 0084 packet 027](tasks/adr-0084-task-027-stop-queued-downloads.md) V1 to V3
  - [ADR 0084 packet 023](tasks/adr-0084-task-023-one-track-page-assembly.md) V1 to V5
  - [ADR 0083 task 007](tasks/adr-0083-task-007-list-and-playlist-row-actions.md) V1 to V11
  - [ADR 0083 task 008](tasks/adr-0083-task-008-drag-track-to-playlist.md) V1 to V9
  - [ADR 0084 packet 016](tasks/adr-0084-task-016-library-and-everything-scopes.md) V1 to V8
  - [ADR 0084 packet 017](tasks/adr-0084-task-017-name-level-artist-page.md) V1 to V11
  - [ADR 0084 packet 010](tasks/adr-0084-task-010-search-results-list-column.md) V1 to V7
  - [ADR 0084 packet 012](tasks/adr-0084-task-012-saved-searches.md) V1 to V7
  - [ADR 0084 packet 028](tasks/adr-0084-task-028-search-open-defects.md) V1 to V4
  - [ADR 0084 packet 029](tasks/adr-0084-task-029-search-link-facts.md) V1 to V6
  - [ADR 0084 packet 026](tasks/adr-0084-task-026-index-singles-groups.md) V2 (the "You Are Enough" row only), V6, V7, V9 and V10. V6 and V10 failed on 2026-10-23 (run record, question 2 and walk log). Packet 035 V1 walks V6 again (section 7). The visual overhaul takes V10
  - [ADR 0084 packet 030](tasks/adr-0084-task-030-music-open-defects.md) V1 to V6
  - [ADR 0084 packet 031](tasks/adr-0084-task-031-release-row-actions-and-selection-drag.md) V1 to V9
  - [ADR 0084 packet 032](tasks/adr-0084-task-032-singles-group-rows.md) V1 to V7
  - [ADR 0084 packet 033](tasks/adr-0084-task-033-download-state-icons.md) V1 to V8
  - [ADR 0084 packet 034](tasks/adr-0084-task-034-store-feed-links-on-download.md) V1 to V4
- Packet 024 adds migration 20, and packet 012 adds migration 21. Before the first start of the new build, make the backup of the Rollback section of packet 012.
- Part B holds the deferred visual requirements of the overhaul plan that present screens can show.

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

## 3. Metadata Migration Repair Report And Readiness — ADR 0075 Task 012

Open and paused - implementation, technical review, and mechanical checks are complete on 2026-09-20. Operator inspection is pending.

- Owner: [packet 012](tasks/adr-0075-task-012-provider-snapshot-migration.md#operator-visual-check).
- Check: separate version-11 repair and version-12 readiness reports, retained backup paths, recorded times, report copy, and normal/narrow presentation.
- Run the packet's temporary-fixture procedure only after mechanical review and the operator resumes visual checks.
- Preserve configuration, audio files, secret files, playlists, source records, and the selected event during the fixture check.
- Retain a failing fixture. Confirm successful preservation and fixture cleanup before closing this gate.
- This check has no runnable procedure and no owner (triage of 2026-10-20). The fixture checks a migration-11 ledger and migration-12 rows, and expects a separate version-12 candidate. Schema version 19 is current. The fixture needs revision for schema 19 before anyone can walk this check.

## 4. MusicBrainz URL Relations By Type — ADR 0080 Task 002

Open and paused - implementation and mechanical checks are complete on 2026-09-29. Operator inspection is pending.

- Owner: [task 002](tasks/adr-0080-task-002-musicbrainz-url-relations-by-type.md#operator-visual-check).
- Scheduling: run the procedure only after the operator resumes visual checks. It uses an isolated database copy and never writes to the Library. It needs network access to `musicbrainz.org`.
- V3: after a write of the homepage and the license on a test copy, an external tag reader shows plain URLs in `WOAR` and `WCOP`.
- V1, V2 and V4 moved to the [overhaul plan](plans/design-and-cleanup-overhaul-plan.md#visual-requirements-moved-from-pending-checks---2026-10-07) on 2026-10-07, under Phase 4: Inspect sources.

## 5. Forward Navigation — ADR 0046 Task 015

Open for macOS only - the operator passed V1, V3 and the Linux part of V2 on 2026-10-07, in Dark and Light, at UI scale L and at narrow width.

- Owner: [task 015](tasks/adr-0046-task-015-forward-navigation.md#operator-visual-check).
- Scheduling: it needs a macOS desktop session.
- V2, macOS part: `cmd-[` and `cmd-]` outside a text box do the same as Back and Forward. Indent and outdent still work in a macOS text box.

## 6. Final Walk Of The Run Of 2026-10-23

Open - the run closed on 2026-10-24 before the end of its walk. Each check below stays open. The fix packets of the walk come first. Walk the checks of a changed screen after its fix packet.

- Owner: the [run record](reviews/orchestrated-run-2026-10-23.md#final-walk).
- The walk starts with ADR 0084 packet 026 (section 2). The operator passed its V1 to V5 and V8 on 2026-10-23, apart from the "You Are Enough" row of V2. The operator passed packet 020 V1 to V6 on 2026-10-23.
- The packets of the run. Each item names its packet and its open checks:
  - [ADR 0086 packet 001](tasks/adr-0086-task-001-parallel-new-releases-detail-reads.md) V1, V2 and V4
  - [ADR 0086 packet 002](tasks/adr-0086-task-002-new-releases-window-groups.md) V4, V6 and V8. Packet 004 replaced V5 and V7
  - [ADR 0086 packet 003](tasks/adr-0086-task-003-pager-closes-windows.md) V1 and V4 to V10
  - [ADR 0086 packet 004](tasks/adr-0086-task-004-window-group-page.md) V4, V6, V9 and V10
  - [ADR 0086 packet 005](tasks/adr-0086-task-005-saved-search-window-groups.md) V1 to V7
  - [ADR 0087 packet 002](tasks/adr-0087-task-002-library-fields-and-index-words.md) V8 and V9
  - [ADR 0087 packet 004](tasks/adr-0087-task-004-field-list.md) V1 and V9. V1 failed, and the visual overhaul takes it. Packet 008 V2 replaces V6 (section 7)
  - [ADR 0087 packet 003](tasks/adr-0087-task-003-live-results.md) V2, V8 and V9. Packet 007 V1 and V3 replace V3, and packet 008 V1 replaces V6 (section 7)
  - [ADR 0087 packet 005](tasks/adr-0087-task-005-field-list-values.md) V3, V6 and V7. V7 failed

## 7. Final Walk Of The Run Of 2026-10-24

Open - the run adds each packet when its mechanical checks are Green.

- Owner: the [run record](reviews/orchestrated-run-2026-10-24.md#final-walk).
- The packets of the run. Each item names its packet and its open checks:
  - [ADR 0084 packet 035](tasks/adr-0084-task-035-download-stores-feed-links.md) V1 to V3. V1 walks packet 026 V6 again
  - [ADR 0087 packet 008](tasks/adr-0087-task-008-field-list-stays-open.md) V1 to V5. V1 and V2 walk packet 003 V6 and packet 004 V6 again
  - [ADR 0087 packet 007](tasks/adr-0087-task-007-live-run-index-rows.md) V1 to V4. V1 and V3 walk packet 003 V3 again. The cause is not proved: if V1 fails, write down the text of the note under the rows
  - [ADR 0084 packet 036](tasks/adr-0084-task-036-group-row-at-artist-name.md) V1 to V3. V1 walks packet 026 V1 again
  - [ADR 0084 packet 037](tasks/adr-0084-task-037-releases-everything-frame-time.md) V1 to V7, with `./target/release/v4vmm`. V7 is the Artists list. A row that does not change after a download or a removal is a defect of the cache
