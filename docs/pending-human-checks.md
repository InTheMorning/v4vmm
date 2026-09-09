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

Two defects came out of those runs and are fixed: text rendered as `...`, and
the collapsed panel overlaid the last card. The checks below re-test only what
those fixes changed.

One defect is recorded, not fixed: the `Check all feeds` result has no room to
read. See item A7 in `docs/plans/hig-product-polish-backlog.md`.

## Order

Do the checks in the order below.

Check 1 needs a running app and nothing else. Checks 2 and 3 change the
configuration file, so they are together at the end and cost one edit.

## Before You Start

The configuration file is at `~/.config/v4vmm/config.toml`. Checks 2 and 3
change it. Make a copy first, and put it back when you finish.

```bash
cp ~/.config/v4vmm/config.toml ~/.config/v4vmm/config.toml.bak
```

## Open 1: Show Interaction After The Fixes

Owner: ADR 0063 tasks 002 and 003, and the fixes of 2026-09-08. Needs a screen
only.

Everything here is new behaviour or a repaired defect. Nothing in it was met by
the earlier screenshots.

1. Start the app with `cargo run --release`.
2. Open `Show`. Read the two summary lines on each card.
3. Make the window narrow, then wide again.
4. Close the panel with the control at its edge, and look at the last card.
5. Select a card. Select the same card again.
6. Select `Live Metadata`, then press `Logs`. Press `Logs` again.
7. Press `Start` on a stopped service, and watch the row.

Look for:

- Every card line holds words. No line reads `...` with no text.
- A narrow window reduces the column count, and a wide window restores it.
- With the panel closed, the last card is whole. The panel rail does not sit on
  top of it.
- A second select on the open card closes the panel.
- `Logs` shows the journal, and a second `Logs` press hides it. `Logs` on the
  other service switches to it, and does not close.
- The row answers `Start` at once with a `Working` state, before the service
  manager replies.
- A command that fails prints its reason under the show title. It does not fail
  in silence.

Wrong:

- Any line reads `...` alone.
- The panel rail covers part of a card.
- `Logs` closes the panel instead of showing the journal.
- A press does nothing visible for seconds.

## Open 2: Remote Host Reachability

Owner: ADR 0059 task 010. Needs a configuration change only, not a second
machine.

```toml
[broadcast]
selected_host = "Broken"

[[broadcast.hosts]]
name = "Local"
transport = "local"
instance_name = "mixxx"

[[broadcast.hosts]]
name = "Broken"
transport = "ssh"
destination = "nosuchhost.invalid"
instance_name = "mixxx"
```

A name that does not resolve fails in under one second. Start the app again.

Look for:

- The `Source` section names the host.
- The state is `Not reachable`. It is not `Failed`, and it is not a raw SSH
  error string.
- The service actions are unavailable, because the host owns the units.

Set `selected_host` to `"Local"` again. The section recovers.

To examine the timeout path, use `destination = "192.0.2.1"` as an alternative.
No router accepts that address, so the check waits for the five second connect
timeout.

## Open 3: Stream Encoder States

Owner: ADR 0059 task 015. The first half needs no encoder.

Remove the `[broadcast.encoder]` group from the configuration file and start the
app again.

Look for:

- The `Stream` section reports that the encoder is not installed.
- The section offers no actions and shows no error string.

The second half needs `butt` on this machine. Leave `default_server_name` out,
so the app connects with a bare `-s`, which is what an operator runs by hand.

```toml
[broadcast.encoder]
binary_path = "butt"
```

Start `butt`, and leave it disconnected. Press `Connect`, then `Disconnect`.

Look for:

- `Connect` is available while the encoder is disconnected.
- Pressing either control changes the section at once, before `butt` answers.
- The connection state and the recording state read as two different facts.
- A failed connect prints its reason under the show title.

**`Connect` did not work on 2026-09-08, and the reason was never visible.** The
message under the show title is new. Read it and record what it says, because
that text is the open question.

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
