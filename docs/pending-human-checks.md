# Pending Human Checks

## Purpose

Some acceptance criteria need a person. An agent must not run this app, because
GPUI does not start an X11 client in an agent session.

This file holds the checks that are open today. It is not a history. When a
check passes, do all three in the same change:

1. Record it in the `Status:` line of the owning packet in `docs/tasks/`.
2. Record it in the row in `docs/plans/broadcast-chain-delivery-order.md`.
3. Remove the section from this file.

When no check is open, this file keeps the function and the method sections
only. A closed check leaves no entry here.

## Before You Start

The configuration file is at `~/.config/v4vmm/config.toml`. Some checks change
it. Make a copy first, and put it back when you finish.

```bash
cp ~/.config/v4vmm/config.toml ~/.config/v4vmm/config.toml.bak
```

## Open: Library Readiness Report

Owner: ADR 0059 task 012. Needs a screen only.

1. Start the app with `cargo run --release`.
2. Open `Show`. Find the readiness count in the `Source` section.
3. Run `v4vmm broadcast readiness --json` in a second terminal. The two counts
   must agree.
4. Select the control next to the count.

Look for:

- The count is easy to read where it is.
- The control opens `Music` with the not-ready rows filtered.
- `Show` does not show a second list of its own.

## Open: Remote Host Reachability

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

Set `selected_host` to `"Local"` again. The section recovers.

To examine the timeout path, use `destination = "192.0.2.1"` as an alternative.
No router accepts that address, so the check waits for the five second connect
timeout.

## Open: Stream Encoder States

Owner: ADR 0059 task 015. The first half needs no encoder.

Remove the `[broadcast.encoder]` group from the configuration file and start the
app again.

Look for:

- The `Stream` section reports that the encoder is not installed.
- The section offers no actions and shows no error string.

The second half needs `butt` on this machine.

```toml
[broadcast.encoder]
binary_path = "butt"
default_server_name = "default"
```

Start `butt`. Connect and disconnect from the app.

Look for:

- The connection state and the recording state read as two different facts.
- The listener count changes with the encoder state.

## Open: Show Card Grid

Owner: ADR 0063 task 002. Needs a screen only.

1. Start the app with `cargo run --release`.
2. Open `Show`.
3. Resize the window from the operator's normal width to a narrow width.

Look for:

- Every card is visible at once, with no scrolling, at the window size the
  operator uses.
- Cards fill the width. The middle of the window carries content.
- Making the window narrow reduces the column count, and the cards stay
  readable.
- Every card is the same height, in every state.
- A card state is readable without color, from its label.

## Open: Show Detail Panel

Owner: ADR 0063 task 003. Needs a screen only.

1. Start the app with `cargo run --release`.
2. Open `Show`.
3. Select each card, close the panel, reopen it, and return detail to the
   cuelist.
4. Open publisher logs from the `Live Metadata` detail.

Look for:

- The panel opens and closes, and the card grid remains usable in both states.
- Selecting a card shows its detail, and the cuelist returns when detail closes.
- Publisher logs read correctly at panel width.
- Transport controls remain reachable while the panel is closed.
- Service state changes do not move layout while detail is open.

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
