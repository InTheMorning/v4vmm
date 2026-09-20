# Pending Human Checks

## Scheduling

The operator paused visual checks on 2026-09-19. Prioritise the
[metadata contract refactor](plans/adr-0075-metadata-contract-phase-plan.md)
before requesting more visual checks. The first four groups retain five open visual packets.
Their acceptance and configuration gates remain open. Group 5 records the remaining metadata document review.

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

Open - corrected documents recorded on 2026-09-19. This group requests no visual check or app launch.

- Owners: document packets 002–008 in the [phase plan](plans/adr-0075-metadata-contract-phase-plan.md).
- Accepted policy: Decisions D–G. Do not request acceptance of those decisions again.
- Remaining review: within-provider selection, stale-value handling, inherited description/publisher placement, and other marked field proposals.
- Check source claims against the named functions. Check proposed rules against the accepted ADR and constructed examples.
- The [field inventory](schema/adr-0075-metadata-field-inventory.md) assigns additional rules to packets 031 and 034.
  Those rules are not yet written. Packet 035 must define comparison details before implementation.
- Keep the code dispatch hold. Document correction and policy discussion do not prove full document acceptance.
- This document review creates no fixture and requires no cleanup. The existing evidence fixture remains unmodified.
