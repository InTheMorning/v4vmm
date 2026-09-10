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

## 1. Compact Event Controls And Item Badges — ADR 0059 Task 017

Status: Open - 2026-09-09. Awaiting implementation and fixture extensions;
the action-feedback prerequisite is complete. Design approval is not visual
acceptance. Task 016's earlier recovery gate remains closed for its shipped scope.

Owner: [task 017](tasks/adr-0059-task-017-compact-event-controls-and-badges.md).
The [operator walkthrough](tasks/adr-0059-task-017-compact-event-controls-and-badges.md#operator-visual-check)
contains numbered terminal commands, expected/wrong outcomes, and marked-fixture
cleanup. Run it after the implementation and fixture work specified in that
packet exist; its new fixture commands are not available in the current script.

- Situational ADR 0059 manual check: saved older-event selection remains
  operable; only the configured target satisfies attachment; Attach/Detach
  preserve unrelated targets and old event/token files; passive rechecks keep
  confirmation while pending and expose failure when it answers.
- Situational ADR 0063 manual check: three compact labeled item badges remain
  readable together, the card reflects them, and Event diagnostics uses the
  shared bottom pane with correct source titles, resizing, and selectable Copy.

Prerequisites: Linux desktop session, Python 3.11+, two terminals, free port
17863, and the task 017 build and extended fixture. Publisher, service states,
and journal output are simulated; no broadcast hardware or real unit changes
are needed. Cleanup closes that app, stops the fixture relay, and removes only
its marked directory. Preserve any earlier literal-placeholder directory that
may still contain a token referenced by a real registry.

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
