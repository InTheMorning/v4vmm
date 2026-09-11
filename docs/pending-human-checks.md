# Pending Human Checks

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

## 2. Identity And Detail Parity — ADR 0037 Tasks 001 And 002

Open - reconciled 2026-09-10. Compare the same entity through local and Index
origins in Music. The separate Library/Discover screen requirement is retired
by ADRs 0047/0048/0060; the identity and hydration requirements survive.

- Owners: [task 001](tasks/adr-0037-task-001-feed-identity-action-parity.md)
  and [task 002](tasks/adr-0037-task-002-track-header-action-parity.md).
- Check: [Identity And Detail Parity](runbooks/inherited-ui-checks.md#identity-and-detail-parity--adr-0037-tasks-001-and-002).
- Record feed and track results separately in the
  [checklist](reviews/adr-0037-review-checklist.md). Both require Light/Dark
  and known populated identity facts. Empty fixtures do not close the gate.

## 3. Toolbar Search — ADR 0043 Task 004

Open - reconciled 2026-09-10. Toolbar readability, focus, and submission
survive. The trailing Now Playing frame, global scope controls, Search tab,
and Recent Feeds root are retired by ADRs 0046/0047/0048/0060/0062.

- Owner: [task 004](tasks/adr-0043-task-004-guards-and-visual-readiness.md).
- Check: [Search Toolbar](runbooks/inherited-ui-checks.md#search-toolbar--adr-0043-task-004).
- Verify normal/narrow widths in Light/Dark; record each in the
  [checklist](reviews/adr-0043-review-checklist.md).

## 4. Playlist Reordering — ADR 0044 Task 003

Open - reconciled 2026-09-10. Handle/menu/insertion and mounted-row update
requirements survive in Music. Inspector-owned Back to Playlist and
InspectorOrigin are retired by ADRs 0046/0047; use frame navigation.

- Owner: [task 003](tasks/adr-0044-task-003-playlist-reorder-guards-visual.md).
- Check: [Playlist Reordering](runbooks/inherited-ui-checks.md#playlist-reordering--adr-0044-task-003).
- Needs populated and unavailable rows in the disposable library. Check
  upward/downward moves, no-op drops, menus, and immediate updates in both
  themes. Record results in the [checklist](reviews/adr-0044-review-checklist.md).

## 5. Stored Metadata In Details — ADR 0054 Tasks 004 And 005

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

The five inherited groups above use the runbook's private database/audio copy and cleanup.
The numbers group checks; they do not change the approved delivery priority.

## 6. Background Tools — ADR 0066 Task 003

Open - 2026-09-10. Only refresh/playback keyboard rejection remains unverified:
the operator's window manager intercepted Super+R and Super+Alt+P. Mechanical
dispatch guards passed, but do not prove desktop key delivery. All other task 003
operator checks, including the search-error correction, final preservation
inspection and fixture cleanup, passed. Those checks need no repeat; the packet
records their [evidence](tasks/adr-0066-task-003-runtime-failure-and-shell-availability.md#operator-evidence--2026-09-10).
Task 002's accepted core checks remain closed.

- Owner: [task 003](tasks/adr-0066-task-003-runtime-failure-and-shell-availability.md).
- Check: [Background tools](runbooks/startup-recovery-check.md#task-003-background-tools).
- Needs a Linux desktop that forwards both shortcuts to the app, Python 3.11 or
  later and the debug binary. The fixture supplies unavailable-runtime mode;
  no real service is required.
- Verify refresh and playback shortcuts report unavailable background tools,
  leave no permanent loading state, and preserve navigation and repair access.
  Use a fresh isolated fixture and remove it after this remaining check.
- Record results in the packet, review checklist and delivery row. Task 004
  waits for this gate; inherited checks above are unchanged.

## Method: Reach A Publisher Service State

This section is not a check. It is the method that each publisher check needs.
It stays here when every check above is closed.

A drop-in file makes the unit go to a state. The cleanup section removes that
file.

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

```bash
rm -rf ~/.config/systemd/user/mixxx-now-playing.service.d \
       ~/.config/systemd/user/musicindex-live-publisher@mixxx.service.d
systemctl --user daemon-reload
systemctl --user reset-failed mixxx-now-playing.service musicindex-live-publisher@mixxx.service
cp ~/.config/v4vmm/config.toml.bak ~/.config/v4vmm/config.toml
```

## References

- `docs/adr/0061-executable-governance.md`, for the mechanical and visual rule
- `docs/plans/broadcast-chain-delivery-order.md`
