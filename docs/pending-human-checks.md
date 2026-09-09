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

## Already Met, Do Not Repeat

An operator ran these on 2026-09-08 and 2026-09-09. They are recorded so nobody
walks them again.

| What | Owner | How it was met |
|---|---|---|
| Cards fill the width, all visible, no scrolling | ADR 0063 task 002 | operator screenshots |
| Every card holds one height in every state | ADR 0063 task 002 | operator screenshots |
| A card state reads without color, from its label | ADR 0063 task 002 | operator screenshots |
| The panel opens, closes, and shows a selected card | ADR 0063 task 003 | operator screenshots |
| The transport stays reachable with the panel closed | ADR 0063 task 003 | operator screenshots |
| The failed state names its reason, and `Reset` is obvious | ADR 0059 task 009 | operator run, `start-limit-hit` |
| The readiness count agrees with `broadcast readiness --json` | ADR 0059 task 012 | operator run |
| The path repair converts a moved library | ADR 0064 task 001 | operator run, 54 rows |
| `Check all feeds` repairs route tags and reports counts | ADR 0065 task 002 | operator run |
| Show interaction after truncation and panel fixes | ADR 0063 tasks 002 and 003 | operator run, 2026-09-09 |
| Remote host reachability shows as its own state | ADR 0059 task 010 | operator run, `Broken` host |
| Stream encoder states render and commands work | ADR 0059 task 015 | operator run, no encoder and `butt` |
| Event target attachment updates and detaches `default` | ADR 0059 task 014 | operator run, 2026-09-09 |

Two defects came out of those runs and are fixed: text rendered as `...`, and
the collapsed panel overlaid the last card.

Three defects are recorded, not fixed: the `Check all feeds` result has no room
to read, service actions can briefly repaint the previous service state after
the immediate transition state, and stream command buttons briefly disappear
during connect or disconnect. See items A7 through A9 in
`docs/plans/hig-product-polish-backlog.md`.

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
